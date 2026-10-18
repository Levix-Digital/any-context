//! System Prompt and Grounding Directives Engine for actx-cli.
//!
//! Provides strict parity with Python AnyContext prompt architecture:
//! - Loads canonical AGENT.md guidelines (compiled in binary with runtime overrides)
//! - Names only the tools the answer path really offers (ReAct) or none (one-hop RAG and Deep Search)
//! - Forbids pre-trained parametric answers in STRICT mode
//! - Directs collaborative dialogue and strict citation footers

use std::path::PathBuf;

/// Canonical AGENT.md embedded at compile time for zero-dependency standalone binaries.
pub const EMBEDDED_AGENT_MD: &str = include_str!("../../../config/AGENT.md");

/// Canonical README.md embedded at compile time for system self-knowledge.
pub const EMBEDDED_README_MD: &str = include_str!("../../../README.md");

/// Canonical SYSTEM_KNOWLEDGE.md technical manual embedded at compile time for system self-knowledge.
pub const EMBEDDED_SYSTEM_KNOWLEDGE_MD: &str = include_str!("../../../config/SYSTEM_KNOWLEDGE.md");

/// Loads AGENT.md from disk if available, otherwise returns the compile-time embedded version.
pub fn load_agent_md() -> String {
    // 1. Explicit env var override
    if let Ok(p) = std::env::var("ACTX_AGENT_MD") {
        if let Ok(content) = std::fs::read_to_string(p.trim()) {
            if !content.trim().is_empty() {
                return content;
            }
        }
    }

    // 2. Local config/AGENT.md relative to working dir
    let local_path = PathBuf::from("config").join("AGENT.md");
    if local_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&local_path) {
            if !content.trim().is_empty() {
                return content;
            }
        }
    }

    // 3. User config directory (%LOCALAPPDATA%/AnyContext/config/AGENT.md or ~/.local/share/any-context/config/AGENT.md)
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local).join("AnyContext").join("config").join("AGENT.md");
            if p.exists() {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    if !content.trim().is_empty() {
                        return content;
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = dirs::home_dir() {
            let p = home.join(".config").join("any-context").join("AGENT.md");
            if p.exists() {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    if !content.trim().is_empty() {
                        return content;
                    }
                }
            }
        }
    }

    // 4. Fallback to compile-time embedded version
    EMBEDDED_AGENT_MD.to_string()
}

/// How the model receives its context, which decides the retrieval section of the system prompt.
#[derive(Debug, Clone, Copy)]
pub enum PromptPath<'a> {
    /// ReAct loop: the model may call exactly these registered tools.
    Tools(&'a [&'a str]),
    /// One-hop RAG and Deep Search synthesis: the context is already in the prompt and no tool is offered.
    RetrievedContext,
}

