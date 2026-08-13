from enum import Enum
from typing import Optional, Dict, Any
from dataclasses import dataclass, field
import datetime

class MemoryLevel(str, Enum):
    SHORT_TERM = "short_term"          # Raw chat messages
    SESSION_SUMMARY = "session_summary"  # Level-1: Compressed block of 10 interactions
    META_SUMMARY = "meta_summary"        # Level-3: High-level consolidated meta-memory

@dataclass
class MemoryEntry:
    content: str
    level: MemoryLevel
    workspace: Optional[str] = None
    thread_id: Optional[str] = None
    timestamp: str = field(default_factory=lambda: datetime.datetime.utcnow().isoformat())
    metadata: Dict[str, Any] = field(default_factory=dict)
