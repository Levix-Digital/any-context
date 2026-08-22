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

    def can_use_server_mode(self) -> bool:
        """Returns True if active tier supports REST API Server daemon mode (Pro, Team, Enterprise)."""
        status = self.get_status()
        return status.capabilities.supports_server_mode

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

    def format_pricing_cards_cli(self) -> str:
        """Formats the official AnyContext pricing and plans as clean text."""
        plans = get_all_plans()
        current_tier = self.get_status().active_tier_id
        lines = []
        lines.append("=" * 80)
        lines.append("                    ANYCONTEXT PLANS & CAPABILITY MATRIX                    ")
        lines.append("=" * 80)

        for idx, p in enumerate(plans, 1):
            is_active = (p.tier_id == current_tier)
            active_badge = " [PLANO ATIVO]" if is_active else ""
            
            if p.tier_id == "community":
                m_price = "Grátis ($0 / sempre)"
                a_price = "Grátis ($0 / ano)"
            elif p.tier_id == "team":
                m_price = f"${p.monthly_price_usd:.0f}/mês (5 seats inclusos) + ${p.extra_seat_price_usd:.0f}/seat extra"
                a_price = f"${p.annual_price_usd:.0f}/ano base (~$65/mês - 20% OFF)"
            else:
                m_price = f"${p.monthly_price_usd:.0f}/mês"
                a_price = f"${p.annual_price_usd:.0f}/ano (~${p.annual_price_usd/12:.0f}/mês - 20% OFF)" if p.annual_price_usd else "-"

            lines.append(f"\n  [{idx}] {p.name}{active_badge}")
            lines.append("  " + "-" * 74)
            lines.append(f"  • Preço Mensal   : {m_price}")
            lines.append(f"  • Preço Anual    : {a_price}")
            lines.append(f"  • Escopo & RAG   : {p.ingestion_scope}")
            lines.append(f"  • Público-Alvo   : {p.target_audience}")

        lines.append("\n" + "=" * 80)
        return "\n".join(lines)


