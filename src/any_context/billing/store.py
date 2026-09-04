import sqlite3
from typing import Optional
from datetime import datetime
from any_context.config.db_store import ConfigDBStore
from any_context.billing.models import SubscriptionStatus
from any_context.billing.registry import get_plan_by_id

class BillingStore:
    """
    SQLite-backed storage manager for AnyContext Subscription License Keys, Active Plan Tiers, and Extra Seats.
    """

    def __init__(self, db_path: Optional[str] = None):
        self.db_path = db_path or ConfigDBStore.find_db_file("settings.db")
        self._init_db()

    def _get_connection(self):
        from any_context.config.database import DatabaseManager
        return DatabaseManager(self.db_path).get_connection()

    def _init_db(self):
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS subscription_license (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    tier_id TEXT NOT NULL DEFAULT 'community',
                    license_key TEXT,
                    activated_at TEXT,
                    expires_at TEXT,
                    extra_seats_purchased INTEGER DEFAULT 0
                );
            """)
            # Check column existence for migration
            cursor.execute("PRAGMA table_info(subscription_license)")
            cols = [row["name"] for row in cursor.fetchall()]
            if "extra_seats_purchased" not in cols:
                cursor.execute("ALTER TABLE subscription_license ADD COLUMN extra_seats_purchased INTEGER DEFAULT 0")

            cursor.execute("SELECT COUNT(*) FROM subscription_license")
            if cursor.fetchone()[0] == 0:
                now_str = datetime.utcnow().isoformat()
                cursor.execute("""
                    INSERT INTO subscription_license (id, tier_id, license_key, activated_at, extra_seats_purchased)
                    VALUES (1, 'community', 'COMMUNITY-OPEN-LICENSE', ?, 0)
                """, (now_str,))
            conn.commit()

    def get_subscription_status(self) -> SubscriptionStatus:
        import os
        from dotenv import load_dotenv
        from any_context.billing.crypto import verify_license_key
        load_dotenv()

        env_key = os.getenv("ANYCONTEXT_LICENSE_KEY") or os.getenv("LEVIX_LICENSE_KEY")
        if env_key and env_key.strip():
            is_valid, claims, err = verify_license_key(env_key.strip())
            if is_valid and claims:
                tier = claims.get("tier", "enterprise")
                plan = get_plan_by_id(tier)
                return SubscriptionStatus(
                    active_tier_id=plan.tier_id,
                    active_tier_name=plan.name,
                    license_key=env_key.strip(),
                    activated_at=claims.get("issued_at", "Activated via .env"),
                    expires_at=claims.get("expires_at"),
                    base_seats=claims.get("seats", plan.base_seats),
                    total_seats=claims.get("seats", plan.base_seats),
                    extra_seat_price_usd=plan.extra_seat_price_usd,
                    capabilities=plan.capabilities
                )

        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT tier_id, license_key, activated_at, expires_at, extra_seats_purchased FROM subscription_license WHERE id = 1")
            r = cursor.fetchone()
            if r:
                tier_id = r["tier_id"]
                plan = get_plan_by_id(tier_id)
                extra_seats = r["extra_seats_purchased"] or 0
                total_seats = plan.base_seats + extra_seats
                return SubscriptionStatus(
                    active_tier_id=plan.tier_id,
                    active_tier_name=plan.name,
                    license_key=r["license_key"],
                    activated_at=str(r["activated_at"]) if r["activated_at"] else None,
                    expires_at=str(r["expires_at"]) if r["expires_at"] else None,
                    base_seats=plan.base_seats,
                    extra_seats_purchased=extra_seats,
                    total_seats=total_seats,
                    extra_seat_price_usd=plan.extra_seat_price_usd,
                    capabilities=plan.capabilities
                )
            plan = get_plan_by_id("community")
            return SubscriptionStatus(
                active_tier_id="community",
                active_tier_name=plan.name,
                base_seats=plan.base_seats,
                total_seats=plan.base_seats,
                capabilities=plan.capabilities
            )

    def set_active_tier(self, tier_id: str, license_key: Optional[str] = None, expires_at: Optional[str] = None, extra_seats: int = 0) -> SubscriptionStatus:
        plan = get_plan_by_id(tier_id)
        now_str = datetime.utcnow().isoformat()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                UPDATE subscription_license
                SET tier_id = ?, license_key = ?, activated_at = ?, expires_at = ?, extra_seats_purchased = ?
                WHERE id = 1
            """, (plan.tier_id, license_key or f"ACTX-{plan.tier_id.upper()}-LICENSE", now_str, expires_at, extra_seats))
            conn.commit()
        return self.get_subscription_status()
