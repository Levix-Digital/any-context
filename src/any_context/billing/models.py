from pydantic import BaseModel, Field
from typing import List, Optional

class PlanCapabilities(BaseModel):
    allowed_sources: List[str] = Field(default_factory=lambda: ["local"])
    max_workspaces: int = 999
    supports_ocr: bool = False
    supports_multi_context: bool = False
    supports_collaboration: bool = False
    supports_custom_vpc: bool = False

class PlanTier(BaseModel):
    tier_id: str
    name: str
    monthly_price_usd: float
    annual_price_usd: Optional[float] = None
    base_seats: int = 1
    extra_seat_price_usd: float = 0.0
    ingestion_scope: str
    target_audience: str
    capabilities: PlanCapabilities

class SubscriptionStatus(BaseModel):
    active_tier_id: str = "community"
    active_tier_name: str = "AnyContext Community (Free)"
    license_key: Optional[str] = None
    activated_at: Optional[str] = None
    expires_at: Optional[str] = None
    base_seats: int = 1
    extra_seats_purchased: int = 0
    total_seats: int = 1
    extra_seat_price_usd: float = 0.0
    capabilities: PlanCapabilities = Field(default_factory=PlanCapabilities)
