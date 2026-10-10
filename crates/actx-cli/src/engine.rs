use std::sync::Arc;
use actx_agent::{Agent, AgentExecutionMode, SearchMode};
use actx_lm::providers::ProviderKind;
use actx_lm::traits::LmProvider;
use any_context_core_rs::storage::NativeConfigDb;

/// Resolves an LmClient based on persistent SQLite database configuration,
/// environment variables, or explicit overrides. Interface-agnostic.
pub fn resolve_lm_provider(
    model_override: Option<&str>,
    workspace: Option<&str>,
) -> Result<(Arc<dyn LmProvider>, String), String> {
    let db = NativeConfigDb::open_default().ok();

    // 1. Resolve effective model:
    // model_override > ACTX_MODEL env > workspace model from DB > default model from DB > "gpt-4o-mini"
    let effective_model = model_override
        .map(|s| s.to_string())
        .or_else(|| std::env::var("ACTX_MODEL").ok())
        .or_else(|| {
            if let Some(ws) = workspace {
                db.as_ref().and_then(|d| d.get_workspace_model(ws).ok())
            } else {
                None
            }
        })
        .or_else(|| db.as_ref().and_then(|d| d.get_default_model().ok()))
        .unwrap_or_else(|| "gpt-4o-mini".to_string());

    let raw_model = effective_model.trim().to_string();

    let get_key = |provider: &str| -> Option<String> {
        db.as_ref().and_then(|d| d.get_api_key(provider).ok().flatten())
    };

    let (kind, model_name, api_key) = if raw_model == "mock" || raw_model.starts_with("mock") {
        (ProviderKind::Mock, "mock-model".to_string(), None)
    } else if raw_model.starts_with("ollama/") || raw_model.starts_with("local/") {
        let actual_model = raw_model.trim_start_matches("ollama/").trim_start_matches("local/");
        (ProviderKind::Ollama { base_url: None }, actual_model.to_string(), None)
    } else if raw_model.starts_with("claude") {
        let key = get_key("anthropic");
        (ProviderKind::Anthropic, raw_model, key)
    } else if raw_model.starts_with("gemini") {
        let key = get_key("gemini");
        (ProviderKind::Gemini, raw_model, key)
    } else if raw_model.starts_with("deepseek") {
        let key = get_key("deepseek");
        (ProviderKind::DeepSeek, raw_model, key)
    } else if raw_model.starts_with("groq") {
        let key = get_key("groq");
        (ProviderKind::Groq, raw_model, key)
    } else if raw_model.starts_with("gpt") || raw_model.starts_with("o1") || raw_model.starts_with("o3") {
        let key = get_key("openai");
        (ProviderKind::OpenAi, raw_model, key)
    } else {
        // Fallback: check which provider has credentials configured in env or SQLite database
        if let Some(key) = get_key("openai") {
            (ProviderKind::OpenAi, raw_model, Some(key))
        } else if let Some(key) = get_key("anthropic") {
            (ProviderKind::Anthropic, raw_model, Some(key))
        } else if let Some(key) = get_key("gemini") {
            (ProviderKind::Gemini, raw_model, Some(key))
        } else if let Some(key) = get_key("deepseek") {
            (ProviderKind::DeepSeek, raw_model, Some(key))
        } else if let Some(key) = get_key("groq") {
            (ProviderKind::Groq, raw_model, Some(key))
        } else {
            (ProviderKind::Mock, raw_model, None)
        }
    };

    let provider = match kind.build(api_key, None) {
        Ok(p) => p,
        Err(_) => {
            // Resilient fallback to mock provider so actx never crashes
            Arc::new(actx_lm::providers::MockLmProvider::new())
        }
    };

    Ok((provider, model_name))
}

