from typing import Optional
from any_context.billing.store import BillingStore
from any_context.billing.registry import get_all_plans
from any_context.billing.models import SubscriptionStatus

class BillingManager:
    """
    High-level Subscription & Feature Gate Enforcement Manager.
    """

    def __init__(self, store: Optional[BillingStore] = None):
        self.store = store or BillingStore()

    def get_status(self) -> SubscriptionStatus:
        return self.store.get_subscription_status()

    def can_ingest_source(self, source_type: str) -> bool:
        """Returns True if the active tier supports the requested ingestor source (e.g. 'local', 'drive', 'web')."""
        status = self.get_status()
        st = source_type.lower().strip()
        return st in status.capabilities.allowed_sources

    def can_use_ocr(self) -> bool:
        """Returns True if active tier supports Image & Scanned PDF OCR parsing (Starter, Pro, Team, Enterprise)."""
        status = self.get_status()
        return status.capabilities.supports_ocr

    def can_use_multi_context(self) -> bool:
        """Returns True if active tier supports multi-context workspaces (combining local+drive+web)."""
        status = self.get_status()
        return status.capabilities.supports_multi_context

    def can_use_collaboration(self) -> bool:
        """Returns True if active tier supports multi-user collaboration & workspace sharing."""
        status = self.get_status()
        return status.capabilities.supports_collaboration

    def can_add_workspace(self, current_workspace_count: int) -> bool:
        """Returns True if user has not exceeded active tier's max workspace limit."""
        status = self.get_status()
        return current_workspace_count < status.capabilities.max_workspaces

    def calculate_team_monthly_price(self, extra_seats: int = 0) -> float:
        """Calculates total monthly price for Team plan with seat add-ons ($79 base + $15/extra seat)."""
        status = self.get_status()
        base_price = 79.0
        extra_cost = (extra_seats or status.extra_seats_purchased) * 15.0
        return base_price + extra_cost

    def format_pricing_table_markdown(self) -> str:
        """Formats the official AnyContext pricing and plans table as clean Markdown."""
        plans = get_all_plans()
        md = []
        md.append("| **Plano** | **Preço Mensal** | **Preço Anual (-20%)** | **Escopo & Funcionalidades** | **Público-Alvo** |")
        md.append("|---|---|---|---|---|")
        for p in plans:
            if p.tier_id == "team":
                m_price = f"${p.monthly_price_usd:.0f}/mês (5 seats) + ${p.extra_seat_price_usd:.0f}/seat extra"
                a_price = f"${p.annual_price_usd:.0f}/ano base"
            elif p.monthly_price_usd == 0:
                m_price = "Grátis ($0)"
                a_price = "-"
            else:
                m_price = f"${p.monthly_price_usd:.0f}/mês"
                a_price = f"${p.annual_price_usd:.0f}/ano" if p.annual_price_usd else "-"
            
            md.append(f"| **{p.name}** | {m_price} | {a_price} | {p.ingestion_scope} | {p.target_audience} |")
        return "\n".join(md)
