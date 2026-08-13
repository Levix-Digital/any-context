import os
import threading
import sqlite3
from langgraph.checkpoint.sqlite import SqliteSaver
from langchain.chat_models import init_chat_model
from langchain_core.messages import SystemMessage

from any_context.ingestion.session_ingestor import index_session
from any_context.config.app_settings import AppSettings
from any_context.core.utils import get_api_key

API_KEY = get_api_key()

settings = AppSettings.load()
local_base_url = settings.models.local_base_url if settings else "http://localhost:1234/v1"
summary_model = settings.models.summary_model if settings else "google/gemma-4-e2b"
model_provider = settings.models.model_provider if settings else "openai"

def _summarize_and_save_task(thread_id: str):
    """
    Internal function that runs in parallel (background).
    """
    try:
        conn = sqlite3.connect("./memory/checkpoints.db", check_same_thread=False)
        saver = SqliteSaver(conn=conn)
        
        config = {"configurable": {"thread_id": thread_id}}
        state = saver.get(config)
        
        if not state or "messages" not in state["channel_values"]:
            print(f"\n⚠️ [Background] No history found for session {thread_id}.")
            return
            
        messages = state["channel_values"]["messages"]
        
        chat_history = ""
        for msg in messages:
            if msg.type in ["human", "ai"]:
                chat_history += f"{msg.type.upper()}: {msg.content}\n"
                
        model = init_chat_model(
            base_url=local_base_url,
            model=summary_model, 
            model_provider=model_provider,
            temperature=0.3,
            api_key=API_KEY
        )
        
        prompt = SystemMessage(content=(
            "You are an assistant responsible for creating long-term memories of conversations.\n"
            "Read the chat history below and create a single paragraph summarizing:\n"
            "1. Who the user is and their preferences or profile (if revealed).\n"
            "2. The context and main goal of what was discussed.\n"
            "3. Conclusions or decisions made.\n\n"
            "History:\n"
            f"{chat_history}"
        ))
        
        print(f"\n🧠 [Background] Generating session summary for long-term memory...")
        
        response = model.invoke([prompt])
        summary = response.content
        
        index_session.invoke({"session_summary": summary})
        
        print(f"✅ [Background] Summary saved successfully!")
        
    except Exception as e:
        print(f"❌ [Background] Error generating summary: {e}")
    finally:
        conn.close()

def run_session_summarizer_async(thread_id: str):
    """
    Main function to be called externally.
    It creates a separate Thread and executes the summary without blocking the interface.
    """
    thread = threading.Thread(target=_summarize_and_save_task, args=(thread_id,))
    thread.start()
