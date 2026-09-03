import os
import sys
import time
import unittest
import tempfile
import shutil

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.observability import (
    obs,
    SpanContext,
    TraceSpan,
    ObservabilityStorage,
    format_recent_spans,
    collect_diagnostic_report,
    format_diagnostic_report
)


class TestObservabilitySpans(unittest.TestCase):
    """
    Unit and Integration Tests for AnyContext Observability Spans & Latency Tracking.
    100% native unittest.TestCase without pytest dependencies.
    """

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="actx_test_obs_")
        self.db_file = os.path.join(self.temp_dir, "test_obs.db")
        self.storage = ObservabilityStorage(db_path=self.db_file)
        obs._storage = self.storage

    def tearDown(self):
        try:
            shutil.rmtree(self.temp_dir)
        except Exception:
            pass

    def test_span_context_manager(self):
        with obs.span("unit_test:operation", workspace="TestWS", custom_meta="hello") as s:
            time.sleep(0.01)  # 10ms
            s.set_metadata("step", "done")

        spans = self.storage.get_recent_spans(limit=10, name="unit_test:operation")
        self.assertEqual(len(spans), 1)
        span = spans[0]
        self.assertEqual(span.name, "unit_test:operation")
        self.assertEqual(span.status, "ok")
        self.assertGreaterEqual(span.duration_ms, 5.0)
        self.assertEqual(span.metadata.get("workspace"), "TestWS")
        self.assertEqual(span.metadata.get("custom_meta"), "hello")
        self.assertEqual(span.metadata.get("step"), "done")

    def test_span_exception_capture(self):
        with self.assertRaises(ValueError):
            with obs.span("unit_test:failing_op", workspace="ErrorWS"):
                raise ValueError("Something broke deliberately")

        spans = self.storage.get_recent_spans(limit=10, name="unit_test:failing_op")
        self.assertEqual(len(spans), 1)
        span = spans[0]
        self.assertEqual(span.status, "error")
        self.assertEqual(span.metadata.get("error_type"), "ValueError")
        self.assertIn("Something broke", span.metadata.get("error_message", ""))

    def test_timed_decorator(self):
        @obs.timed("decorated_calc", env="test")
        def compute_val(a, b):
            time.sleep(0.005)
            return a + b

        res = compute_val(10, 20)
        self.assertEqual(res, 30)

        spans = self.storage.get_recent_spans(limit=10, name="decorated_calc")
        self.assertEqual(len(spans), 1)
        self.assertEqual(spans[0].name, "decorated_calc")
        self.assertGreaterEqual(spans[0].duration_ms, 3.0)

    def test_latency_summary_aggregation(self):
        for i in range(3):
            with obs.span("rag:test_query"):
                time.sleep(0.002)

        summary = self.storage.get_latency_summary(limit=50)
        rag_stat = next((s for s in summary if s["name"] == "rag:test_query"), None)
        self.assertIsNotNone(rag_stat)
        self.assertEqual(rag_stat["count"], 3)
        self.assertGreater(rag_stat["avg_ms"], 0.0)

    def test_format_recent_spans(self):
        test_spans = [
            TraceSpan(
                span_id="s1",
                name="cli:boot",
                status="ok",
                start_time="2026-08-30T18:00:00Z",
                duration_ms=45.2,
                metadata={"workspace": "Default"}
            ),
            TraceSpan(
                span_id="s2",
                name="rag:retrieval",
                status="error",
                start_time="2026-08-30T18:00:01Z",
                duration_ms=120.8,
                metadata={"error": "timeout"}
            )
        ]
        output = format_recent_spans(test_spans)
        self.assertIn("cli:boot", output)
        self.assertIn("rag:retrieval", output)
        self.assertIn("45.20 ms", output)


if __name__ == "__main__":
    unittest.main()
