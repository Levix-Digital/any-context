"""
AnyContext Textual TUI - Modern Reactive Terminal User Interface.
Provides a Cline/Claude Code style interactive terminal with:
- Persistent scrollable chat history with Markdown & syntax highlighting
- Permanent anchored input bar and live status footer dock
- Smooth token streaming and background tool activity tickers
- Interactive Slash Command Palette and Workspace/Model management
"""

import sys
import os
import asyncio
from typing import Optional, List, Dict, Any

from textual.app import App, ComposeResult
from textual.containers import Vertical, Horizontal, VerticalScroll, Container
from textual.widgets import Header, Footer, Static, Input, Markdown, Button, Label
from textual.reactive import reactive
from textual import work
from textual.binding import Binding

from any_context import __version__
from any_context.config.db_store import ConfigDBStore
from any_context.core.models_catalog import get_available_models, validate_model_key_availability
from any_context.ingestion.local_folder_ingestor import BackgroundSyncManager, check_workspace_changes


# Custom TCSS Styling
TUI_CSS = """
Screen {
    background: #1a1b26;
    color: #c0caf5;
}

#header-bar {
    dock: top;
    height: 3;
    background: #16161e;
    border-bottom: heavy #3b4261;
    padding: 0 1;
    content-align: center middle;
}

#header-title {
    text-style: bold;
    color: #7aa2f7;
}

#header-subtitle {
    color: #565f89;
    margin-left: 2;
}

#chat-scroll {
    height: 1fr;
    scrollbar-gutter: stable;
    padding: 1 2;
}

.welcome-box {
    border: round #3b4261;
    background: #1f2335;
    padding: 1 2;
    margin: 1 0;
}

.welcome-title {
    text-style: bold;
    color: #e0af68;
}

.user-message-card {
    background: #24283b;
    border-left: heavy #7aa2f7;
    padding: 1 2;
    margin: 1 0;
}

.user-header {
    text-style: bold;
    color: #7dcfff;
    margin-bottom: 0;
}

.ai-message-card {
    background: #1f2335;
    border-left: heavy #bb9af7;
    padding: 1 2;
    margin: 1 0;
}

.ai-header {
    text-style: bold;
    color: #e0af68;
    margin-bottom: 0;
}

.tool-ticker {
    color: #ff9e64;
    text-style: italic;
    margin: 0 0 1 0;
}

.system-notice-card {
    background: #16161e;
    border-left: heavy #9ece6a;
    padding: 1 2;
    margin: 1 0;
}

#input-container {
    dock: bottom;
    height: auto;
    background: #16161e;
    border-top: heavy #3b4261;
    padding: 0 1;
}

#prompt-input {
    background: #24283b;
    border: round #3b4261;
    color: #c0caf5;
    margin: 1 0 0 0;
}

#prompt-input:focus {
    border: round #7aa2f7;
}

#status-dock {
    height: 1;
    background: #16161e;
    padding: 0 1;
    color: #a9b1d6;
    margin-bottom: 0;
}
"""

class StatusFooterDock(Static):
    """Permanent anchored status bar at the bottom of the screen."""

    def __init__(self, workspace: str, model: str, mode: str, web_search: bool, **kwargs):
        super().__init__(**kwargs)
        self.workspace = workspace
        self.model = model
        self.mode = mode
        self.web_search = web_search
        self.sync_info = ""

    def update_values(
        self,
        workspace: Optional[str] = None,
        model: Optional[str] = None,
        mode: Optional[str] = None,
        web_search: Optional[bool] = None,
        sync_info: Optional[str] = None
    ):
        if workspace is not None:
            self.workspace = workspace
        if model is not None:
            self.model = model
        if mode is not None:
            self.mode = mode
        if web_search is not None:
            self.web_search = web_search
        if sync_info is not None:
            self.sync_info = sync_info
        self.refresh()

    def render(self) -> str:
        search_badge = "[bold #73daca]🌐 Search: ON[/]" if self.web_search else "[#565f89]🌐 Search: OFF[/]"
        mode_cap = (self.mode or "strict").capitalize()
        sync_badge = f" [bold #ff9e64]│ ⚡ {self.sync_info}[/]" if self.sync_info else ""

        return (
            f" [bold #e0af68]📂 {self.workspace}[/] │ "
            f"[bold #bb9af7]🤖 {self.model}[/] │ "
            f"[bold #7aa2f7]🛡️ {mode_cap}[/] │ "
            f"{search_badge}{sync_badge} │ "
            f"[#565f89]💡 /help │ 🚪 /exit[/]"
        )


