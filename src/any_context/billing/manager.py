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

    def can_use_multi_context(self) -> bool:
        """Returns True if active tier supports multi-context workspaces (combining local+drive+web)."""
        status = self.get_status()
        return status.capabilities.supports_multi_context

    def can_use_collaboration(self) -> bool:
        """Returns True if active tier supports multi-user collaboration & workspace sharing."""
        status = self.get_status()
        return status.capabilities.supports_collaboration

    def format_pricing_table_markdown(self) -> str:
        """Formats the official AnyContext pricing and plans table as clean Markdown."""
        plans = get_all_plans()
        md = []
        md.append("| **Plano** | **Preço Mensal** | **Preço Anual** | **Escopo de Ingestão e Capacidades** | **Público-Alvo** |")
        md.append("|---|---|---|---|---|")
        for p in plans:
            m_price = f"${p.monthly_price_usd:.0f}" if p.monthly_price_usd > 0 else "Grátis"
            a_price = f"${p.annual_price_usd:.0f}+" if p.annual_price_usd else "-"
            md.append(f"| {p.name} | {m_price} | {a_price} | {p.ingestion_scope} | {p.target_audience} |")
        return "\n".join(md)
