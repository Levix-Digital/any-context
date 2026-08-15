import os
import uuid
import sqlite3

from langchain.chat_models import init_chat_model
from langchain.agents import create_agent
from langgraph.checkpoint.sqlite import SqliteSaver

from any_context.tools.search_tools import search_db, add_web_source, list_web_sources, remove_web_source
from any_context.ingestion.local_folder_ingestor import index_folder
from any_context.ingestion.session_ingestor import index_session
from any_context.core.utils import get_system_prompt, get_api_key
from any_context.config.app_settings import AppSettings

os.makedirs("./memory", exist_ok=True)
conn = sqlite3.connect("./memory/checkpoints.db", check_same_thread=False)
saver = SqliteSaver(conn=conn)

def create_anycontext_agent(
    active_workspace: str = None, 
    checkpointer=None,
    model_override: str = None,
    provider_override: str = None
):
    """
    Dynamically creates an AnyContext AI Agent with temperature=0.0 for deterministic RAG synthesis,
    active workspace context awareness, on-the-fly model switching, and fresh configuration.
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

    # Standard models use temperature=0.0 for deterministic RAG; reasoning models (o1/o3/gpt-5) reject custom temperature
    is_reasoning = any(r in inference_model.lower() for r in ["o1-", "o1", "o3-", "o3", "reasoner", "gpt-5"])
    if not is_reasoning and model_provider in ["openai", "anthropic", "google_genai", "groq", "local"]:
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
    elif model_provider in ["google_genai", "gemini"]:
        init_kwargs["model_provider"] = "google_genai"

    model = init_chat_model(**init_kwargs)

    system_prompt = get_system_prompt(active_workspace=active_workspace)

    return create_agent(
        model=model,
        system_prompt=system_prompt,
        tools=[search_db, add_web_source, list_web_sources, remove_web_source, index_folder, index_session],
        checkpointer=checkpointer if checkpointer is not None else saver
    )

class LazyAgentProxy:
    def __init__(self, checkpointer=saver):
        self.checkpointer = checkpointer

    def stream(self, input_data, stream_mode="messages", config=None):
        active_ws = config.get("configurable", {}).get("active_workspace") if config else None
        model_override = config.get("configurable", {}).get("model") or config.get("configurable", {}).get("model_override") if config else None
        agent_inst = create_anycontext_agent(
            active_workspace=active_ws, 
            checkpointer=self.checkpointer,
            model_override=model_override
        )
        return agent_inst.stream(input_data, stream_mode=stream_mode, config=config)

    def invoke(self, input_data, config=None):
        active_ws = config.get("configurable", {}).get("active_workspace") if config else None
        model_override = config.get("configurable", {}).get("model") or config.get("configurable", {}).get("model_override") if config else None
        agent_inst = create_anycontext_agent(
            active_workspace=active_ws, 
            checkpointer=self.checkpointer,
            model_override=model_override
        )
        return agent_inst.invoke(input_data, config=config)

# Lazy global exports (instantiated at runtime when called)
agent = LazyAgentProxy(checkpointer=None)
cli_agent = LazyAgentProxy(checkpointer=saver)
