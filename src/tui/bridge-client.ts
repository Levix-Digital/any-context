import { spawn, ChildProcess } from "child_process";
import * as path from "path";
import * as readline from "readline";

export interface AnyContextState {
  version: string;
  workspace: string;
  model: string;
  grounding_mode: string;
  web_search_enabled: boolean;
  sync_info: string;
  is_syncing: boolean;
}

export interface SlashCommandMeta {
  command: string;
  args: string;
  description: string;
  category: string;
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
    version: "0.26.0",
    workspace: "Default",
    model: "gpt-4o-mini",
    grounding_mode: "strict",
    web_search_enabled: false,
    sync_info: "",
    is_syncing: false,
  };
  public commands: SlashCommandMeta[] = [];
  public onStateChange?: (state: AnyContextState) => void;

  constructor(private initialWorkspace: string = "Default") {
    this.state.workspace = initialWorkspace;
  }

  public async start(): Promise<void> {
    const repoRoot = path.resolve(__dirname, "..", "..");
    const pythonExe = process.platform === "win32" ? "python" : "python3";

    this.process = spawn(pythonExe, ["-m", "any_context.server.rpc_bridge", this.initialWorkspace], {
      cwd: repoRoot,
      env: { ...process.env, PYTHONPATH: path.join(repoRoot, "src") },
      stdio: ["pipe", "pipe", "inherit"],
    });

    if (!this.process.stdout || !this.process.stdin) {
      throw new Error("Failed to initialize stdin/stdout pipes for AnyContext RPC Bridge");
    }

    const rl = readline.createInterface({ input: this.process.stdout });
    rl.on("line", (line) => this.handleLine(line));

    this.process.on("exit", (code) => {
      console.log(`\nAnyContext Core backend closed (code: ${code})`);
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

  public async startSync(force: boolean = false): Promise<any> {
    return this.sendRequest("start_sync", { force });
  }

  public async listSources(): Promise<any> {
    return this.sendRequest("list_sources");
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
    this.state = { ...this.state, ...newState };
    if (this.onStateChange) {
      this.onStateChange(this.state);
    }
  }

  public stop() {
    if (this.process) {
      this.process.kill();
      this.process = null;
    }
  }
}

