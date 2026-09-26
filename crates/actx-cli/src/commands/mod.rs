pub mod registry;
pub mod dispatcher;

pub use registry::{autocomplete_commands, find_command, SlashCommand, DEFAULT_SLASH_COMMANDS};
pub use dispatcher::dispatch_slash_command;
