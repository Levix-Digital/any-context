"""
Onboarding Service - Centralized First-Time Setup and AI Provider Configuration Engine.
Decoupled domain service providing declarative onboarding states and setup actions
for CLI, OpenTUI, REST API, and MCP Server.
"""

from typing import Dict, Any, Optional, List
from pydantic import BaseModel, Field
from any_context.config.db_store import ConfigDBStore
from any_context.core.interaction.schemas import OptionItemSchema, OptionsGroupSchema


class OnboardingState(BaseModel):
    """Declarative onboarding state consumed by presentation adapters."""
    needs_onboarding: bool = False
    stage: str = "ready"  # "first_time", "missing_key", "ready"
    title: str = "🤖 Welcome to AnyContext AI Setup!"
    description: str = (
        "By default, AnyContext uses OpenAI Cloud models (gpt-4o-mini & "
        "text-embedding-3-small) for fast reasoning and semantic search."
    )
    active_provider: str = "openai"
    active_model: str = "gpt-4o-mini"
    options_group: OptionsGroupSchema = Field(default_factory=lambda: OptionsGroupSchema(type="onboarding", title="Configure AI Provider"))


class OnboardingResult(BaseModel):
    """Result of completing an onboarding action."""
    success: bool = True
    message: str = ""
    error: Optional[str] = None
    state_updates: Dict[str, Any] = Field(default_factory=dict)


