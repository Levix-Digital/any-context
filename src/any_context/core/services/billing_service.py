"""
BillingService - Core Application Service for subscription tier, limits, and plan details.
Pure domain logic: decoupled from terminal UI, CLI formatters, HTTP, and RPC transports.
"""

from typing import Dict, Any, List, Optional
from any_context.billing.manager import BillingManager


class BillingService:
    """Service managing plan tiers, subscription quotas, and billing status."""

    def __init__(self):
        self.mgr = BillingManager()

    def get_billing_info(self) -> Dict[str, Any]:
        """Returns the current billing plan status and full capabilities matrix."""
        status = self.mgr.get_status()
        matrix = self.mgr.format_pricing_cards_cli()
        tier_id = getattr(status, "tier_id", "community")
        is_active = getattr(status, "is_active", True)

        return {
            "current_tier": tier_id,
            "status": "active" if is_active else "inactive",
            "matrix_text": matrix,
            "subscription": {
                "tier_id": tier_id,
                "is_active": is_active
            }
        }
