"""
Observability Schemas for AnyContext Core.
Defines data models for structured logs, system metrics, performance traces,
and health diagnostic reports.
"""

from enum import Enum
from typing import Dict, Any, Optional, List
from datetime import datetime, timezone
from pydantic import BaseModel, Field


class LogLevel(str, Enum):
    """Supported logging severity levels."""
    DEBUG = "DEBUG"
    INFO = "INFO"
    WARN = "WARN"
    ERROR = "ERROR"
    CRITICAL = "CRITICAL"


class LogEvent(BaseModel):
    """Structured system log event for SQLite and diagnostic streams."""
    timestamp: str = Field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    component: str = "CORE"
    level: LogLevel = LogLevel.INFO
    message: str
    metadata: Dict[str, Any] = Field(default_factory=dict)
    traceback: Optional[str] = None


class MetricEvent(BaseModel):
    """Quantitative performance or usage metric."""
    timestamp: str = Field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    metric_name: str
    value: float
    unit: str = "count"  # "ms", "bytes", "count", "percent"
    tags: Dict[str, str] = Field(default_factory=dict)


class TraceSpan(BaseModel):
    """Execution span for async task profiling, latency tracking, and pipeline analysis."""
    span_id: str
    parent_id: Optional[str] = None
    name: str
    status: str = "ok"  # "ok" or "error"
    start_time: str = Field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    end_time: Optional[str] = None
    duration_ms: Optional[float] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)



class DiagnosticReport(BaseModel):
    """Complete environment health and integrity report."""
    timestamp: str = Field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    os_name: str
    os_platform: str
    python_version: str
    actx_version: str
    executable_path: str
    is_frozen: bool
    bun_installed: bool
    bun_version: Optional[str] = None
    bun_path: Optional[str] = None
    database_path: str
    database_exists: bool
    database_size_bytes: int = 0
    onboarding_completed: bool = False
    active_model: str = "unknown"
    active_provider: str = "unknown"
    latency_summary: List[Dict[str, Any]] = Field(default_factory=list)
    recent_errors: List[LogEvent] = Field(default_factory=list)

