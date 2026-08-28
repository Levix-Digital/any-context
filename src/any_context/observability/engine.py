"""
Observability Engine.
Centralized high-performance engine for structured logging, metric capture,
and async span profiling across CLI, OpenTUI, RPC Bridge, and Background Tasks.
"""

import os
import sys
import traceback
import threading
from typing import Dict, Any, Optional
from any_context.observability.schemas import LogEvent, LogLevel, MetricEvent, TraceSpan
from any_context.observability.storage import ObservabilityStorage


class ObservabilityEngine:
    """Singleton Observability Engine providing sub-millisecond logging and telemetry."""

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


# Global singleton instance for easy import across modules
obs = ObservabilityEngine()
