from pydantic import BaseModel, Field
from typing import List, Optional

class PlanCapabilities(BaseModel):
    allowed_sources: List[str] = Field(default_factory=lambda: ["local"])
    supports_multi_context: bool = False
    supports_collaboration: bool = False
    supports_custom_vpc: bool = False

class PlanTier(BaseModel):
    tier_id: str
    name: str
    monthly_price_usd: float
    annual_price_usd: Optional[float] = None
    ingestion_scope: str
    target_audience: str
    capabilities: PlanCapabilities

class SubscriptionStatus(BaseModel):
    active_tier_id: str = "community"
    active_tier_name: str = "AnyContext Community (Free)"
    license_key: Optional[str] = None
    activated_at: Optional[str] = None
    expires_at: Optional[str] = None
    capabilities: PlanCapabilities = Field(default_factory=PlanCapabilities)
