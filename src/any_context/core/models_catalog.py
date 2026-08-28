import os
from typing import Dict, List, Any, Optional, Tuple
from any_context.core.utils import get_api_key
from any_context.config.app_settings import AppSettings

PROVIDER_CATALOG: Dict[str, Dict[str, Any]] = {
    "openai": {
        "display_name": "OpenAI Cloud",
        "env_var": "OPENAI_API_KEY",
        "models": [
            {"id": "gpt-4o-mini", "name": "GPT-4o Mini (Universal - Fast & Efficient)", "provider": "openai"},
            {"id": "gpt-4o", "name": "GPT-4o (High-Capability Multimodal)", "provider": "openai"},
            {"id": "gpt-4-turbo", "name": "GPT-4 Turbo (High Context 128k)", "provider": "openai"},
            {"id": "gpt-3.5-turbo", "name": "GPT-3.5 Turbo (Legacy Low Cost)", "provider": "openai"}
        ]
    },
    "anthropic": {
        "display_name": "Anthropic Claude",
        "env_var": "ANTHROPIC_API_KEY",
        "models": [
            {"id": "claude-haiku-4-5-20251001", "name": "Claude 4.5 Haiku (Ultra-Fast & Smart)", "provider": "anthropic"},
            {"id": "claude-sonnet-4-5-20250929", "name": "Claude 4.5 Sonnet (State-of-the-Art Analysis & Code)", "provider": "anthropic"},
            {"id": "claude-sonnet-4-6", "name": "Claude 4.6 Sonnet (Next-Gen High Reasoning)", "provider": "anthropic"},
            {"id": "claude-opus-4-5-20251101", "name": "Claude 4.5 Opus (Deep Analysis)", "provider": "anthropic"}
        ]
    },
    "google_genai": {
        "display_name": "Google Gemini",
        "env_var": "GEMINI_API_KEY",
        "models": [
            {"id": "gemini-flash-latest", "name": "Gemini Flash Latest (100% Free Tier - Ultra Fast)", "provider": "google_genai"},
            {"id": "gemini-3.5-flash", "name": "Gemini 3.5 Flash (Next-Gen Fast Multi-Tool)", "provider": "google_genai"},
            {"id": "gemini-3.5-flash-lite", "name": "Gemini 3.5 Flash Lite (Sub-Second Latency)", "provider": "google_genai"},
            {"id": "gemini-pro-latest", "name": "Gemini Pro Latest (Deep Reasoning & 2M Context)", "provider": "google_genai"}
        ]
    },
    "deepseek": {
        "display_name": "DeepSeek Platform",
        "env_var": "DEEPSEEK_API_KEY",
        "models": [
            {"id": "deepseek-chat", "name": "DeepSeek V3 (DeepSeek Chat - $0.14/M)", "provider": "deepseek"}
        ]
    },
    "groq": {
        "display_name": "Groq Cloud",
        "env_var": "GROQ_API_KEY",
        "models": [
            {"id": "llama-3.3-70b-versatile", "name": "Llama 3.3 70B (Groq Free Tier - Ultra Fast)", "provider": "groq"},
            {"id": "llama-3.1-8b-instant", "name": "Llama 3.1 8B (Groq Instant Speed)", "provider": "groq"},
            {"id": "mixtral-8x7b-32768", "name": "Mixtral 8x7B (Groq 32k Context)", "provider": "groq"},
            {"id": "gemma2-9b-it", "name": "Gemma 2 9B (Groq Fast Inference)", "provider": "groq"}
        ]
    },
    "openrouter": {
        "display_name": "OpenRouter",
        "env_var": "OPENROUTER_API_KEY",
        "models": [
            {"id": "openrouter/auto", "name": "OpenRouter Auto (Best Available Model)", "provider": "openrouter"},
            {"id": "meta-llama/llama-3.3-70b-instruct:free", "name": "Llama 3.3 70B (OpenRouter Free Tier)", "provider": "openrouter"},
            {"id": "google/gemini-flash-1.5-8b", "name": "Gemini Flash 8B (Ultra Low Cost)", "provider": "openrouter"},
            {"id": "deepseek/deepseek-chat", "name": "DeepSeek V3 via OpenRouter", "provider": "openrouter"}
        ]
    },
    "xai": {
        "display_name": "xAI Grok",
        "env_var": "XAI_API_KEY",
        "models": [
            {"id": "grok-2-1212", "name": "Grok 2 (xAI Function Calling)", "provider": "xai"},
            {"id": "grok-2", "name": "Grok 2 (xAI Standard)", "provider": "xai"},
            {"id": "grok-beta", "name": "Grok Beta (xAI High Speed)", "provider": "xai"}
        ]
    },
    "mistral": {
        "display_name": "Mistral AI",
        "env_var": "MISTRAL_API_KEY",
        "models": [
            {"id": "mistral-small-latest", "name": "Mistral Small (Fast & Cost-Effective)", "provider": "mistral"},
            {"id": "open-mistral-nemo", "name": "Mistral NeMo 12B (Free Tier Credits)", "provider": "mistral"},
            {"id": "mistral-large-latest", "name": "Mistral Large (High-Tier Reasoning)", "provider": "mistral"}
        ]
    },
    "local": {
        "display_name": "Local Server (LM Studio / Ollama)",
        "env_var": "LOCAL_BASE_URL",
        "models": [
            {"id": "local-model", "name": "Local Loaded Model (100% Free & Offline)", "provider": "local"}
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
    elif m.startswith("mistral-") or m.startswith("open-mistral-"):
        return "mistral"
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


def format_inference_error(error: Exception, model_name: str, provider: str = None) -> Dict[str, str]:
    """
    Translates raw provider exceptions into user-friendly explanations and actionable next steps.
    """
    prov = provider or infer_provider_for_model(model_name)
    err_str = str(error)
    err_lower = err_str.lower()

    title = f"⚠️ Inference Error with Model '{model_name}' ({prov.upper()})"
    cause = ""
    action = ""

    if "model_not_found" in err_lower or "does not exist or you do not have access" in err_lower or "404" in err_str:
        if prov == "openai" and ("o1" in model_name or "o3" in model_name):
            cause = (
                f"Your OpenAI API key does not have permission to access reasoning model '{model_name}'.\n"
                "  OpenAI restricts 'o1' and 'o3' reasoning models to accounts with 'Usage Tier 1+'\n"
                "  (accounts with at least $5 in prepaid credits deposited on OpenAI platform)."
            )
            action = (
                "1. Switch back to a universal model: type '/model gpt-4o-mini' or '/model gpt-4o'\n"
                "  2. Or unlock reasoning models by adding credits at: https://platform.openai.com/settings/organization/billing"
            )
        else:
            cause = f"The model '{model_name}' was not found or is not enabled for your API key by {prov.capitalize()}."
            action = (
                f"1. Switch to a standard model for {prov.capitalize()} (type '/model' to view available list)\n"
                "  2. Check your project/key permissions in the provider developer dashboard."
            )

    elif "invalid_api_key" in err_lower or "authentication" in err_lower or "401" in err_str or "unauthorized" in err_lower:
        cause = f"The API key configured for provider '{prov.capitalize()}' is invalid, expired, or revoked."
        action = "Update your API key via '/config' -> '🔑 Manage Saved API Keys'."

    elif "insufficient_quota" in err_lower or "quota" in err_lower or "credit" in err_lower or "billing" in err_lower:
        cause = f"Your {prov.capitalize()} account has run out of credits or has an unpaid balance (Quota Exceeded)."
        action = (
            f"1. Add credits in your {prov.capitalize()} billing dashboard.\n"
            "  2. Or switch to another configured provider or local offline model with '/model'."
        )

    elif "rate_limit" in err_lower or "429" in err_str or "too many requests" in err_lower:
        cause = f"You exceeded the requests-per-minute (RPM) or tokens-per-minute (TPM) limit on {prov.capitalize()}."
        action = "Wait a few moments before retrying, or upgrade your account Tier on the provider dashboard."

    elif "connection" in err_lower or "timeout" in err_lower or "refused" in err_lower:
        if prov == "local":
            cause = "Could not connect to local offline model server (LM Studio / Ollama)."
            action = "Ensure LM Studio or Ollama is running and the local server is started on http://localhost:1234/v1 (or your configured URL)."
        else:
            cause = f"Network connection error or timeout while reaching {prov.capitalize()} API."
            action = "Check your internet connection and proxy settings."

    elif "reasoning_effort" in err_lower or "function tools" in err_lower or ("tool" in err_lower and "not supported" in err_lower):
        cause = (
            f"The model '{model_name}' does not support Function Calling / Agent Tools on this endpoint ({prov.capitalize()}).\n"
            f"  OpenAI reason: {err_str[:250]}"
        )
        action = (
            "Switch to a standard model that fully supports Agent Function Calling:\n"
            "  • OpenAI   : /model gpt-4o   or   /model gpt-4o-mini\n"
            "  • Claude   : /model claude-3-5-sonnet-20241022\n"
            "  • DeepSeek : /model deepseek-chat"
        )

    elif "decompress" in err_lower or "truncated stream" in err_lower or "error -5" in err_lower or "zlib" in err_lower:
        cause = "Houve uma interrupção ou oscilação na transmissão de streaming comprimido (gzip) da API ou no estado local de checkpoints."
        action = "O AnyContext recuperou e estabilizou sua sessão automaticamente com fallback resiliente. Tente reenviar sua pergunta agora."

    else:
        cause = f"Provider {prov.capitalize()} returned an unexpected error:\n  {err_str[:250]}"
        action = "Try switching to another model with '/model' or verify your configuration in '/config'."

    formatted_box = (
        f"\n\033[91m{'='*75}\033[0m\n"
        f"\033[93m{title}\033[0m\n"
        f"\033[91m{'='*75}\033[0m\n"
        f"\033[96m🔍 What happened:\033[0m\n  {cause}\n\n"
        f"\033[92m👉 What you can do:\033[0m\n  {action}\n"
        f"\033[91m{'='*75}\033[0m\n"
    )

    return {
        "title": title,
        "model": model_name,
        "provider": prov,
        "cause": cause,
        "action": action,
        "formatted_box": formatted_box
    }


def normalize_model_id(model_input: str) -> str:
    """
    Normalizes any model name or display title back to its canonical ID.
    e.g. 'GPT-4o Mini (Universal - Fast & Efficient)' -> 'gpt-4o-mini'
    """
    if not model_input:
        return "gpt-4o-mini"
    clean = model_input.strip()

    for prov_data in PROVIDER_CATALOG.values():
        for m in prov_data.get("models", []):
            m_id = m.get("id", "")
            m_name = m.get("name", "")
            if clean.lower() == m_id.lower():
                return m_id
            if clean.lower() == m_name.lower():
                return m_id
            if clean.lower().startswith(m_id.lower()) and (len(clean) == len(m_id) or clean[len(m_id):].startswith(" ")):
                return m_id
            if m_name and clean.lower().startswith(m_name.lower()):
                return m_id

    return clean
