import os
from typing import Dict, List, Any, Optional, Tuple
from any_context.core.utils import get_api_key
from any_context.config.app_settings import AppSettings

PROVIDER_CATALOG: Dict[str, Dict[str, Any]] = {
    "openai": {
        "display_name": "OpenAI Cloud",
        "env_var": "OPENAI_API_KEY",
        "models": [
            {"id": "gpt-4o-mini", "name": "GPT-4o Mini (Fast & Cost-Efficient)", "provider": "openai"},
            {"id": "gpt-4o", "name": "GPT-4o (High-Capability Multimodal)", "provider": "openai"},
            {"id": "o1-mini", "name": "o1 Mini (Reasoning & STEM)", "provider": "openai"},
            {"id": "o3-mini", "name": "o3 Mini (High-Speed Reasoning)", "provider": "openai"},
            {"id": "gpt-4-turbo", "name": "GPT-4 Turbo (High Context)", "provider": "openai"}
        ]
    },
    "anthropic": {
        "display_name": "Anthropic Claude",
        "env_var": "ANTHROPIC_API_KEY",
        "models": [
            {"id": "claude-3-5-sonnet-20241022", "name": "Claude 3.5 Sonnet (State-of-the-Art Coding & Analysis)", "provider": "anthropic"},
            {"id": "claude-3-5-haiku-20241022", "name": "Claude 3.5 Haiku (Ultra-Fast & Smart)", "provider": "anthropic"},
            {"id": "claude-3-opus-20240229", "name": "Claude 3 Opus (Deep Analysis)", "provider": "anthropic"}
        ]
    },
    "google_genai": {
        "display_name": "Google Gemini",
        "env_var": "GEMINI_API_KEY",
        "models": [
            {"id": "gemini-1.5-flash", "name": "Gemini 1.5 Flash (Ultra-Fast 1M Context)", "provider": "google_genai"},
            {"id": "gemini-1.5-pro", "name": "Gemini 1.5 Pro (Deep Reasoning 2M Context)", "provider": "google_genai"},
            {"id": "gemini-2.0-flash-exp", "name": "Gemini 2.0 Flash (Next-Gen Experimental)", "provider": "google_genai"}
        ]
    },
    "deepseek": {
        "display_name": "DeepSeek Platform",
        "env_var": "DEEPSEEK_API_KEY",
        "models": [
            {"id": "deepseek-chat", "name": "DeepSeek V3 (DeepSeek Chat)", "provider": "deepseek"},
            {"id": "deepseek-reasoner", "name": "DeepSeek R1 (DeepSeek Reasoner)", "provider": "deepseek"}
        ]
    },
    "groq": {
        "display_name": "Groq Cloud",
        "env_var": "GROQ_API_KEY",
        "models": [
            {"id": "llama-3.3-70b-versatile", "name": "Llama 3.3 70B Versatile (Groq Ultra-Fast)", "provider": "groq"},
            {"id": "mixtral-8x7b-32768", "name": "Mixtral 8x7B (Groq Ultra-Fast)", "provider": "groq"},
            {"id": "gemma2-9b-it", "name": "Gemma 2 9B (Groq Ultra-Fast)", "provider": "groq"}
        ]
    },
    "xai": {
        "display_name": "xAI Grok",
        "env_var": "XAI_API_KEY",
        "models": [
            {"id": "grok-2", "name": "Grok 2 (xAI)", "provider": "xai"},
            {"id": "grok-beta", "name": "Grok Beta (xAI)", "provider": "xai"}
        ]
    },
    "openrouter": {
        "display_name": "OpenRouter",
        "env_var": "OPENROUTER_API_KEY",
        "models": [
            {"id": "openrouter/auto", "name": "OpenRouter Auto Router (Best Model)", "provider": "openrouter"}
        ]
    },
    "local": {
        "display_name": "Local Server (LM Studio / Ollama)",
        "env_var": "LOCAL_BASE_URL",
        "models": [
            {"id": "local-model", "name": "Local Server Loaded Model", "provider": "local"}
        ]
    }
}


def infer_provider_for_model(model_name: str, fallback_provider: str = "openai") -> str:
    """
    Intelligently maps a model ID string to its corresponding provider.
    """
    if not model_name:
        return fallback_provider

    m = model_name.lower().strip()

    if m.startswith("gpt-") or m.startswith("o1-") or m.startswith("o3-") or m.startswith("text-embedding-") or m.startswith("chatgpt-"):
        return "openai"
    elif m.startswith("claude-"):
        return "anthropic"
    elif m.startswith("gemini-"):
        return "google_genai"
    elif m.startswith("deepseek-"):
        return "deepseek"
    elif m.startswith("grok-"):
        return "xai"
    elif m.startswith("openrouter/"):
        return "openrouter"
    elif m.startswith("llama-") or m.startswith("mixtral-") or m.startswith("gemma"):
        if get_api_key("groq"):
            return "groq"
        return "local" if is_local_configured() else fallback_provider
    elif is_local_configured():
        return "local"

    return fallback_provider


def is_local_configured() -> bool:
    """Returns True if local offline base URL (localhost/127.0.0.1) is configured."""
    try:
        settings = AppSettings.load()
        if settings and settings.models and settings.models.local_base_url:
            url = settings.models.local_base_url
            if "localhost" in url or "127.0.0.1" in url:
                return True
    except Exception:
        pass
    return False


def get_configured_providers_with_keys() -> List[str]:
    """
    Returns the list of providers for which a valid API key exists or a local server is configured.
    """
    valid_providers = []

    for provider_key in PROVIDER_CATALOG.keys():
        if provider_key == "local":
            if is_local_configured():
                valid_providers.append(provider_key)
        else:
            key = get_api_key(provider_key)
            if key and key.strip() and key != "lm-studio":
                valid_providers.append(provider_key)

    # Fallback to local if no cloud keys found but local mode active
    if not valid_providers and is_local_configured():
        valid_providers.append("local")

    return valid_providers


def get_available_models() -> List[Dict[str, Any]]:
    """
    Returns only models whose provider has a configured, valid API key (or local offline mode).
    Strict key-aware filtering prevents runtime authentication failures.
    """
    available_providers = get_configured_providers_with_keys()
    models = []

    for prov in available_providers:
        if prov in PROVIDER_CATALOG:
            models.extend(PROVIDER_CATALOG[prov]["models"])

    return models


def validate_model_key_availability(model_name: str) -> Tuple[bool, str, Optional[str]]:
    """
    Validates whether a model can be used given currently configured API keys.
    Returns: (is_available: bool, provider: str, error_message: Optional[str])
    """
    provider = infer_provider_for_model(model_name)

    if provider == "local":
        return True, "local", None

    key = get_api_key(provider)
    if not key or not key.strip() or key == "lm-studio":
        prov_info = PROVIDER_CATALOG.get(provider, {"display_name": provider.capitalize(), "env_var": f"{provider.upper()}_API_KEY"})
        err = (
            f"⚠️ Provider '{prov_info['display_name']}' does not have a configured API key.\n"
            f"👉 Add the key via '/config' -> '🔑 Manage Saved API Keys' or set 'export {prov_info['env_var']}=sk-...'"
        )
        return False, provider, err

    return True, provider, None
