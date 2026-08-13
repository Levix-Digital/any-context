import os
import dotenv
import uuid
import sqlite3

from langchain.chat_models import init_chat_model
from langchain.agents import create_agent
from langgraph.checkpoint.sqlite import SqliteSaver

from tools.search_tools import search_db
from ingestion.local_folder_ingestor import index_folder
from ingestion.session_ingestor import index_session
from core.utils import get_system_prompt
from config.app_settings import AppSettings

dotenv.load_dotenv()
# API_KEY = os.getenv("LOCAL_API_KEY")
API_KEY = os.getenv("OPENAI_API_KEY")

settings = AppSettings.load()
base_url = settings.models.local_base_url
model_provider = settings.models.model_provider
inference_model = settings.models.inference_model

model = init_chat_model(
    # base_url=base_url,
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

# End of agent configuration