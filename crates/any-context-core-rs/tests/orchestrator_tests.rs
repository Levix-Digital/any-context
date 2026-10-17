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

#[tokio::test]
async fn test_orchestrator_quality_gate_and_metrics() {
    let pid = std::process::id();
    let temp_dir = std::env::temp_dir().join(format!("actx_orch_qg_test_{}", pid));
    let db_dir = temp_dir.join("db");
    let docs_dir = temp_dir.join("docs");
    let _ = std::fs::create_dir_all(&db_dir);
    let _ = std::fs::create_dir_all(&docs_dir);

    let db_path = db_dir.join("settings.db");
    let lance_path = db_dir.join("lancedb");
    let db = Arc::new(NativeConfigDb::open(&db_path).expect("open db"));
    let lance = Arc::new(NativeLanceStore::open(&lance_path).expect("open lance"));

    let ws = "TestQgWs";
    db.create_workspace(ws, Some("Testing Quality Gate")).expect("create ws");
    db.add_workspace_folder(ws, &docs_dir.to_string_lossy()).expect("add folder");

    let orchestrator = NativeSyncOrchestrator::new(lance.clone(), db.clone());

    // Valid code file
    let file_valid = docs_dir.join("valid.rs");
    std::fs::write(
        &file_valid,
        "pub fn calculate_metrics() -> i32 {\n    let val = 42;\n    println!(\"val={}\", val);\n    val\n}\n",
    )
    .unwrap();

    // Noise file with repeated dashes delimiter (low entropy)
    let file_noise = docs_dir.join("noise.txt");
    std::fs::write(
        &file_noise,
        "----------------------------------------------------------------------------------------------------\n",
    )
    .unwrap();

    let opts = SyncOptions {
        workspace: ws.to_string(),
        force: false,
        target_folder: None,
        verbose: true,
        model: Some("mock".to_string()),
    };

    let res = orchestrator.run(&opts).await.expect("sync run");
    assert_eq!(res.indexed_files, 2, "Both files were visited");
    assert!(
        res.chunks_dropped_quality >= 1,
        "Expected at least 1 noise chunk to be dropped by Quality Gate, got {}",
        res.chunks_dropped_quality
    );
    assert!(
        res.quality_pass_rate < 1.0,
        "Quality pass rate should reflect dropped noise chunks (pass rate: {})",
        res.quality_pass_rate
    );

    let _ = std::fs::remove_dir_all(temp_dir);
}

/// Sandbox with a workspace, one or more folders with sample files, and an orchestrator.
fn embedding_sandbox(
    name: &str,
    folders: &[&str],
) -> (std::path::PathBuf, Arc<NativeConfigDb>, Arc<NativeLanceStore>, NativeSyncOrchestrator, Vec<std::path::PathBuf>) {
    let temp_dir = std::env::temp_dir().join(format!("actx_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let db_dir = temp_dir.join("db");
    std::fs::create_dir_all(&db_dir).unwrap();
    let db = Arc::new(NativeConfigDb::open(db_dir.join("settings.db")).expect("open db"));
    let lance = Arc::new(NativeLanceStore::open(db_dir.join("lancedb")).expect("open lance"));
    let ws = "EmbeddingTargetWs";
    db.create_workspace(ws, None).expect("create ws");
    let mut dirs = Vec::new();
    for f in folders {
        let dir = temp_dir.join(f);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("notes.md"), format!("# Notes {f}\nThe {f} folder documents the deployment.")).unwrap();
        std::fs::write(dir.join("guide.md"), format!("# Guide {f}\nSteps to configure the {f} service.")).unwrap();
        db.add_workspace_folder(ws, &dir.to_string_lossy()).expect("add folder");
        dirs.push(dir);
    }
    let orchestrator = NativeSyncOrchestrator::new(lance.clone(), db.clone());
    (temp_dir, db, lance, orchestrator, dirs)
}

fn mock_sync(target_folder: Option<String>) -> SyncOptions {
    SyncOptions {
        workspace: "EmbeddingTargetWs".to_string(),
        force: false,
        target_folder,
        verbose: false,
        model: Some("mock".to_string()),
    }
}

/// Regression BUG-003: after the embedding target changes, the next sync re-embeds every file,
/// not only the changed ones, and records the new target.
#[tokio::test]
async fn test_embedding_target_change_reindexes_whole_workspace() {
    let (temp_dir, db, lance, orchestrator, _) = embedding_sandbox("emb_target_change", &["docs"]);
    let ws = "EmbeddingTargetWs";

    let first = orchestrator.run(&mock_sync(None)).await.expect("sync 1");
    assert_eq!(first.indexed_files, 2);
    assert_eq!(first.embedding_reindex, None, "a new workspace adopts the current target");
    assert_eq!(db.get_workspace_embedding_target(ws).unwrap(), Some(Some("mock/mock-embed".to_string())));
    let chunks_before = lance.count_records(Some(ws), None).unwrap();

    let second = orchestrator.run(&mock_sync(None)).await.expect("sync 2");
    assert!(second.is_up_to_date);
    assert_eq!(second.embedding_reindex, None);

    // The workspace was indexed with another target (e.g. before `/model` changed the chat model).
    db.set_workspace_embedding_target(ws, Some("openai/text-embedding-3-small")).unwrap();

    let third = orchestrator.run(&mock_sync(None)).await.expect("sync 3");
    assert_eq!(third.indexed_files, 2, "every unchanged file must be re-embedded");
    let msg = third.embedding_reindex.expect("reindex reported");
    assert!(msg.contains("openai/text-embedding-3-small → mock/mock-embed"), "{msg}");
    assert_eq!(db.get_workspace_embedding_target(ws).unwrap(), Some(Some("mock/mock-embed".to_string())));
    assert_eq!(lance.count_records(Some(ws), None).unwrap(), chunks_before, "old vectors are purged, not duplicated");

    let fourth = orchestrator.run(&mock_sync(None)).await.expect("sync 4");
    assert!(fourth.is_up_to_date, "a completed reindex is not repeated");
    assert_eq!(fourth.embedding_reindex, None);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// A folder-scoped sync cannot leave the other folders in the old vector space.
#[tokio::test]
async fn test_scoped_sync_with_target_change_reindexes_all_folders() {
    let (temp_dir, db, _lance, orchestrator, dirs) = embedding_sandbox("emb_target_scoped", &["alpha", "beta"]);
    let ws = "EmbeddingTargetWs";

    let first = orchestrator.run(&mock_sync(None)).await.expect("sync 1");
    assert_eq!(first.indexed_files, 4);

    db.set_workspace_embedding_target(ws, Some("openai/text-embedding-3-small")).unwrap();
    let scoped = mock_sync(Some(dirs[0].to_string_lossy().to_string()));
    let res = orchestrator.run(&scoped).await.expect("scoped sync");
    assert_eq!(res.indexed_files, 4, "both folders are re-embedded");
    assert!(res.embedding_reindex.is_some());
    assert_eq!(db.get_workspace_embedding_target(ws).unwrap(), Some(Some("mock/mock-embed".to_string())));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
