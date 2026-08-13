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

API_KEY = get_api_key()

settings = AppSettings.load()
base_url = settings.models.local_base_url if settings else "http://localhost:1234/v1"
model_provider = settings.models.model_provider if settings else "openai"
inference_model = settings.models.inference_model if settings else "gpt-4o-mini"

model = init_chat_model(
    model=inference_model,
    model_provider=model_provider,
    temperature=1.0,
    api_key=API_KEY
)

system_prompt = get_system_prompt()

os.makedirs("./memory", exist_ok=True)
conn = sqlite3.connect("./memory/checkpoints.db", check_same_thread=False)
saver = SqliteSaver(conn=conn)

# Basic agent exported for LangGraph Studio (which injects its own checkpointer)
agent = create_agent(
    model=model,
    system_prompt=system_prompt,
    tools=[search_db, index_folder, index_session]
)

# CLI Agent used when running locally in the terminal
cli_agent = create_agent(
    model=model,
    system_prompt=system_prompt,
    tools=[search_db, index_folder, index_session],
    checkpointer=saver
)
