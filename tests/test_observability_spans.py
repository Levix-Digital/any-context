import os
import time
import pytest
from any_context.observability import (
    obs,
    SpanContext,
    TraceSpan,
    ObservabilityStorage,
    format_recent_spans,
    collect_diagnostic_report,
    format_diagnostic_report
)


def test_span_context_manager(tmp_path):
    db_file = str(tmp_path / "test_obs.db")
    storage = ObservabilityStorage(db_path=db_file)
    obs._storage = storage

    with obs.span("unit_test:operation", workspace="TestWS", custom_meta="hello") as s:
        time.sleep(0.01)  # 10ms
        s.set_metadata("step", "done")

    spans = storage.get_recent_spans(limit=10, name="unit_test:operation")
    assert len(spans) == 1
    span = spans[0]
    assert span.name == "unit_test:operation"
    assert span.status == "ok"
    assert span.duration_ms >= 5.0
    assert span.metadata.get("workspace") == "TestWS"
    assert span.metadata.get("custom_meta") == "hello"
    assert span.metadata.get("step") == "done"


def test_span_exception_capture(tmp_path):
    db_file = str(tmp_path / "test_obs_err.db")
    storage = ObservabilityStorage(db_path=db_file)
    obs._storage = storage

    with pytest.raises(ValueError):
        with obs.span("unit_test:failing_op", workspace="ErrorWS"):
            raise ValueError("Something broke deliberately")

    spans = storage.get_recent_spans(limit=10, name="unit_test:failing_op")
    assert len(spans) == 1
    span = spans[0]
    assert span.status == "error"
    assert span.metadata.get("error_type") == "ValueError"
    assert "Something broke" in span.metadata.get("error_message", "")


def test_timed_decorator(tmp_path):
    db_file = str(tmp_path / "test_obs_dec.db")
    storage = ObservabilityStorage(db_path=db_file)
    obs._storage = storage

    @obs.timed("decorated_calc", env="test")
    def compute_val(a, b):
        time.sleep(0.005)
        return a + b

    res = compute_val(10, 20)
    assert res == 30

    spans = storage.get_recent_spans(limit=10, name="decorated_calc")
    assert len(spans) == 1
    assert spans[0].name == "decorated_calc"
    assert spans[0].duration_ms >= 3.0


def test_latency_summary_aggregation(tmp_path):
    db_file = str(tmp_path / "test_obs_sum.db")
    storage = ObservabilityStorage(db_path=db_file)
    obs._storage = storage

    for i in range(3):
        with obs.span("rag:test_query"):
            time.sleep(0.002)

    summary = storage.get_latency_summary(limit=50)
    rag_stat = next((s for s in summary if s["name"] == "rag:test_query"), None)
    assert rag_stat is not None
    assert rag_stat["count"] == 3
    assert rag_stat["avg_ms"] > 0.0


def test_format_recent_spans():
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
    assert "cli:boot" in output
    assert "rag:retrieval" in output
    assert "45.20 ms" in output
