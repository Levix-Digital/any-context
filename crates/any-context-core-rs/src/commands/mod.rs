//! Universal Command Engine module for AnyContext.
//! Provides a completely decoupled, UI-agnostic command dispatcher and standardized execution contracts.

pub mod models;
pub mod engine;

pub use models::{
    CommandAction, CommandResult, CommandStateUpdates, ExecutionContext, GroundingMode, SearchDepthMode,
};
pub use engine::CommandEngine;
