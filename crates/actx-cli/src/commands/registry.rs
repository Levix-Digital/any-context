#[derive(Debug, Clone, Copy)]
pub struct SlashCommand {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub usage: &'static str,
    pub category: &'static str,
}

pub const DEFAULT_SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "switch",
        aliases: &["workspace", "workspaces", "ws"],
        description: "Lists all workspaces or switches active context workspace",
        usage: "/switch [name] [--delete <name>]",
        category: "Workspace",
    },
    SlashCommand {
        name: "sync",
        aliases: &["reindex", "resync", "index"],
        description: "Performs incremental SHA-256 sync of documents and folders",
        usage: "/sync [--force]",
        category: "Sources",
    },
    SlashCommand {
        name: "model",
        aliases: &["m"],
        description: "Inspects or selects the active LLM/SLM provider model",
        usage: "/model [name]",
        category: "Engine",
    },
    SlashCommand {
        name: "models",
        aliases: &["list-models", "model-list"],
        description: "Displays catalog of supported AI models and providers",
        usage: "/models",
        category: "Engine",
    },
    SlashCommand {
        name: "sources",
        aliases: &["source", "list-sources", "src"],
        description: "Lists indexed local folders and web documentation sources",
        usage: "/sources [--all]",
        category: "Sources",
    },
    SlashCommand {
        name: "diagnostics",
        aliases: &["diag", "perf", "health", "diagnistics"],
        description: "Inspects system health, memory, database, and latency metrics",
        usage: "/diagnostics",
        category: "System",
    },
    SlashCommand {
        name: "status",
        aliases: &[],
        description: "Displays system health, LanceDB vector index and engine status",
        usage: "/status",
        category: "System",
    },
    SlashCommand {
        name: "info",
        aliases: &[],
        description: "Displays workspace statistics, chunk count, and model tier",
        usage: "/info",
        category: "Workspace",
    },
    SlashCommand {
        name: "keys",
        aliases: &["key", "api-key", "api-keys"],
        description: "Audits AI provider API credentials in environment and vault",
        usage: "/keys",
        category: "Config",
    },
    SlashCommand {
        name: "config",
        aliases: &["settings"],
        description: "Opens or views persistent SQLite configuration settings",
        usage: "/config [key] [val]",
        category: "Config",
    },
    SlashCommand {
        name: "history",
        aliases: &[],
        description: "Displays conversation turns from active session history",
        usage: "/history",
        category: "Session",
    },
    SlashCommand {
        name: "clear",
        aliases: &["cls"],
        description: "Clears the active chat viewport buffer",
        usage: "/clear",
        category: "General",
    },
    SlashCommand {
        name: "version",
        aliases: &["v"],
        description: "Displays AnyContext version, build, and runtime details",
        usage: "/version",
        category: "System",
    },
    SlashCommand {
        name: "logs",
        aliases: &["log"],
        description: "Views recent observability logs from local storage",
        usage: "/logs [--limit N]",
        category: "Observability",
    },
    SlashCommand {
        name: "update",
        aliases: &["self-update", "upgrade"],
        description: "Checks for updates or executes atomic self-update",
        usage: "/update [--check]",
        category: "System",
    },
    SlashCommand {
        name: "fast",
        aliases: &[],
        description: "Executes next query using Fast RAG mode (single-turn)",
        usage: "/fast <query>",
        category: "RAG",
    },
    SlashCommand {
        name: "deep",
        aliases: &[],
        description: "Executes next query using Deep Search Reflexive mode",
        usage: "/deep <query>",
        category: "RAG",
    },
    SlashCommand {
        name: "search",
        aliases: &[],
        description: "Configures workspace search policy: auto, fast, or deep",
        usage: "/search <auto|fast|deep>",
        category: "RAG",
    },
    SlashCommand {
        name: "inspect",
        aliases: &["chunks", "lance"],
        description: "Inspects indexed chunks and document taxonomy breakdown",
        usage: "/inspect [source]",
        category: "Sources",
    },
    SlashCommand {
        name: "purge",
        aliases: &[],
        description: "Purges indexed documents or vectors from active workspace",
        usage: "/purge [--all]",
        category: "Sources",
    },
    SlashCommand {
        name: "folder",
        aliases: &["add", "dir"],
        description: "Add, list, or remove local folder from workspace",
        usage: "/folder [--add <path>|--remove <path>]",
        category: "Sources",
    },
    SlashCommand {
        name: "web",
        aliases: &["url"],
        description: "Add, list, or crawl documentation portal or web URL",
        usage: "/web [--add <url>|--remove <url>]",
        category: "Sources",
    },
    SlashCommand {
        name: "transfer",
        aliases: &["move-source"],
        description: "Transfer source to another workspace in <50ms ($0.00)",
        usage: "/transfer <from_ws> <to_ws> <item>",
        category: "Sources",
    },
    SlashCommand {
        name: "link",
        aliases: &[],
        description: "Link shared source across workspaces",
        usage: "/link <source> [target_ws]",
        category: "Sources",
    },
    SlashCommand {
        name: "unlink",
        aliases: &[],
        description: "Unlink shared source from workspace",
        usage: "/unlink <source>",
        category: "Sources",
    },
    SlashCommand {
        name: "shared",
        aliases: &[],
        description: "List reusable indexed shared sources",
        usage: "/shared",
        category: "Sources",
    },
    SlashCommand {
        name: "rename",
        aliases: &[],
        description: "Rename custom workspace and migrate vector records",
        usage: "/rename <old> <new>",
        category: "Workspace",
    },
    SlashCommand {
        name: "mode",
        aliases: &["grounding", "grounding-mode", "answer-mode", "am"],
        description: "Select AI Grounding Strategy mode (strict, hybrid, proactive, auto)",
        usage: "/mode [--strict|--hybrid|--proactive|--auto]",
        category: "RAG",
    },
    SlashCommand {
        name: "web-search",
        aliases: &["websearch"],
        description: "Toggle real-time workspace Web Search (on|off)",
        usage: "/web-search [on|off]",
        category: "RAG",
    },
    SlashCommand {
        name: "billing",
        aliases: &["plan", "pricing"],
        description: "Manage subscription tier and plans",
        usage: "/billing",
        category: "System",
    },
    SlashCommand {
        name: "reset-memory",
        aliases: &["forget"],
        description: "Reset long-term session memory database",
        usage: "/reset-memory",
        category: "Session",
    },
    SlashCommand {
        name: "paste",
        aliases: &["multiline", "mline"],
        description: "Enter dedicated multi-line capture mode",
        usage: "/paste",
        category: "General",
    },
    SlashCommand {
        name: "check-update",
        aliases: &["check"],
        description: "Check if a newer AnyContext release is available on GitHub",
        usage: "/check-update",
        category: "System",
    },
    SlashCommand {
        name: "density",
        aliases: &[],
        description: "Configure UI compact/comfortable visual density",
        usage: "/density [compact|comfortable]",
        category: "System",
    },
    SlashCommand {
        name: "spans",
        aliases: &[],
        description: "Display recent performance spans and latency timings",
        usage: "/spans [--limit N]",
        category: "Observability",
    },
    SlashCommand {
        name: "onboarding",
        aliases: &["setup"],
        description: "Launch guided AI onboarding and setup wizard",
        usage: "/onboarding",
        category: "System",
    },
    SlashCommand {
        name: "vision",
        aliases: &["vis"],
        description: "Toggle Multimodal Vision LLM ingestion (on|off)",
        usage: "/vision [on|off]",
        category: "Engine",
    },
    SlashCommand {
        name: "ocr",
        aliases: &["scan"],
        description: "Inspect OCR engine status and configuration",
        usage: "/ocr",
        category: "Engine",
    },
    SlashCommand {
        name: "exit",
        aliases: &["quit", "q"],
        description: "Gracefully terminates the AnyContext terminal session",
        usage: "/exit",
        category: "General",
    },
    SlashCommand {
        name: "menu",
        aliases: &["palette"],
        description: "Opens interactive full-screen configuration and workspace menu",
        usage: "/menu",
        category: "General",
    },
    SlashCommand {
        name: "help",
        aliases: &["commands", "slash"],
        description: "Displays comprehensive command help and usage guide",
        usage: "/help [command]",
        category: "General",
    },
];

