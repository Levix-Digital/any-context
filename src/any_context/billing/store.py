import sqlite3
from typing import Optional
from datetime import datetime
from any_context.config.db_store import ConfigDBStore
from any_context.billing.models import SubscriptionStatus
from any_context.billing.registry import get_plan_by_id

class BillingStore:
    """
    SQLite-backed storage manager for AnyContext Subscription License Keys & Active Plan Tiers.
    """

    def __init__(self, db_path: Optional[str] = None):
        self.db_path = db_path or ConfigDBStore.find_db_file("settings.db")
        self._init_db()

    def _get_connection(self):
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        return conn

    def _init_db(self):
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS subscription_license (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    tier_id TEXT NOT NULL DEFAULT 'community',
                    license_key TEXT,
                    activated_at TEXT,
                    expires_at TEXT
                );
            """)
            # Ensure row 1 exists
            cursor.execute("SELECT COUNT(*) FROM subscription_license")
            if cursor.fetchone()[0] == 0:
                now_str = datetime.utcnow().isoformat()
                cursor.execute("""
                    INSERT INTO subscription_license (id, tier_id, license_key, activated_at)
                    VALUES (1, 'community', 'COMMUNITY-OPEN-LICENSE', ?)
                """, (now_str,))
            conn.commit()

    def get_subscription_status(self) -> SubscriptionStatus:
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT tier_id, license_key, activated_at, expires_at FROM subscription_license WHERE id = 1")
            r = cursor.fetchone()
            if r:
                tier_id = r["tier_id"]
                plan = get_plan_by_id(tier_id)
                return SubscriptionStatus(
                    active_tier_id=plan.tier_id,
                    active_tier_name=plan.name,
                    license_key=r["license_key"],
                    activated_at=str(r["activated_at"]) if r["activated_at"] else None,
                    expires_at=str(r["expires_at"]) if r["expires_at"] else None,
                    capabilities=plan.capabilities
                )
            plan = get_plan_by_id("community")
            return SubscriptionStatus(
                active_tier_id="community",
                active_tier_name=plan.name,
                capabilities=plan.capabilities
            )

    def set_active_tier(self, tier_id: str, license_key: Optional[str] = None, expires_at: Optional[str] = None) -> SubscriptionStatus:
        plan = get_plan_by_id(tier_id)
        now_str = datetime.utcnow().isoformat()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                UPDATE subscription_license
                SET tier_id = ?, license_key = ?, activated_at = ?, expires_at = ?
                WHERE id = 1
            """, (plan.tier_id, license_key or f"ACTX-{plan.tier_id.upper()}-LICENSE", now_str, expires_at))
            conn.commit()
        return self.get_subscription_status()