/// Constructs the system instruction prompt for one answer path.
///
/// Everything except the retrieval section is shared by both paths and names no tool.
pub fn build_system_prompt(
    workspace: &str,
    grounding_mode: &str,
    search_mode: &str,
    web_search_enabled: bool,
    path: PromptPath<'_>,
) -> String {
    let base_agent_md = load_agent_md();
    let effective_mode = match grounding_mode.trim().to_lowercase().as_str() {
        "hybrid" => "hybrid",
        "proactive" => "proactive",
        _ => "strict",
    };

    let web_status_str = if web_search_enabled { "ENABLED (Active)" } else { "DISABLED (Offline)" };

    let mut prompt = String::new();
    prompt.push_str(&base_agent_md);
    prompt.push_str("\n\n");

    // Operational Environment
    prompt.push_str(&format!(
        "### 🎯 ACTIVE WORKSPACE & OPERATIONAL ENVIRONMENT:\n\
         - Active Workspace: '{workspace}'\n\
         - Grounding Strategy: {mode_upper}\n\
         - Search Retrieval Depth: {depth_upper}\n\
         - Real-Time Web Search: {web_status_str}\n\n",
        mode_upper = effective_mode.to_uppercase(),
        depth_upper = search_mode.to_uppercase(),
    ));

    // Persistent Memory
    prompt.push_str(
        "### 🧠 PERSISTENT LONG-TERM CONVERSATION MEMORY:\n\
         - You maintain persistent conversation memory across turns and sessions stored locally in SQLite for each workspace.\n\
         - You remember prior conversations, past context, decisions, and instructions given in earlier turns of this workspace.\n\
         - When the user asks about previous topics, past conversations, or asks if you have long-term memory, ALWAYS recognize and reference your persistent memory and conversation history.\n\
         - NEVER claim that you lack long-term memory or that interactions are independent. You are AnyContext and you retain workspace memory.\n\n"
    );

    // Active Grounding Mode Directives
    match effective_mode {
        "strict" => {
            prompt.push_str(
                "### 🛡️ ACTIVE GROUNDING MODE: STRICT (AUDIT & LEGAL - 100% FACTUAL & ZERO PARAMETRIC ANSWERS)\n\
                 - **ZERO GENERAL KNOWLEDGE / ZERO PARAMETRIC MEMORY:** You are an internal workspace assistant. You are STRICTLY FORBIDDEN from using your pre-trained weights or general knowledge to answer ANY question (including recipes, general knowledge, external facts, outside programs, or trivia). You must answer SOLELY based on the workspace context you were given (retrieved excerpts or tool results).\n\
                 - **FACTUAL ABSENCE PROTOCOL (NATURAL & HUMAN CONVERSATION):** If the workspace context has no relevant excerpt for the user's query:\n\
                   1. Answer naturally, directly, and politely like a helpful human teammate.\n\
                   2. State directly what could not be found or what information is currently available in the active workspace.\n\
                   3. DO NOT add robotic warning disclaimers, alarms, or warning emojis (such as caution or exclamation symbols). Maintain a fluid and collaborative tone.\n\
                   4. Proactively suggest relevant alternative topics, keywords, or offer to search with different parameters.\n\
                   5. DO NOT invent or synthesize facts from pre-training memory when in strict mode.\n\
                   6. NEVER output fictitious source citations or placeholders like `[Filename.ext]`.\n\
                 - **MANDATORY SOURCE CITATIONS:** Conclude every answer that used documents with:\n\
                   ---\n\
                   📄 **Sources Consulted (Local Files):**\n\
                   - `<real_file_name>` (Actual path or URL present in the workspace context)\n\
                   NEVER invent filenames or output literal placeholder strings like `[Filename.ext]`.\n\n"
            );
        }
        "hybrid" => {
            prompt.push_str(
                "### ⚖️ ACTIVE GROUNDING MODE: HYBRID (BALANCED - WORKSPACE FIRST + LABELED MODEL KNOWLEDGE)\n\
                 - **WORKSPACE PRIORITY:** Present local workspace facts first.\n\
                 - **DUAL-LAYER STRUCTURE:**\n\
                   `### 📂 Workspace Information` (based on local documents with citations)\n\
                   `### 💡 General Model Knowledge` (properly labeled parametric knowledge)\n\
                 - **PARAMETRIC MEMORY TRANSPARENCY:** Disclose general knowledge with: \"According to my general knowledge...\"\n\
                 - **MANDATORY CITATIONS:** Conclude with `📄 Sources Consulted:` listing documents consulted.\n\n"
            );
        }
        "proactive" => {
            prompt.push_str(
                "### 🚀 ACTIVE GROUNDING MODE: PROACTIVE (RESEARCH & STRATEGY)\n\
                 - Freely combine workspace documents, web intelligence, and domain knowledge.\n\
                 - Tag each insight by source (`[Document: ...]`, `[Web: ...]`, `[Recommendation]`).\n\
                 - NEVER output generic template placeholder phrases such as `[insert topic...]` or fabricate past session details.\n\
                 - If the user asks about prior conversations or historical context and no record is present in this workspace, state honestly that no previous record was found.\n\
                 - Conclude with `📄 Sources Consulted:`.\n\n"
            );
        }
        _ => {}
    }

    if !web_search_enabled {
        prompt.push_str(
            "### 🔒 LIVE WEB SEARCH: DISABLED (OFFLINE-FIRST LOCAL ISOLATION)\n\
             - Web search is DISABLED for this workspace. Answer exclusively using local workspace documents.\n\n"
        );
    }

    match path {
        PromptPath::Tools(tools) => push_tool_section(&mut prompt, tools),
        PromptPath::RetrievedContext => push_retrieved_context_section(&mut prompt, web_search_enabled),
    }

    // Universal Temporal Recency Rule
    prompt.push_str(
        "### ⏱️ UNIVERSAL TEMPORAL RECENCY RULE:\n\
         - Whenever multiple sources within the same priority tier contain overlapping or differing information, THE MOST RECENT SOURCE ALWAYS PREVAILS AND SUPERSEDES OLDER DATA.\n\n"
    );

    // Language Consistency
    prompt.push_str(
        "### 🗣️ LANGUAGE CONSISTENCY:\n\
         - ALWAYS answer in the exact language used by the user in their prompt. (If the user asks in English, reply in English. If in Portuguese, reply in Portuguese, etc.).\n"
    );

    prompt
}

