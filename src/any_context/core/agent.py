import os
import uuid
import sqlite3

from langchain.chat_models import init_chat_model
from langchain.agents import create_agent
from langgraph.checkpoint.sqlite import SqliteSaver

from any_context.tools.search_tools import search_db
from any_context.ingestion.local_folder_ingestor import index_folder
from any_context.ingestion.session_ingestor import index_session
from any_context.core.utils import get_system_prompt, get_api_key
from any_context.config.app_settings import AppSettings

os.makedirs("./memory", exist_ok=True)
conn = sqlite3.connect("./memory/checkpoints.db", check_same_thread=False)
saver = SqliteSaver(conn=conn)

def create_anycontext_agent(active_workspace: str = None, checkpointer=None):
    """
    Dynamically creates an AnyContext AI Agent with temperature=0.0 for deterministic RAG synthesis,
    active workspace context awareness, and fresh configuration.
    """
    settings = AppSettings.load()
    base_url = settings.models.local_base_url if settings else "http://localhost:1234/v1"
    model_provider = settings.models.model_provider if settings else "openai"
    inference_model = settings.models.inference_model if settings else "gpt-4o-mini"
    api_key = get_api_key(provider=model_provider)
    if not api_key:
        api_key = "sk-placeholder" if model_provider == "openai" else "lm-studio"

    model_kwargs = {}
    if model_provider in ["openai", "local"]:
        model_kwargs["temperature"] = 0.0

    init_kwargs = {
        "model": inference_model,
        "model_provider": model_provider,
        "temperature": 0.0,
        "api_key": api_key
    }
    if base_url:
        init_kwargs["base_url"] = base_url

    model = init_chat_model(**init_kwargs)

    system_prompt = get_system_prompt(active_workspace=active_workspace)

    return create_agent(
        model=model,
        system_prompt=system_prompt,
        tools=[search_db, index_folder, index_session],
        checkpointer=checkpointer if checkpointer is not None else saver
    )

class LazyAgentProxy:
    def __init__(self, checkpointer=saver):
        self.checkpointer = checkpointer

    def stream(self, input_data, stream_mode="messages", config=None):
        active_ws = config.get("configurable", {}).get("active_workspace") if config else None
        agent_inst = create_anycontext_agent(active_workspace=active_ws, checkpointer=self.checkpointer)
        return agent_inst.stream(input_data, stream_mode=stream_mode, config=config)

    def invoke(self, input_data, config=None):
        active_ws = config.get("configurable", {}).get("active_workspace") if config else None
        agent_inst = create_anycontext_agent(active_workspace=active_ws, checkpointer=self.checkpointer)
        return agent_inst.invoke(input_data, config=config)

# Lazy global exports (instantiated at runtime when called)
agent = LazyAgentProxy(checkpointer=None)
cli_agent = LazyAgentProxy(checkpointer=saver)

