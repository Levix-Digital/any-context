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

# 2. Create the Config with Thread ID
thread_id = f"chat_{uuid.uuid4()}"
config = {
    "configurable": {
        "thread_id": thread_id
    }
}

def start_chat():
    print("\n🔄 Synchronizing file database...")
    index_folder.invoke({})
    
    print("\n=======================================================")
    print("💬 Chat started! Press Ctrl+C to exit.")
    print("=======================================================\n")
    while True:
        try:
            user_input = input("\n\033[96m👤 You:\033[0m ")
            if not user_input.strip():
                continue
    
            print("\033[93m🤖 AI:\033[0m ", end="", flush=True)
    
            for token, metadata in cli_agent.stream(
                {
                    "messages": [user_input]
                },
                stream_mode="messages",
                config=config
            ):
                if hasattr(token, "type") and token.type in ["ai", "AIMessageChunk", "AIMessage"]:
                    if isinstance(token.content, str) and token.content:
                        print(token.content, end="", flush=True)
                elif hasattr(token, "type") and token.type in ["tool", "ToolMessage", "ToolMessageChunk"]:
                    print("\n📚 Reading retrieved documents... Please wait for AI analysis.")
                    print("\033[93m🤖 AI:\033[0m ", end="", flush=True)
            print()
            
        except KeyboardInterrupt:
            print("\nExiting...")
            from core.memory_manager import run_session_summarizer_async
            # Trigger background summary passing the current session ID
            run_session_summarizer_async(thread_id)
            break

if __name__ == "__main__":
    start_chat()