from typing import Optional
from any_context.memory.models import MemoryLevel, MemoryEntry
from any_context.memory.store import MemoryStore
from any_context.memory.compressor import MemoryCompressor
from any_context.memory.manager import MemoryManager, run_session_summarizer_async

def reset_session_memory(workspace: Optional[str] = "Default", thread_id: Optional[str] = None) -> int:
    """Convenience function to reset long-term session memory for a workspace."""
    manager = MemoryManager()
    return manager.reset_memory(workspace=workspace)

__all__ = [
    "MemoryLevel",
    "MemoryEntry",
    "MemoryStore",
    "MemoryCompressor",
    "MemoryManager",
    "run_session_summarizer_async",
    "reset_session_memory",
]
