"""
Facade for backward-compatibility with core.memory_manager
Delegates memory operations to the modular any_context.memory package
"""

from any_context.memory import MemoryManager, run_session_summarizer_async

__all__ = ["MemoryManager", "run_session_summarizer_async"]
