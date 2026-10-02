//! Dispatcher adapter for interactive TUI slash commands.
//! Delegates all domain command execution to the universal UI-agnostic `CommandEngine`.

use any_context_core_rs::commands::CommandEngine;
use crate::tui::app::App;

/// Dispatches interactive slash commands within the TUI session.
pub fn dispatch_slash_command(raw_cmd: &str, args: &[&str], app: &mut App) {
    let ctx = app.to_execution_context();
    let result = CommandEngine::execute(raw_cmd, args, &ctx);
    app.apply_command_result(result);
}
