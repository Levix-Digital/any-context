use any_context_core_rs::ingestion::{NativeSyncOrchestrator, SyncOptions, WorkspaceScanner};
use any_context_core_rs::storage::{NativeConfigDb, NativeLanceStore, VectorRecord};
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
        embedding_model: Some("mock".to_string()),
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
        embedding_model: Some("mock".to_string()),
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
        embedding_model: Some("mock".to_string()),
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
        embedding_model: Some("mock".to_string()),
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
    assert_eq!(db.get_workspace_embedding_target(ws).unwrap(), Some(Some("mock/mock-embed@1536".to_string())));
    let chunks_before = lance.count_records(Some(ws), None).unwrap();

    let second = orchestrator.run(&mock_sync(None)).await.expect("sync 2");
    assert!(second.is_up_to_date);
    assert_eq!(second.embedding_reindex, None);

    // The workspace was indexed with another target (e.g. before `/model` changed the chat model).
    db.set_workspace_embedding_target(ws, Some("openai/text-embedding-3-small")).unwrap();

    let third = orchestrator.run(&mock_sync(None)).await.expect("sync 3");
    assert_eq!(third.indexed_files, 2, "every unchanged file must be re-embedded");
    let msg = third.embedding_reindex.expect("reindex reported");
    assert!(msg.contains("openai/text-embedding-3-small → mock/mock-embed@1536"), "{msg}");
    assert_eq!(db.get_workspace_embedding_target(ws).unwrap(), Some(Some("mock/mock-embed@1536".to_string())));
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
    assert_eq!(db.get_workspace_embedding_target(ws).unwrap(), Some(Some("mock/mock-embed@1536".to_string())));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Builds a token-shaped sample at run time (prefix + pseudo-random body), so no complete
/// credential literal is ever committed.
fn fake_secret(prefix: &str, len: usize, seed: u64) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let body: String = (0..len)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ALPHABET[((state >> 33) as usize) % ALPHABET.len()] as char
        })
        .collect();
    format!("{prefix}{body}")
}

