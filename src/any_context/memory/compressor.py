from typing import List
from langchain.chat_models import init_chat_model
from langchain_core.messages import SystemMessage
from any_context.config.app_settings import AppSettings
from any_context.core.utils import get_api_key

class MemoryCompressor:
    """
    LLM Engine for rich, high-fidelity hierarchical memory extraction and consolidation.
    Extracts structured, multi-dimensional long-term memory blocks capturing exact parameters,
    architectural decisions, touched files, user directives, and unresolved tasks.
    """

    def __init__(self, settings: AppSettings = None):
        self.settings = settings or AppSettings.load()

    def _get_model(self):
        from any_context.core.models_catalog import infer_provider_for_model
        
        default_provider = self.settings.models.model_provider if self.settings else "openai"
        model_name = self.settings.models.summary_model if (self.settings and self.settings.models.summary_model) else "gpt-4o-mini"
        provider = infer_provider_for_model(model_name, fallback_provider=default_provider)
        
        api_key = get_api_key(provider=provider)
        if not api_key:
            api_key = "sk-placeholder" if provider == "openai" else "lm-studio"
            
        init_kwargs = {
            "model": model_name,
            "model_provider": provider,
            "api_key": api_key,
            "temperature": 0.2
        }
        
        base_url = self.settings.models.local_base_url if self.settings else None
        if provider in ["local", "lm-studio", "ollama"] or (base_url and ("localhost" in base_url or "127.0.0.1" in base_url)):
            init_kwargs["base_url"] = base_url or "http://localhost:1234/v1"
        elif provider == "deepseek":
            init_kwargs["base_url"] = "https://api.deepseek.com/v1"
            init_kwargs["model_provider"] = "openai"
        elif provider == "groq":
            init_kwargs["base_url"] = "https://api.groq.com/openai/v1"
            init_kwargs["model_provider"] = "openai"
        elif provider == "xai":
            init_kwargs["base_url"] = "https://api.x.ai/v1"
            init_kwargs["model_provider"] = "openai"
        elif provider == "openrouter":
            init_kwargs["base_url"] = "https://openrouter.ai/api/v1"
            init_kwargs["model_provider"] = "openai"
        elif provider == "mistral":
            init_kwargs["base_url"] = "https://api.mistral.ai/v1"
            init_kwargs["model_provider"] = "openai"
        elif provider in ["google_genai", "gemini"]:
            init_kwargs["model_provider"] = "google_genai"

        return init_chat_model(**init_kwargs)

    def summarize_chat_block(self, formatted_chat_history: str) -> str:
        """
        Level-1 Extraction: Generates a high-fidelity, structured long-term memory block
        capturing technical specifics, parameters, code files, user directives, and decisions.
        """
        model = self._get_model()
        prompt = SystemMessage(content=(
            "You are an Expert Knowledge Extraction & Long-Term Memory Engine for an AI Assistant.\n"
            "Analyze the conversation history below and extract a comprehensive, highly-structured Long-Term Memory Block.\n"
            "DO NOT over-compress or generate a generic, vague summary. Retain exact parameters, numbers, code symbols, filenames, and architectural decisions.\n\n"
            "Produce your output using the following structured sections (in the same language as the conversation):\n"
            "### 👤 User Directives & Preferences\n"
            "- Explicit instructions, preferences, constraints, workflow rules, or corrections specified by the user.\n\n"
            "### 🏗️ Technical Architecture & Key Decisions\n"
            "- Exact parameters, schemas, configuration values, mathematical/algorithmic choices, and architectural rationale.\n\n"
            "### 📁 Files, Code Symbols & Databases\n"
            "- Specific files created/modified, classes, functions, SQLite tables, columns, API endpoints, or version tags.\n\n"
            "### 📌 Critical Context & Problem Resolution\n"
            "- Root-cause diagnosis of issues discussed, errors resolved, and why specific approaches were taken.\n\n"
            "### 🚀 Pending Tasks & Next Steps\n"
            "- Outstanding user requests, future milestones, unverified tests, or next steps agreed upon.\n\n"
            "Chat History Block:\n"
            f"{formatted_chat_history}"
        ))

        response = model.invoke([prompt])
        return response.content.strip()

    def compress_meta_summaries(self, summary_texts: List[str], workspace: str = "global") -> str:
        """
        Level-3 Compression: Synthesizes multiple Level-1 structured summaries into a cohesive Meta-Summary
        while preserving all architectural decisions, constants, and user preferences.
        """
        model = self._get_model()
        combined_text = "\n---\n".join(summary_texts)

        prompt = SystemMessage(content=(
            f"You are a Senior Memory Architect consolidating historical session memories for workspace '{workspace}'.\n"
            "Synthesize the historical session summaries below into a consolidated, authoritative Meta-Summary.\n"
            "Retain all critical architectural decisions, recurring user preferences, exact configuration parameters, and key milestones.\n\n"
            "Structure the Meta-Summary with:\n"
            "### 👤 User Profile & Established Preferences\n"
            "### 🏗️ Consolidated Architecture & Key Milestones\n"
            "### 📁 Project Schemas, Components & Critical Constants\n"
            "### 🚀 Current Project State & Ongoing Roadmap\n\n"
            "Historical Summaries:\n"
            f"{combined_text}"
        ))

        response = model.invoke([prompt])
        return response.content.strip()
