pub mod models;
pub mod ingestion;
pub mod retrieval;
pub mod storage;
pub mod commands;
pub mod security;
pub mod theme;

pub use models::{ChunkPayload, SemanticEnvelope};
pub use ingestion::{IngestionRouter, WorkspaceScanner};
pub use retrieval::{
    HybridRetrieverEngine, QueryPreprocessor, ProcessedQuery,
    NativeHybridPipeline, HybridSearchRequest, HybridSearchResult, RetrievalPreset,
    estimate_token_count, truncate_to_token_ceiling, get_embedding_token_limit_rs,
    extract_temporal_clauses_native, expand_query_temporal_native, extract_filename_mentions_native,
};
pub use storage::{
    NativeConfigDb, NativeLanceStore,
    get_default_settings_db_path, get_default_lancedb_path, get_default_logs_dir,
};
pub use commands::{
    CommandEngine, CommandResult, CommandAction, CommandStateUpdates,
    ExecutionContext, GroundingMode, SearchDepthMode,
};
pub use security::NativeSecurityEngine;
pub use theme::{AuroraTheme, RgbColor};
