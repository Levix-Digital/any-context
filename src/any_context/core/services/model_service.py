"""
ModelService - Core Application Service for AI model selection, catalog inspection, and API key management.
Pure domain logic: decoupled from terminal UI, CLI formatters, HTTP, and RPC transports.
"""

from typing import Dict, Any, List, Optional
from any_context.config.db_store import ConfigDBStore
from any_context.config.app_settings import AppSettings
from any_context.core.models_catalog import (
    get_available_models,
    validate_model_key_availability,
    PROVIDER_CATALOG,
    infer_provider_for_model,
    normalize_model_id
)


class ModelService:
    """Service managing inference model selection, catalog metadata, and provider API keys."""

    def __init__(self, store: Optional[ConfigDBStore] = None):
        self.store = store or ConfigDBStore()

    def get_current_model(self, workspace_name: Optional[str] = None) -> str:
        """Returns the currently active inference model for a workspace, strictly defaulting to 'gpt-4o-mini'."""
        if workspace_name and workspace_name.strip():
            try:
                ws_model = self.store.get_workspace_model(workspace_name.strip())
                if ws_model:
                    return normalize_model_id(ws_model)
            except Exception:
                pass
        try:
            settings = self.store.get_app_settings()
            if settings and settings.models and settings.models.inference_model:
                return normalize_model_id(settings.models.inference_model)
        except Exception:
            pass
        return "gpt-4o-mini"

    def set_model(self, model_name: str, workspace_name: Optional[str] = None) -> Dict[str, Any]:
        """Validates and switches the active AI model globally and for the workspace."""
        clean_model = normalize_model_id(model_name.strip())
        if not clean_model:
            raise ValueError("Model name cannot be empty.")

        settings = self.store.get_app_settings() or AppSettings()
        if not settings.models:
            from any_context.config.app_settings import ModelSettings
            settings.models = ModelSettings()

        settings.models.inference_model = clean_model
        self.store.update_model_settings(settings.models)

        if workspace_name and workspace_name.strip():
            self.store.set_workspace_model(workspace_name.strip(), clean_model)

        # Check API key status for feedback
        has_key, provider, _ = validate_model_key_availability(clean_model)

        return {
            "model": clean_model,
            "provider": provider,
            "has_key": has_key,
            "message": f"Active model set to '{clean_model}' (Provider: {provider}, Key Configured: {has_key})."
        }


    def list_models(self) -> List[Dict[str, Any]]:
        """Returns the full catalog of models with provider and availability status."""
        catalog = []
        for prov_key, prov_data in PROVIDER_CATALOG.items():
            for m in prov_data.get("models", []):
                model_id = m.get("id", "")
                has_key, provider, _ = validate_model_key_availability(model_id)
                catalog.append({
                    "id": model_id,
                    "name": m.get("name", model_id),
                    "provider": prov_data.get("display_name", prov_key.capitalize()),
                    "is_available": has_key,
                })
        return catalog

    def set_api_key(self, provider: str, api_key: str) -> Dict[str, Any]:
        """Sets or updates an API key for an AI provider."""
        clean_provider = provider.strip().lower()
        clean_key = api_key.strip()

        if not clean_provider or not clean_key:
            raise ValueError("Provider name and API key cannot be empty.")

        self.store.set_api_key(clean_provider, clean_key)
        self.store.set_onboarding_completed(True)
        return {
            "provider": clean_provider,
            "configured": True,
            "message": f"API key for provider '{clean_provider}' configured successfully."
        }