pub fn find_command(query: &str) -> Option<&'static SlashCommand> {
    let clean = query.trim_start_matches('/').trim().to_lowercase();
    DEFAULT_SLASH_COMMANDS
        .iter()
        .find(|cmd| cmd.name == clean || cmd.aliases.contains(&clean.as_str()))
}

pub fn autocomplete_commands(prefix: &str) -> Vec<&'static SlashCommand> {
    let clean = prefix.trim_start_matches('/').trim().to_lowercase();
    DEFAULT_SLASH_COMMANDS
        .iter()
        .filter(|cmd| {
            cmd.name.starts_with(&clean) || cmd.aliases.iter().any(|a| a.starts_with(&clean))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_command_direct_and_aliases() {
        assert_eq!(find_command("switch").unwrap().name, "switch");
        assert_eq!(find_command("/switch").unwrap().name, "switch");
        assert_eq!(find_command("workspace").unwrap().name, "switch");
        assert_eq!(find_command("/workspaces").unwrap().name, "switch");

        assert_eq!(find_command("sync").unwrap().name, "sync");
        assert_eq!(find_command("reindex").unwrap().name, "sync");

        assert_eq!(find_command("diagnostics").unwrap().name, "diagnostics");
        assert_eq!(find_command("diag").unwrap().name, "diagnostics");
        assert_eq!(find_command("perf").unwrap().name, "diagnostics");
        assert_eq!(find_command("health").unwrap().name, "diagnostics");

        assert_eq!(find_command("quit").unwrap().name, "exit");
        assert_eq!(find_command("q").unwrap().name, "exit");
    }

    #[test]
    fn test_autocomplete_aliases() {
        let matches = autocomplete_commands("work");
        assert!(matches.iter().any(|c| c.name == "switch"));
    }
}
