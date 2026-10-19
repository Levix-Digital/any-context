//! Upgrade from per-workspace embedding targets (ADR-0138) to the application-wide embedding
//! model with per-chunk model metadata (ADR-0149).
//!
//! Additive and idempotent: the global model is chosen once from the existing index, then the
//! stat cache rows and the LanceDB rows written before v0.46.0 get the key of the model that
//! built them. Rows of another model than the active one become stale and are re-embedded file
//! by file. `workspaces.embedding_target` and `global_knowledge_embedding_target` are left in
//! place, so an older version keeps reading them after a downgrade.

use std::collections::BTreeMap;

use crate::ingestion::embedding::{
    remember_embedding_config, set_active_embedding_config, stored_embedding_config, EmbeddingConfig,
};
use crate::storage::lancedb::NativeLanceStore;
use crate::storage::sqlite::NativeConfigDb;

/// Set to `1` once the LanceDB rows written before v0.46.0 carry their embedding model.
pub const CHUNK_MODELS_BACKFILLED_SETTING: &str = "embedding_model_chunks_backfilled";

/// Key of a target recorded before v0.46.0 (`provider/model`). Targets whose vectors match a
/// preset map to its key; the others map to `legacy:<target>`, a key no active model has, so
/// their files are stale: `gemini/text-embedding-004` (shut down), `mock/mock-embed` (constant
/// vectors) and `ollama/nomic-embed-text` (768 values zero-padded into the 1536 table).
pub fn map_legacy_target(target: &str) -> String {
    let target = target.trim();
    let clean = ["openai", "gemini"]
        .into_iter()
        .filter_map(EmbeddingConfig::preset)
        .find(|preset| target == legacy_target_of(preset));
    match clean {
        Some(preset) => preset.key(),
        None => format!("legacy:{target}"),
    }
}

/// The `provider/model` text a preset was recorded with before v0.46.0.
fn legacy_target_of(preset: &EmbeddingConfig) -> String {
    let provider = match &preset.runtime {
        crate::ingestion::embedding::EmbeddingRuntime::Api { key_name: Some(name), .. } => name.as_str(),
        crate::ingestion::embedding::EmbeddingRuntime::Api { key_name: None, .. } => "ollama",
        _ => "mock",
    };
    format!("{provider}/{}", preset.model)
}

/// The target that built a workspace's vectors before v0.46.0: the recorded one, or for a
/// workspace indexed before targets were recorded, what the resolver of that time gave.
fn legacy_workspace_target(db: &NativeConfigDb, workspace: &str) -> String {
    match db.get_workspace_embedding_target(workspace) {
        Ok(Some(Some(recorded))) if !recorded.trim().is_empty() => recorded,
        _ => legacy_resolved_target(db, workspace),
    }
}

/// The resolver of v0.45.x without the environment overrides: provider from the chat model,
/// falling back to OpenAI and then the mock provider without a key. Unrecorded Gemini
/// workspaces date from before v0.45.1 and hold `text-embedding-004` vectors.
fn legacy_resolved_target(db: &NativeConfigDb, workspace: &str) -> String {
    let model = db
        .get_workspace_model(workspace)
        .ok()
        .or_else(|| db.get_default_model().ok())
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let has_key = |provider: &str| db.get_api_key(provider).ok().flatten().is_some();
    let fallback = || {
        if has_key("openai") {
            "openai/text-embedding-3-small"
        } else {
            "mock/mock-embed"
        }
    };
    let target = if model.starts_with("mock") {
        "mock/mock-embed"
    } else if model.starts_with("ollama") || model.starts_with("local") {
        "ollama/nomic-embed-text"
    } else if model.starts_with("gemini") {
        if has_key("gemini") {
            "gemini/text-embedding-004"
        } else {
            fallback()
        }
    } else {
        fallback()
    };
    target.to_string()
}

/// Preset a legacy target votes for when the global model is chosen; mock vectors vote for none.
fn preset_for_legacy_target(target: &str) -> Option<&'static str> {
    match target.split('/').next().unwrap_or_default() {
        "openai" => Some("openai"),
        "gemini" => Some("gemini"),
        "ollama" => Some("ollama"),
        _ => None,
    }
}

