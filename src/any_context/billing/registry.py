from typing import Dict, List
from any_context.billing.models import PlanTier, PlanCapabilities

PLANS_REGISTRY: Dict[str, PlanTier] = {
    "community": PlanTier(
        tier_id="community",
        name="AnyContext Community",
        monthly_price_usd=0.0,
        annual_price_usd=0.0,
        ingestion_scope="Fonte única: Pastas locais personalizadas",
        target_audience="Desenvolvedores, estudantes e entusiastas de IA local",
        capabilities=PlanCapabilities(
            allowed_sources=["local"],
            supports_multi_context=False,
            supports_collaboration=False,
            supports_custom_vpc=False
        )
    ),
    "local": PlanTier(
        tier_id="local",
        name="AnyContext Local",
        monthly_price_usd=19.0,
        annual_price_usd=None,
        ingestion_scope="Fonte única: Pastas locais do sistema operacional e imagens via Daemon",
        target_audience="Pesquisadores individuais, advogados solo",
        capabilities=PlanCapabilities(
            allowed_sources=["local"],
            supports_multi_context=False,
            supports_collaboration=False,
            supports_custom_vpc=False
        )
    ),
    "drive": PlanTier(
        tier_id="drive",
        name="AnyContext Drive",
        monthly_price_usd=39.0,
        annual_price_usd=None,
        ingestion_scope="Fonte única: Google Drive ou OneDrive",
        target_audience="Freelancers, trabalhadores remotos",
        capabilities=PlanCapabilities(
            allowed_sources=["drive"],
            supports_multi_context=False,
            supports_collaboration=False,
            supports_custom_vpc=False
        )
    ),
    "web": PlanTier(
        tier_id="web",
        name="AnyContext Web",
        monthly_price_usd=49.0,
        annual_price_usd=None,
        ingestion_scope="Fonte única: Web Scraping e motor de polling",
        target_audience="Escritores técnicos, analistas",
        capabilities=PlanCapabilities(
            allowed_sources=["web"],
            supports_multi_context=False,
            supports_collaboration=False,
            supports_custom_vpc=False
        )
    ),
    "pro": PlanTier(
        tier_id="pro",
        name="AnyContext Pro",
        monthly_price_usd=89.0,
        annual_price_usd=None,
        ingestion_scope="Workspaces Multi-Context (combina Local + Drive + Web em 1 chat)",
        target_audience="Usuários avançados, consultores seniores",
        capabilities=PlanCapabilities(
            allowed_sources=["local", "drive", "web"],
            supports_multi_context=True,
            supports_collaboration=False,
            supports_custom_vpc=False
        )
    ),
    "team": PlanTier(
        tier_id="team",
        name="AnyContext Team",
        monthly_price_usd=199.0,
        annual_price_usd=None,
        ingestion_scope="Workspaces Multi-Context + Colaboração Multi-Usuário, RBAC e Workspaces Compartilhados",
        target_audience="Escritórios de advocacia boutique, equipes de engenharia",
        capabilities=PlanCapabilities(
            allowed_sources=["local", "drive", "web"],
            supports_multi_context=True,
            supports_collaboration=True,
            supports_custom_vpc=False
        )
    ),
    "enterprise": PlanTier(
        tier_id="enterprise",
        name="AnyContext Enterprise",
        monthly_price_usd=1250.0,
        annual_price_usd=15000.0,
        ingestion_scope="Pilha totalmente containerizada no VPC do cliente + Licença offline + Suporte dedicado",
        target_audience="Instituições financeiras, B2B empresarial",
        capabilities=PlanCapabilities(
            allowed_sources=["local", "drive", "web", "vpc_custom"],
            supports_multi_context=True,
            supports_collaboration=True,
            supports_custom_vpc=True
        )
    )
}

def get_all_plans() -> List[PlanTier]:
    return list(PLANS_REGISTRY.values())

def get_plan_by_id(tier_id: str) -> PlanTier:
    return PLANS_REGISTRY.get(tier_id.lower().strip(), PLANS_REGISTRY["community"])