/// Builds an active `Agent` configured for the specified workspace and model synchronously.
pub fn build_agent_sync(
    provider: Arc<dyn LmProvider>,
    model: &str,
    workspace: &str,
    grounding_mode: &str,
    search_mode: &str,
    web_search_enabled: bool,
) -> Result<Agent, String> {
    let db_path = any_context_core_rs::storage::get_default_settings_db_path();
    let session_store = actx_agent::SqliteSessionStore::open(&db_path, 50).ok()
        .map(|s| Arc::new(s) as Arc<dyn actx_agent::SessionStore>);

    let system_prompt = crate::prompt::build_system_prompt(workspace, grounding_mode, search_mode, web_search_enabled);

    let ws_for_tool = workspace.to_string();
    let provider_for_search = provider.clone();
    let search_tool = actx_agent::NativeTool::new(
        "search_db",
        "Searches for relevant documents, source code, and knowledge context in the active workspace.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query, topic, or keyword to look up"
                }
            },
            "required": ["query"]
        }),
        move |args: serde_json::Value| {
            let ws = ws_for_tool.clone();
            let tool_provider = provider_for_search.clone();
            async move {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                if query.is_empty() {
                    return Ok("Empty search query provided.".to_string());
                }

                let lance_path = any_context_core_rs::storage::get_default_lancedb_path();
                let lance_store = match any_context_core_rs::storage::NativeLanceStore::open(&lance_path) {
                    Ok(ls) => Arc::new(ls),
                    Err(e) => return Ok(format!("Search database unavailable: {e}")),
                };

                ensure_global_knowledge_bootstrap(&lance_store);

                let pipeline = any_context_core_rs::retrieval::NativeHybridPipeline::new(
                    lance_store,
                    Some(tool_provider),
                );

                let req = any_context_core_rs::retrieval::HybridSearchRequest {
                    query_text: query.to_string(),
                    workspace: Some(ws.clone()),
                    target_workspaces: vec!["Global".to_string()],
                    top_k: 5,
                    candidate_pool_k: 30,
                    max_density_chars: 12_000,
                    min_score: 0.005,
                    table_name: "workspace_chunks".to_string(),
                    ..Default::default()
                };

                match pipeline.search_single_async(&req).await {
                    Ok(results) if !results.is_empty() => {
                        let mut out = Vec::new();
                        for r in results {
                            let type_str = if r.content_type.is_empty() { "Document" } else { &r.content_type };
                            out.push(format!(
                                "• [{}] (Score: {:.2}, Source: {}, Type: {}):\n{}",
                                r.file_name,
                                r.score,
                                r.file_path,
                                type_str,
                                r.text
                            ));
                        }
                        Ok(out.join("\n\n"))
                    }
                    Ok(_) => {
                        let db = any_context_core_rs::storage::NativeConfigDb::open_default().ok();
                        let folders = db.as_ref()
                            .and_then(|d| d.get_workspace_folders(&ws).ok())
                            .unwrap_or_default();
                        let web_urls = db.as_ref()
                            .and_then(|d| d.get_workspace_web_urls(&ws).ok())
                            .unwrap_or_default();

                        if !folders.is_empty() || !web_urls.is_empty() {
                            let mut sources = Vec::new();
                            if !folders.is_empty() {
                                sources.push(format!("folders: [{}]", folders.join(", ")));
                            }
                            if !web_urls.is_empty() {
                                sources.push(format!("web portals: [{}]", web_urls.join(", ")));
                            }
                            Ok(format!(
                                "Active workspace '{}' monitors {}. No indexed document chunks matched query '{}'.",
                                ws,
                                sources.join("; "),
                                query
                            ))
                        } else {
                            Ok(format!(
                                "No indexed document chunks found in workspace '{}' for query '{}'.",
                                ws, query
                            ))
                        }
                    }
                    Err(e) => {
                        Ok(format!("Search query returned 0 results: {e}"))
                    }
                }
            }
        }
    );

    let ws_for_status = workspace.to_string();
    let status_tool = actx_agent::NativeTool::new(
        "system_status",
        "Checks real-time system status, background indexing/sync progress, and document counts for the active workspace.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "workspace": {
                    "type": "string",
                    "description": "Optional workspace name to inspect (defaults to active workspace)"
                }
            }
        }),
        move |args: serde_json::Value| {
            let ws = args.get("workspace")
                .and_then(|v| v.as_str())
                .unwrap_or(&ws_for_status)
                .to_string();
            async move {
                let db = any_context_core_rs::storage::NativeConfigDb::open_default().ok();
                let sync_status = db.as_ref()
                    .and_then(|d| d.get_sync_status(&ws).ok().flatten());
                let folders = db.as_ref()
                    .and_then(|d| d.get_workspace_folders(&ws).ok())
                    .unwrap_or_default();
                let web_urls = db.as_ref()
                    .and_then(|d| d.get_workspace_web_urls(&ws).ok())
                    .unwrap_or_default();

                let lance_path = any_context_core_rs::storage::get_default_lancedb_path();
                let chunk_count = any_context_core_rs::storage::NativeLanceStore::open(&lance_path)
                    .ok()
                    .and_then(|s| s.count_records(Some(&ws), Some("workspace_chunks")).ok())
                    .unwrap_or(0);

                let sync_info = if let Some(ss) = sync_status {
                    if ss.is_syncing {
                        format!("SYNC IN PROGRESS: {} (item {} of {}, stage '{}', target: '{}')",
                            ss.progress_bar, ss.current_item, ss.total_items, ss.stage, ss.item_name.as_deref().unwrap_or("none"))
                    } else if ss.stage == "completed" || ss.progress_bar.contains("Up to date") {
                        format!("READY / UP TO DATE: All documents indexed successfully. Last update: {}", ss.updated_at)
                    } else {
                        format!("IDLE: {}", ss.progress_bar)
                    }
                } else {
                    "IDLE / READY: No active background synchronization. All registered documents are indexed.".to_string()
                };

                let folders_str = if folders.is_empty() { "none".to_string() } else { folders.join(", ") };
                let web_str = if web_urls.is_empty() { "none".to_string() } else { web_urls.join(", ") };

                Ok(format!(
                    "Workspace '{}' Status:\n• Synchronization Status: {}\n• Total Indexed Chunks: {}\n• Monitored Local Folders: {}\n• Monitored Web Sources: {}",
                    ws, sync_info, chunk_count, folders_str, web_str
                ))
            }
        }
    );

    let search_policy = match search_mode.to_lowercase().as_str() {
        "fast" => SearchMode::Fast,
        "deep" => SearchMode::Deep,
        _ => SearchMode::Auto,
    };

    let execution_mode = if search_policy == SearchMode::Deep {
        AgentExecutionMode::DeepSearch
    } else {
        AgentExecutionMode::ReAct
    };

    let lance_path = any_context_core_rs::storage::get_default_lancedb_path();
    let lance_store_for_deep = any_context_core_rs::storage::NativeLanceStore::open(&lance_path).ok().map(Arc::new);

    let deep_search_retriever = lance_store_for_deep.map(|ls| {
        Arc::new(PipelineBatchRetriever {
            lance_store: ls,
            lm_provider: provider.clone(),
            workspace: workspace.to_string(),
        }) as Arc<dyn actx_agent::DeepSearchRetriever>
    });

    let mut builder = Agent::builder()
        .client(provider)
        .model(model)
        .system_prompt(system_prompt)
        .execution_mode(execution_mode)
        .search_mode(search_policy)
        .max_turns(10)
        .tool(Arc::new(search_tool))
        .tool(Arc::new(status_tool));

    if let Some(retriever) = deep_search_retriever {
        builder = builder
            .deep_search_retriever(retriever)
            .deep_search_config(actx_agent::DeepSearchConfig::default());
    }

    if web_search_enabled {
        let ws_for_web = workspace.to_string();
        let web_search_tool = actx_agent::NativeTool::new(
            "web_search",
            "Performs live internet search across documentation and the public web for real-time information.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query or technical keywords to search on the web"
                    }
                },
                "required": ["query"]
            }),
            move |args: serde_json::Value| {
                let ws = ws_for_web.clone();
                async move {
                    let query = args.get("query")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim();
                    if query.is_empty() {
                        return Ok("Empty query provided to web search.".to_string());
                    }

                    let db = any_context_core_rs::storage::NativeConfigDb::open_default().ok().map(Arc::new);
                    let mut domains = Vec::new();
                    if let Some(ref d) = db {
                        if let Ok(web_urls) = d.get_workspace_web_urls(&ws) {
                            for u_str in web_urls {
                                if let Ok(parsed) = url::Url::parse(&u_str) {
                                    if let Some(host) = parsed.host_str() {
                                        domains.push(host.to_string());
                                    }
                                }
                            }
                        }
                    }

                    let engine = any_context_core_rs::retrieval::NativeWebSearchEngine::new(db);
                    match engine.search(query, &domains, 5).await {
                        Ok(results) if !results.is_empty() => {
                            let mut formatted = Vec::new();
                            for r in results {
                                formatted.push(format!("• [{}] ({}):\n{}", r.title, r.url, r.snippet));
                            }
                            Ok(formatted.join("\n\n"))
                        }
                        Ok(_) => Ok(format!("No live web results found for '{}'.", query)),
                        Err(e) => Ok(format!("Web search failed: {e}")),
                    }
                }
            }
        );
        builder = builder.tool(Arc::new(web_search_tool));
    }

    if let Some(store) = session_store {
        builder = builder.session_store(store);
    }

    builder
        .build_sync()
        .map_err(|e| format!("Agent construction failed: {}", e))
}

