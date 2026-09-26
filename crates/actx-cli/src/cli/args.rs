use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone, Default)]
#[command(
    name = "actx",
    author = "LeviGuilherme <contato@levix.digital>",
    version = "0.31.2",
    disable_version_flag = true,
    about = "AnyContext (actx) - 100% Native Rust Agentic Context Engine & TUI",
    long_about = "AnyContext (actx) is an ultra-fast, local-first agentic context engine and RAG pipeline.\nBy default, launching 'actx' opens the full interactive terminal TUI.\nHeadless flags and direct piped inputs execute in terminal stdout mode."
)]
pub struct CliArgs {
    /// Print version information (supports -v, -V, --version)
    #[arg(short = 'v', short_alias = 'V', long = "version", action = clap::ArgAction::Version)]
    pub version: Option<bool>,

    /// Context workspace to operate on
    #[arg(short = 'w', long = "workspace", default_value = "Default")]
    pub workspace: String,

    /// Check if a newer release is available on GitHub
    #[arg(long = "check-update", alias = "check")]
    pub check_update: bool,

    /// Self-update AnyContext binary to the latest release
    #[arg(short = 'u', long = "update", alias = "upgrade")]
    pub update: bool,

    /// Run system diagnostics report in headless stdout mode
    #[arg(short = 'd', long = "diagnostics", alias = "diag", alias = "health", alias = "diagnistics")]
    pub diagnostics: bool,

    /// Synchronize workspace folders and sources
    #[arg(short = 's', long = "sync", alias = "reindex")]
    pub sync: bool,

    /// Force full reindex during sync
    #[arg(short = 'f', long = "force")]
    pub force: bool,

    /// Direct one-shot prompt query (executes without opening full TUI)
    #[arg(short = 'p', long = "prompt", short_alias = 'q', alias = "query")]
    pub prompt: Option<String>,

    /// Model identifier to override configuration (e.g. gpt-4o, claude-3-5-sonnet, ollama/qwen2.5-coder)
    #[arg(short = 'm', long = "model")]
    pub model: Option<String>,

    /// Subcommands for daemons, protocols, and headless maintenance
    #[command(subcommand)]
    pub command: Option<CliCommand>,

    /// Positional arguments for direct query (e.g. `actx "how does auth work?"`)
    #[arg(trailing_var_arg = true)]
    pub positional_query: Vec<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommand {
    /// Starts the native REST API Server
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 8000)]
        port: u16,
    },
    /// Runs the Model Context Protocol (MCP) server via stdio
    Mcp,
    /// Runs the Stdio JSON-RPC 2.0 bridge server for IDE extensions
    Rpc,
    /// Triggers incremental synchronization for workspace folders and sources
    #[command(alias = "reindex")]
    Sync {
        #[arg(short, long)]
        force: bool,
    },
    /// Checks or performs self-updates for the standalone binary
    #[command(alias = "self-update", alias = "upgrade")]
    Update {
        /// Only check for updates without applying
        #[arg(long)]
        check: bool,
    },
    /// Emits system diagnostic report and telemetry
    #[command(alias = "diag", alias = "perf", alias = "health", alias = "diagnistics")]
    Diagnostics,
}

impl CliArgs {
    /// Determines whether the invocation should run headless (one-shot query)
    /// instead of launching the interactive full-screen TUI.
    pub fn is_headless(&self) -> bool {
        if self.check_update || self.update || self.diagnostics || self.sync {
            return true;
        }
        if self.command.is_some() {
            return true;
        }
        if self.prompt.is_some() {
            return true;
        }
        if !self.positional_query.is_empty() {
            return true;
        }
        // If stdin is piped/redirected (not a tty)
        #[cfg(not(test))]
        if !crossterm::tty::IsTty::is_tty(&std::io::stdin()) {
            return true;
        }
        false
    }

    /// Resolves the effective one-shot query text, if any.
    pub fn resolved_query(&self) -> Option<String> {
        if let Some(ref p) = self.prompt {
            let trimmed = p.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        if !self.positional_query.is_empty() {
            let joined = self.positional_query.join(" ").trim().to_string();
            if !joined.is_empty() {
                return Some(joined);
            }
        }
        None
    }
}