class OnboardingService:
    """Centralized Onboarding Service managing first-time setup and API key validation."""

    def __init__(self, store: Optional[ConfigDBStore] = None):
        self.store = store or ConfigDBStore()

    def check_status(self) -> OnboardingState:
        """
        Evaluates whether first-time onboarding or quick provider setup is required.
        """
        onboarding_completed = self.store.get_onboarding_completed()
        settings = self.store.get_app_settings()

        provider = "openai"
        inference_model = "gpt-4o-mini"
        base_url = "https://api.openai.com/v1"
        if settings and settings.models:
            provider = settings.models.model_provider or "openai"
            inference_model = settings.models.inference_model or "gpt-4o-mini"
            base_url = settings.models.local_base_url or "https://api.openai.com/v1"

        # Check if active provider is missing an API key
        is_local = "localhost" in base_url or "127.0.0.1" in base_url
        has_key = False
        if is_local:
            has_key = True
        else:
            api_key = self.store.get_api_key(provider)
            if not api_key:
                from any_context.core.utils import get_api_key
                api_key = get_api_key(provider=provider)
            if api_key and api_key != "lm-studio":
                has_key = True

        stored_api_key = self.store.get_api_key(provider)
        # Check if ANY provider has a stored API key in SQLite
        any_stored_key = False
        try:
            with self.store._get_connection() as conn:
                cur = conn.cursor()
                cur.execute("SELECT api_key FROM api_keys WHERE length(api_key) > 3 LIMIT 1")
                if cur.fetchone():
                    any_stored_key = True
        except Exception:
            pass

        # If user configured any API key in SQLite in a prior version or current session:
        # auto-heal onboarding completion flag so it is permanently remembered.
        if stored_api_key or any_stored_key:
            if not onboarding_completed:
                self.store.set_onboarding_completed(True)
                onboarding_completed = True

        needs_onboarding = False
        stage = "ready"

        if not onboarding_completed:
            needs_onboarding = True
            stage = "first_time"
        elif not has_key and not is_local:
            needs_onboarding = True
            stage = "missing_key"

        from any_context.observability import obs
        obs.info("ONBOARDING:STATUS", f"Evaluated onboarding requirement: {needs_onboarding}", {
            "needs_onboarding": needs_onboarding,
            "stage": stage,
            "provider": provider,
            "has_key": has_key,
            "any_stored_key": any_stored_key,
            "is_local": is_local,
            "onboarding_completed": onboarding_completed
        })

        options = [
            OptionItemSchema(
                id="openai",
                title="⚡ OpenAI Cloud (Enter OpenAI API Key - Recommended)",
                description="Fastest reasoning and embeddings with official gpt-4o-mini & text-embedding-3-small.",
                icon="⚡",
                badge="Recommended",
                is_active=(provider == "openai")
            ),
            OptionItemSchema(
                id="local_offline",
                title="🏠 Local Offline Server (LM Studio / Ollama - 100% Free & Offline)",
                description="Runs completely offline on your own machine without sending data to external clouds.",
                icon="🏠",
                badge="Offline",
                is_active=is_local
            ),
            OptionItemSchema(
                id="custom",
                title="🛠️ Custom Setup (Configure custom models, base URL & keys)",
                description="Connect to Anthropic, Gemini, DeepSeek, Groq, OpenRouter, or a custom API gateway.",
                icon="🛠️",
                badge="Advanced",
                is_active=False
            )
        ]

        title = "🤖 Welcome to AnyContext AI Setup!" if stage == "first_time" else f"🔑 AI Provider Setup ({provider.upper()})"
        description = (
            "By default, AnyContext uses OpenAI Cloud models (gpt-4o-mini & "
            "text-embedding-3-small) for fast reasoning and semantic search."
            if stage == "first_time" else
            f"The active provider '{provider}' requires an API key. Select your configuration method:"
        )

        return OnboardingState(
            needs_onboarding=needs_onboarding,
            stage=stage,
            title=title,
            description=description,
            active_provider=provider,
            active_model=inference_model,
            options_group=OptionsGroupSchema(
                type="onboarding",
                title="Configure AI Provider",
                description=description,
                active_id=provider if not is_local else "local_offline",
                items=options
            )
        )

    def complete_onboarding(
        self,
        choice_id: str,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model_name: Optional[str] = None,
        workspace_name: Optional[str] = None
    ) -> OnboardingResult:
        """
        Executes the setup choice, configures settings in SQLite, and marks onboarding as completed.
        """
        choice = (choice_id or "").strip().lower()
        settings = self.store.get_app_settings()

        if workspace_name and workspace_name.strip():
            clean_ws = workspace_name.strip()
            if not self.store.get_workspace_meta(clean_ws):
                self.store.add_workspace(clean_ws, paths=[])
        else:
            self.store.ensure_default_workspace()

        if choice == "openai":
            clean_key = (api_key or "").strip()
            if not clean_key:
                return OnboardingResult(
                    success=False,
                    error="OpenAI API Key cannot be empty. Please provide a valid key (sk-...)."
                )

            self.store.set_api_key("openai", clean_key)
            if settings and settings.models:
                settings.models.model_provider = "openai"
                settings.models.inference_model = "gpt-4o-mini"
                settings.models.summary_model = "gpt-4o-mini"
                settings.models.embedding_model = "text-embedding-3-small"
                settings.models.local_base_url = "https://api.openai.com/v1"
                self.store.save_app_settings(settings)

            self.store.set_onboarding_completed(True)
            return OnboardingResult(
                success=True,
                message="✅ OpenAI Cloud configured successfully with gpt-4o-mini!",
                state_updates={
                    "model": "gpt-4o-mini",
                    "model_display": "GPT-4o Mini",
                    "provider": "openai",
                    "needs_onboarding": False
                }
            )

        elif choice in ("local_offline", "lm_studio", "ollama"):
            target_url = (base_url or "http://localhost:1234/v1").strip()
            self.store.set_api_key("openai", "lm-studio")
            if settings and settings.models:
                settings.models.model_provider = "openai"
                settings.models.local_base_url = target_url
                settings.models.inference_model = model_name or "local-model"
                settings.models.summary_model = model_name or "local-model"
                settings.models.embedding_model = "text-embedding-3-small"
                self.store.save_app_settings(settings)

            self.store.set_onboarding_completed(True)
            return OnboardingResult(
                success=True,
                message="✅ Local Offline Server configured successfully!",
                state_updates={
                    "model": settings.models.inference_model if settings and settings.models else "local-model",
                    "model_display": "Local Offline Model",
                    "provider": "openai",
                    "needs_onboarding": False
                }
            )

        elif choice == "custom":
            self.store.set_onboarding_completed(True)
            return OnboardingResult(
                success=True,
                message="🛠️ Custom setup selected. Please configure your models and keys in /menu.",
                state_updates={
                    "action": "open_config_modal",
                    "needs_onboarding": False
                }
            )

        else:
            return OnboardingResult(
                success=False,
                error=f"Unknown onboarding choice: '{choice_id}'"
            )
