export interface SlashCommandMeta {
  command: string;
  args: string;
  description: string;
  category: string;
  direct_execution?: boolean;
}

export const DEFAULT_SLASH_COMMANDS: SlashCommandMeta[] = [
  { command: "/sources", args: "", description: "List indexed documents and web portals in active workspace", category: "Sources", direct_execution: true },
  { command: "/sources --all", args: "", description: "List all indexed sources across all workspaces", category: "Sources", direct_execution: true },
  { command: "/sync", args: "", description: "Synchronize local folders and web sources", category: "Sources", direct_execution: true },
  { command: "/sync --force", args: "", description: "Force full re-indexing of all documents and web portals", category: "Sources", direct_execution: true },
  { command: "/switch", args: "[workspace]", description: "Switch, list, or select active workspace", category: "Workspace", direct_execution: true },
  { command: "/model", args: "<name>", description: "Change active AI inference model", category: "Model", direct_execution: false },
  { command: "/mode", args: "[strategy]", description: "Select AI Grounding Strategy mode", category: "AI Grounding", direct_execution: true },
  { command: "/web-search", args: "[--on|--off]", description: "Toggle real-time workspace Web Search", category: "Web Search", direct_execution: true },
  { command: "/web-search --on", args: "", description: "Enable real-time Web Search grounding for active workspace", category: "Web Search", direct_execution: true },
  { command: "/web-search --off", args: "", description: "Disable real-time Web Search grounding for active workspace", category: "Web Search", direct_execution: true },
  { command: "/folder", args: "--add <path>", description: "Add, list, or remove local folder from workspace", category: "Sources", direct_execution: false },
  { command: "/web", args: "--add <url>", description: "Add, list, or crawl documentation portal or web URL", category: "Sources", direct_execution: false },
  { command: "/transfer", args: "<from_ws> <to_ws> <item>", description: "Transfer source to another workspace in <50ms ($0.00)", category: "Sources", direct_execution: false },
  { command: "/link", args: "<source> [ws]", description: "Link shared source across workspaces", category: "Sources", direct_execution: false },
  { command: "/unlink", args: "<source>", description: "Unlink shared source from workspace", category: "Sources", direct_execution: false },
  { command: "/shared", args: "", description: "List reusable indexed shared sources", category: "Sources", direct_execution: true },
  { command: "/rename", args: "<old> <new>", description: "Rename custom workspace and migrate vector records", category: "Workspace", direct_execution: false },
  { command: "/config", args: "", description: "Open interactive configuration and settings wizard", category: "System", direct_execution: true },
  { command: "/key", args: "<provider> <api-key>", description: "Configure AI provider API keys", category: "System", direct_execution: false },
  { command: "/models", args: "--list", description: "Display catalog of available AI models", category: "Model", direct_execution: true },
  { command: "/billing", args: "", description: "Manage subscription tier and plans", category: "System", direct_execution: true },
  { command: "/reset-memory", args: "", description: "Reset long-term session memory database", category: "Memory", direct_execution: true },
  { command: "/clear", args: "", description: "Clear visual chat history view", category: "Chat", direct_execution: true },
  { command: "/paste", args: "", description: "Enter dedicated multi-line capture mode", category: "Chat", direct_execution: true },
  { command: "/version", args: "", description: "Display AnyContext version and build info", category: "System", direct_execution: true },
  { command: "/check-update", args: "", description: "Check for available AnyContext updates from GitHub", category: "System", direct_execution: true },
  { command: "/update", args: "[version]", description: "Check and install latest AnyContext binary releases", category: "System", direct_execution: true },
  { command: "/inspect", args: "[limit]", description: "Inspect vector database chunks and session memory", category: "Sources", direct_execution: true },
  { command: "/density", args: "[level]", description: "Configure UI compact/comfortable visual density", category: "System", direct_execution: true },
  { command: "/history", args: "[clear]", description: "View or clear recent conversation history", category: "Chat", direct_execution: true },
  { command: "/help", args: "[command]", description: "Display interactive help documentation", category: "Help", direct_execution: true },
  { command: "/menu", args: "", description: "Open interactive Slash Command Palette", category: "Help", direct_execution: true },
  { command: "/exit", args: "", description: "Save session memory and exit", category: "System", direct_execution: true }
];

export const MAX_PALETTE_ITEMS = 6;

/**
 * Pure filtering function for slash commands with relevance-based scoring.
 * Prioritizes prefix matches on command name over substring, description, and category matches.
 */
export function filterSlashCommands(commands: SlashCommandMeta[], filterText: string): SlashCommandMeta[] {
  const list = commands && commands.length > 0 ? commands : DEFAULT_SLASH_COMMANDS;
  const raw = filterText || "";
  const query = raw.startsWith("/") ? raw.slice(1).toLowerCase().trim() : raw.toLowerCase().trim();

  if (!query) {
    return list;
  }

  const queryBase = query.split(" ")[0];

  const scored = list
    .map((c) => {
      const cmdName = c.command.toLowerCase();
      const cmdClean = cmdName.startsWith("/") ? cmdName.slice(1) : cmdName;
      const desc = c.description.toLowerCase();
      const cat = c.category.toLowerCase();

      let score = 999;
      if (cmdClean === query || cmdClean.startsWith(query)) {
        score = 1; // Highest priority: exact or prefix match on full command (including flags)
      } else if (query.includes(" ") && cmdClean.startsWith(queryBase)) {
        score = 2; // Subcommand / flag match when base command matches
      } else if (cmdClean.includes(query)) {
        score = 3; // Substring in command name
      } else if (desc.includes(query)) {
        score = 4; // Substring in description
      } else if (cat.includes(query)) {
        score = 5; // Substring in category
      }

      return { cmd: c, score };
    })
    .filter((item) => item.score < 999);

  scored.sort((a, b) => a.score - b.score);

  return scored.map((item) => item.cmd);
}

/**
 * Determines whether a command can be executed immediately without requiring user-provided arguments.
 */
export function isDirectExecutionCommand(cmd: SlashCommandMeta): boolean {
  if (cmd.direct_execution !== undefined) {
    return cmd.direct_execution;
  }
  if (!cmd.args || cmd.args.trim() === "") {
    return true;
  }
  const cleanArgs = cmd.args.trim();
  return cleanArgs.startsWith("[");
}
