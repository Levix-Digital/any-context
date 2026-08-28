"""
Telemetry & Usage Analytics (Future-Ready).
Provides anonymized metrics aggregation, latency tracking, and extensible hooks
for future product improvements and performance visualizations.
"""

from typing import Dict, Any, Optional
from any_context.observability.engine import obs


class TelemetryService:
    """Manages telemetry events, duration profiling, and anonymous product health metrics."""

    def __init__(self):
        self.enabled = False  # Default to 100% offline / disabled

    def track_event(self, category: str, action: str, label: Optional[str] = None, value: Optional[float] = None):
        """Records an anonymous usage event for future aggregation."""
        obs.info(
            component="TELEMETRY",
            message=f"Event: {category}.{action}",
            metadata={"category": category, "action": action, "label": label, "value": value}
        )

    def track_duration(self, operation: str, duration_ms: float, tags: Optional[Dict[str, str]] = None):
        """Records the latency of an async operation or LLM request."""
        obs.record_metric(
            name=f"duration.{operation}",
            value=duration_ms,
            unit="ms",
            tags=tags or {}
        )


telemetry = TelemetryService()
