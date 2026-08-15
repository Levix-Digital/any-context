import os
import sys
from typing import Callable, Iterable, List, Optional
from prompt_toolkit import PromptSession
from prompt_toolkit.styles import Style
from prompt_toolkit.completion import Completer, Completion
from prompt_toolkit.history import InMemoryHistory
from prompt_toolkit.formatted_text import FormattedText

from any_context.core.models_catalog import get_available_models


SLASH_COMMANDS = [
    ("/help", "Open interactive help center & manual"),
    ("/switch", "Switch active workspace context"),
    ("/model", "Switch active AI model on-the-fly"),
    ("/web", "Manage web sources and deep crawler"),
    ("/web add", "Discover and crawl website into context"),
    ("/web list", "List configured web documentation sources"),
    ("/web sync", "Re-sync and update all web sources"),
    ("/sync", "Synchronize local folder files to vector database"),
    ("/keys", "Guide to obtaining API keys for all providers"),
    ("/billing", "View subscription plan tiers and capabilities"),
    ("/update", "Update AnyContext to the latest release"),
    ("/check-update", "Check if a newer version is available"),
    ("/reset-memory", "Clear long-term memory for active workspace"),
    ("/config", "Open full interactive configuration menu"),
    ("/version", "Display AnyContext version and build info"),
    ("/exit", "Save session memory and exit chat")
]


class AnyContextCompleter(Completer):
    """
    Intelligent auto-completer for Slash Commands (/) and One-Shot Model switches (@).
    """
    def __init__(self):
        super().__init__()

    def get_completions(self, document, complete_event) -> Iterable[Completion]:
        text = document.text_before_cursor

        # If user types starting with '/', suggest slash commands
        if text.startswith("/"):
            word = text.lower()
            for cmd, desc in SLASH_COMMANDS:
                if cmd.lower().startswith(word):
                    yield Completion(
                        cmd,
                        start_position=-len(text),
                        display=cmd,
                        display_meta=desc
                    )

        # If user types starting with '@', suggest available AI models
        elif text.startswith("@") and " " not in text:
            word = text[1:].lower()
            try:
                models = get_available_models()
            except Exception:
                models = ["gpt-4o-mini", "gpt-4o", "claude-3-5-sonnet-latest", "deepseek-chat"]
            
            for m in models:
                if m.lower().startswith(word):
                    yield Completion(
                        f"@{m} ",
                        start_position=-len(text),
                        display=f"@{m}",
                        display_meta="One-shot prompt with this model"
                    )


TOOLBAR_STYLE = Style.from_dict({
    "bottom-toolbar": "bg:#181825 #cdd6f4",
    "toolbar_ws": "bg:#313244 #f9e2af bold",
    "toolbar_sep": "bg:#181825 #6c7086",
    "toolbar_model": "bg:#313244 #cba6f7 bold",
    "toolbar_plan": "bg:#313244 #a6e3a1 bold",
    "toolbar_help": "bg:#181825 #89dceb italic",
    "prompt": "#89b4fa bold",
})


def create_anycontext_prompt_session(
    get_workspace_name: Callable[[], str],
    get_model_name: Callable[[], str]
) -> PromptSession:
    """
    Creates an interactive PromptSession equipped with the signature Bottom Toolbar,
    auto-completion, history navigation, and sleek modern styling.
    """
    history = InMemoryHistory()
    completer = AnyContextCompleter()

    def get_bottom_toolbar():
        ws_name = get_workspace_name() or "Global"
        model_name = get_model_name() or "gpt-4o-mini"
        
        tier_display = "Community"
        try:
            from any_context.billing import BillingManager
            status = BillingManager().get_status()
            tier_id = (status.active_tier_id or "community").lower().strip()
            tier_names = {
                "community": "Community",
                "starter": "Starter",
                "pro": "Pro",
                "team": "Team",
                "enterprise": "Enterprise"
            }
            tier_display = tier_names.get(tier_id, status.active_tier_name.split()[0])
        except Exception:
            pass

        return [
            ("class:toolbar_ws", f" 📂 Workspace: {ws_name} "),
            ("class:toolbar_sep", " │ "),
            ("class:toolbar_model", f" 🤖 Model: {model_name} "),
            ("class:toolbar_sep", " │ "),
            ("class:toolbar_plan", f" 🏷️ Plan: {tier_display} "),
            ("class:toolbar_sep", " │ "),
            ("class:toolbar_help", " 💡 /help • @model ")
        ]

    return PromptSession(
        history=history,
        completer=completer,
        style=TOOLBAR_STYLE,
        bottom_toolbar=get_bottom_toolbar
    )
