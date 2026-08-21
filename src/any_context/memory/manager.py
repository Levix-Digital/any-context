import os
import sqlite3
import threading
from typing import Optional
from langgraph.checkpoint.sqlite import SqliteSaver

from any_context.config.app_settings import AppSettings
from any_context.memory.models import MemoryEntry, MemoryLevel
from any_context.memory.store import MemoryStore
from any_context.memory.compressor import MemoryCompressor

class MemoryManager:
    """
    Main Orchestrator for 3-Level Hierarchical Memory Compression
    """

    def __init__(self, settings: AppSettings = None):
        self.settings = settings or AppSettings.load()
        self.store = MemoryStore(settings=self.settings)
        self.compressor = MemoryCompressor(settings=self.settings)

    def process_session_background(self, thread_id: str, workspace: Optional[str] = None):
        """
        Level-1 & Level-2: Extract history from SQLite, summarize Level-1, and perform Level-2 rolling window
        """
        db_dir = os.path.abspath(self.settings.session.db_path if self.settings and self.settings.session else "./memory")
        os.makedirs(db_dir, exist_ok=True)
        checkpoints_path = os.path.join(db_dir, "checkpoints.db")
        if not os.path.exists(checkpoints_path):
            return

        conn = None
        try:
            conn = sqlite3.connect(checkpoints_path, check_same_thread=False)
            saver = SqliteSaver(conn=conn)

            config = {"configurable": {"thread_id": thread_id}}
            try:
                state = saver.get(config)
            except Exception:
                return

            if not state or "channel_values" not in state or "messages" not in state["channel_values"]:
                return

            messages = state["channel_values"]["messages"]
            if not messages:
                return

            # Format chat history for LLM
            formatted_chat = ""
            for msg in messages:
                if hasattr(msg, "type") and msg.type in ["human", "ai"]:
                    role = "USER" if msg.type == "human" else "ASSISTANT"
                    formatted_chat += f"{role}: {msg.content}\n"

            if not formatted_chat.strip():
                return

            # 1. Level-1 Summarization: Generate Session Memory Chunk
            print(f"\n🧠 [Hierarchical Memory - Level 1] Generating session summary block...")
            summary = self.compressor.summarize_chat_block(formatted_chat)

            # Save to ChromaDB Store
            entry = MemoryEntry(
                content=summary,
                level=MemoryLevel.SESSION_SUMMARY,
                workspace=workspace or "global",
                thread_id=thread_id
            )
            self.store.save_memory_entry(entry)
            print(f"✅ [Hierarchical Memory - Level 1] Level-1 Summary saved to vector store!")

            # 2. Level-3 Check: Trigger Meta-Summarization if threshold reached
            self.run_meta_summarizer_if_needed(workspace=workspace or "global")

        except Exception as e:
            pass
        finally:
            if conn:
                try:
                    conn.close()
                except Exception:
                    pass

    def run_meta_summarizer_if_needed(self, workspace: str = "global"):
        """
        Level-3 Compression: Compresses older Level-1 summaries into a single Meta-Summary
        """
        threshold = self.settings.memory.meta_summary_threshold if self.settings else 30
        batch_size = self.settings.memory.meta_summary_batch_size if self.settings else 10

        entries = self.store.get_entries_by_level(MemoryLevel.SESSION_SUMMARY, workspace=workspace)
        if len(entries) >= threshold:
            print(f"\n⚡ [Hierarchical Memory - Level 3] Threshold of {threshold} summaries reached for workspace '{workspace}'. Compressing older entries...")
            
            # Select oldest batch
            oldest_batch = entries[:batch_size]
            batch_ids = [item["id"] for item in oldest_batch]
            batch_texts = [item["content"] for item in oldest_batch]

            # Generate Level-3 Consolidated Meta-Summary
            meta_summary_text = self.compressor.compress_meta_summaries(batch_texts, workspace=workspace)

            # Save new Meta-Summary
            meta_entry = MemoryEntry(
                content=meta_summary_text,
                level=MemoryLevel.META_SUMMARY,
                workspace=workspace
            )
            self.store.save_memory_entry(meta_entry)

            # Delete individual Level-1 entries that were merged
            self.store.delete_entries_by_ids(batch_ids)
            print(f"🎉 [Hierarchical Memory - Level 3] Compressed {len(batch_ids)} session summaries into 1 Meta-Summary!")

    def reset_memory(self, workspace: Optional[str] = None) -> int:
        """
        Resets long-term memory entries for a specific workspace or all workspaces.
        """
        return self.store.reset_memory(workspace=workspace)

def run_session_summarizer_async(thread_id: str, workspace: Optional[str] = None):
    """
    Asynchronous Entry Point: Launches background thread for non-blocking memory processing
    """
    manager = MemoryManager()
    thread = threading.Thread(
        target=manager.process_session_background,
        args=(thread_id, workspace)
    )
    thread.start()
