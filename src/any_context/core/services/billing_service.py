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
        tier_id = getattr(status, "active_tier_id", getattr(status, "tier_id", "community"))
        tier_name = getattr(status, "active_tier_name", "AnyContext Community")
        license_key = getattr(status, "license_key", None)

        return {
            "current_tier": tier_id,
            "tier_id": tier_id,
            "tier_name": tier_name,
            "active_tier_id": tier_id,
            "active_tier_name": tier_name,
            "license_key": license_key,
            "status": "active",
            "matrix_text": matrix,
            "subscription": {
                "tier_id": tier_id,
                "tier_name": tier_name,
                "license_key": license_key,
                "is_active": True
            }
        }