class AnyContextApp(App):
    """Main Textual TUI Application for AnyContext."""

    CSS = TUI_CSS
    TITLE = "AnyContext"
    BINDINGS = [
        Binding("ctrl+c", "quit", "Quit", show=False),
        Binding("escape", "cancel_active", "Cancel", show=False),
        Binding("f1", "show_help", "Help", show=True),
        Binding("f2", "switch_workspace", "Switch WS", show=True),
        Binding("f3", "switch_model", "Model", show=True),
    ]

    active_workspace: reactive[str] = reactive("Default")
    current_model: reactive[str] = reactive("gpt-4o-mini")
    grounding_mode: reactive[str] = reactive("strict")
    web_search_enabled: reactive[bool] = reactive(False)

    def __init__(self, initial_workspace: str = "Default", **kwargs):
        super().__init__(**kwargs)
        self.active_workspace = initial_workspace or "Default"
        self.store = ConfigDBStore()
        self.agent_instance = None
        self._is_generating = False
        self._active_stream_widget: Optional[Markdown] = None
        self._active_ticker_widget: Optional[Static] = None
        self._input_history: List[str] = []
        self._history_index: int = -1

    def compose(self) -> ComposeResult:
        # 1. Header Bar
        yield Container(
            Label(f"🤖 AnyContext (actx) v{__version__}", id="header-title"),
            Label("Universal Multi-Context RAG Assistant & Engine", id="header-subtitle"),
            id="header-bar"
        )

        # 2. Scrollable Chat History
        yield VerticalScroll(
            Container(
                Label("🚀 Welcome to AnyContext TUI!", classes="welcome-title"),
                Label(
                    "• Type your question below to chat with your documents and web knowledge.\n"
                    "• Slash commands: /switch, /model, /mode, /sync, /help, /clear, /exit.\n"
                    "• Mouse wheel & keyboard navigation enabled.",
                ),
                classes="welcome-box",
                id="welcome-card"
            ),
            id="chat-scroll"
        )

        # 3. Input & Status Dock Container
        yield Vertical(
            Input(
                placeholder="Ask a question or type / for commands...",
                id="prompt-input"
            ),
            StatusFooterDock(
                workspace=self.active_workspace,
                model=self.current_model,
                mode=self.grounding_mode,
                web_search=self.web_search_enabled,
                id="status-dock"
            ),
            id="input-container"
        )

    def on_mount(self) -> None:
        self._load_workspace_settings()
        self.query_one("#prompt-input", Input).focus()
        self.set_interval(1.5, self._poll_background_sync)

    def _load_workspace_settings(self) -> None:
        try:
            settings = self.store.get_app_settings()
            if settings and settings.models and settings.models.inference_model:
                self.current_model = settings.models.inference_model
            self.grounding_mode = self.store.get_grounding_mode(workspace_name=self.active_workspace) or "strict"
            self.web_search_enabled = self.store.get_web_search_status(workspace_name=self.active_workspace) or False
        except Exception:
            pass
        self._update_footer()

    def _update_footer(self, sync_info: str = "") -> None:
        try:
            dock = self.query_one("#status-dock", StatusFooterDock)
            dock.update_values(
                workspace=self.active_workspace,
                model=self.current_model,
                mode=self.grounding_mode,
                web_search=self.web_search_enabled,
                sync_info=sync_info
            )
        except Exception:
            pass

    def _poll_background_sync(self) -> None:
        try:
            bg_mgr = BackgroundSyncManager()
            if bg_mgr.is_syncing(self.active_workspace):
                prog = bg_mgr.format_progress_bar(self.active_workspace, width=8)
                self._update_footer(sync_info=f"Syncing {prog}")
            else:
                self._update_footer(sync_info="")
        except Exception:
            pass

    def action_cancel_active(self) -> None:
        if self._is_generating:
            self._is_generating = False
            self.post_system_notice("⏹️ Generation cancelled by user.")

    def action_show_help(self) -> None:
        self.handle_slash_command("/help")

    def action_switch_workspace(self) -> None:
        self.handle_slash_command("/switch")

    def action_switch_model(self) -> None:
        self.handle_slash_command("/model")

    def post_system_notice(self, message: str) -> None:
        chat_scroll = self.query_one("#chat-scroll", VerticalScroll)
        card = Container(
            Label(f"💡 {message}"),
            classes="system-notice-card"
        )
        chat_scroll.mount(card)
        card.scroll_visible()

    async def on_input_submitted(self, event: Input.Submitted) -> None:
        text = event.value.strip()
        if not text:
            return

        input_widget = self.query_one("#prompt-input", Input)
        input_widget.value = ""

        # History tracking
        self._input_history.append(text)
        self._history_index = len(self._input_history)

        # Intercept slash commands
        if text.startswith("/"):
            self.handle_slash_command(text)
            return

        if self._is_generating:
            self.post_system_notice("⚠️ Please wait for current response to finish or press Esc to cancel.")
            return

        # 1. Mount User Message
        chat_scroll = self.query_one("#chat-scroll", VerticalScroll)
        user_card = Container(
            Label("👤 You", classes="user-header"),
            Label(text),
            classes="user-message-card"
        )
        chat_scroll.mount(user_card)
        user_card.scroll_visible()

        # 2. Mount AI Response Card & Start Streaming
        self._start_ai_generation(text)


    def handle_slash_command(self, cmd_text: str) -> None:
        parts = cmd_text.split()
        cmd = parts[0].lower()

        if cmd in ["/exit", "/quit", "/q"]:
            self.post_system_notice("👋 Saving session memory and exiting...")
            self.exit()
            return

        if cmd in ["/clear", "/cls"]:
            chat_scroll = self.query_one("#chat-scroll", VerticalScroll)
            for child in list(chat_scroll.children):
                child.remove()
            self.post_system_notice("🧹 Chat history cleared.")
            return

        if cmd in ["/version", "/v"]:
            self.post_system_notice(f"🤖 AnyContext (actx) v{__version__} - Levix Digital")
            return

        if cmd in ["/help", "/menu"]:
            help_content = (
                "**📚 Available AnyContext Commands:**\n\n"
                "- `/switch <workspace>` : Switch active workspace\n"
                "- `/model <name>` : Switch AI inference model\n"
                "- `/mode <strict|hybrid|proactive>` : Change Grounding Mode\n"
                "- `/web-search [on|off]` : Toggle real-time Web Search\n"
                "- `/sync [--force|--status]` : Synchronize local folders and web sources\n"
                "- `/sources` : List indexed documents in current workspace\n"
                "- `/clear` : Clear chat history view\n"
                "- `/exit` : Save and exit"
            )
            self.post_system_notice(help_content)
            return

        if cmd == "/switch":
            if len(parts) > 1:
                target_ws = parts[1].strip()
                self.active_workspace = target_ws
                self.store.get_or_create_workspace(target_ws)
                self.agent_instance = None
                self._load_workspace_settings()
                self.post_system_notice(f"Switched to workspace '{target_ws}'.")
            else:
                settings = self.store.get_app_settings()
                known = [w.name for w in settings.workspaces] if settings else ["Default"]
                self.post_system_notice(f"Configured workspaces: {', '.join(known)}. Usage: `/switch <name>`")
            return

        if cmd == "/model":
            if len(parts) > 1:
                new_model = parts[1].strip()
                self.current_model = new_model
                self.store.update_model_settings(inference_model=new_model)
                self.agent_instance = None
                self._update_footer()
                self.post_system_notice(f"Switched model to '{new_model}'.")
            else:
                models = get_available_models()
                self.post_system_notice(f"Available models: {', '.join(models)}. Usage: `/model <name>`")
            return

        if cmd == "/mode":
            if len(parts) > 1 and parts[1].lower() in ["strict", "hybrid", "proactive"]:
                new_mode = parts[1].lower()
                self.grounding_mode = new_mode
                self.store.set_grounding_mode(workspace_name=self.active_workspace, mode=new_mode)
                self.agent_instance = None
                self._update_footer()
                self.post_system_notice(f"Grounding mode updated to '{new_mode.capitalize()}'.")
            else:
                self.post_system_notice("Usage: `/mode <strict|hybrid|proactive>`")
            return

        if cmd in ["/web-search", "/search", "/web"]:
            if len(parts) > 1:
                val = parts[1].lower() in ["on", "true", "1", "enable"]
            else:
                val = not self.web_search_enabled
            self.web_search_enabled = val
            self.store.set_web_search_status(workspace_name=self.active_workspace, enabled=val)
            self.agent_instance = None
            self._update_footer()
            status_str = "ENABLED (ON)" if val else "DISABLED (OFF)"
            self.post_system_notice(f"Real-time Web Search is now {status_str}.")
            return

        if cmd in ["/sync", "/resync"]:
            bg_mgr = BackgroundSyncManager()
            bg_mgr.start_background_sync(workspace_name=self.active_workspace, verbose=False)
            self.post_system_notice(f"⚡ Background synchronization started for workspace '{self.active_workspace}'.")
            return

        self.post_system_notice(f"Unknown command '{cmd}'. Type `/help` for available commands.")


    def _start_ai_generation(self, user_prompt: str) -> None:
        self._is_generating = True
        chat_scroll = self.query_one("#chat-scroll", VerticalScroll)

        ticker_widget = Label("", classes="tool-ticker")
        markdown_widget = Markdown("Thinking...", id="streaming-ai-markdown")

        ai_card = Container(
            Label(f"🤖 AI [{self.current_model}]", classes="ai-header"),
            ticker_widget,
            markdown_widget,
            classes="ai-message-card"
        )

        chat_scroll.mount(ai_card)
        ai_card.scroll_visible()

        self._active_stream_widget = markdown_widget
        self._active_ticker_widget = ticker_widget

        # Run generation on background worker
        self._stream_agent_response(user_prompt)

    @work(thread=True)
    def _stream_agent_response(self, prompt_text: str) -> None:
        from any_context.core.agent import create_anycontext_agent

        try:
            thread_id = f"tui_session_{self.active_workspace}"
            config = {
                "configurable": {
                    "thread_id": thread_id,
                    "active_workspace": self.active_workspace,
                    "grounding_mode": self.grounding_mode,
                    "web_search_enabled": self.web_search_enabled
                }
            }

            if self.agent_instance is None:
                self.agent_instance = create_anycontext_agent(
                    model_name=self.current_model,
                    workspace_name=self.active_workspace,
                    grounding_mode=self.grounding_mode,
                    web_search_enabled=self.web_search_enabled
                )

            accumulated_response = ""
            for token, metadata in self.agent_instance.stream(
                {"messages": [prompt_text]},
                stream_mode="messages",
                config=config
            ):
                if not self._is_generating:
                    break

                if hasattr(token, "type") and token.type in ["ai", "AIMessageChunk", "AIMessage"]:
                    content_chunk = ""
                    if isinstance(token.content, str) and token.content:
                        content_chunk = token.content
                    elif isinstance(token.content, list):
                        parts = [p if isinstance(p, str) else p.get("text", "") for p in token.content if isinstance(p, (str, dict))]
                        content_chunk = "".join(parts)

                    if content_chunk:
                        accumulated_response += content_chunk
                        self.call_from_thread(self._update_ai_token_ui, accumulated_response)

                elif hasattr(token, "type") and token.type in ["tool", "ToolMessage", "ToolMessageChunk"]:
                    t_name = str(getattr(token, "name", "") or "")
                    if "web" in t_name.lower():
                        msg = "🌐 Searching the web in real-time..."
                    else:
                        msg = "📚 Reading indexed context documents..."
                    self.call_from_thread(self._update_ai_ticker_ui, msg)

            self.call_from_thread(self._finish_ai_generation, accumulated_response)

        except Exception as e:
            self.call_from_thread(self._handle_ai_error, str(e))

    def _update_ai_token_ui(self, full_text: str) -> None:
        if self._active_ticker_widget:
            self._active_ticker_widget.update("")
        if self._active_stream_widget:
            self._active_stream_widget.update(full_text)
            self._active_stream_widget.scroll_visible()

    def _update_ai_ticker_ui(self, ticker_text: str) -> None:
        if self._active_ticker_widget:
            self._active_ticker_widget.update(f"⚡ {ticker_text}")

    def _finish_ai_generation(self, final_text: str) -> None:
        self._is_generating = False
        if self._active_ticker_widget:
            self._active_ticker_widget.update("")
        if self._active_stream_widget and final_text:
            self._active_stream_widget.update(final_text)
            self._active_stream_widget.scroll_visible()

    def _handle_ai_error(self, err_msg: str) -> None:
        self._is_generating = False
        if self._active_stream_widget:
            self._active_stream_widget.update(f"❌ **Error generating response**: {err_msg}")
        self.agent_instance = None


def run_tui(workspace_name: str = "Default") -> None:
    """Launches the Textual TUI Application."""
    app = AnyContextApp(initial_workspace=workspace_name)
    app.run()


if __name__ == "__main__":
    run_tui()

