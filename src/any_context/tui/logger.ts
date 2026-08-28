import * as fs from "fs";
import * as path from "path";
import * as os from "os";

export type TuiLogLevel = "DEBUG" | "INFO" | "WARN" | "ERROR";

class TuiLogger {
  private logFilePath: string;

  constructor() {
    let logDir: string;
    if (process.platform === "win32") {
      logDir = path.join(process.env.LOCALAPPDATA || path.join(os.homedir(), "AppData", "Local"), "AnyContext", "logs");
    } else if (process.platform === "darwin") {
      logDir = path.join(os.homedir(), "Library", "Application Support", "any-context", "logs");
    } else {
      logDir = path.join(process.env.XDG_DATA_HOME || path.join(os.homedir(), ".local", "share"), "any-context", "logs");
    }

    try {
      if (!fs.existsSync(logDir)) {
        fs.mkdirSync(logDir, { recursive: true });
      }
    } catch (_) {}

    this.logFilePath = path.join(logDir, "tui_debug.log");
  }

  public log(level: TuiLogLevel, component: string, message: string, metadata?: Record<string, any>) {
    const timestamp = new Date().toISOString();
    const metaStr = metadata ? ` | ${JSON.stringify(metadata)}` : "";
    const logLine = `[${timestamp}] [${level}] [${component}] ${message}${metaStr}\n`;

    try {
      // Rotate if log exceeds 5MB
      if (fs.existsSync(this.logFilePath)) {
        const stats = fs.statSync(this.logFilePath);
        if (stats.size > 5 * 1024 * 1024) {
          const backupPath = `${this.logFilePath}.old`;
          fs.renameSync(this.logFilePath, backupPath);
        }
      }
      fs.appendFileSync(this.logFilePath, logLine, "utf8");
    } catch (_) {}
  }

  public info(component: string, message: string, metadata?: Record<string, any>) {
    this.log("INFO", component, message, metadata);
  }

  public warn(component: string, message: string, metadata?: Record<string, any>) {
    this.log("WARN", component, message, metadata);
  }

  public error(component: string, message: string, metadata?: Record<string, any>) {
    this.log("ERROR", component, message, metadata);
  }

  public debug(component: string, message: string, metadata?: Record<string, any>) {
    this.log("DEBUG", component, message, metadata);
  }

  public getLogPath(): string {
    return this.logFilePath;
  }
}

export const tuiLog = new TuiLogger();
