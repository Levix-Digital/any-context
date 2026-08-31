import os
import sys
import time
import uuid
import functools
import traceback
import threading
from typing import Dict, Any, Optional, Callable
from datetime import datetime, timezone
from any_context.observability.schemas import LogEvent, LogLevel, MetricEvent, TraceSpan
from any_context.observability.storage import ObservabilityStorage


class SpanContext:
    """
    High-precision performance span context manager.
    Measures elapsed execution time in milliseconds and automatically persists TraceSpan.
    """

    def __init__(
        self,
        engine: "ObservabilityEngine",
        name: str,
        parent_id: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None
    ):
        self.engine = engine
        self.name = name
        self.parent_id = parent_id
        self.metadata = metadata or {}
        self.span_id = f"span_{uuid.uuid4().hex[:12]}"
        self.start_perf = 0.0
        self.start_time_iso = ""
        self.end_time_iso = ""
        self.duration_ms = 0.0
        self.status = "ok"

    def __enter__(self) -> "SpanContext":
        self.start_perf = time.perf_counter()
        self.start_time_iso = datetime.now(timezone.utc).isoformat()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        end_perf = time.perf_counter()
        self.end_time_iso = datetime.now(timezone.utc).isoformat()
        self.duration_ms = round((end_perf - self.start_perf) * 1000, 3)

        if exc_type is not None:
            self.status = "error"
            self.metadata["error_type"] = exc_type.__name__
            self.metadata["error_message"] = str(exc_val)

        span = TraceSpan(
            span_id=self.span_id,
            parent_id=self.parent_id,
            name=self.name,
            status=self.status,
            start_time=self.start_time_iso,
            end_time=self.end_time_iso,
            duration_ms=self.duration_ms,
            metadata=self.metadata,
        )
        self.engine._storage.insert_span(span)
        return False  # Do not suppress exceptions

    def set_metadata(self, key: str, value: Any):
        """Adds or updates metadata on the active span."""
        self.metadata[key] = value


class ObservabilityEngine:
    """Singleton Observability Engine providing sub-millisecond logging, latency spans, and telemetry."""

    _instance = None
    _lock = threading.Lock()

    def __new__(cls):
        with cls._lock:
            if cls._instance is None:
                cls._instance = super(ObservabilityEngine, cls).__new__(cls)
                cls._instance._storage = ObservabilityStorage()
            return cls._instance

    def log(
        self,
        component: str,
        level: LogLevel,
        message: str,
        metadata: Optional[Dict[str, Any]] = None,
        exc: Optional[Exception] = None
    ):
        """Emits a structured log event to SQLite."""
        tb_str = None
        if exc is not None:
            tb_str = "".join(traceback.format_exception(type(exc), exc, exc.__traceback__))

        event = LogEvent(
            component=component.upper(),
            level=level,
            message=message,
            metadata=metadata or {},
            traceback=tb_str,
        )
        self._storage.insert_log(event)

    def debug(self, component: str, message: str, metadata: Optional[Dict[str, Any]] = None):
        self.log(component, LogLevel.DEBUG, message, metadata)

    def info(self, component: str, message: str, metadata: Optional[Dict[str, Any]] = None):
        self.log(component, LogLevel.INFO, message, metadata)

    def warn(self, component: str, message: str, metadata: Optional[Dict[str, Any]] = None):
        self.log(component, LogLevel.WARN, message, metadata)

    def error(
        self,
        component: str,
        message: str,
        metadata: Optional[Dict[str, Any]] = None,
        exc: Optional[Exception] = None
    ):
        self.log(component, LogLevel.ERROR, message, metadata, exc=exc)

    def record_metric(self, name: str, value: float, unit: str = "count", tags: Optional[Dict[str, str]] = None):
        event = MetricEvent(metric_name=name, value=value, unit=unit, tags=tags or {})
        self._storage.insert_metric(event)

    def span(self, name: str, parent_id: Optional[str] = None, **metadata) -> SpanContext:
        """
        Creates a high-precision performance execution span.
        Usage:
            with obs.span("rag:hybrid_search", workspace="Default") as s:
                results = search()
        """
        return SpanContext(engine=self, name=name, parent_id=parent_id, metadata=metadata)

    def timed(self, name: Optional[str] = None, **metadata):
        """
        Decorator for tracking function execution duration and recording a TraceSpan.
        Usage:
            @obs.timed("model:inference")
            def run_inference():
                ...
        """
        def decorator(func: Callable):
            span_name = name or f"{func.__module__}.{func.__qualname__}"

            @functools.wraps(func)
            def wrapper(*args, **kwargs):
                with self.span(span_name, **metadata):
                    return func(*args, **kwargs)
            return wrapper
        return decorator

    def get_recent_spans(self, limit: int = 50, name: Optional[str] = None):
        return self._storage.get_recent_spans(limit=limit, name=name)

    def get_latency_summary(self, limit: int = 200):
        return self._storage.get_latency_summary(limit=limit)


# Global singleton instance for easy import across modules
obs = ObservabilityEngine()

