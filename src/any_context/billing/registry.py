from typing import Dict, List
from any_context.billing.models import PlanTier, PlanCapabilities

PLANS_REGISTRY: Dict[str, PlanTier] = {
    "community": PlanTier(
        tier_id="community",
        name="AnyContext Community",
        monthly_price_usd=0.0,
        annual_price_usd=0.0,
        base_seats=1,
        extra_seat_price_usd=0.0,
        ingestion_scope="1 Workspace Local (até 3 pastas locais).",
        target_audience="Desenvolvedores, estudantes e entusiastas de IA local",
        capabilities=PlanCapabilities(
            allowed_sources=["local"],
            max_workspaces=1,
            supports_ocr=False,
            supports_multi_context=False,
            supports_collaboration=False,
            supports_custom_vpc=False
        )
    ),
    "starter": PlanTier(
        tier_id="starter",
        name="AnyContext Personal / Starter",
        monthly_price_usd=12.0,
        annual_price_usd=108.0, # $9/mo billed annually
        base_seats=1,
        extra_seat_price_usd=0.0,
        ingestion_scope="Workspaces Locais Ilimitados + OCR de Imagens e PDFs Escaneados",
        target_audience="Pesquisadores individuais, advogados solo, consultores",
        capabilities=PlanCapabilities(
            allowed_sources=["local"],
            max_workspaces=999,
            supports_ocr=True,
            supports_multi_context=False,
            supports_collaboration=False,
            supports_custom_vpc=False
        )
    ),
    "pro": PlanTier(
        tier_id="pro",
        name="AnyContext Pro (Multi-Context)",
        monthly_price_usd=29.0,
        annual_price_usd=288.0, # $24/mo billed annually
        base_seats=1,
        extra_seat_price_usd=0.0,
        ingestion_scope="Multi-Context: Pastas Locais + Google Drive + OneDrive + Web Scraping + OCR",
        target_audience="Power users, analistas seniores, freelancers",
        capabilities=PlanCapabilities(
            allowed_sources=["local", "drive", "web"],
            max_workspaces=999,
            supports_ocr=True,
            supports_multi_context=True,
            supports_collaboration=False,
            supports_custom_vpc=False
        )
    ),
    "team": PlanTier(
        tier_id="team",
        name="AnyContext Team",
        monthly_price_usd=79.0,
        annual_price_usd=780.0, # $65/mo base billed annually
        base_seats=5,
        extra_seat_price_usd=15.0, # +$15/seat/month
        ingestion_scope="Multi-Context + Multi-Usuário (RBAC, Convites SHARE-WKS, Workspaces Compartilhados)",
        target_audience="Escritórios de advocacia, consultorias imigratórias, equipes de engenharia",
        capabilities=PlanCapabilities(
            allowed_sources=["local", "drive", "web"],
            max_workspaces=999,
            supports_ocr=True,
            supports_multi_context=True,
            supports_collaboration=True,
            supports_custom_vpc=False
        )
    ),
    "enterprise": PlanTier(
        tier_id="enterprise",
        name="AnyContext Enterprise",
        monthly_price_usd=499.0,
        annual_price_usd=4900.0,
        base_seats=999,
        extra_seat_price_usd=0.0,
        ingestion_scope="Containers Docker em VPC Privada + SSO/SAML + Licença Offline + Suporte SLA Dedicado",
        target_audience="Instituições financeiras, governamentais, B2B empresarial",
        capabilities=PlanCapabilities(
            allowed_sources=["local", "drive", "web", "vpc_custom"],
            max_workspaces=999,
            supports_ocr=True,
            supports_multi_context=True,
            supports_collaboration=True,
            supports_custom_vpc=True
        )
    )
}

def get_all_plans() -> List[PlanTier]:
    return list(PLANS_REGISTRY.values())

def get_plan_by_id(tier_id: str) -> PlanTier:
    key = tier_id.lower().strip()
    if key in PLANS_REGISTRY:
        return PLANS_REGISTRY[key]
    if key == "local":
        return PLANS_REGISTRY["starter"]
    return PLANS_REGISTRY["community"]
