export interface SlashCommandMeta {
  command: string;
  args: string;
  description: string;
  category: string;
  direct_execution?: boolean;
}

export const DEFAULT_SLASH_COMMANDS: SlashCommandMeta[] = [
  { command: "/switch", args: "<workspace>", description: "Switch, list, or create active workspace", category: "Workspace", direct_execution: false },
  { command: "/model", args: "<name>", description: "Change active AI inference model", category: "Model", direct_execution: false },
  { command: "/mode", args: "<strict|hybrid|proactive>", description: "Change Grounding Strategy mode", category: "AI Grounding", direct_execution: false },
  { command: "/web-search", args: "[on|off]", description: "Toggle real-time workspace Web Search", category: "Web Search", direct_execution: true },
  { command: "/sync", args: "[--force|--status]", description: "Synchronize local folders and web sources", category: "Sources", direct_execution: true },
  { command: "/sources", args: "[--all]", description: "List indexed documents and web portals in workspace", category: "Sources", direct_execution: true },
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
  { command: "/help", args: "[command]", description: "Display interactive help documentation", category: "Help", direct_execution: true },
  { command: "/version", args: "", description: "Display AnyContext version and build info", category: "System", direct_execution: true },
  { command: "/exit", args: "", description: "Save session memory and exit", category: "System", direct_execution: true }
];

export const MAX_PALETTE_ITEMS = 6;

/**
 * Pure filtering function for slash commands.
 * Matches against command name (with or without '/'), description, and category.
 */
export function filterSlashCommands(commands: SlashCommandMeta[], filterText: string): SlashCommandMeta[] {
  const list = commands && commands.length > 0 ? commands : DEFAULT_SLASH_COMMANDS;
  const raw = filterText || "";
  const query = raw.startsWith("/") ? raw.slice(1).toLowerCase().trim() : raw.toLowerCase().trim();

  if (!query) {
    return list;
  }

  return list.filter((c) => {
    const cmdName = c.command.toLowerCase();
    const cmdClean = cmdName.startsWith("/") ? cmdName.slice(1) : cmdName;
    const desc = c.description.toLowerCase();
    const cat = c.category.toLowerCase();
    return cmdClean.includes(query) || desc.includes(query) || cat.includes(query);
  });
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
