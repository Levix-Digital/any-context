export interface SlashCommandMeta {
  command: string;
  args: string;
  description: string;
  category: string;
  direct_execution?: boolean;
  aliases?: string[];
}

export const DEFAULT_SLASH_COMMANDS: SlashCommandMeta[] = [
  { command: "/switch", args: "[workspace]", description: "Switch, list, or select active workspace", category: "Workspace", direct_execution: true, aliases: ["/workspace"] },
  { command: "/model", args: "<name>", description: "Change active AI inference model", category: "Model", direct_execution: false, aliases: ["/m"] },
  { command: "/mode", args: "[strategy]", description: "Select AI Grounding Strategy mode", category: "AI Grounding", direct_execution: true, aliases: ["/grounding"] },
  { command: "/web-search", args: "[on|off]", description: "Toggle real-time workspace Web Search", category: "Web Search", direct_execution: true, aliases: ["/search"] },
  { command: "/sync", args: "[--force|--status]", description: "Synchronize local folders and web sources", category: "Sources", direct_execution: true, aliases: ["/reindex"] },
  { command: "/sources", args: "[--all|--delete]", description: "List or delete indexed documents and web portals in workspace", category: "Sources", direct_execution: true, aliases: ["/list-sources"] },
  { command: "/folder", args: "--add <path>", description: "Add, list, or remove local folder from workspace", category: "Sources", direct_execution: false, aliases: ["/dir"] },
  { command: "/web", args: "--add <url>", description: "Add, list, or crawl documentation portal or web URL", category: "Sources", direct_execution: false, aliases: ["/url"] },
  { command: "/transfer", args: "<from_ws> <to_ws> <item>", description: "Transfer source to another workspace in <50ms ($0.00)", category: "Sources", direct_execution: false, aliases: ["/move-source"] },
  { command: "/link", args: "<source> [ws]", description: "Link shared source across workspaces", category: "Sources", direct_execution: false, aliases: [] },
  { command: "/unlink", args: "<source>", description: "Unlink shared source from workspace", category: "Sources", direct_execution: false, aliases: [] },
  { command: "/shared", args: "", description: "List reusable indexed shared sources", category: "Sources", direct_execution: true, aliases: [] },
  { command: "/rename", args: "<old> <new>", description: "Rename custom workspace and migrate vector records", category: "Workspace", direct_execution: false, aliases: [] },
  { command: "/config", args: "", description: "Open interactive configuration and settings wizard", category: "System", direct_execution: true, aliases: ["/settings"] },
  { command: "/key", args: "<provider> <api-key>", description: "Configure AI provider API keys", category: "System", direct_execution: false, aliases: ["/api-key"] },
  { command: "/models", args: "--list", description: "Display catalog of available AI models", category: "Model", direct_execution: true, aliases: [] },
  { command: "/billing", args: "", description: "Manage subscription tier and plans", category: "System", direct_execution: true, aliases: ["/plan", "/pricing"] },
  { command: "/reset-memory", args: "", description: "Reset long-term session memory database", category: "Memory", direct_execution: true, aliases: ["/forget"] },
  { command: "/clear", args: "", description: "Clear visual chat history view", category: "Chat", direct_execution: true, aliases: ["/cls"] },
  { command: "/paste", args: "", description: "Enter dedicated multi-line capture mode", category: "Chat", direct_execution: true, aliases: ["/multiline", "/mline"] },
  { command: "/help", args: "[command]", description: "Display interactive help documentation", category: "Help", direct_execution: true, aliases: ["/menu", "/commands", "/slash"] },
  { command: "/version", args: "", description: "Display AnyContext version and build info", category: "System", direct_execution: true, aliases: ["/v"] },
  { command: "/check-update", args: "", description: "Check if a newer AnyContext release is available on GitHub", category: "System", direct_execution: true, aliases: ["/check"] },
  { command: "/update", args: "[version]", description: "Download and install latest AnyContext release", category: "System", direct_execution: true, aliases: ["/self-update", "/upgrade"] },
  { command: "/inspect", args: "[limit]", description: "Inspect vector database chunks and session memory", category: "Sources", direct_execution: true, aliases: ["/chunks", "/lance"] },
  { command: "/density", args: "[level]", description: "Configure UI compact/comfortable visual density", category: "System", direct_execution: true, aliases: [] },
  { command: "/history", args: "[clear]", description: "View or clear recent conversation history", category: "Chat", direct_execution: true, aliases: ["/clear-history"] },
  { command: "/menu", args: "", description: "Open interactive Slash Command Palette", category: "Help", direct_execution: true, aliases: ["/palette"] },
  { command: "/logs", args: "[limit]", description: "Display recent system observability and execution logs", category: "System", direct_execution: true, aliases: ["/log"] },
  { command: "/diagnostics", args: "", description: "Inspect system health, Bun runtime, database, and latency metrics", category: "System", direct_execution: true, aliases: ["/diag", "/perf", "/health"] },
  { command: "/onboarding", args: "", description: "Launch first-time AI onboarding and API key setup wizard", category: "System", direct_execution: true, aliases: ["/setup"] },
  { command: "/exit", args: "", description: "Save session memory and exit", category: "System", direct_execution: true, aliases: ["/quit", "/q"] }
];

export const MAX_PALETTE_ITEMS = 6;

/**
 * Pure filtering function for slash commands with relevance-based scoring.
 * Prioritizes prefix matches on command name over aliases, substring, description, and category matches.
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
      const aliasCleans = (c.aliases || []).map((a) => (a.startsWith("/") ? a.slice(1).toLowerCase() : a.toLowerCase()));

      let score = 999;
      if (cmdClean === query || cmdClean.startsWith(query)) {
        score = 1; // Highest priority: exact or prefix match on full command
      } else if (aliasCleans.some((a) => a === query || a.startsWith(query))) {
        score = 1.5; // High priority: exact or prefix match on alias
      } else if (query.includes(" ") && (cmdClean.startsWith(queryBase) || aliasCleans.some((a) => a.startsWith(queryBase)))) {
        score = 2; // Subcommand / flag match when base command matches
      } else if (cmdClean.includes(query) || aliasCleans.some((a) => a.includes(query))) {
        score = 3; // Substring in command name or alias
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
