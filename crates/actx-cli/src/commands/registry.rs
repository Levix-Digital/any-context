#[derive(Debug, Clone, Copy)]
pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub usage: &'static str,
    pub category: &'static str,
}

pub const DEFAULT_SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "help",
        description: "Displays comprehensive command help and usage guide",
        usage: "/help [command]",
        category: "General",
    },
    SlashCommand {
        name: "workspace",
        description: "Switches or lists context workspaces",
        usage: "/workspace [name]",
        category: "Context",
    },
    SlashCommand {
        name: "sync",
        description: "Performs incremental SHA-256 sync of documents and folders",
        usage: "/sync [--force]",
        category: "Context",
    },
    SlashCommand {
        name: "model",
        description: "Inspects or selects the active LLM/SLM provider model",
        usage: "/model [name]",
        category: "Engine",
    },
    SlashCommand {
        name: "clear",
        description: "Clears the active chat viewport buffer",
        usage: "/clear",
        category: "General",
    },
    SlashCommand {
        name: "history",
        description: "Displays conversation turns from active session history",
        usage: "/history",
        category: "Session",
    },
    SlashCommand {
        name: "info",
        description: "Displays workspace statistics, chunk count, and model tier",
        usage: "/info",
        category: "Context",
    },
    SlashCommand {
        name: "status",
        description: "Displays system health, LanceDB vector index and engine status",
        usage: "/status",
        category: "General",
    },
    SlashCommand {
        name: "exit",
        description: "Gracefully terminates the AnyContext terminal session",
        usage: "/exit",
        category: "General",
    },
    SlashCommand {
        name: "quit",
        description: "Alias for /exit",
        usage: "/quit",
        category: "General",
    },
    SlashCommand {
        name: "keys",
        description: "Configures or audits API credentials in the local secure vault",
        usage: "/keys [provider] [key]",
        category: "Config",
    },
    SlashCommand {
        name: "config",
        description: "Opens or views persistent SQLite configuration settings",
        usage: "/config [key] [val]",
        category: "Config",
    },
    SlashCommand {
        name: "diagnostics",
        description: "Generates an end-to-end diagnostic report",
        usage: "/diagnostics",
        category: "Observability",
    },
    SlashCommand {
        name: "logs",
        description: "Views recent observability logs from local storage",
        usage: "/logs [--limit N]",
        category: "Observability",
    },
    SlashCommand {
        name: "update",
        description: "Checks for updates or executes atomic self-update",
        usage: "/update [--check]",
        category: "System",
    },
    SlashCommand {
        name: "fast",
        description: "Executes next query using Fast RAG mode (single-turn)",
        usage: "/fast <query>",
        category: "RAG",
    },
    SlashCommand {
        name: "deep",
        description: "Executes next query using Deep Search Reflexive mode",
        usage: "/deep <query>",
        category: "RAG",
    },
    SlashCommand {
        name: "search",
        description: "Configures workspace search policy: auto, fast, or deep",
        usage: "/search <auto|fast|deep>",
        category: "RAG",
    },
    SlashCommand {
        name: "inspect",
        description: "Inspects indexed chunks and document taxonomy breakdown",
        usage: "/inspect [source]",
        category: "Context",
    },
    SlashCommand {
        name: "purge",
        description: "Purges indexed documents or vectors from active workspace",
        usage: "/purge [--all]",
        category: "Context",
    },
];

pub fn find_command(query: &str) -> Option<&'static SlashCommand> {
    let clean = query.trim_start_matches('/').trim().to_lowercase();
    DEFAULT_SLASH_COMMANDS.iter().find(|cmd| cmd.name == clean)
}

pub fn autocomplete_commands(prefix: &str) -> Vec<&'static SlashCommand> {
    let clean = prefix.trim_start_matches('/').trim().to_lowercase();
    DEFAULT_SLASH_COMMANDS
        .iter()
        .filter(|cmd| cmd.name.starts_with(&clean))
        .collect()
}
