from typing import List
from langchain.chat_models import init_chat_model
from langchain_core.messages import SystemMessage
from any_context.config.app_settings import AppSettings
from any_context.core.utils import get_api_key

class MemoryCompressor:
    """
    LLM Engine for hierarchical summarization (Level-1 Session Summaries & Level-3 Meta-Summaries)
    """

    def __init__(self, settings: AppSettings = None):
        self.settings = settings or AppSettings.load()
        self.api_key = get_api_key()
        self.local_base_url = self.settings.models.local_base_url if self.settings else "http://localhost:1234/v1"
        self.summary_model = self.settings.models.summary_model if self.settings else "google/gemma-4-e2b"
        self.model_provider = self.settings.models.model_provider if self.settings else "openai"

    def _get_model(self):
        return init_chat_model(
            base_url=self.local_base_url,
            model=self.summary_model,
            model_provider=self.model_provider,
            temperature=0.3,
            api_key=self.api_key
        )

    def summarize_chat_block(self, formatted_chat_history: str) -> str:
        """
        Level-1 Compression: Summarizes a block of chat interactions (e.g. 10 interactions)
        """
        model = self._get_model()
        prompt = SystemMessage(content=(
            "You are an AI assistant responsible for generating dense long-term memory summaries.\n"
            "Analyze the conversation block below and produce a single concise paragraph capturing:\n"
            "1. User preferences, profile, or key facts revealed.\n"
            "2. Main goals, technical requirements, or topics discussed.\n"
            "3. Decisions, conclusions, or progress made.\n\n"
            "Chat History Block:\n"
            f"{formatted_chat_history}"
        ))

        response = model.invoke([prompt])
        return response.content.strip()

    def compress_meta_summaries(self, summary_texts: List[str], workspace: str = "global") -> str:
        """
        Level-3 Compression: Combines multiple Level-1 summaries into 1 Consolidated Meta-Summary
        """
        model = self._get_model()
        combined_text = "\n---\n".join(summary_texts)

        prompt = SystemMessage(content=(
            f"You are a Senior Memory Architect consolidating historical session summaries for workspace '{workspace}'.\n"
            "Review the historical session summaries below and synthesize them into a single, cohesive, high-level Meta-Summary.\n"
            "Retain all critical architectural decisions, recurring user preferences, and key milestones, while stripping out redundant or temporary discussion details.\n\n"
            "Historical Summaries:\n"
            f"{combined_text}"
        ))

        response = model.invoke([prompt])
        return response.content.strip()
