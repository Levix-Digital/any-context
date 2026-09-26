pub mod tokenizer;
pub mod bm25;
pub mod rrf;
pub mod diversifier;
pub mod hybrid;
pub mod query;
pub mod token_budget;
pub mod pipeline;

pub use tokenizer::tokenize;
pub use bm25::{BM25Index, DocRecord, Posting};
pub use rrf::{reciprocal_rank_fusion, RRFScoreResult};
pub use diversifier::{RankedChunk, apply_source_diversification, apply_density_budget, apply_threshold};
pub use hybrid::HybridRetrieverEngine;
pub use query::{QueryPreprocessor, ProcessedQuery, extract_temporal_clauses_native, expand_query_temporal_native, extract_filename_mentions_native};
pub use token_budget::{estimate_token_count, truncate_to_token_ceiling, get_embedding_token_limit_rs};
pub use pipeline::{NativeHybridPipeline, HybridSearchRequest, HybridSearchResult};
