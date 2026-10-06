//! System Prompt and Grounding Directives Engine for actx-cli.
//!
//! Provides strict parity with Python AnyContext prompt architecture:
//! - Loads canonical AGENT.md guidelines (compiled in binary with runtime overrides)
//! - Enforces Mandatory Autonomous Retrieval via `search_db`
//! - Forbids pre-trained parametric answers in STRICT mode
//! - Directs collaborative dialogue and strict citation footers

use std::path::PathBuf;

/// Canonical AGENT.md embedded at compile time for zero-dependency standalone binaries.
pub const EMBEDDED_AGENT_MD: &str = include_str!("../../../config/AGENT.md");

/// Canonical README.md embedded at compile time for system self-knowledge.
pub const EMBEDDED_README_MD: &str = include_str!("../../../README.md");

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

/// Constructs the comprehensive system instruction prompt matching Python parity.
pub fn build_system_prompt(
    workspace: &str,
    grounding_mode: &str,
    search_mode: &str,
    web_search_enabled: bool,
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

    // Specialized Agent Skills (Lean & Token-Efficient)
    prompt.push_str(
        "### 🧩 SPECIALIZED AGENT SKILLS:\n\
         - **Skill `system-knowledge`**: You are the main conversational interface of AnyContext (actx). When the user asks questions about AnyContext itself (capabilities, how it works, supported commands like `/folder`, `/switch`, `/sync`, `/mode`, options, keyboard shortcuts, or workflows), you MUST call `search_db` to retrieve the authoritative documentation from the knowledge base and explain it clearly in Portuguese or the user's language.\n\
         - **Skill `system-status`**: When the user asks about background tasks, synchronization, or indexing status (e.g. \"Já foi tudo indexado?\", \"Qual o status do sync?\", \"O que está rodando em segundo plano?\"), you MUST call the tool `system_status` to inspect the live background synchronization telemetry and report the real-time indexing status clearly.\n\n"
    );

    // Active Grounding Mode Directives
    match effective_mode {
        "strict" => {
            prompt.push_str(
                "### 🛡️ ACTIVE GROUNDING MODE: STRICT (AUDIT & LEGAL - 100% FACTUAL & ZERO PARAMETRIC ANSWERS)\n\
                 - **ZERO GENERAL KNOWLEDGE / ZERO PARAMETRIC MEMORY:** You are an internal workspace assistant. You are STRICTLY FORBIDDEN from using your pre-trained weights or general knowledge to answer ANY question (including recipes, general knowledge, external facts, outside programs, or trivia). You must answer SOLELY based on facts returned by `search_db` or `system_status`.\n\
                 - **MANDATORY AUTONOMOUS RETRIEVAL:** You MUST call `search_db` or `system_status` immediately on EVERY user query before answering. Do NOT answer directly without calling your tools first!\n\
                 - **FACTUAL ABSENCE PROTOCOL:** If `search_db` returns no relevant chunks for the user's query:\n\
                   1. DO NOT invent or synthesize an answer from training memory or general knowledge.\n\
                   2. State clearly: \"⚠️ Essa informação não consta nos documentos deste workspace.\"\n\
                   3. Mention which workspace is active and suggest relevant keywords, topics, or adding folder/web sources.\n\
                   4. NEVER output fictitious source citations or placeholders like `[Nome_do_Arquivo.ext]`.\n\
                 - **MANDATORY SOURCE CITATIONS:** Conclude every answer that used documents with:\n\
                   ---\n\
                   📄 **Fontes Consultadas (Arquivos Locais):**\n\
                   - `<nome_real_do_arquivo>` (Caminho ou URL real retornado pelo search_db)\n\
                   NEVER invent filenames or output literal placeholder strings like `[Nome_do_Arquivo.ext]`.\n\n"
            );
        }
        "hybrid" => {
            prompt.push_str(
                "### ⚖️ ACTIVE GROUNDING MODE: HYBRID (BALANCED - WORKSPACE FIRST + LABELED MODEL KNOWLEDGE)\n\
                 - **WORKSPACE PRIORITY:** Query `search_db` first. Present local workspace facts first.\n\
                 - **DUAL-LAYER STRUCTURE:**\n\
                   `### 📂 Informações do Workspace` (baseado nos documentos locais com citações)\n\
                   `### 💡 Conhecimento Geral do Modelo` (conhecimento paramétrico devidamente rotulado)\n\
                 - **PARAMETRIC MEMORY TRANSPARENCY:** Disclose general knowledge with: \"De acordo com meus conhecimentos gerais...\"\n\
                 - **MANDATORY CITATIONS:** Conclude with `📄 Fontes Consultadas:` listing documents consulted.\n\n"
            );
        }
        "proactive" => {
            prompt.push_str(
                "### 🚀 ACTIVE GROUNDING MODE: PROACTIVE (RESEARCH & STRATEGY)\n\
                 - Freely combine workspace documents, web intelligence, and domain knowledge.\n\
                 - Tag each insight by source (`[Documento: ...]`, `[Web: ...]`, `[Recomendação]`).\n\
                 - NEVER output generic template placeholder phrases such as `[insira o tópico...]` or fabricate past session details.\n\
                 - If the user asks about prior conversations or historical context and no record is present in this workspace, state honestly that no previous record was found.\n\
                 - Conclude with `📄 Fontes Consultadas:`.\n\n"
            );
        }
        _ => {}
    }

    // Web Search Engine Directives
    if web_search_enabled {
        prompt.push_str(
            "### 🌐 LIVE WEB SEARCH ENGINE: ACTIVE\n\
             - You have access to real-time web search capabilities.\n\
             - In STRICT mode, web search is PERMISSION-GATED: only search the web if the user explicitly commanded online search or confirmed permission.\n\
             - In HYBRID and PROACTIVE modes, you may query complementary web facts autonomously.\n\n"
        );
    } else {
        prompt.push_str(
            "### 🔒 LIVE WEB SEARCH: DISABLED (OFFLINE-FIRST LOCAL ISOLATION)\n\
             - Web search is DISABLED for this workspace. Answer exclusively using local workspace documents.\n\n"
        );
    }

    // Universal Temporal Recency Rule
    prompt.push_str(
        "### ⏱️ UNIVERSAL TEMPORAL RECENCY RULE:\n\
         - Whenever multiple sources within the same priority tier contain overlapping or differing information, THE MOST RECENT SOURCE ALWAYS PREVAILS AND SUPERSEDES OLDER DATA.\n\n"
    );

    // Language Consistency
    prompt.push_str(
        "### 🗣️ LANGUAGE CONSISTENCY:\n\
         - ALWAYS answer in the exact language used by the user in their prompt. (If the user asks in Portuguese, reply in Portuguese. If in English, reply in English).\n"
    );

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_agent_md_not_empty() {
        assert!(!EMBEDDED_AGENT_MD.trim().is_empty());
        assert!(EMBEDDED_AGENT_MD.contains("Mandatory Retrieval Strategy"));
        assert!(EMBEDDED_AGENT_MD.contains("search_db"));
    }

    #[test]
    fn test_build_system_prompt_strict_mode() {
        let p = build_system_prompt("TestWS", "strict", "auto", false);
        assert!(p.contains("Active Workspace: 'TestWS'"));
        assert!(p.contains("GROUNDING MODE: STRICT"));
        assert!(p.contains("ZERO PARAMETRIC MEMORY"));
        assert!(p.contains("MANDATORY AUTONOMOUS RETRIEVAL"));
        assert!(p.contains("system_status"));
        assert!(p.contains("LIVE WEB SEARCH: DISABLED"));
        assert!(p.contains("Fontes Consultadas"));
    }

    #[test]
    fn test_build_system_prompt_hybrid_mode() {
        let p = build_system_prompt("TestWS", "hybrid", "fast", true);
        assert!(p.contains("GROUNDING MODE: HYBRID"));
        assert!(p.contains("Informações do Workspace"));
        assert!(p.contains("Conhecimento Geral do Modelo"));
        assert!(p.contains("LIVE WEB SEARCH ENGINE: ACTIVE"));
    }

    #[test]
    fn test_build_system_prompt_proactive_mode() {
        let p = build_system_prompt("TestWS", "proactive", "deep", false);
        assert!(p.contains("GROUNDING MODE: PROACTIVE"));
        assert!(p.contains("RESEARCH & STRATEGY"));
    }
}