/// Preset derived from a chat model, as the embedding model followed the chat model before.
pub fn preset_for_chat_model(chat_model: &str) -> &'static str {
    let m = chat_model.trim().to_lowercase();
    if m.starts_with("mock") {
        "mock"
    } else if m.starts_with("gemini") {
        "gemini"
    } else if m.starts_with("ollama") || m.starts_with("local") {
        "ollama"
    } else {
        "openai"
    }
}

/// Chooses the global model when none is stored, records the model of every cached file and
/// stores the choice. The preset voted for by the most indexed files wins; ties, and an
/// installation without indexed files, resolve from the default chat model. Returns the stored
/// model when another process stored one first.
pub fn adopt_global_model(db: &NativeConfigDb) -> EmbeddingConfig {
    if let Some(stored) = stored_embedding_config(db) {
        return stored;
    }

    let mut votes: BTreeMap<&'static str, usize> = BTreeMap::new();
    for workspace in db.list_workspace_names().unwrap_or_default() {
        let files = db.count_cached_files(&workspace).unwrap_or(0);
        if files == 0 {
            continue;
        }
        let target = legacy_workspace_target(db, &workspace);
        if let Some(preset) = preset_for_legacy_target(&target) {
            *votes.entry(preset).or_default() += files;
        }
        let _ = db.backfill_stat_cache_embedding_model(&workspace, &map_legacy_target(&target));
    }

    let chat_preset = preset_for_chat_model(&db.get_default_model().unwrap_or_default());
    let top = votes.values().copied().max().unwrap_or(0);
    let leaders: Vec<&str> = votes.iter().filter(|(_, n)| **n == top && top > 0).map(|(p, _)| *p).collect();
    let choice = match leaders.as_slice() {
        [single] => *single,
        _ => chat_preset,
    };
    let config = EmbeddingConfig::preset(choice).expect("built-in preset");

    // The presets whose vectors are already stored stay buildable while their files migrate.
    for preset in votes.keys().filter_map(|p| EmbeddingConfig::preset(p)) {
        let _ = remember_embedding_config(db, &preset);
    }
    let _ = set_active_embedding_config(db, &config);
    config
}

