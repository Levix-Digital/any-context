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
        ingestion_scope="CLI Local Completo (Pastas Ilimitadas + Web Scraping + OCR + Multi-Workspaces)",
        target_audience="Desenvolvedores, estudantes e usuários individuais",
        capabilities=PlanCapabilities(
            allowed_sources=["local", "drive", "web"],
            max_workspaces=999,
            supports_ocr=True,
            supports_multi_context=True,
            supports_collaboration=False,
            supports_server_mode=False,
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
        ingestion_scope="CLI Local Completo + Suporte Prioritário",
        target_audience="Pesquisadores individuais, advogados solo, consultores",
        capabilities=PlanCapabilities(
            allowed_sources=["local", "drive", "web"],
            max_workspaces=999,
            supports_ocr=True,
            supports_multi_context=True,
            supports_collaboration=False,
            supports_server_mode=False,
            supports_custom_vpc=False
        )
    ),
    "pro": PlanTier(
        tier_id="pro",
        name="AnyContext Pro (Server & Multi-Context)",
        monthly_price_usd=29.0,
        annual_price_usd=288.0, # $24/mo billed annually
        base_seats=1,
        extra_seat_price_usd=0.0,
        ingestion_scope="CLI Completo + REST API Server Monoposto (actx --serve)",
        target_audience="Power users, analistas seniores, freelancers, desenvolvedores de integrações",
        capabilities=PlanCapabilities(
            allowed_sources=["local", "drive", "web"],
            max_workspaces=999,
            supports_ocr=True,
            supports_multi_context=True,
            supports_collaboration=False,
            supports_server_mode=True,
            supports_custom_vpc=False
        )
    ),
    "team": PlanTier(
        tier_id="team",
        name="AnyContext Team (Server & Collaboration)",
        monthly_price_usd=79.0,
        annual_price_usd=780.0, # $65/mo base billed annually
        base_seats=5,
        extra_seat_price_usd=15.0, # +$15/seat/month
        ingestion_scope="REST API Server Multi-Tenant + RBAC + Workspaces Compartilhados",
        target_audience="Escritórios de advocacia, consultorias imigratórias, equipes de engenharia",
        capabilities=PlanCapabilities(
            allowed_sources=["local", "drive", "web"],
            max_workspaces=999,
            supports_ocr=True,
            supports_multi_context=True,
            supports_collaboration=True,
            supports_server_mode=True,
            supports_custom_vpc=False
        )
    ),
    "enterprise": PlanTier(
        tier_id="enterprise",
        name="AnyContext Enterprise (Dedicated VPC & SLA)",
        monthly_price_usd=499.0,
        annual_price_usd=4900.0,
        base_seats=999,
        extra_seat_price_usd=0.0,
        ingestion_scope="REST API Server em VPC Privada + SSO/SAML + Licença Offline no .env + Suporte SLA",
        target_audience="Instituições financeiras, governamentais, B2B empresarial",
        capabilities=PlanCapabilities(
            allowed_sources=["local", "drive", "web", "vpc_custom"],
            max_workspaces=999,
            supports_ocr=True,
            supports_multi_context=True,
            supports_collaboration=True,
            supports_server_mode=True,
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
