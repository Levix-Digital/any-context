import { spawn, ChildProcess } from "child_process";
import * as path from "path";
import * as fs from "fs";
import * as readline from "readline";

export interface AnyContextState {
  version: string;
  workspace: string;
  model: string;
  grounding_mode: string;
  web_search_enabled: boolean;
  sync_info: string;
  is_syncing: boolean;
  tier_name?: string;
}

import { DEFAULT_SLASH_COMMANDS } from "./commands";
import type { SlashCommandMeta } from "./commands";
export type { SlashCommandMeta };

export interface CommandExecutionResult {
  success: boolean;
  message: string;
  action?: string;
  error?: string;
  state: AnyContextState;
  state_updates?: Record<string, any>;
}

export interface OptionItemSchema {
  id: string;
  title: string;
  description?: string;
  icon?: string;
  badge?: string;
  is_active: boolean;
  metadata?: Record<string, any>;
}

export interface OptionsGroupSchema {
  type: string;
  title: string;
  description?: string;
  active_id?: string;
  items: OptionItemSchema[];
}

export interface MenuItemSchema {
  id: string;
  title: string;
  description?: string;
  icon?: string;
  badge?: string;
  type: "submenu" | "action" | "toggle" | "select" | "input";
  command_shortcut?: string;
  is_active?: boolean;
  current_value?: string;
  options?: OptionItemSchema[];
  subitems?: MenuItemSchema[];
  metadata?: Record<string, any>;
}

export interface MenuTreeSchema {
  menu_id: string;
  title: string;
  subtitle?: string;
  workspace: string;
  breadcrumbs: string[];
  items: MenuItemSchema[];
}

export interface MenuActionResult {
  success: boolean;
  message: string;
  error?: string;
  state_updates?: Record<string, any>;
  next_menu_id?: string;
  action?: string;
}

export interface StreamCallbacks {
  onToken: (token: string) => void;
  onTicker: (ticker: string) => void;
  onDone: (fullReply: string) => void;
  onError: (error: string) => void;
}

export class BridgeClient {
  private process: ChildProcess | null = null;
  private reqIdCounter = 0;
  private pendingRequests = new Map<number, { resolve: (res: any) => void; reject: (err: any) => void }>();
  private activeStreams = new Map<number, StreamCallbacks>();
  public state: AnyContextState = {
    version: "0.28.38",
    workspace: "Default",
    model: "...",
    grounding_mode: "strict",
    web_search_enabled: false,
    sync_info: "",
    is_syncing: false,
  };
  public commands: SlashCommandMeta[] = [...DEFAULT_SLASH_COMMANDS];
  public onStateChange?: (state: AnyContextState) => void;

  constructor(private initialWorkspace: string = "Default") {
    this.state.workspace = initialWorkspace;
  }

  public async start(): Promise<void> {
    const repoRoot = path.resolve(__dirname, "..", "..", "..");
    
    let command = process.env.ACTX_EXECUTABLE || "";
    let args: string[] = [];

    if (command && (command.toLowerCase().endsWith("actx.exe") || command.toLowerCase().endsWith("actx"))) {
      args = ["--rpc", this.initialWorkspace];
    } else {
      const venvPythonWin = path.join(repoRoot, ".venv", "Scripts", "python.exe");
      const venvPythonUnix = path.join(repoRoot, ".venv", "bin", "python");

      if (fs.existsSync(venvPythonWin)) {
        command = venvPythonWin;
      } else if (fs.existsSync(venvPythonUnix)) {
        command = venvPythonUnix;
      } else {
        command = process.platform === "win32" ? "python" : "python3";
      }
      args = ["-m", "any_context.server.rpc_bridge", this.initialWorkspace];
    }

    const childEnv: Record<string, string> = {};
    for (const [key, value] of Object.entries(process.env)) {
      if (value !== undefined) {
        const lowerKey = key.toLowerCase();
        if (!lowerKey.startsWith("_mei") && !lowerKey.startsWith("pyi_") && !lowerKey.includes("meipass")) {
          childEnv[key] = value;
        }
      }
    }
    if (childEnv.PATH) {
      const separator = process.platform === "win32" ? ";" : ":";
      childEnv.PATH = childEnv.PATH.split(separator)
        .filter((p) => !p.toLowerCase().includes("_mei") && !p.toLowerCase().includes("pyi"))
        .join(separator);
    }
    if (path.isAbsolute(repoRoot)) {
      childEnv.PYTHONPATH = path.join(repoRoot, "src");
    }
    if (process.env.ACTX_SETTINGS_DB) {
      childEnv.ACTX_SETTINGS_DB = process.env.ACTX_SETTINGS_DB;
    }
    const callerCwd = process.env.ACTX_CALLER_CWD || process.cwd();
    childEnv.ACTX_CALLER_CWD = callerCwd;
    childEnv.ACTX_FRONTEND = "tui";

    this.process = spawn(command, args, {
      cwd: callerCwd,
      env: childEnv,
      stdio: ["pipe", "pipe", "pipe"],
    });

    if (!this.process.stdout || !this.process.stdin) {
      throw new Error("Failed to initialize stdin/stdout pipes for AnyContext RPC Bridge");
    }

    if (this.process.stderr) {
      this.process.stderr.on("data", () => {
        // Silently consume backend stderr logs so they never corrupt terminal text or input
      });
    }

    const rl = readline.createInterface({ input: this.process.stdout });
    rl.on("line", (line) => this.handleLine(line));

    this.process.on("exit", () => {
      // Backend process terminated cleanly
    });

    await this.refreshState();
    await this.fetchCommands();
  }