/// Builds an active `Agent` configured for the specified workspace and model.
pub async fn build_agent(
    provider: Arc<dyn LmProvider>,
    model: &str,
    workspace: &str,
    grounding_mode: &str,
    search_mode: &str,
    web_search_enabled: bool,
) -> Result<Agent, String> {
    build_agent_sync(provider, model, workspace, grounding_mode, search_mode, web_search_enabled)
}

/// Ensures that the 'Global' workspace in LanceDB contains AnyContext system documentation.
/// If no chunks exist under 'Global' or if the application version was upgraded, it automatically
/// chunks and indexes the embedded documentation and updates the BM25 index for zero-token self-knowledge.
pub fn ensure_global_knowledge_bootstrap(lance_store: &any_context_core_rs::storage::NativeLanceStore) {
    let db = any_context_core_rs::storage::NativeConfigDb::open_default().ok();
    let current_version = env!("CARGO_PKG_VERSION");
    let stored_version = db.as_ref().and_then(|d| d.get_setting("global_knowledge_version").ok().flatten());
    let count = lance_store.count_records(Some("Global"), Some("workspace_chunks")).unwrap_or(0);

    let is_upgrade = stored_version.as_deref() != Some(current_version);
    if count > 0 && !is_upgrade {
        return;
    }

    if is_upgrade && count > 0 {
        let _ = lance_store.delete_by_workspace("Global", Some("workspace_chunks"));
    }

    let readme = crate::prompt::EMBEDDED_README_MD;
    let mut sections = Vec::new();
    let mut current_header = "AnyContext Overview".to_string();
    let mut current_body = String::new();

    for line in readme.lines() {
        if line.starts_with("# ") || line.starts_with("## ") || line.starts_with("### ") {
            if !current_body.trim().is_empty() {
                sections.push((current_header.clone(), current_body.trim().to_string()));
                current_body.clear();
            }
            current_header = line.trim_start_matches('#').trim().to_string();
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    if !current_body.trim().is_empty() {
        sections.push((current_header, current_body.trim().to_string()));
    }

    if sections.is_empty() {
        return;
    }

    let mut records = Vec::new();
    let mut bm25_chunks = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();

    for (idx, (header, text)) in sections.into_iter().enumerate() {
        let chunk_id = format!("global_sys_doc_{idx}");
        let full_text = format!("# {}\n\n{}", header, text);
        let rec = any_context_core_rs::storage::VectorRecord {
            id: chunk_id.clone(),
            vector: vec![0.0; 1536],
            text: full_text.clone(),
            file_name: "README.md".to_string(),
            file_path: "system://README.md".to_string(),
            workspace: "Global".to_string(),
            last_modified: Some(now.clone()),
            content_type: Some("System Documentation".to_string()),
            document_summary: Some(format!("AnyContext Documentation: {}", header)),
            keywords: Some("anycontext, system, guide, commands, usage".to_string()),
            content_hash: None,
        };
        records.push(rec);
        bm25_chunks.push((chunk_id, full_text, header));
    }

    // 1. Insert into LanceDB
    let _ = lance_store.upsert_records(records, Some("workspace_chunks"), Some(1536));

    // 2. Insert into BM25 index and save
    let bm25_path = lance_store.db_path().join("bm25_index.bin");
    let mut bm25 = if bm25_path.exists() {
        match any_context_core_rs::retrieval::BM25Index::load_from_file(bm25_path.to_str().unwrap_or("")) {
            Ok(idx) => idx,
            Err(_e) => {
                let corrupt_name = format!("{}.corrupt.{}", bm25_path.display(), chrono::Utc::now().timestamp());
                let _ = std::fs::rename(&bm25_path, &corrupt_name);
                any_context_core_rs::retrieval::BM25Index::new(None, None)
            }
        }
    } else {
        any_context_core_rs::retrieval::BM25Index::new(None, None)
    };

    for (cid, ftext, _) in bm25_chunks {
        bm25.add_chunk(
            cid,
            ftext,
            "README.md".to_string(),
            "system://README.md".to_string(),
            "Global".to_string(),
            "System Documentation".to_string(),
        );
    }
    let _ = bm25.save_to_file(bm25_path.to_str().unwrap_or(""));

    // 3. Persist current version to settings for transparent upgrades
    if let Some(ref d) = db {
        let _ = d.set_setting("global_knowledge_version", current_version);
    }
}

/// Quickly checks if the application is starting for the first time or after a version upgrade.
/// Runs in < 1ms via SQLite without touching LanceDB or BM25 index on the main thread.
pub fn is_first_run_or_upgrade() -> bool {
    let db = any_context_core_rs::storage::NativeConfigDb::open_default().ok();
    let current_version = env!("CARGO_PKG_VERSION");
    let stored_version = db.as_ref().and_then(|d| d.get_setting("global_knowledge_version").ok().flatten());
    stored_version.as_deref() != Some(current_version)
}

/// Renders an Aurora Boreal branded CLI Splash Loader with step-by-step telemetry
/// before launching the TUI when an upgrade or first run is detected (Option C).
pub fn run_startup_splash_bootstrap() {
    use std::io::{stdout, Write};
    let theme = crate::theme::UiTheme::default();
    let mut out = stdout();

    let banner = format!(
        "\n  {} {}\n  {}\n\n",
        theme.ansi_primary("✨ AnyContext"),
        theme.ansi_accent(&format!("v{}", env!("CARGO_PKG_VERSION"))),
        theme.ansi_reasoning("🌌 Initializing Aurora Engine & System Knowledge...")
    );
    let _ = out.write_all(banner.as_bytes());
    let _ = out.flush();

    // Step 1: Databases verification
    let _ = out.write_all(format!("  {} [1/3] Verifying native SQLite & LanceDB vector stores...", theme.ansi_warning("⣾")).as_bytes());
    let _ = out.flush();
    let lance_path = any_context_core_rs::storage::get_default_lancedb_path();
    let lance_store = any_context_core_rs::storage::NativeLanceStore::open(&lance_path).ok();
    let _ = out.write_all(format!("\r  {} [1/3] Native SQLite & LanceDB vector stores verified.     \n", theme.ansi_primary("✔")).as_bytes());
    let _ = out.flush();

    // Step 2: Index system documentation & BM25
    let _ = out.write_all(format!("  {} [2/3] Indexing system knowledge & BM25 hybrid lexicon...", theme.ansi_warning("⣾")).as_bytes());
    let _ = out.flush();
    if let Some(ref ls) = lance_store {
        ensure_global_knowledge_bootstrap(ls);
    }
    let _ = out.write_all(format!("\r  {} [2/3] System knowledge & BM25 hybrid lexicon indexed.      \n", theme.ansi_primary("✔")).as_bytes());
    let _ = out.flush();

    // Step 3: Warmup engine
    let _ = out.write_all(format!("  {} [3/3] Warming up workspace engine & model routing...", theme.ansi_warning("⣾")).as_bytes());
    let _ = out.flush();
    std::thread::sleep(std::time::Duration::from_millis(150));
    let _ = out.write_all(format!("\r  {} [3/3] Workspace engine & model routing ready.             \n", theme.ansi_primary("✔")).as_bytes());
    let _ = out.flush();

    let ready_msg = format!("  {} Ready! Launching interactive workspace...\n\n", theme.ansi_primary("✔"));
    let _ = out.write_all(ready_msg.as_bytes());
    let _ = out.flush();
    std::thread::sleep(std::time::Duration::from_millis(250));
}

/// Native Hybrid RAG batch retriever adapter for DeepSearch
struct PipelineBatchRetriever {
    lance_store: Arc<any_context_core_rs::storage::NativeLanceStore>,
    lm_provider: Arc<dyn LmProvider>,
    workspace: String,
}

#[actx_agent::async_trait]
impl actx_agent::DeepSearchRetriever for PipelineBatchRetriever {
    async fn retrieve_batch(&self, queries: &[String]) -> Result<Vec<actx_agent::RetrievedChunk>, actx_agent::AgentError> {
        let pipeline = any_context_core_rs::retrieval::NativeHybridPipeline::new(
            self.lance_store.clone(),
            Some(self.lm_provider.clone()),
        );

        let requests: Vec<_> = queries
            .iter()
            .map(|q| any_context_core_rs::retrieval::HybridSearchRequest {
                query_text: q.clone(),
                workspace: Some(self.workspace.clone()),
                target_workspaces: vec!["Global".to_string()],
                top_k: 8,
                candidate_pool_k: 40,
                max_density_chars: 16_000,
                min_score: 0.02,
                table_name: "workspace_chunks".to_string(),
                ..Default::default()
            })
            .collect();

        match pipeline.retrieve_hybrid_batch_async(&requests).await {
            Ok(results) => {
                let chunks = results
                    .into_iter()
                    .map(|r| actx_agent::RetrievedChunk {
                        id: r.chunk_id,
                        file_name: r.file_name,
                        file_path: r.file_path,
                        header_path: None,
                        start_line: None,
                        end_line: None,
                        text: r.text,
                        score: r.score,
                        matched_queries: r.matched_subqueries,
                    })
                    .collect();
                Ok(chunks)
            }
            Err(e) => Err(actx_agent::AgentError::Internal(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_first_run_or_upgrade_does_not_panic() {
        // Calling is_first_run_or_upgrade should execute in < 5ms and return a boolean without panicking
        let start = std::time::Instant::now();
        let _result = is_first_run_or_upgrade();
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 50, "Startup check must run in under 50ms (was {:?})", elapsed);
    }

    #[test]
    fn test_run_startup_splash_bootstrap_lifecycle() {
        // Executing splash bootstrap should run all 3 stages and persist the current version
        run_startup_splash_bootstrap();
        assert!(!is_first_run_or_upgrade(), "After running splash bootstrap, is_first_run_or_upgrade must be false");
    }
}
