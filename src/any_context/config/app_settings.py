import json
import os
import sys
from typing import Optional
from pydantic import BaseModel, Field

class ContextSettings(BaseModel):
    db_path: str
    collection_name: str

class WorkspaceSettings(BaseModel):
    name: str
    paths: list[str]

class SessionSettings(BaseModel):
    db_path: str
    collection_name: str

class ModelSettings(BaseModel):
    local_embedding_model: str
    local_openai_embedding_model: str
    inference_model: str
    summary_model: str
    model_provider: str
    local_base_url: str

class MemorySettings(BaseModel):
    short_term_buffer_size: int = Field(default=20, description="Number of messages before Level-1 summary trigger")
    rolling_window_messages: int = Field(default=10, description="Active messages kept in active LLM context")
    meta_summary_threshold: int = Field(default=30, description="Number of summaries before Level-3 Meta compression")
    meta_summary_batch_size: int = Field(default=10, description="Number of summaries combined into 1 Meta-Summary")

class AppSettings(BaseModel):
    workspaces: list[WorkspaceSettings]
    context: ContextSettings
    session: SessionSettings
    models: ModelSettings
    memory: MemorySettings = Field(default_factory=MemorySettings)

    @classmethod
    def find_config_file(cls, filename: str = "settings.json") -> Optional[str]:
        """Finds the config file in candidate locations"""
        candidates = [
            # 1. Working directory ./config/
            os.path.join(os.getcwd(), "config", filename),
            # 2. Working directory root ./
            os.path.join(os.getcwd(), filename),
            # 3. User home config (~/.config/any-context/ or AppData)
            os.path.expanduser(os.path.join("~", ".config", "any-context", filename)),
        ]

        if sys.platform == "win32" and "APPDATA" in os.environ:
            candidates.append(os.path.join(os.environ["APPDATA"], "any-context", filename))

        # 4. PyInstaller bundle location (if frozen binary)
        if hasattr(sys, "_MEIPASS"):
            candidates.append(os.path.join(sys._MEIPASS, "config", filename))
            candidates.append(os.path.join(sys._MEIPASS, filename))

        # 5. Package source location fallback
        package_dir = os.path.dirname(os.path.abspath(__file__))
        candidates.append(os.path.join(package_dir, filename))
        candidates.append(os.path.join(package_dir, "..", "..", "..", "config", filename))

        for candidate in candidates:
            if candidate and os.path.exists(candidate):
                return os.path.abspath(candidate)
        return None

    @classmethod
    def load(cls, path: str = None):
        """Reads the JSON file and returns the validated Settings instance"""
        target_path = path if (path and os.path.exists(path)) else cls.find_config_file("settings.json")
        if not target_path or not os.path.exists(target_path):
            print("❌ Error: Config file 'settings.json' was not found.")
            return None

        try:
            with open(target_path, "r", encoding="utf-8") as f:
                data = json.load(f)
                return cls(**data)
        except json.JSONDecodeError:
            print(f"❌ Error: The file {target_path} is not a valid JSON file.")
            return None
        except Exception as e:
            print(f"❌ Error loading settings from {target_path}: {e}")
            return None