/// Retrieval rules for the ReAct loop, one per registered tool; no other tool is ever named.
fn push_tool_section(prompt: &mut String, tools: &[&str]) {
    let has = |name: &str| tools.contains(&name);

    prompt.push_str("### 🧰 AVAILABLE TOOLS (THIS ANSWER):\n");
    let names: Vec<String> = tools.iter().map(|t| format!("`{t}`")).collect();
    prompt.push_str(&format!(
        "- You may call only these tools: {}. Never call or mention any other tool.\n",
        names.join(", ")
    ));

    if has("search_db") {
        prompt.push_str(
            "- **`search_db(query)`**: You MUST call `search_db` before answering any question about the workspace or about AnyContext itself (its commands, options and workflows are documented in the knowledge base). Call it AT MOST ONCE per question and analyze the returned excerpts immediately. When the question is in Portuguese (or another language) about topics documented in English, write the query with domain keywords in both languages.\n"
        );
    }
    if has("system_status") {
        prompt.push_str(
            "- **`system_status(workspace?)`**: When the user asks about background tasks, synchronization or indexing status (e.g. \"Has everything been indexed?\", \"What is the sync status?\"), call `system_status` and report the live status clearly.\n"
        );
    }
    if has("web_search") {
        prompt.push_str(
            "- **`web_search(query)`**: Real-time web search. In STRICT mode it is PERMISSION-GATED: only search the web if the user explicitly commanded online search or confirmed permission. In HYBRID and PROACTIVE modes you may query complementary web facts autonomously. Conclude answers with `📄 Sources Consulted:`.\n"
        );
    }
    prompt.push('\n');
}

