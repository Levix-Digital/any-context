import json
import os
import sys
from typing import Optional, List, Dict, Any, Union
from pydantic import BaseModel, Field

class ContextSettings(BaseModel):
    db_path: str = Field(default="./context_db")
    collection_name: str = Field(default="anycontext")
    chunk_size: int = Field(default=1024, description="Target token size per document chunk")
    chunk_overlap: int = Field(default=200, description="Token overlap between adjacent chunks")
    top_k: int = Field(default=40, description="Target number of diversified document chunks returned to AI agent")
    candidate_pool_size: int = Field(default=100, description="Initial candidate pool size retrieved from ChromaDB before source diversification")
    max_chunks_per_source: int = Field(default=3, description="Maximum chunks allowed per document/URL to enforce cross-source diversity")
    retrieval_preset: str = Field(default="balanced", description="RAG Retrieval Density Preset: 'balanced', 'turbo', 'deep_research', 'custom'")
    grounding_mode: str = Field(default="strict", description="AI Grounding & Answer Mode: 'strict' (default), 'hybrid', 'proactive'")
    web_search_enabled: bool = Field(default=False, description="Default/Global Web Search Toggle")
    default_web_engine: str = Field(default="auto", description="Default Web Search Engine: 'auto', 'tavily', 'serper', 'duckduckgo'")

    def apply_preset(self, preset_name: str):
        p = preset_name.lower().strip()
        if p in ["turbo", "fast", "speed"]:
            self.retrieval_preset = "turbo"
            self.candidate_pool_size = 50
            self.top_k = 20
            self.max_chunks_per_source = 2
        elif p in ["deep_research", "deep-research", "deep", "max"]:
            self.retrieval_preset = "deep_research"
            self.candidate_pool_size = 150
            self.top_k = 60
            self.max_chunks_per_source = 4
        elif p == "custom":
            self.retrieval_preset = "custom"
        else: # balanced default
            self.retrieval_preset = "balanced"
            self.candidate_pool_size = 100
            self.top_k = 40
            self.max_chunks_per_source = 3

class WorkspaceWebSource(BaseModel):
    id: str
    url: str
    root_url: Optional[str] = None
    title: Optional[str] = None
    page_count: int = 1
    scope: Optional[str] = None
    last_scraped_at: Optional[str] = None
    created_at: Optional[str] = None

class WorkspaceCloudDrive(BaseModel):
    id: str
    provider: str
    mount_path_or_id: str
    title: Optional[str] = None
    auth_status: str = "pending"
    last_synced_at: Optional[str] = None
    created_at: Optional[str] = None

class WorkspaceSourceItem(BaseModel):
    type: str  # 'folder', 'web', 'cloud_drive'
    id: Optional[Union[str, int]] = None
    identifier: str
    title: Optional[str] = None
    details: Dict[str, Any] = Field(default_factory=dict)

class WorkspaceSettings(BaseModel):
    id: str = Field(default="ws_default", description="Unique immutable workspace identifier")
    name: str = Field(default="Default")
    paths: List[str] = Field(default_factory=list)
    sources: List[WorkspaceSourceItem] = Field(default_factory=list)
    total_sources: int = 0
    grounding_mode: str = Field(default="strict", description="Per-workspace grounding mode: 'strict' (default), 'hybrid', 'proactive'")
    web_search_enabled: bool = Field(default=False, description="Per-workspace web search toggle")
    default_web_engine: str = Field(default="auto", description="Per-workspace search engine preference: 'auto', 'tavily', 'serper', 'duckduckgo'")

class SessionSettings(BaseModel):
    db_path: str = Field(default="./memory")
    collection_name: str = Field(default="session_memory")

class ModelSettings(BaseModel):
    embedding_model: str = Field(default="text-embedding-3-small")
    inference_model: str = Field(default="gpt-4o-mini")
    summary_model: str = Field(default="gpt-4o-mini")
    model_provider: str = Field(default="openai")
    local_base_url: str = Field(default="https://api.openai.com/v1")

    # Retrocompatibility properties for legacy code / DB rows
    @property
    def local_embedding_model(self) -> str:
        return self.embedding_model

    @local_embedding_model.setter
    def local_embedding_model(self, value: str):
        self.embedding_model = value

    @property
    def local_openai_embedding_model(self) -> str:
        return self.embedding_model

    @local_openai_embedding_model.setter
    def local_openai_embedding_model(self, value: str):
        self.embedding_model = value


class MemorySettings(BaseModel):
    short_term_buffer_size: int = Field(default=20, description="Number of messages before Level-1 summary trigger")
    rolling_window_messages: int = Field(default=10, description="Active messages kept in active LLM context")
    meta_summary_threshold: int = Field(default=30, description="Number of summaries before Level-3 Meta compression")
    meta_summary_batch_size: int = Field(default=10, description="Number of summaries combined into 1 Meta-Summary")

class AppSettings(BaseModel):
    workspaces: List[WorkspaceSettings] = Field(default_factory=list)
    context: ContextSettings = Field(default_factory=ContextSettings)
    session: SessionSettings = Field(default_factory=SessionSettings)
    models: ModelSettings = Field(default_factory=ModelSettings)
    memory: MemorySettings = Field(default_factory=MemorySettings)

    @classmethod
    def find_config_file(cls, filename: str = "settings.db") -> Optional[str]:
        """Finds the config file in candidate locations"""
        candidates = [
            os.path.join(os.getcwd(), "config", filename),
            os.path.join(os.getcwd(), filename),
            os.path.expanduser(os.path.join("~", ".config", "any-context", filename)),
        ]

        if sys.platform == "win32" and "APPDATA" in os.environ:
            candidates.append(os.path.join(os.environ["APPDATA"], "any-context", filename))

        if hasattr(sys, "_MEIPASS"):
            candidates.append(os.path.join(sys._MEIPASS, "config", filename))
            candidates.append(os.path.join(sys._MEIPASS, filename))

        package_dir = os.path.dirname(os.path.abspath(__file__))
        candidates.append(os.path.join(package_dir, filename))
        candidates.append(os.path.join(package_dir, "..", "..", "..", "config", filename))

        for candidate in candidates:
            if candidate and os.path.exists(candidate):
                return os.path.abspath(candidate)
        return None

    @classmethod
    def load(cls, path: str = None):
        """
        Loads Settings exclusively from SQLite ConfigDBStore.
        """
        try:
            from any_context.config.db_store import ConfigDBStore
            store = ConfigDBStore(db_path=path)
            settings = store.get_app_settings()
            if settings:
                return settings
        except Exception as e:
            try:
                print(f"❌ Error loading settings from SQLite DB: {e}")
            except Exception:
                print(f"[Error] Error loading settings from SQLite DB: {e}")
        return None
