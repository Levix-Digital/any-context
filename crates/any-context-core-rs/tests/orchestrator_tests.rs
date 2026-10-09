use any_context_core_rs::ingestion::{NativeSyncOrchestrator, SyncOptions};
use any_context_core_rs::storage::{NativeConfigDb, NativeLanceStore};
use std::sync::Arc;

#[tokio::test]
async fn test_orchestrator_full_lifecycle() {
    let pid = std::process::id();
    let temp_dir = std::env::temp_dir().join(format!("actx_orch_test_{}", pid));
    let db_dir = temp_dir.join("db");
    let docs_dir = temp_dir.join("docs");
    let _ = std::fs::create_dir_all(&db_dir);
    let _ = std::fs::create_dir_all(&docs_dir);

    // 1. Create SQLite DB and LanceStore in sandbox
    let db_path = db_dir.join("settings.db");
    let lance_path = db_dir.join("lancedb");
    let db = Arc::new(NativeConfigDb::open(&db_path).expect("open db"));
    let lance = Arc::new(NativeLanceStore::open(&lance_path).expect("open lance"));

    let ws = "TestSyncWs";
    db.create_workspace(ws, Some("Testing Sync")).expect("create ws");
    db.add_workspace_folder(ws, &docs_dir.to_string_lossy()).expect("add folder");

    let orchestrator = NativeSyncOrchestrator::new(lance.clone(), db.clone());

    // 2. Populate docs_dir with sample documents
    let file1 = docs_dir.join("readme.md");
    let file2 = docs_dir.join("main.py");
    let file3 = docs_dir.join("config.json");

    std::fs::write(&file1, "# Readme\nThis is a sample project for testing native sync.").unwrap();
    std::fs::write(&file2, "def calculate():\n    return 42\n").unwrap();
    std::fs::write(&file3, "{\"env\": \"production\", \"port\": 8080}").unwrap();

    // 3. First Sync: Initial Indexing
    let opts = SyncOptions {
        workspace: ws.to_string(),
        force: false,
        target_folder: None,
        verbose: true,
        model: Some("mock".to_string()),
    };

    let res1 = orchestrator.run(&opts).await.expect("sync run 1");
    assert!(!res1.is_up_to_date, "First sync should not be up to date");
    assert_eq!(res1.indexed_files, 3, "All 3 files should be indexed");
    assert!(res1.chunks_created >= 3, "Chunks should be created");
    assert_eq!(res1.deleted_files, 0);

    // Verify LanceDB has chunks
    let lance_count = lance.count_records(Some(ws), None).expect("count records");
    assert!(lance_count >= 3, "LanceDB should contain at least 3 records");

    // Verify SQLite file cache
    let cache = db.get_workspace_files_stat_cache(ws).expect("get cache");
    assert_eq!(cache.len(), 3, "SQLite cache should track 3 files");

    // 4. Second Sync: 100% Up to Date (Sub-30ms)
    let res2 = orchestrator.run(&opts).await.expect("sync run 2");
    assert!(res2.is_up_to_date, "Second run with no changes must be up to date");
    assert_eq!(res2.indexed_files, 0, "No files should be re-indexed");
    assert_eq!(res2.chunks_created, 0);

    // 5. Modification: Modify readme.md
    // Sleep briefly to ensure mtime changes
    std::thread::sleep(std::time::Duration::from_millis(100));
    std::fs::write(&file1, "# Readme Updated\nAdditional content and information.").unwrap();

    let res3 = orchestrator.run(&opts).await.expect("sync run 3");
    assert!(!res3.is_up_to_date, "Should detect modified file");
    assert_eq!(res3.indexed_files, 1, "Only 1 modified file should be re-indexed");

    // 6. Deletion: Delete main.py
    std::fs::remove_file(&file2).unwrap();

    let res4 = orchestrator.run(&opts).await.expect("sync run 4");
    assert!(!res4.is_up_to_date, "Should detect deleted file");
    assert_eq!(res4.deleted_files, 1, "1 deleted file purged");

    let cache_after_del = db.get_workspace_files_stat_cache(ws).expect("get cache");
    assert_eq!(cache_after_del.len(), 2, "SQLite cache should now have 2 files");

    // 7. Force Full Reindex
    let force_opts = SyncOptions {
        workspace: ws.to_string(),
        force: true,
        target_folder: None,
        verbose: true,
        model: Some("mock".to_string()),
    };
    let res5 = orchestrator.run(&force_opts).await.expect("sync run 5 force");
    assert!(!res5.is_up_to_date);
    assert_eq!(res5.indexed_files, 2, "Both remaining files re-indexed under force");

    // 8. Cooperative Cancellation Test
    // Create multiple files so loop runs over multiple items
    for i in 1..=10 {
        let f = docs_dir.join(format!("cancel_test_{}.txt", i));
        std::fs::write(&f, format!("Content for cancellation test file {}", i)).unwrap();
    }
    let db_cancel = db.clone();
    let ws_cancel = ws.to_string();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(2));
        let _ = db_cancel.update_sync_status(&ws_cancel, false, None, 0, 10, "cancelled", None, None);
    });
    let res6 = orchestrator.run(&opts).await.expect("sync run 6");
    assert!(res6.cancelled || res6.indexed_files < 10, "Cancellation should take effect during multi-file loop");

    // 9. Cleanup sandbox
    let _ = std::fs::remove_dir_all(temp_dir);
}