/// Records the embedding model on the LanceDB rows written before v0.46.0, once. `Global` is
/// left out: its rows are rewritten as placeholders on every upgrade and embedded again with
/// the active model. Returns the number of rows updated.
pub fn backfill_chunk_models(db: &NativeConfigDb, lance_store: &NativeLanceStore) -> Result<usize, String> {
    if db.get_setting(CHUNK_MODELS_BACKFILLED_SETTING).ok().flatten().as_deref() == Some("1") {
        return Ok(0);
    }
    // The stat cache is recorded with the choice of the global model.
    adopt_global_model(db);
    lance_store.ensure_added_columns(None)?;

    let mut updated = 0;
    for workspace in db.list_workspace_names().map_err(|e| e.to_string())? {
        let key = map_legacy_target(&legacy_workspace_target(db, &workspace));
        updated += lance_store.backfill_embedding_model(&workspace, &key, None)?;
    }
    db.set_setting(CHUNK_MODELS_BACKFILLED_SETTING, "1").map_err(|e| e.to_string())?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion::embedding::{active_embedding_config, known_embedding_configs};
    use crate::storage::lancedb::VectorRecord;

    fn env_is_clean() -> bool {
        let vars = ["OPENAI_API_KEY", "GEMINI_API_KEY", "GOOGLE_API_KEY", crate::ingestion::embedding::EMBEDDING_MODEL_ENV];
        let clean = vars.iter().all(|v| std::env::var(v).map(|s| s.trim().is_empty()).unwrap_or(true));
        if !clean {
            eprintln!("skipped: one of {vars:?} is set");
        }
        clean
    }

    #[test]
    fn legacy_targets_map_to_preset_keys_or_stale_keys() {
        assert_eq!(map_legacy_target("openai/text-embedding-3-small"), "text-embedding-3-small#1536");
        assert_eq!(map_legacy_target("gemini/gemini-embedding-2"), "gemini-embedding-2#1536");
        assert_eq!(map_legacy_target("gemini/text-embedding-004"), "legacy:gemini/text-embedding-004");
        assert_eq!(map_legacy_target("mock/mock-embed"), "legacy:mock/mock-embed");
        // Padded into the 1536 table: stale, so the rows move to their own table when migrated.
        assert_eq!(map_legacy_target("ollama/nomic-embed-text"), "legacy:ollama/nomic-embed-text");
        for preset in ["openai", "gemini", "ollama", "mock"] {
            let key = EmbeddingConfig::preset(preset).unwrap().key();
            assert!(!map_legacy_target("mock/mock-embed").eq(&key));
        }
    }

    #[test]
    fn chat_models_map_to_presets() {
        assert_eq!(preset_for_chat_model("gpt-4o-mini"), "openai");
        assert_eq!(preset_for_chat_model("gemini-2.5-flash"), "gemini");
        assert_eq!(preset_for_chat_model("ollama/llama3"), "ollama");
        assert_eq!(preset_for_chat_model("claude-sonnet-4"), "openai");
        assert_eq!(preset_for_chat_model("mock"), "mock");
    }

    fn cache_files(db: &NativeConfigDb, workspace: &str, n: usize) {
        for i in 0..n {
            db.upsert_file_stat_cache(workspace, &format!("/{workspace}/f{i}.md"), 1.0, 10, Some("h"), None).unwrap();
        }
    }

    /// The model recorded by the most indexed files becomes the global model; the cache is
    /// backfilled from each workspace's target and files of other models are stale.
    #[test]
    fn global_model_follows_the_most_indexed_files_and_backfills_the_cache() {
        if !env_is_clean() {
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        db.set_default_model("gemini-2.5-flash").unwrap();
        for ws in ["Big", "Small", "Old"] {
            db.create_workspace(ws, None).unwrap();
        }
        db.set_workspace_embedding_target("Big", Some("openai/text-embedding-3-small")).unwrap();
        db.set_workspace_embedding_target("Small", Some("gemini/gemini-embedding-2")).unwrap();
        db.set_workspace_embedding_target("Old", Some("gemini/text-embedding-004")).unwrap();
        cache_files(&db, "Big", 5);
        cache_files(&db, "Small", 2);
        cache_files(&db, "Old", 1);

        let config = active_embedding_config(&db, None).unwrap();
        assert_eq!(config.key(), "text-embedding-3-small#1536", "5 OpenAI files against 3 Gemini files");
        assert_eq!(stored_embedding_config(&db), Some(config.clone()));
        assert_eq!(db.count_stale_files("Big", &config.key()).unwrap(), 0);
        assert_eq!(db.count_stale_files("Small", &config.key()).unwrap(), 2);
        assert_eq!(db.stale_files("Old", &config.key()).unwrap(), vec!["/Old/f0.md".to_string()]);
        // Both presets with stored vectors stay buildable for the migration.
        let known: Vec<String> = known_embedding_configs(&db).into_keys().collect();
        assert_eq!(known, vec!["gemini-embedding-2#1536", "text-embedding-3-small#1536"]);

        // Chosen once: later changes to the index or the chat model do not move it.
        cache_files(&db, "Small", 10);
        db.set_default_model("ollama/llama3").unwrap();
        assert_eq!(active_embedding_config(&db, None).unwrap().key(), "text-embedding-3-small#1536");
    }

    #[test]
    fn ties_and_new_installations_follow_the_default_chat_model() {
        if !env_is_clean() {
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        db.set_default_model("ollama/llama3").unwrap();
        assert_eq!(adopt_global_model(&db).key(), "nomic-embed-text#768", "nothing indexed");

        let db = NativeConfigDb::open_in_memory().expect("db");
        db.set_default_model("gemini-2.5-flash").unwrap();
        for (ws, target) in [("A", "openai/text-embedding-3-small"), ("B", "ollama/nomic-embed-text")] {
            db.create_workspace(ws, None).unwrap();
            db.set_workspace_embedding_target(ws, Some(target)).unwrap();
            cache_files(&db, ws, 3);
        }
        assert_eq!(adopt_global_model(&db).key(), "gemini-embedding-2#1536", "tie");
    }

    /// Workspaces indexed before targets were recorded follow the v0.45.x resolver: without a
    /// key it gave the mock provider, whose vectors are stale.
    #[test]
    fn unrecorded_workspaces_use_the_legacy_resolution() {
        if !env_is_clean() {
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        db.create_workspace("Keyless", None).unwrap();
        db.set_workspace_model("Keyless", "gpt-4o-mini").unwrap();
        cache_files(&db, "Keyless", 2);
        db.set_default_model("gpt-4o-mini").unwrap();
        let config = adopt_global_model(&db);
        assert_eq!(config.key(), "text-embedding-3-small#1536");
        assert_eq!(db.count_stale_files("Keyless", &config.key()).unwrap(), 2, "mock vectors are stale");

        let db = NativeConfigDb::open_in_memory().expect("db");
        db.set_api_key("gemini", "gm-test").unwrap();
        db.create_workspace("Gem", None).unwrap();
        db.set_workspace_model("Gem", "gemini-2.5-flash").unwrap();
        assert_eq!(legacy_workspace_target(&db, "Gem"), "gemini/text-embedding-004");
    }

    fn row(id: &str, workspace: &str, model: Option<&str>) -> VectorRecord {
        VectorRecord {
            id: id.into(),
            vector: vec![0.5; 4],
            text: format!("chunk {id}"),
            file_name: "a.md".into(),
            file_path: format!("/{workspace}/a.md"),
            workspace: workspace.into(),
            last_modified: None,
            content_type: None,
            document_summary: None,
            keywords: None,
            content_hash: None,
            header_path: None,
            start_line: None,
            end_line: None,
            embedding_model: model.map(str::to_string),
        }
    }

    /// The first start of v0.46.0 records each workspace's model on its old rows, once; rows
    /// already carrying a model and the `Global` placeholders are left as they are.
    #[test]
    fn chunk_rows_are_backfilled_once() {
        if !env_is_clean() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("actx_emb_backfill_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let lance = NativeLanceStore::open(&dir).expect("lance");
        let db = NativeConfigDb::open_in_memory().expect("db");
        db.set_default_model("gpt-4o-mini").unwrap();
        db.create_workspace("Proj", None).unwrap();
        db.set_workspace_embedding_target("Proj", Some("openai/text-embedding-3-small")).unwrap();
        db.create_workspace("Local", None).unwrap();
        db.set_workspace_embedding_target("Local", Some("ollama/nomic-embed-text")).unwrap();
        lance
            .upsert_records(
                vec![
                    row("p0", "Proj", None),
                    row("p1", "Proj", None),
                    row("p2", "Proj", Some("text-embedding-3-small#1536")),
                    row("l0", "Local", None),
                    row("g0", "Global", None),
                ],
                None,
                Some(4),
            )
            .unwrap();

        assert_eq!(backfill_chunk_models(&db, &lance), Ok(3));
        let count = |filter: &str| lance.count_rows_where(filter, None).unwrap();
        assert_eq!(count("workspace = 'Proj' AND embedding_model = 'text-embedding-3-small#1536'"), 3);
        assert_eq!(count("workspace = 'Local' AND embedding_model = 'legacy:ollama/nomic-embed-text'"), 1);
        assert_eq!(count("workspace = 'Global' AND embedding_model IS NULL"), 1, "placeholders keep no model");

        // Done once: a later row without a model (written by an older version) is left stale.
        lance.upsert_records(vec![row("p3", "Proj", None)], None, Some(4)).unwrap();
        assert_eq!(backfill_chunk_models(&db, &lance), Ok(0));
        assert_eq!(count("embedding_model IS NULL"), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
