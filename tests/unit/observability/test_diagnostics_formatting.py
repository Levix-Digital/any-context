import unittest
from datetime import datetime

from any_context.observability.schemas import LogEvent, LogLevel, TraceSpan
from any_context.observability.diagnostics import (
    format_recent_logs,
    format_recent_spans,
    format_diagnostic_report,
    collect_diagnostic_report,
)
from any_context.commands.dispatcher import dispatch_command


class TestDiagnosticsFormatting(unittest.TestCase):
    """Verifies that diagnostics, logs, and spans format in clean Markdown with zero ANSI escape codes."""

    def test_format_recent_logs_zero_ansi_and_fenced_code_block(self):
        log_sample = [
            LogEvent(
                id=1,
                timestamp="2026-09-03T04:18:59.700375+00:00",
                level=LogLevel.INFO,
                component="RPC:RECV",
                message="Received method 'get_state' (id=728)",
                metadata={"test": "val"},
                traceback=None
            ),
            LogEvent(
                id=2,
                timestamp="2026-09-03T04:19:00.123456+00:00",
                level=LogLevel.ERROR,
                component="CORE",
                message="Sample warning message",
                metadata={},
                traceback="Traceback:\n  File foo.py, line 10\nValueError: boom"
            )
        ]

        formatted = format_recent_logs(log_sample)
        self.assertNotIn("\033", formatted, "Formatted logs must not contain raw octal 033 escape code")
        self.assertNotIn("\x1b", formatted, "Formatted logs must not contain raw hex 1b escape code")
        self.assertIn("```text", formatted, "Logs should be wrapped inside a fenced text code block")
        self.assertIn("```", formatted)
        self.assertIn("Received method 'get_state'", formatted)
        self.assertIn("ValueError: boom", formatted)

    def test_format_recent_logs_empty(self):
        formatted = format_recent_logs([])
        self.assertNotIn("\033", formatted)
        self.assertNotIn("\x1b", formatted)
        self.assertIn("No system logs", formatted)

    def test_format_recent_spans_zero_ansi_and_fenced_code_block(self):
        spans = [
            TraceSpan(
                span_id="sp1",
                trace_id="tr1",
                parent_span_id=None,
                name="cmd:/web",
                start_time="2026-09-03T04:18:59.000",
                end_time="2026-09-03T04:18:59.025",
                duration_ms=25.5,
                status="ok",
                metadata={"url": "https://example.com"}
            )
        ]

        formatted = format_recent_spans(spans)
        self.assertNotIn("\033", formatted)
        self.assertNotIn("\x1b", formatted)
        self.assertIn("```text", formatted)
        self.assertIn("cmd:/web", formatted)
        self.assertIn("25.50 ms", formatted)

    def test_format_diagnostic_report_zero_ansi(self):
        report = collect_diagnostic_report()
        formatted = format_diagnostic_report(report)
        self.assertNotIn("\033", formatted)
        self.assertNotIn("\x1b", formatted)
        self.assertTrue(formatted.startswith("### 📊 AnyContext Diagnostics"))
        self.assertIn("#### ⚡ OpenTUI Desktop Runtime (Bun)", formatted)
        self.assertIn("#### 💾 SQLite Configuration & Settings DB", formatted)

    def test_dispatch_logs_command_is_ansi_free(self):
        result = dispatch_command("/logs 5", active_workspace="Default")
        self.assertTrue(result.success)
        self.assertNotIn("\033", result.message)
        self.assertNotIn("\x1b", result.message)


if __name__ == "__main__":
    unittest.main()