/// Retrieval rules for one-hop RAG and the Deep Search synthesis, where no tool is offered.
fn push_retrieved_context_section(prompt: &mut String, web_search_enabled: bool) {
    prompt.push_str(
        "### 📥 RETRIEVED WORKSPACE CONTEXT (THIS ANSWER):\n\
         - The workspace excerpts for this question were already retrieved and are in `<retrieved_context>`. Answer from them.\n\
         - No tools are available in this answer: never write tool calls, function calls or JSON requests, and never say that you searched, added, changed or removed anything.\n\
         - Questions about AnyContext itself (commands, options, workflows) are answered from the AnyContext manual excerpts when they are present.\n\
         - The live synchronization and indexing status is not visible in this answer: for such questions, point the user to the status bar and the `/status` command.\n"
    );
    if web_search_enabled {
        prompt.push_str("- Web search is enabled for this workspace, but no live web search runs for this answer: do not claim web results.\n");
    }
    prompt.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every tool name the prompts ever used, registered or not.
    const ALL_TOOL_NAMES: &[&str] = &[
        "search_db",
        "system_status",
        "web_search",
        "add_web_source",
        "list_web_sources",
        "remove_web_source",
        "search_session_memory",
    ];

    /// Tool names that no answer path registers.
    const PHANTOM_TOOL_NAMES: &[&str] =
        &["add_web_source", "list_web_sources", "remove_web_source", "search_session_memory"];

    const REGISTERED_TOOLS: &[&str] = &["search_db", "system_status"];

    fn both_paths() -> [PromptPath<'static>; 2] {
        [PromptPath::Tools(REGISTERED_TOOLS), PromptPath::RetrievedContext]
    }

    #[test]
    fn test_embedded_agent_md_not_empty() {
        assert!(!EMBEDDED_AGENT_MD.trim().is_empty());
        assert!(EMBEDDED_AGENT_MD.contains("Workspace Grounding"));
    }

    #[test]
    fn test_embedded_agent_md_names_no_tools() {
        for name in ALL_TOOL_NAMES {
            assert!(!EMBEDDED_AGENT_MD.contains(name), "AGENT.md names the tool `{name}`");
        }
        assert!(EMBEDDED_AGENT_MD.contains("`/folder <path>`"));
        assert!(EMBEDDED_AGENT_MD.contains("`/web <url>`"));
        assert!(EMBEDDED_AGENT_MD.contains("`/sources`"));
        assert!(EMBEDDED_AGENT_MD.contains("Never claim that a source was added, changed, or removed."));
    }

    #[test]
    fn test_embedded_system_knowledge_md_not_empty() {
        assert!(!EMBEDDED_SYSTEM_KNOWLEDGE_MD.trim().is_empty());
        assert!(EMBEDDED_SYSTEM_KNOWLEDGE_MD.contains("System Identity, Philosophy & Architecture"));
        assert!(EMBEDDED_SYSTEM_KNOWLEDGE_MD.contains("Complete Command Reference"));
        assert!(EMBEDDED_SYSTEM_KNOWLEDGE_MD.contains("Smart Path Healing"));
    }

    #[test]
    fn test_embedded_system_knowledge_md_names_no_phantom_tools() {
        for name in PHANTOM_TOOL_NAMES {
            assert!(!EMBEDDED_SYSTEM_KNOWLEDGE_MD.contains(name), "SYSTEM_KNOWLEDGE.md names the tool `{name}`");
        }
        assert!(!EMBEDDED_SYSTEM_KNOWLEDGE_MD.contains("The tool `search_db` must be executed before"));
        assert!(!EMBEDDED_SYSTEM_KNOWLEDGE_MD.contains("must invoke retrieval tools"));
    }

    #[test]
    fn test_retrieved_context_prompt_names_no_tools() {
        for mode in ["strict", "hybrid", "proactive"] {
            for web in [false, true] {
                let p = build_system_prompt("TestWS", mode, "auto", web, PromptPath::RetrievedContext);
                for name in ALL_TOOL_NAMES {
                    assert!(!p.contains(name), "{mode}/web={web}: the prompt names the tool `{name}`");
                }
                assert!(p.contains("were already retrieved and are in `<retrieved_context>`"));
                assert!(p.contains("No tools are available in this answer"));
                assert!(!p.contains("LIVE WEB SEARCH ENGINE: ACTIVE"));
                assert_eq!(p.contains("no live web search runs for this answer"), web);
            }
        }
    }

    #[test]
    fn test_tool_prompt_names_only_registered_tools() {
        let p = build_system_prompt("TestWS", "strict", "auto", false, PromptPath::Tools(REGISTERED_TOOLS));
        assert!(p.contains("You may call only these tools: `search_db`, `system_status`."));
        assert!(p.contains("You MUST call `search_db` before answering"));
        assert!(p.contains("call `system_status`"));
        assert!(!p.contains("web_search"));
        for name in PHANTOM_TOOL_NAMES {
            assert!(!p.contains(name), "the prompt names the tool `{name}`");
        }
        assert!(!p.contains("No tools are available in this answer"));

        let with_web = ["search_db", "system_status", "web_search"];
        let p = build_system_prompt("TestWS", "hybrid", "auto", true, PromptPath::Tools(&with_web));
        assert!(p.contains("`web_search(query)`"));
        assert!(p.contains("PERMISSION-GATED"));

        let only_search = ["search_db"];
        let p = build_system_prompt("TestWS", "strict", "auto", false, PromptPath::Tools(&only_search));
        assert!(!p.contains("system_status"));
    }

    #[test]
    fn test_build_system_prompt_strict_mode() {
        for path in both_paths() {
            let p = build_system_prompt("TestWS", "strict", "auto", false, path);
            assert!(p.contains("Active Workspace: 'TestWS'"));
            assert!(p.contains("GROUNDING MODE: STRICT"));
            assert!(p.contains("ZERO PARAMETRIC MEMORY"));
            assert!(p.contains("LIVE WEB SEARCH: DISABLED"));
            assert!(p.contains("Sources Consulted"));
        }
    }

    #[test]
    fn test_build_system_prompt_hybrid_mode() {
        for path in both_paths() {
            let p = build_system_prompt("TestWS", "hybrid", "fast", true, path);
            assert!(p.contains("GROUNDING MODE: HYBRID"));
            assert!(p.contains("Workspace Information"));
            assert!(p.contains("General Model Knowledge"));
            assert!(p.contains("Real-Time Web Search: ENABLED"));
        }
    }

    #[test]
    fn test_build_system_prompt_proactive_mode() {
        for path in both_paths() {
            let p = build_system_prompt("TestWS", "proactive", "deep", false, path);
            assert!(p.contains("GROUNDING MODE: PROACTIVE"));
            assert!(p.contains("RESEARCH & STRATEGY"));
        }
    }

    #[test]
    fn test_build_system_prompt_strict_mode_natural_conversation() {
        for path in both_paths() {
            let p = build_system_prompt("TestWS", "strict", "auto", false, path);
            assert!(!p.contains("⚠️"));
            assert!(p.contains("FACTUAL ABSENCE PROTOCOL (NATURAL & HUMAN CONVERSATION)"));
        }
    }
}
