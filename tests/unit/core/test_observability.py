"""
Unit Tests for any_context.observability.
Tests schemas, SQLite storage, ObservabilityEngine singleton,
and diagnostic report generation.
"""

import os
import tempfile
import pytest
from any_context.observability.schemas import (
    LogLevel,
    LogEvent,
    MetricEvent,
    TraceSpan,
    DiagnosticReport,
)
from any_context.observability.storage import ObservabilityStorage
from any_context.observability.engine import ObservabilityEngine
from any_context.observability.diagnostics import (
    collect_diagnostic_report,
    format_diagnostic_report,
    format_recent_logs,
)
from any_context.observability.telemetry import TelemetryService


def test_observability_schemas():
    event = LogEvent(
        component="CLI",
        level=LogLevel.WARN,
        message="Warning test message",
        metadata={"code": 404}
    )
    assert event.component == "CLI"
    assert event.level == LogLevel.WARN
    assert event.message == "Warning test message"
    assert event.metadata["code"] == 404
    assert event.timestamp is not None

    metric = MetricEvent(
        metric_name="latency.search",
        value=42.5,
        unit="ms",
        tags={"provider": "chroma"}
    )
    assert metric.metric_name == "latency.search"
    assert metric.value == 42.5
    assert metric.tags["provider"] == "chroma"


def test_observability_storage_roundtrip():
    with tempfile.TemporaryDirectory() as tmpdir:
        db_file = os.path.join(tmpdir, "test_obs.db")
        storage = ObservabilityStorage(db_path=db_file)

        # Insert logs
        storage.insert_log(LogEvent(component="AUTH", level=LogLevel.INFO, message="User logged in", metadata={"uid": 1}))
        storage.insert_log(LogEvent(component="RAG", level=LogLevel.ERROR, message="Vector search timeout", traceback="Traceback..."))

        logs = storage.get_recent_logs(limit=10)
        assert len(logs) == 2
        assert logs[0].component == "AUTH"
        assert logs[1].component == "RAG"
        assert logs[1].traceback == "Traceback..."

        # Filter by level
        error_logs = storage.get_recent_logs(limit=10, level="ERROR")
        assert len(error_logs) == 1
        assert error_logs[0].message == "Vector search timeout"

        # Insert metrics
        storage.insert_metric(MetricEvent(metric_name="cpu.load", value=12.4, unit="percent"))
        storage.prune_old_logs(max_entries=1)
        pruned_logs = storage.get_recent_logs(limit=10)
        assert len(pruned_logs) == 1


def test_observability_engine_methods():
    engine = ObservabilityEngine()
    engine.info("TEST", "Information message", {"key": "val"})
    engine.warn("TEST", "Warning message")
    engine.error("TEST", "Error message", exc=ValueError("Invalid parameter"))
    engine.record_metric("test.count", 1.0)


def test_diagnostics_report_generation():
    report = collect_diagnostic_report()
    assert isinstance(report, DiagnosticReport)
    assert report.actx_version is not None
    assert report.os_name is not None
    assert report.python_version is not None

    formatted = format_diagnostic_report(report)
    assert "AnyContext Diagnostics" in formatted
    assert report.actx_version in formatted

    logs = [LogEvent(component="CLI", level=LogLevel.INFO, message="Boot ok")]
    logs_formatted = format_recent_logs(logs)
    assert "AnyContext System Logs" in logs_formatted
    assert "Boot ok" in logs_formatted


def test_telemetry_service():
    tel = TelemetryService()
    tel.track_event("ui", "click", "modal_button", 1.0)
    tel.track_duration("query", 15.2, {"model": "gpt-4o-mini"})