/// Regression BUG-024: secret values in `.env` files and PEM private keys are never stored
/// in the index (chunk text, document summary or keywords), while the key names stay searchable.
#[tokio::test]
async fn test_secrets_are_never_indexed() {
    let (temp_dir, _db, lance, orchestrator, dirs) = embedding_sandbox("secrets_never_indexed", &["app"]);
    let ws = "EmbeddingTargetWs";

    let db_password = fake_secret("", 16, 1);
    let openai_key = fake_secret(&["sk", "-proj-"].concat(), 48, 2);
    let stripe_key = fake_secret(&["sk", "_live_"].concat(), 32, 3);
    let pem_body = fake_secret("MIIEvQIBADANBgkqhkiG9w0BAQEFAASC", 32, 4);
    std::fs::write(
        dirs[0].join(".env"),
        format!(
            "# Application settings\nAPP_NAME=billing\nDATABASE_URL=postgres://app:{db_password}@db.internal:5432/app\nOPENAI_API_KEY={openai_key}\nSTRIPE_KEY={stripe_key}\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dirs[0].join("server.key"),
        format!(
            "{}\n{pem_body}\n{pem_body}\n{}\n",
            ["-----BEGIN", " PRIVATE KEY-----"].concat(),
            ["-----END", " PRIVATE KEY-----"].concat()
        ),
    )
    .unwrap();

    let res = orchestrator.run(&mock_sync(None)).await.expect("sync");
    assert!(res.errors.is_empty(), "{:?}", res.errors);
    assert_eq!(res.secrets_redacted, 4, "password, two tokens and the private key block");

    let rows = lance
        .search_metadata(&format!("workspace = '{ws}'"), 10_000, Some(ws), None)
        .expect("read rows");
    assert!(rows.iter().any(|r| r.file_name == ".env"), "the .env file stays indexed");
    for row in &rows {
        let stored = format!(
            "{}\n{}\n{}\n{}",
            row.text,
            row.document_summary.clone().unwrap_or_default(),
            row.keywords.clone().unwrap_or_default(),
            row.header_path.clone().unwrap_or_default()
        );
        for secret in [&db_password, &openai_key, &stripe_key, &pem_body] {
            assert!(!stored.contains(secret.as_str()), "secret stored in a chunk of {}", row.file_name);
        }
    }
    let env_text: String = rows.iter().filter(|r| r.file_name == ".env").map(|r| r.text.as_str()).collect();
    assert!(env_text.contains("OPENAI_API_KEY"), "key names stay searchable");
    assert!(env_text.contains("APP_NAME=billing"), "values that are not secrets are kept");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// BUG-024 repair: chunks an earlier version stored with secrets are purged and their file is
/// reindexed redacted on the next sync, once per workspace.
#[tokio::test]
async fn test_secrets_indexed_before_the_fix_are_reindexed() {
    let (temp_dir, db, lance, orchestrator, dirs) = embedding_sandbox("secrets_repair", &["app"]);
    let ws = "EmbeddingTargetWs";
    let secret = fake_secret("", 24, 5);
    std::fs::write(dirs[0].join("settings.ini"), format!("[db]\nhost=db.internal\npassword={secret}\n")).unwrap();

    let first = orchestrator.run(&mock_sync(None)).await.expect("sync 1");
    assert_eq!(first.secrets_repaired_files, 0, "a new workspace has nothing to repair");
    assert_eq!(db.get_setting(&format!("secret_redaction_version:{ws}")).unwrap().as_deref(), Some("1"));

    // What an earlier version stored: the same file's chunk with the secret in clear, and a
    // workspace that was never scanned.
    let rows = lance.search_metadata(&format!("workspace = '{ws}'"), 1000, Some(ws), None).unwrap();
    let ini = rows.iter().find(|r| r.file_name == "settings.ini").expect("ini indexed").clone();
    lance
        .upsert_records(
            vec![VectorRecord {
                id: format!("{}_legacy", ini.id),
                vector: vec![0.1; 8],
                text: format!("[db]\nhost=db.internal\npassword={secret}\n"),
                file_name: ini.file_name.clone(),
                file_path: ini.file_path.clone(),
                workspace: ws.to_string(),
                last_modified: None,
                content_type: ini.content_type.clone(),
                document_summary: None,
                keywords: None,
                content_hash: ini.content_hash.clone(),
                header_path: None,
                start_line: None,
                end_line: None,
                embedding_target: None,
            }],
            None,
            None,
        )
        .unwrap();
    db.set_setting(&format!("secret_redaction_version:{ws}"), "0").unwrap();

    let second = orchestrator.run(&mock_sync(None)).await.expect("sync 2");
    assert_eq!(second.secrets_repaired_files, 1);
    assert_eq!(second.indexed_files, 1, "only the affected file is reindexed");
    let rows = lance.search_metadata(&format!("workspace = '{ws}'"), 1000, Some(ws), None).unwrap();
    assert!(rows.iter().all(|r| !r.text.contains(&secret)), "the legacy chunk is gone");
    assert!(rows.iter().any(|r| r.file_name == "settings.ini" && r.text.contains("password=[SECRET REDACTED]")));
    assert!(rows.iter().any(|r| r.file_name == "notes.md"), "other files keep their chunks");

    let third = orchestrator.run(&mock_sync(None)).await.expect("sync 3");
    assert_eq!(third.secrets_repaired_files, 0, "the repair runs once");
    assert!(third.is_up_to_date);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Regression BUG-022: a sync scoped to one folder (what adding a folder starts) never takes
/// the other folders' files for deleted ones.
#[tokio::test]
async fn test_scoped_sync_keeps_the_other_folders() {
    let (temp_dir, db, lance, orchestrator, dirs) = embedding_sandbox("scoped_keeps_others", &["alpha", "beta"]);
    let ws = "EmbeddingTargetWs";

    orchestrator.run(&mock_sync(None)).await.expect("full sync");
    let chunks = lance.count_records(Some(ws), None).unwrap();
    assert_eq!(db.get_workspace_files_stat_cache(ws).unwrap().len(), 4);

    let scoped = orchestrator.run(&mock_sync(Some(dirs[0].to_string_lossy().to_string()))).await.expect("scoped sync");
    assert_eq!(scoped.deleted_files, 0, "beta's files are not deleted");
    assert!(scoped.is_up_to_date);
    assert_eq!(db.get_workspace_files_stat_cache(ws).unwrap().len(), 4);
    assert_eq!(lance.count_records(Some(ws), None).unwrap(), chunks);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Regression BUG-025: an unreachable folder goes offline and keeps its index, and it is
/// reconciled by the normal diff when it comes back.
#[tokio::test]
async fn test_unreachable_folder_keeps_its_index() {
    let (temp_dir, db, lance, orchestrator, dirs) = embedding_sandbox("unreachable_keeps_index", &["alpha", "beta"]);
    let ws = "EmbeddingTargetWs";

    orchestrator.run(&mock_sync(None)).await.expect("full sync");
    let chunks = lance.count_records(Some(ws), None).unwrap();

    let parked = temp_dir.join("beta-unplugged");
    std::fs::rename(&dirs[1], &parked).unwrap();
    let offline = orchestrator.run(&mock_sync(None)).await.expect("sync with beta unreachable");
    assert_eq!(offline.deleted_files, 0, "beta's files are not deleted");
    assert_eq!(lance.count_records(Some(ws), None).unwrap(), chunks, "beta keeps its chunks");
    assert_eq!(db.get_workspace_files_stat_cache(ws).unwrap().len(), 4);
    assert_eq!(offline.offline_folders.len(), 1, "{:?}", offline.offline_folders);
    assert!(offline.offline_folders[0].contains("beta"), "{:?}", offline.offline_folders);

    std::fs::rename(&parked, &dirs[1]).unwrap();
    std::fs::write(dirs[1].join("added.md"), "# Added\nWritten while the drive was unplugged.").unwrap();
    let back = orchestrator.run(&mock_sync(None)).await.expect("sync with beta back");
    assert_eq!(back.indexed_files, 1, "only the new file is indexed");
    assert_eq!(back.deleted_files, 0);
    assert!(back.offline_folders.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Regression BUG-025: a subfolder the walk cannot read keeps its index (Unix permissions;
/// skipped as root, where they are not enforced).
#[cfg(unix)]
#[tokio::test]
async fn test_unreadable_subfolder_keeps_its_index() {
    use std::os::unix::fs::PermissionsExt;
    let (temp_dir, db, _lance, orchestrator, dirs) = embedding_sandbox("unreadable_subfolder", &["alpha"]);
    let ws = "EmbeddingTargetWs";
    let private = dirs[0].join("private");
    std::fs::create_dir_all(&private).unwrap();
    std::fs::write(private.join("plan.md"), "# Plan\nThe private plan of the alpha team.").unwrap();
    orchestrator.run(&mock_sync(None)).await.expect("full sync");
    assert_eq!(db.get_workspace_files_stat_cache(ws).unwrap().len(), 3);

    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read_dir(&private).is_ok() {
        // Running as root: permissions are not enforced, nothing to check.
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o755)).unwrap();
        let _ = std::fs::remove_dir_all(&temp_dir);
        return;
    }
    let res = orchestrator.run(&mock_sync(None)).await.expect("sync with an unreadable subfolder");
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(res.deleted_files, 0, "the unreadable subfolder's file is not deleted");
    assert_eq!(db.get_workspace_files_stat_cache(ws).unwrap().len(), 3);
    assert!(res.offline_folders.iter().any(|p| p.contains("private")), "{:?}", res.offline_folders);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Removing a folder from the workspace still purges its files on the next full sync.
#[tokio::test]
async fn test_removed_folder_is_purged_on_full_sync() {
    let (temp_dir, db, _lance, orchestrator, dirs) = embedding_sandbox("removed_folder_purged", &["alpha", "beta"]);
    let ws = "EmbeddingTargetWs";
    orchestrator.run(&mock_sync(None)).await.expect("full sync");

    assert!(db.remove_workspace_folder(ws, &dirs[1].to_string_lossy()).unwrap());
    let res = orchestrator.run(&mock_sync(None)).await.expect("sync after removal");
    assert_eq!(res.deleted_files, 2, "beta's two files are purged");
    assert_eq!(db.get_workspace_files_stat_cache(ws).unwrap().len(), 2);
    assert!(res.offline_folders.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Regression BUG-026: a missing folder is never redirected to another folder with the same
/// name, and its saved path is never rewritten.
#[tokio::test]
async fn test_missing_folder_is_never_redirected() {
    let temp_dir = std::env::temp_dir().join(format!("actx_never_redirected_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let db_dir = temp_dir.join("db");
    std::fs::create_dir_all(&db_dir).unwrap();
    let db = Arc::new(NativeConfigDb::open(db_dir.join("settings.db")).expect("open db"));
    let lance = Arc::new(NativeLanceStore::open(db_dir.join("lancedb")).expect("open lance"));
    let ws = "EmbeddingTargetWs";
    db.create_workspace(ws, None).unwrap();
    let chosen = temp_dir.join("projX").join("docs");
    std::fs::create_dir_all(&chosen).unwrap();
    std::fs::write(chosen.join("spec.md"), "# Spec\nThe chosen folder.").unwrap();
    let chosen_str = chosen.to_string_lossy().to_string();
    db.add_workspace_folder(ws, &chosen_str).unwrap();
    let orchestrator = NativeSyncOrchestrator::new(lance.clone(), db.clone());
    orchestrator.run(&mock_sync(None)).await.expect("full sync");

    std::fs::remove_dir_all(temp_dir.join("projX")).unwrap();
    let unrelated = temp_dir.join("docs");
    std::fs::create_dir_all(&unrelated).unwrap();
    std::fs::write(unrelated.join("other.md"), "# Other\nA folder the user never chose.").unwrap();

    let res = orchestrator.run(&mock_sync(None)).await.expect("sync with the folder missing");
    let saved = db.get_workspace_folders(ws).unwrap();
    assert_eq!(saved.len(), 1);
    assert!(saved[0].contains("projX"), "the saved path is unchanged: {saved:?}");
    assert_eq!(res.indexed_files, 0, "the unrelated folder is not indexed");
    let rows = lance.search_metadata(&format!("workspace = '{ws}'"), 100, Some(ws), None).unwrap();
    assert!(rows.iter().all(|r| r.file_name != "other.md"));
    assert!(res.offline_folders.iter().any(|p| p.contains("projX")), "{:?}", res.offline_folders);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Regression BUG-027: a workspace without sources indexes nothing (the test process runs in
/// the crate directory, which is full of files) and says so.
#[tokio::test]
async fn test_workspace_without_sources_indexes_nothing() {
    let temp_dir = std::env::temp_dir().join(format!("actx_no_sources_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let db_dir = temp_dir.join("db");
    std::fs::create_dir_all(&db_dir).unwrap();
    let db = Arc::new(NativeConfigDb::open(db_dir.join("settings.db")).expect("open db"));
    let lance = Arc::new(NativeLanceStore::open(db_dir.join("lancedb")).expect("open lance"));
    db.create_workspace("EmbeddingTargetWs", None).unwrap();
    let orchestrator = NativeSyncOrchestrator::new(lance.clone(), db.clone());

    let res = orchestrator.run(&mock_sync(None)).await.expect("sync");
    assert_eq!(res.indexed_files, 0);
    assert_eq!(lance.count_records(Some("EmbeddingTargetWs"), None).unwrap(), 0);
    assert!(res.no_sources);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Files whose name is in the shared set look like online-only cloud placeholders.
fn cloud_probe(names: &Arc<std::sync::Mutex<Vec<String>>>) -> WorkspaceScanner {
    let names = names.clone();
    WorkspaceScanner::new().with_online_only_probe(move |path, _meta| {
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        names.lock().unwrap().contains(&name)
    })
}

/// Regression BUG-028: an online-only cloud file is never read (reading it would download
/// it); it is counted, and the files on disk are indexed normally.
#[tokio::test]
async fn test_online_only_files_are_never_read() {
    let (temp_dir, db, lance, orchestrator, dirs) = embedding_sandbox("online_only_never_read", &["onedrive"]);
    let ws = "EmbeddingTargetWs";
    std::fs::write(dirs[0].join("report.md"), "# Report\nThis text only exists in the cloud.").unwrap();
    let cloud = Arc::new(std::sync::Mutex::new(vec!["report.md".to_string()]));
    let orchestrator = orchestrator.with_scanner(cloud_probe(&cloud));

    let res = orchestrator.run(&mock_sync(None)).await.expect("sync");
    assert_eq!(res.indexed_files, 2, "only the files on disk are indexed");
    assert_eq!(res.online_only_files.len(), 1, "{:?}", res.online_only_files);
    assert!(res.online_only_files[0].ends_with("report.md"), "{:?}", res.online_only_files);
    let rows = lance.search_metadata(&format!("workspace = '{ws}'"), 1000, Some(ws), None).unwrap();
    assert!(rows.iter().all(|r| r.file_name != "report.md"), "the placeholder was not read");
    assert!(!db.get_workspace_files_stat_cache(ws).unwrap().keys().any(|k| k.ends_with("report.md")));

    // Marked "Always keep on this device": indexed on the next sync.
    cloud.lock().unwrap().clear();
    let local = orchestrator.run(&mock_sync(None)).await.expect("sync after download");
    assert_eq!(local.indexed_files, 1);
    assert!(local.online_only_files.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Regression BUG-028: a file the cloud client frees back to online-only keeps its index.
#[tokio::test]
async fn test_file_freed_to_the_cloud_keeps_its_index() {
    let (temp_dir, db, lance, orchestrator, _dirs) = embedding_sandbox("freed_keeps_index", &["onedrive"]);
    let ws = "EmbeddingTargetWs";
    let cloud = Arc::new(std::sync::Mutex::new(Vec::new()));
    let orchestrator = orchestrator.with_scanner(cloud_probe(&cloud));

    orchestrator.run(&mock_sync(None)).await.expect("sync");
    let chunks = lance.count_records(Some(ws), None).unwrap();

    cloud.lock().unwrap().push("guide.md".to_string());
    let freed = orchestrator.run(&mock_sync(None)).await.expect("sync after free up space");
    assert_eq!(freed.deleted_files, 0, "the freed file is not deleted");
    assert_eq!(freed.online_only_files.len(), 1);
    assert_eq!(lance.count_records(Some(ws), None).unwrap(), chunks);
    assert_eq!(db.get_workspace_files_stat_cache(ws).unwrap().len(), 2);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// SC-003: changing the chat model never changes the embedding model, so the next sync
/// re-embeds nothing.
#[tokio::test]
async fn test_chat_model_change_reembeds_nothing() {
    use any_context_core_rs::ingestion::{active_embedding_model, EMBEDDING_MODEL_SETTING};
    let (temp_dir, db, lance, orchestrator, _) = embedding_sandbox("emb_chat_change", &["docs"]);
    let ws = "EmbeddingTargetWs";

    db.set_setting(EMBEDDING_MODEL_SETTING, "bge-m3").unwrap();
    let first = orchestrator.run(&mock_sync(None)).await.expect("sync 1");
    assert_eq!(first.indexed_files, 2);
    let chunks = lance.count_records(Some(ws), None).unwrap();

    for chat in ["gpt-4o-mini", "gemini-2.5-flash", "llama3.2"] {
        db.set_setting("active_model", chat).unwrap();
        let (spec, _) = active_embedding_model(&db, None).expect("model");
        assert_eq!(spec.id, "bge-m3", "the chat model `{chat}` changed the embedding model");
        let res = orchestrator.run(&mock_sync(None)).await.expect("sync");
        assert!(res.is_up_to_date, "chat model `{chat}` triggered a reindex");
        assert_eq!(res.indexed_files, 0);
        assert_eq!(res.embedding_reindex, None);
    }
    assert_eq!(lance.count_records(Some(ws), None).unwrap(), chunks);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// SC-004: when the embedding model cannot run, the sync fails before indexing, writes no
/// vector from any other model and keeps the existing index.
#[tokio::test]
async fn test_unavailable_embedding_model_writes_nothing() {
    use any_context_core_rs::ingestion::EMBEDDING_MODEL_SETTING;
    if std::env::var("OPENAI_API_KEY").map(|k| !k.trim().is_empty()).unwrap_or(false) {
        eprintln!("skipped: OPENAI_API_KEY is set, the model would be available");
        return;
    }
    let (temp_dir, db, lance, orchestrator, dirs) = embedding_sandbox("emb_unavailable", &["docs"]);
    let ws = "EmbeddingTargetWs";

    orchestrator.run(&mock_sync(None)).await.expect("sync 1");
    let chunks = lance.count_records(Some(ws), None).unwrap();
    assert!(chunks > 0);

    // A cloud model without its key: unavailable.
    db.set_setting(EMBEDDING_MODEL_SETTING, "text-embedding-3-small").unwrap();
    std::fs::write(dirs[0].join("new.md"), "# New\nA file added after the switch.").unwrap();
    let options = SyncOptions { embedding_model: None, ..mock_sync(None) };
    let err = orchestrator.run(&options).await.expect_err("the sync must fail");
    assert!(err.contains("text-embedding-3-small") && err.contains("API key"), "{err}");

    assert_eq!(lance.count_records(Some(ws), None).unwrap(), chunks, "no vector was written or purged");
    assert_eq!(db.get_workspace_embedding_target(ws).unwrap(), Some(Some("mock/mock-embed@1536".to_string())));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
