"""RFC-042 Deep Search and Adaptive Routing Pydantic Models.

Defines the core data structures for query decomposition, information gap evaluation,
evidence collection, citations, and adaptive search modes (/fast vs /deep).
"""

from enum import Enum
from typing import List, Optional
from pydantic import BaseModel, Field


class SearchMode(str, Enum):
    """Execution search mode for the agent engine."""
    AUTO = "auto"
    FAST = "fast"
    DEEP = "deep"


class QueryCategory(str, Enum):
    """Categorization of decomposed sub-queries."""
    CODE = "code"
    ARCHITECTURE = "architecture"
    CONFIG = "config"
    DOCUMENTATION = "documentation"
    GENERAL = "general"


class SubQuery(BaseModel):
    """A decomposed orthogonal sub-query generated during Phase 1."""
    id: str = Field(description="Unique identifier for the sub-query (e.g. 'sub_1')")
    query: str = Field(description="Search text tailored for lexical/semantic retrieval")
    category: QueryCategory = Field(default=QueryCategory.GENERAL, description="Category of information targeted")
    rationale: str = Field(default="", description="Why this sub-query is essential to resolve the user request")
    target_sources: List[str] = Field(default_factory=list, description="Optional target filenames or extensions")


class QueryPlan(BaseModel):
    """Initial query execution plan decomposing the user request."""
    original_query: str = Field(description="The unmodified input user prompt")
    mode: SearchMode = Field(default=SearchMode.AUTO, description="Resolved search mode")
    sub_queries: List[SubQuery] = Field(default_factory=list, description="List of orthogonal sub-queries")
    estimated_complexity: str = Field(default="moderate", description="Heuristic complexity rating: simple, moderate, complex")


class IterationEvidence(BaseModel):
    """Evidence collected during a single sub-query execution turn."""
    sub_query_id: str
    query_text: str
    chunks_retrieved: int = 0
    sources_cited: List[str] = Field(default_factory=list)
    summary_observation: str = ""


class GapAnalysisResult(BaseModel):
    """Result of Phase 3 factual reflection evaluating if retrieved context is sufficient."""
    iteration: int = Field(description="Current reflection iteration counter (1-indexed)")
    is_sufficient: bool = Field(description="True if collected evidence thoroughly answers all query facets")
    confidence_score: float = Field(default=1.0, ge=0.0, le=1.0, description="Confidence in answerability (0.0 - 1.0)")
    missing_aspects: List[str] = Field(default_factory=list, description="Specific blind spots or missing details")
    follow_up_queries: List[str] = Field(default_factory=list, description="Targeted follow-up queries for Phase 4")
    overlap_ratio: float = Field(default=0.0, ge=0.0, le=1.0, description="Overlap with prior iterations to detect diminishing returns")


class Citation(BaseModel):
    """Source code or document citation supporting an assertion in the final answer."""
    file_path: str = Field(description="Relative or canonical path of the cited file")
    lines: Optional[str] = Field(default=None, description="Line range if known, e.g. '45-60'")
    snippet: Optional[str] = Field(default=None, description="Short relevant snippet excerpt")


class DeepSearchFinalResult(BaseModel):
    """Comprehensive final output of the Deep Search cycle."""
    answer: str = Field(description="Synthesized markdown response with citations")
    total_iterations: int = Field(default=1, description="Number of reflection loops executed")
    total_subqueries_executed: int = Field(default=0, description="Total sub-queries dispatched")
    citations: List[Citation] = Field(default_factory=list, description="Extracted file citations")
    execution_time_ms: float = Field(default=0.0, description="Total latency in milliseconds")
    search_mode_used: SearchMode = Field(default=SearchMode.FAST, description="Search mode executed")
