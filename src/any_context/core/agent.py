import os
import uuid
import sqlite3

from langchain.chat_models import init_chat_model
from langchain.agents import create_agent
from langgraph.checkpoint.sqlite import SqliteSaver
from langgraph.checkpoint.memory import MemorySaver

from any_context.tools.search_tools import search_db, add_web_source, list_web_sources, remove_web_source
from any_context.tools.web_search_tool import live_web_search
from any_context.ingestion.local_folder_ingestor import index_folder
from any_context.ingestion.session_ingestor import index_session
from any_context.core.utils import get_system_prompt, get_api_key
from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore

class ResilientSqliteSaver(SqliteSaver):
    """
    Auto-healing SqliteSaver that safely recovers from corrupted zlib streams,
    incomplete checkpoint bytes, or database locking errors without crashing the agent.
    """
    def get_tuple(self, config):
        try:
            return super().get_tuple(config)
        except Exception:
            try:
                thread_id = config.get("configurable", {}).get("thread_id") if config else None
                if thread_id:
                    self.delete_thread(thread_id)
            except Exception:
                pass
            return None

    def list(self, config=None, *, filter=None, before=None, limit=None):
        try:
            for item in super().list(config, filter=filter, before=before, limit=limit):
                yield item
        except Exception:
            return

    def get_delta_channel_history(self, *, config, channels):
        try:
            return super().get_delta_channel_history(config=config, channels=channels)
        except Exception:
            return {ch: {"writes": []} for ch in channels}


_global_checkpointer = None

def get_safe_checkpointer():
    """
    Returns a resilient checkpoint saver with automatic corruption detection,
    database healing, and fallback to MemorySaver if SQLite checkpoint decompressing fails.
    """
    global _global_checkpointer
    if _global_checkpointer is not None:
        return _global_checkpointer

    try:
        user_home = os.path.expanduser("~/.anycontext/memory")
        os.makedirs(user_home, exist_ok=True)
        db_path = os.path.join(user_home, "checkpoints.db")
        conn = sqlite3.connect(db_path, check_same_thread=False)
        saver_inst = ResilientSqliteSaver(conn=conn)
        saver_inst.setup()
        _global_checkpointer = saver_inst
        return _global_checkpointer
    except Exception:
        try:
            local_mem = os.path.abspath("./memory")
            os.makedirs(local_mem, exist_ok=True)
            db_path = os.path.join(local_mem, "checkpoints.db")
            conn = sqlite3.connect(db_path, check_same_thread=False)
            saver_inst = ResilientSqliteSaver(conn=conn)
            saver_inst.setup()
            _global_checkpointer = saver_inst
            return _global_checkpointer
        except Exception:
            _global_checkpointer = MemorySaver()
            return _global_checkpointer

# Backward compatible module export
saver = get_safe_checkpointer()

