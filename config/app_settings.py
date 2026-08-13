import json
from pydantic import BaseModel

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

class AppSettings(BaseModel):
    workspaces: list[WorkspaceSettings]
    context: ContextSettings
    session: SessionSettings
    models: ModelSettings

    @classmethod
    def load(cls, path: str = "config/settings.json"):
        """Reads the JSON file and returns the validated Settings instance"""
        try:
            with open(path, "r", encoding="utf-8") as f:
                data = json.load(f)
                return cls(**data)
        except FileNotFoundError:
            print(f"❌ Error: The file {path} was not found.")
            return None
        except json.JSONDecodeError:
            print(f"❌ Error: The file {path} is not a valid JSON file.")
            return None