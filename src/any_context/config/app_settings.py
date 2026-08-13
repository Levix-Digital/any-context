import json
import os
import sys
from typing import Optional, List
from pydantic import BaseModel, Field

class ContextSettings(BaseModel):
    db_path: str = Field(default="./context_db")
    collection_name: str = Field(default="anycontext")

class WorkspaceSettings(BaseModel):
    name: str = Field(default="Default")
    paths: List[str] = Field(default_factory=list)

class SessionSettings(BaseModel):
    db_path: str = Field(default="./memory")
    collection_name: str = Field(default="session_memory")

class ModelSettings(BaseModel):
    local_embedding_model: str = Field(default="BAAI/bge-small-en-v1.5")
    local_openai_embedding_model: str = Field(default="text-embedding-3-small")
    inference_model: str = Field(default="gpt-4o-mini")
    summary_model: str = Field(default="gpt-4o-mini")
    model_provider: str = Field(default="openai")
    local_base_url: str = Field(default="http://localhost:1234/v1")

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
            print(f"❌ Error loading settings from SQLite DB: {e}")
        return None