def create_anycontext_agent(
    active_workspace: str = None, 
    checkpointer=None,
    model_override: str = None,
    provider_override: str = None,
    grounding_mode: str = None,
    web_search_enabled: bool = None
):
    """
    Dynamically creates an AnyContext AI Agent with temperature=0.0 for deterministic RAG synthesis,
    active workspace context awareness, on-the-fly model switching, grounding mode directives, and fresh configuration.
    """
    from any_context.core.models_catalog import infer_provider_for_model

    settings = AppSettings.load()
    default_provider = settings.models.model_provider if settings else "openai"
    default_model = settings.models.inference_model if settings else "gpt-4o-mini"
    base_url = settings.models.local_base_url if settings else "http://localhost:1234/v1"

    inference_model = model_override or default_model
    model_provider = provider_override or infer_provider_for_model(inference_model, fallback_provider=default_provider)

    api_key = get_api_key(provider=model_provider)
    if not api_key:
        api_key = "sk-placeholder" if model_provider == "openai" else "lm-studio"

    init_kwargs = {
        "model": inference_model,
        "model_provider": model_provider,
        "api_key": api_key
    }

    # Standard models use temperature=0.0 for deterministic RAG; reasoning models (o1/o3/gpt-5/sonnet-5) reject custom temperature
    is_reasoning = any(r in inference_model.lower() for r in ["o1-", "o1", "o3-", "o3", "reasoner", "gpt-5", "claude-sonnet-5", "claude-opus-5"])
    if not is_reasoning and model_provider in ["openai", "anthropic", "google_genai", "groq", "mistral", "local"]:
        init_kwargs["temperature"] = 0.0

    # Route provider base URLs when switching on the fly
    if model_provider in ["local", "lm-studio", "ollama"] or (base_url and ("localhost" in base_url or "127.0.0.1" in base_url)):
        init_kwargs["base_url"] = base_url
    elif model_provider == "deepseek":
        init_kwargs["base_url"] = "https://api.deepseek.com/v1"
        init_kwargs["model_provider"] = "openai" # langchain deepseek uses openai-compatible client
    elif model_provider == "groq":
        init_kwargs["base_url"] = "https://api.groq.com/openai/v1"
        init_kwargs["model_provider"] = "openai"
    elif model_provider == "xai":
        init_kwargs["base_url"] = "https://api.x.ai/v1"
        init_kwargs["model_provider"] = "openai"
    elif model_provider == "openrouter":
        init_kwargs["base_url"] = "https://openrouter.ai/api/v1"
        init_kwargs["model_provider"] = "openai"
    elif model_provider == "mistral":
        init_kwargs["base_url"] = "https://api.mistral.ai/v1"
        init_kwargs["model_provider"] = "openai"
    elif model_provider in ["google_genai", "gemini"]:
        init_kwargs["model_provider"] = "google_genai"

    model = init_chat_model(**init_kwargs)

    # Resolve web search status if not explicitly passed
    if web_search_enabled is None:
        try:
            store = ConfigDBStore()
            web_search_enabled = store.get_web_search_status(workspace_name=active_workspace)
        except Exception:
            web_search_enabled = False

    system_prompt = get_system_prompt(
        active_workspace=active_workspace,
        grounding_mode=grounding_mode,
        web_search_enabled=web_search_enabled
    )

    tools = [search_db, add_web_source, list_web_sources, remove_web_source, index_folder, index_session]
    if web_search_enabled:
        tools.append(live_web_search)

    chk = checkpointer if checkpointer is not None else get_safe_checkpointer()
    try:
        return create_agent(
            model=model,
            system_prompt=system_prompt,
            tools=tools,
            checkpointer=chk
        )
    except Exception:
        # If checkpointer threw zlib or database error, fall back cleanly to MemorySaver
        from langgraph.checkpoint.memory import MemorySaver
        return create_agent(
            model=model,
            system_prompt=system_prompt,
            tools=tools,
            checkpointer=MemorySaver()
        )

class LazyAgentProxy:
    def __init__(self, checkpointer=saver):
        self.checkpointer = checkpointer

    def stream(self, input_data, stream_mode="messages", config=None):
        active_ws = config.get("configurable", {}).get("active_workspace") if config else None
        model_override = config.get("configurable", {}).get("model") or config.get("configurable", {}).get("model_override") if config else None
        grounding_override = config.get("configurable", {}).get("grounding_mode") or config.get("configurable", {}).get("mode") if config else None
        web_search_override = config.get("configurable", {}).get("web_search_enabled") if config else None
        agent_inst = create_anycontext_agent(
            active_workspace=active_ws, 
            checkpointer=self.checkpointer,
            model_override=model_override,
            grounding_mode=grounding_override,
            web_search_enabled=web_search_override
        )
        return agent_inst.stream(input_data, stream_mode=stream_mode, config=config)

    def invoke(self, input_data, config=None):
        active_ws = config.get("configurable", {}).get("active_workspace") if config else None
        model_override = config.get("configurable", {}).get("model") or config.get("configurable", {}).get("model_override") if config else None
        grounding_override = config.get("configurable", {}).get("grounding_mode") or config.get("configurable", {}).get("mode") if config else None
        web_search_override = config.get("configurable", {}).get("web_search_enabled") if config else None
        agent_inst = create_anycontext_agent(
            active_workspace=active_ws, 
            checkpointer=self.checkpointer,
            model_override=model_override,
            grounding_mode=grounding_override,
            web_search_enabled=web_search_override
        )
        return agent_inst.invoke(input_data, config=config)

# Lazy global exports (instantiated at runtime when called)
agent = LazyAgentProxy(checkpointer=None)
cli_agent = LazyAgentProxy(checkpointer=saver)
