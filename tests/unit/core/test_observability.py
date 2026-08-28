"""
Unit Tests for any_context.observability.
Tests schemas, SQLite storage, ObservabilityEngine singleton,
and diagnostic report generation using unittest.TestCase.
"""

import os
import unittest
import tempfile
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


class TestObservabilityModule(unittest.TestCase):
    """Unit tests for AnyContext observability subsystem."""

    def test_observability_schemas(self):
        event = LogEvent(
            component="CLI",
            level=LogLevel.WARN,
            message="Warning test message",
            metadata={"code": 404}
        )
        self.assertEqual(event.component, "CLI")
        self.assertEqual(event.level, LogLevel.WARN)
        self.assertEqual(event.message, "Warning test message")
        self.assertEqual(event.metadata["code"], 404)
        self.assertIsNotNone(event.timestamp)

        metric = MetricEvent(
            metric_name="latency.search",
            value=42.5,
            unit="ms",
            tags={"provider": "chroma"}
        )
        self.assertEqual(metric.metric_name, "latency.search")
        self.assertEqual(metric.value, 42.5)
        self.assertEqual(metric.tags["provider"], "chroma")

    def test_observability_storage_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            db_file = os.path.join(tmpdir, "test_obs.db")
            # Reset storage singleton instance for isolated test
            ObservabilityStorage._instance = None
            storage = ObservabilityStorage(db_path=db_file)

            # Insert logs
            storage.insert_log(LogEvent(component="AUTH", level=LogLevel.INFO, message="User logged in", metadata={"uid": 1}))
            storage.insert_log(LogEvent(component="RAG", level=LogLevel.ERROR, message="Vector search timeout", traceback="Traceback..."))

            logs = storage.get_recent_logs(limit=10)
            self.assertEqual(len(logs), 2)
            self.assertEqual(logs[0].component, "AUTH")
            self.assertEqual(logs[1].component, "RAG")
            self.assertEqual(logs[1].traceback, "Traceback...")

            # Filter by level
            error_logs = storage.get_recent_logs(limit=10, level="ERROR")
            self.assertEqual(len(error_logs), 1)
            self.assertEqual(error_logs[0].message, "Vector search timeout")

            # Insert metrics and prune
            storage.insert_metric(MetricEvent(metric_name="cpu.load", value=12.4, unit="percent"))
            storage.prune_old_logs(max_entries=1)
            pruned_logs = storage.get_recent_logs(limit=10)
            self.assertEqual(len(pruned_logs), 1)

            # Reset singleton instance
            ObservabilityStorage._instance = None

    def test_observability_engine_methods(self):
        engine = ObservabilityEngine()
        engine.info("TEST", "Information message", {"key": "val"})
        engine.warn("TEST", "Warning message")
        engine.error("TEST", "Error message", exc=ValueError("Invalid parameter"))
        engine.record_metric("test.count", 1.0)

    def test_diagnostics_report_generation(self):
        report = collect_diagnostic_report()
        self.assertIsInstance(report, DiagnosticReport)
        self.assertIsNotNone(report.actx_version)
        self.assertIsNotNone(report.os_name)
        self.assertIsNotNone(report.python_version)

        formatted = format_diagnostic_report(report)
        self.assertIn("AnyContext Diagnostics", formatted)
        self.assertIn(report.actx_version, formatted)

        logs = [LogEvent(component="CLI", level=LogLevel.INFO, message="Boot ok")]
        logs_formatted = format_recent_logs(logs)
        self.assertIn("AnyContext System Logs", logs_formatted)
        self.assertIn("Boot ok", logs_formatted)

    def test_telemetry_service(self):
        tel = TelemetryService()
        tel.track_event("ui", "click", "modal_button", 1.0)
        tel.track_duration("query", 15.2, {"model": "gpt-4o-mini"})


if __name__ == "__main__":
    unittest.main()