  private handleLine(line: string) {
    const trimmed = line.trim();
    if (!trimmed) return;

    try {
      const msg = JSON.parse(trimmed);

      if (msg.event === "ready" && msg.state) {
        this.updateState(msg.state);
        return;
      }

      const reqId = msg.id;

      if (reqId !== undefined && this.activeStreams.has(reqId)) {
        const stream = this.activeStreams.get(reqId)!;
        if (msg.type === "token") {
          stream.onToken(msg.content);
        } else if (msg.type === "ticker") {
          stream.onTicker(msg.content);
        } else if (msg.type === "done") {
          stream.onDone(msg.full_reply || "");
          this.activeStreams.delete(reqId);
        } else if (msg.type === "error") {
          stream.onError(msg.message || "Unknown error");
          this.activeStreams.delete(reqId);
        }
        return;
      }

      if (reqId !== undefined && this.pendingRequests.has(reqId)) {
        const { resolve, reject } = this.pendingRequests.get(reqId)!;
        this.pendingRequests.delete(reqId);

        if (msg.error) {
          reject(new Error(msg.error.message || "RPC Error"));
        } else {
          resolve(msg.result);
        }
      }
    } catch {
      // Ignore unparseable lines
    }
  }

  private sendRequest<T = any>(method: string, params: Record<string, any> = {}): Promise<T> {
    return new Promise((resolve, reject) => {
      if (!this.process || !this.process.stdin) {
        return reject(new Error("RPC bridge is not running"));
      }

      const id = ++this.reqIdCounter;
      this.pendingRequests.set(id, { resolve, reject });

      const payload = JSON.stringify({ id, method, params }) + "\n";
      this.process.stdin.write(payload);
    });
  }

  public async refreshState(): Promise<AnyContextState> {
    try {
      const state = await this.sendRequest<AnyContextState>("get_state");
      this.updateState(state);
      return state;
    } catch {
      return this.state;
    }
  }

  public async fetchCommands(): Promise<SlashCommandMeta[]> {
    try {
      const cmds = await this.sendRequest<SlashCommandMeta[]>("list_commands");
      this.commands = cmds || [];
      return this.commands;
    } catch {
      return [];
    }
  }

  public async switchWorkspace(workspaceName: string): Promise<AnyContextState> {
    const state = await this.sendRequest<AnyContextState>("switch_workspace", { workspace: workspaceName });
    this.updateState(state);
    return state;
  }

  public async setModel(modelName: string): Promise<AnyContextState> {
    const state = await this.sendRequest<AnyContextState>("set_model", { model: modelName });
    this.updateState(state);
    return state;
  }

  public async setMode(modeName: string): Promise<AnyContextState> {
    const state = await this.sendRequest<AnyContextState>("set_mode", { mode: modeName });
    this.updateState(state);
    return state;
  }

  public async setWebSearch(enabled: boolean): Promise<AnyContextState> {
    const state = await this.sendRequest<AnyContextState>("set_web_search", { enabled });
    this.updateState(state);
    return state;
  }

  public async getMenuTree(menuId: string = "main", workspace?: string): Promise<MenuTreeSchema> {
    return this.sendRequest<MenuTreeSchema>("get_menu_tree", {
      menu_id: menuId,
      workspace: workspace || this.state.workspace,
    });
  }

  public async executeMenuAction(
    actionId: string,
    params: Record<string, any> = {},
    workspace?: string
  ): Promise<MenuActionResult> {
    const res = await this.sendRequest<MenuActionResult>("execute_menu_action", {
      action_id: actionId,
      params,
      workspace: workspace || this.state.workspace,
    });
    if (res && res.state_updates) {
      this.updateState(res.state_updates as any);
    }
    return res;
  }

  public async getOptions(type: string, extraParams?: Record<string, any>, workspace?: string): Promise<OptionsGroupSchema> {
    return this.sendRequest<OptionsGroupSchema>("get_options", {
      type,
      workspace: workspace || this.state.workspace,
      ...(extraParams || {}),
    });
  }

  public async setOption(
    type: string,
    value: string,
    workspace?: string,
    applyGlobal: boolean = false
  ): Promise<MenuActionResult> {
    const res = await this.sendRequest<MenuActionResult>("set_option", {
      type,
      value,
      workspace: workspace || this.state.workspace,
      apply_global: applyGlobal,
    });
    if (res && res.state_updates) {
      this.updateState(res.state_updates as any);
    }
    return res;
  }

  public async startSync(force: boolean = false): Promise<any> {
    return this.sendRequest("start_sync", { force });
  }

  public async executeCommand(cmdLine: string): Promise<CommandExecutionResult> {
    const res = await this.sendRequest<CommandExecutionResult>("execute_command", { command: cmdLine });
    if (res && res.state) {
      this.updateState(res.state);
    }
    return res;
  }

  public streamChat(prompt: string, callbacks: StreamCallbacks): number {
    if (!this.process || !this.process.stdin) {
      callbacks.onError("Bridge process not connected");
      return -1;
    }

    const id = ++this.reqIdCounter;
    this.activeStreams.set(id, callbacks);

    const payload = JSON.stringify({ id, method: "chat", params: { message: prompt } }) + "\n";
    this.process.stdin.write(payload);
    return id;
  }

  private updateState(newState: AnyContextState) {
    let changed = false;
    for (const [key, value] of Object.entries(newState)) {
      if ((this.state as any)[key] !== value) {
        (this.state as any)[key] = value;
        changed = true;
      }
    }
    if (changed && this.onStateChange) {
      this.onStateChange({ ...this.state });
    }
  }

  public stop() {
    if (this.process) {
      this.process.kill();
      this.process = null;
    }
  }
}

