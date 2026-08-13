import os
import dotenv
import threading
import sqlite3
from langgraph.checkpoint.sqlite import SqliteSaver
from langchain.chat_models import init_chat_model
from langchain_core.messages import SystemMessage

# Import the function that already saves to ChromaDB
from ingestion.session_ingestor import index_session
from config.app_settings import AppSettings

dotenv.load_dotenv()
API_KEY = os.getenv("LOCAL_API_KEY")

settings = AppSettings.load()
local_base_url = settings.models.local_base_url
summary_model = settings.models.summary_model
model_provider = settings.models.model_provider

def _summarize_and_save_task(thread_id: str):
    """
    Internal function that runs in parallel (background).
    """
    try:
        # 1. Pull the history from SQLite (LangGraph Checkpointer)
        # Using check_same_thread=False remains crucial here for background tasks
        conn = sqlite3.connect("./memory/checkpoints.db", check_same_thread=False)
        saver = SqliteSaver(conn=conn)
        
        # Retrieves the complete state of that specific session
        config = {"configurable": {"thread_id": thread_id}}
        state = saver.get(config)
        
        # If there's no history, do nothing
        if not state or "messages" not in state["channel_values"]:
            print(f"\n⚠️ [Background] No history found for session {thread_id}.")
            return
            
        # Extract the list of BaseMessage objects from the raw graph state.
        # LangGraph stores the data flowing through the nodes in 'channel_values'.
        messages = state["channel_values"]["messages"]
        
        # 2. Format the messages for the LLM to read
        # Let's convert the list of objects (HumanMessage, AIMessage) into clean text
        chat_history = ""
        for msg in messages:
            # Ignore tool messages (ToolMessage) to avoid cluttering the summary
            if msg.type in ["human", "ai"]:
                chat_history += f"{msg.type.upper()}: {msg.content}\n"
                
        # 3. Initialize the Summary model
        # HINT: This is where you would use a smaller/faster model (e.g., gemma-2b).
        model = init_chat_model(
            base_url=local_base_url,
            model=summary_model, 
            model_provider=model_provider,
            temperature=0.3, # Low temperature ensures more analytical and direct summaries
            api_key=API_KEY
        )
        
        # 4. Create the Summary Prompt
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
        
        # 5. Make the simple call to the LLM (without using the agent)
        response = model.invoke([prompt])
        summary = response.content
        
        # 6. Save to ChromaDB using the tool we already created!
        # We use .invoke passing the dictionary with the exact tool parameter
        index_session.invoke({"session_summary": summary})
        
        print(f"✅ [Background] Summary saved successfully!")
        
    except Exception as e:
        print(f"❌ [Background] Error generating summary: {e}")
    finally:
        # It is important to close the connection at the end of the thread
        conn.close()

def run_session_summarizer_async(thread_id: str):
    """
    Main function to be called externally.
    It creates a separate Thread and executes the summary without blocking the interface.
    """
    # Creates a new thread pointing to the internal function and passing the session ID
    thread = threading.Thread(target=_summarize_and_save_task, args=(thread_id,))
    
    # Starts the thread immediately in the background
    thread.start()
    
    # The function returns instantly, while the thread keeps running in parallel
