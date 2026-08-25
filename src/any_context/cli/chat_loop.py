import os
import sys
import uuid
from typing import Optional, List, Dict, Any
import questionary
from any_context.cli.workspace_selector import show_workspace_menu, get_active_workspace
from any_context.cli.config_menu import show_config_menu
from any_context.cli.banner import print_banner
from any_context.cli.updater import print_startup_update_notice, check_for_updates, run_self_update
from any_context.cli.spinner import Spinner
from any_context.help import handle_command_help_interception
from any_context import __version__
from any_context.config.db_store import ConfigDBStore


from any_context.cli.history import safe_prompt_input


def safe_stdout_write(msg: str):
    try:
        sys.stdout.write(msg)
        sys.stdout.flush()
    except (UnicodeEncodeError, Exception):
        try:
            clean_msg = msg.encode("ascii", errors="ignore").decode("ascii")
            sys.stdout.write(clean_msg)
            sys.stdout.flush()
        except Exception:
            pass


def format_session_error(error: Exception) -> str:
    """
    Translates raw runtime exceptions into a user-friendly, reassuring message,
    while discretely displaying technical details for troubleshooting.
    """
    err_type = type(error).__name__
    err_msg = str(error).strip()

    if isinstance(error, FileNotFoundError) or "no such file" in err_msg.lower():
        friendly_desc = "O arquivo ou diretório solicitado não foi encontrado."
        tip = "Verifique o caminho informado e tente novamente."
    elif isinstance(error, PermissionError) or "permission denied" in err_msg.lower():
        friendly_desc = "Permissão de acesso negada pelo sistema operacional."
        tip = "Verifique as permissões de leitura/escrita ou execute o terminal como administrador."
    elif "connection" in err_msg.lower() or "timeout" in err_msg.lower():
        friendly_desc = "Houve uma instabilidade temporária na conexão de rede."
        tip = "Verifique sua conexão com a internet e tente novamente."
    elif "decompress" in err_msg.lower() or "truncated stream" in err_msg.lower() or "error -5" in err_msg.lower() or "zlib" in err_msg.lower():
        friendly_desc = "Houve uma oscilação na resposta comprimida da rede ou no stream da API."
        tip = "O AnyContext recuperou e estabilizou sua sessão automaticamente. Basta reenviar sua mensagem."
    elif isinstance(error, (UnboundLocalError, NameError, AttributeError, TypeError, ValueError)):
        friendly_desc = "Ocorreu uma falha interna temporária ao processar esta ação."
        tip = "Sua sessão e seus dados continuam intactos. Tente executar o comando novamente ou digite '/help'."
    else:
        friendly_desc = "Ocorreu um erro inesperado ao executar esta ação."
        tip = "Sua sessão permanece ativa. Tente executar o comando novamente ou digite '/help'."

    return (
        f"\n\033[93m⚠️ Ops! Não foi possível concluir a ação:\033[0m\n"
        f"  • {friendly_desc}\n"
        f"  • \033[96mDica:\033[0m {tip}\n"
        f"  \033[90m[Nota técnica: {err_type} - {err_msg}]\033[0m\n\n"
    )


def collect_multiline_paste(active_workspace: Optional[str] = None, initial_text: str = "") -> Optional[str]:
    """
    Explicit multiline / paste capture mode.
    Allows users to freely paste or type large blocks of text with line breaks.
    Terminates when the user types '\"\"\"', 'EOF' (Ctrl+D / Ctrl+Z), or '/send'.
    Aborts cleanly on '/cancel'.
    """
    safe_stdout_write("\n┌" + "─" * 68 + "┐\n")
    safe_stdout_write("│ 📋 Multi-line Paste Mode Active                                    │\n")
    safe_stdout_write("│   • Paste (Ctrl+V) or type your text with line breaks below.       │\n")
    safe_stdout_write("│   • Type '\"\"\"' or '/send' on a new line to finish & send.          │\n")
    safe_stdout_write("│   • Type '/cancel' or press Ctrl+C to abort.                       │\n")
    safe_stdout_write("└" + "─" * 68 + "┘\n\n")

    lines = []
    if initial_text:
        lines.append(initial_text)

    while True:
        try:
            line = safe_prompt_input("\033[90m... \033[0m", workspace_name=active_workspace)
            if line is None:
                return None

            stripped = line.strip()
            if stripped.lower() == "/cancel":
                safe_stdout_write("\n↩️ Multi-line paste cancelled.\n\n")
                return None
            if stripped.lower() in ["/send", '"""', "'''"]:
                break
            if stripped.lower().endswith("/send"):
                content = line[:line.lower().rfind("/send")].rstrip()
                if content:
                    lines.append(content)
                break
            if stripped.endswith('"""') or stripped.endswith("'''"):
                delim = '"""' if stripped.endswith('"""') else "'''"
                content = line[:line.rfind(delim)].rstrip()
                if content:
                    lines.append(content)
                break

            lines.append(line)
        except (KeyboardInterrupt, EOFError):
            safe_stdout_write("\n↩️ Multi-line paste cancelled.\n\n")
            return None

    full_text = "\n".join(lines).strip()
    if not full_text:
        safe_stdout_write("⚠️ Empty text entered. Returning to normal chat.\n\n")
        return None
    return full_text


def show_slash_commands_palette(active_workspace: Optional[str] = None) -> Optional[str]:
    """
    Interactive Slash Command Palette triggered by typing '/' or '/menu'.
    Allows the user to easily pick and execute any command or view documentation.
    """
    choice = questionary.select(
        "⚡ AnyContext Slash Commands Palette (Choose a command to run):",
        choices=[
            "🔄 /sync         - Unified sync across all sources (folders, web, drives)",
            "📁 /folder       - Add, list, remove, or sync local folders",
            "🌐 /web          - Ingest, crawl, list, or sync web portals",
            "☁️ /drive        - Connect, list, remove, or sync cloud drives",
            "🔍 /inspect      - View vector DB chunks, LanceDB records & snippets",
            "📂 /switch       - Switch or create active workspace",
            "📁 /sources      - View all sources (folders, web portals, drives)",
            "✏️ /rename       - Rename a workspace and migrate vector records",
            "🔄 /transfer     - Instant zero-cost transfer of folders/websites",
            "📋 /paste        - Enter multi-line paste mode for long texts",
            "🤖 /model        - Change active AI inference model on-the-fly",
            "🎛️ /mode         - Select AI grounding mode (Strict, Hybrid, Proactive)",
            "🌐 /web-search   - Toggle real-time live web search for active workspace",
            "🔑 /key          - Manage and configure API Keys (Tavily, OpenAI, Gemini, etc.)",
            "⚙️ /config       - Open interactive configuration & settings menu",
            "🔍 /density      - Configure RAG retrieval density presets",
            "🧠 /reset-memory - Reset/purge long-term session memory",
            "💳 /billing      - View subscription tiers and pricing matrix",
            "🔐 /auth         - Manage user accounts and security tokens",
            "🧹 /clear        - Clear terminal screen",
            "ℹ️ /version      - Show current AnyContext version",
            "📖 /help         - Open complete interactive Help & Documentation",
            "👋 /exit         - Save session memory and exit AnyContext",
            "🔙 [Cancel]"
        ]
    ).ask()

    if not choice or choice.startswith("🔙"):
        return None

    cmd_token = choice.split()[1]
    return cmd_token


def parse_command_args(command_line: str) -> List[str]:
    """Safely tokenizes CLI command lines while respecting quotes and Windows backslashes."""
    import shlex
    try:
        raw_parts = shlex.split(command_line.strip(), posix=False)
        return [p.strip().strip("'\"") for p in raw_parts if p.strip()]
    except Exception:
        return [p.strip().strip("'\"") for p in command_line.strip().split() if p.strip()]


from any_context.cli.formatters import format_sync_status_box



def create_bottom_toolbar_renderer(
    workspace_name: str,
    model_name: str,
    grounding_mode: str
):
    """
    Constructs a dynamic callable for prompt_toolkit bottom_toolbar.
    Renders a continuous full-width horizontal divider and clean status dock with right-aligned exit:
    ────────────────────────────────────────────────────────────────────────────
     📂 CanadaImmigration  │  🤖 gpt-4o-mini  │  🛡️ Hybrid  │  🌐 Search: ON  │  💡 /menu  │  ⚡ Syncing...  │          🚪 /exit 
    """
    import shutil
    import unicodedata
    import re
    from prompt_toolkit.formatted_text import HTML
    from any_context.ingestion.local_folder_ingestor import BackgroundSyncManager
    from any_context.config.db_store import ConfigDBStore

    def _char_width(c: str) -> int:
        if unicodedata.east_asian_width(c) in ('F', 'W'):
            return 2
        if ord(c) > 0x2000 and unicodedata.category(c) in ('So', 'Sk'):
            return 2
        return 1

    def _visible_len(s: str) -> int:
        clean = re.sub(r'<[^>]+>', '', s)
        return sum(_char_width(c) for c in clean)

    def _render():
        # Dynamically check grounding mode and web search status for active workspace
        try:
            store = ConfigDBStore()
            ws_mode = store.get_grounding_mode(workspace_name=workspace_name)
            ws_search = store.get_web_search_status(workspace_name=workspace_name)
        except Exception:
            ws_mode = grounding_mode or "strict"
            ws_search = False

        if not ws_mode:
            ws_mode = grounding_mode or "strict"

        clean_mode = (ws_mode or "strict").capitalize()

        search_badge = (
            "<style fg='#73daca'><b>🌐 Search: ON</b></style>"
            if ws_search
            else "<style fg='#7a84a0'>🌐 Search: OFF</style>"
        )

        # Check background sync status dynamically on each render frame
        sync_badge = ""
        if_sync_part = ""
        try:
            bg_mgr = BackgroundSyncManager()
            if bg_mgr.is_syncing(workspace_name):
                prog_bar = bg_mgr.format_progress_bar(workspace_name, width=8)
                sync_badge = f"  <style fg='#565f89'>│</style>  <style fg='#ff9e64'><b>⚡ Syncing {prog_bar}</b></style>"
                if_sync_part = f"  │  ⚡ Syncing {prog_bar}"
        except Exception:
            pass

        # Calculate exact terminal width to stretch horizontal divider line across entire screen
        try:
            cols = shutil.get_terminal_size((100, 24)).columns
        except Exception:
            cols = 100
        divider_line = "─" * max(cols, 20)

        left_html = (
            f" <style fg='#e0af68'><b>📂 {workspace_name}</b></style>  "
            f"<style fg='#565f89'>│</style>  "
            f"<style fg='#bb9af7'><b>🤖 {model_name}</b></style>  "
            f"<style fg='#565f89'>│</style>  "
            f"<style fg='#7dcfff'><b>🛡️ {clean_mode}</b></style>  "
            f"<style fg='#565f89'>│</style>  "
            f"{search_badge}  "
            f"<style fg='#565f89'>│</style>  "
            f"<style fg='#e0af68'><b>💡 /menu</b></style>"
            f"{sync_badge}"
        )

        right_html = "<style fg='#f7768e'><b>🚪 /exit</b></style> "

        left_visible = (
            f" 📂 {workspace_name}  │  "
            f"🤖 {model_name}  │  "
            f"🛡️ {clean_mode}  │  "
            f"🌐 Search: {'ON' if ws_search else 'OFF'}  │  "
            f"💡 /menu"
            f"{if_sync_part}"
        )
        right_visible = "🚪 /exit "

        pad_count = max(2, cols - _visible_len(left_visible) - _visible_len(right_visible) - 1)
        padding = " " * pad_count

        return HTML(
            f"<style fg='#444b6a'>{divider_line}</style>\n"
            f"{left_html}{padding}{right_html}"
        )

    return _render


def run_chat_loop(active_workspace: str = "Default"):
    active_workspace = (active_workspace or "Default").strip()
    if not active_workspace:
        active_workspace = "Default"

    thread_id = f"chat_{uuid.uuid4()}"
    config = {
        "configurable": {
            "thread_id": thread_id,
            "active_workspace": active_workspace
        },
        "recursion_limit": 50
    }

    from any_context.ingestion.local_folder_ingestor import check_workspace_changes, run_index_folder, BackgroundSyncManager
    diff = check_workspace_changes(active_workspace)
    if diff.get("is_virgin"):
        with Spinner(f"Indexing new workspace '{active_workspace}'...", done_message=f"Workspace '{active_workspace}' ready"):
            run_index_folder(workspace_name=active_workspace, verbose=False)
    elif diff.get("is_up_to_date"):
        safe_stdout_write(f"✔ Workspace '\033[93m{active_workspace}\033[0m' ready (Up to date)\n")
    else:
        safe_stdout_write(f"✔ Workspace '\033[93m{active_workspace}\033[0m' ready\n")
        safe_stdout_write(f"\033[90m📦 Context update available ({diff.get('summary', '')}). Auto-syncing in background...\033[0m\n")
        bg_mgr = BackgroundSyncManager()
        bg_mgr.start_background_sync(active_workspace, verbose=False)

    from any_context.config.app_settings import AppSettings
    from any_context.config.db_store import ConfigDBStore
    store = ConfigDBStore()
    settings = AppSettings.load()
    current_model = settings.models.inference_model if (settings and settings.models and settings.models.inference_model) else "gpt-4o-mini"
    current_grounding_mode = store.get_grounding_mode(workspace_name=active_workspace)

    safe_stdout_write("\n┌" + "─" * 72 + "┐\n")
    safe_stdout_write("│ 💬 Chat started! Type '/' for command palette or '/exit' to quit.      │\n")
    safe_stdout_write("└" + "─" * 72 + "┘\n\n")

    agent_instance = None
    active_workspace_for_agent = None
    active_model_for_agent = None
    active_mode_for_agent = None

    while True:
        try:
            # Dynamically refresh settings in case changed during past turn
            store = ConfigDBStore()
            settings = AppSettings.load()
            if settings and settings.models and settings.models.inference_model:
                current_model = settings.models.inference_model
            current_grounding_mode = store.get_grounding_mode(workspace_name=active_workspace)

            toolbar_fn = create_bottom_toolbar_renderer(
                workspace_name=active_workspace,
                model_name=current_model,
                grounding_mode=current_grounding_mode
            )

            raw_input = safe_prompt_input(
                "\n\033[96m👤 You:\033[0m ",
                workspace_name=active_workspace,
                bottom_toolbar=toolbar_fn
            )
            if raw_input is None:
                continue

            user_input = raw_input.strip()
            cmd = user_input.lower()
            if not cmd:
                continue

            # Slash Command Palette triggered by typing '/' or '/menu' or '/commands'
            if cmd in ["/", "/menu", "/commands", "/slash"]:
                selected_cmd = show_slash_commands_palette(active_workspace=active_workspace)
                if not selected_cmd:
                    continue
                user_input = selected_cmd
                cmd = user_input.lower()

            # Strip trailing /send if user typed /send at end of single-line prompt
            if user_input.lower().endswith("/send"):
                user_input = user_input[:user_input.lower().rfind("/send")].strip()
                cmd = user_input.lower()
                if not user_input:
                    continue

            # Multi-line / Paste command
            if cmd in ["/paste", "/multiline", "/mline"]:
                pasted_text = collect_multiline_paste(active_workspace=active_workspace)
                if not pasted_text:
                    continue
                user_input = pasted_text
                cmd = user_input.lower()

            # Triple quotes block delimiter support (""" or ''')
            elif user_input.startswith('"""') or user_input.startswith("'''"):
                delimiter = '"""' if user_input.startswith('"""') else "'''"
                if len(user_input) >= 6 and user_input.endswith(delimiter):
                    user_input = user_input[3:-3].strip()
                    if not user_input:
                        continue
                    cmd = user_input.lower()
                else:
                    initial_part = user_input[3:].strip()
                    lines = [initial_part] if initial_part else []
                    safe_stdout_write(f"\n\033[90m[Multi-line mode: finish with {delimiter} or /send | abort with /cancel]\033[0m\n")
                    is_closed = False
                    while True:
                        try:
                            line = safe_prompt_input("\033[90m... \033[0m", workspace_name=active_workspace)
                            if line is None:
                                break
                            stripped = line.strip()
                            if stripped.lower() == "/cancel":
                                safe_stdout_write("\n↩️ Multi-line block cancelled.\n\n")
                                lines = []
                                break
                            if stripped.lower() in ["/send", delimiter]:
                                is_closed = True
                                break
                            if stripped.lower().endswith("/send"):
                                content = line[:line.lower().rfind("/send")].rstrip()
                                if content:
                                    lines.append(content)
                                is_closed = True
                                break
                            if stripped.endswith(delimiter):
                                content = line[:line.rfind(delimiter)].rstrip()
                                if content:
                                    lines.append(content)
                                is_closed = True
                                break
                            lines.append(line)
                        except (KeyboardInterrupt, EOFError):
                            safe_stdout_write("\n↩️ Multi-line block cancelled.\n\n")
                            lines = []
                            break

                    if is_closed and lines:
                        user_input = "\n".join(lines).strip()
                        cmd = user_input.lower()
                    else:
                        continue

            # Shell-style line continuation with trailing backslash (\)
            elif user_input.endswith("\\"):
                lines = [user_input[:-1].rstrip()]
                is_cancelled = False
                while True:
                    try:
                        line = safe_prompt_input("\033[90m... \033[0m", workspace_name=active_workspace)
                        if line is None:
                            is_cancelled = True
                            break
                        stripped = line.strip()
                        if stripped.lower() == "/cancel":
                            safe_stdout_write("\n↩️ Line continuation cancelled.\n\n")
                            is_cancelled = True
                            break
                        if stripped.lower() == "/send":
                            break
                        if stripped.lower().endswith("/send"):
                            content = line[:line.lower().rfind("/send")].rstrip()
                            if content:
                                lines.append(content)
                            break
                        if line.endswith("\\"):
                            lines.append(line[:-1].rstrip())
                        else:
                            lines.append(line)
                            break
                    except (KeyboardInterrupt, EOFError):
                        safe_stdout_write("\n↩️ Line continuation cancelled.\n\n")
                        is_cancelled = True
                        break

                if not is_cancelled and lines:
                    user_input = "\n".join(lines).strip()
                    cmd = user_input.lower()
                else:
                    continue

            # Intercept command help flags and /help commands
            if handle_command_help_interception(user_input):
                continue

            elif cmd in ["/exit", "/quit", "/q", "exit", "quit"]:
                safe_stdout_write("\n👋 Saving session memory and exiting AnyContext. See you soon!\n\n")
                try:
                    from any_context.memory import run_session_summarizer_async
                    run_session_summarizer_async(thread_id, active_workspace)
                except Exception:
                    pass
                break

            elif cmd in ["/version", "/v"]:
                safe_stdout_write(f"\033[93m🤖 AnyContext (actx) v{__version__}\033[0m - Levix Digital\n")
                continue

            elif cmd in ["/clear", "/cls", "clear", "cls"]:
                from any_context.cli.banner import clear_terminal
                clear_terminal()
                print_banner()
                safe_stdout_write(f"🧹 Screen cleared | Workspace: \033[93m{active_workspace or 'Global'}\033[0m | Model: \033[95m{current_model}\033[0m\n\n")
                continue

            elif cmd == "/switch" or cmd.startswith("/switch ") or cmd == "/workspace" or cmd.startswith("/workspace "):
                parts = parse_command_args(user_input)
                store = ConfigDBStore()

                if len(parts) == 1:
                    new_workspace = show_workspace_menu()
                    if new_workspace:
                        active_workspace = new_workspace
                        config["configurable"]["active_workspace"] = active_workspace
                        from any_context.ingestion.local_folder_ingestor import check_workspace_changes, run_index_folder, BackgroundSyncManager
                        diff = check_workspace_changes(active_workspace)
                        if diff.get("is_virgin"):
                            with Spinner(f"Indexing new workspace '{active_workspace}'...", done_message=f"Workspace '{active_workspace}' ready"):
                                run_index_folder(workspace_name=active_workspace, verbose=False)
                        elif diff.get("is_up_to_date"):
                            safe_stdout_write(f"✔ Workspace '\033[93m{active_workspace}\033[0m' ready (Up to date)\n")
                        else:
                            safe_stdout_write(f"✔ Workspace '\033[93m{active_workspace}\033[0m' ready\n")
                            safe_stdout_write(f"\033[90m📦 Context update available ({diff.get('summary', '')}). Auto-syncing in background...\033[0m\n")
                            bg_mgr = BackgroundSyncManager()
                            bg_mgr.start_background_sync(active_workspace, verbose=False)
                        agent_instance = None
                    continue

                if "--list" in parts or "-l" in parts:
                    settings = store.get_app_settings()
                    known_workspaces = [w.name for w in settings.workspaces] if settings else []
                    safe_stdout_write(f"\n📂 Configured Workspaces ({len(known_workspaces)}):\n")
                    for w in known_workspaces:
                        active_tag = " \033[92m[Active]\033[0m" if w == active_workspace else ""
                        safe_stdout_write(f"  • \033[93m{w}\033[0m{active_tag}\n")
                    safe_stdout_write("\n")
                    continue

                if "--delete" in parts or "-d" in parts or "--remove" in parts:
                    idx = next(i for i, p in enumerate(parts) if p in ["--delete", "-d", "--remove"])
                    if len(parts) > idx + 1:
                        target_ws_del = parts[idx + 1]
                    else:
                        safe_stdout_write("\n❌ Please specify the workspace name to delete: /switch --delete <name>\n\n")
                        continue
                    if target_ws_del.lower() in ["default", "global", "shared sources"]:
                        safe_stdout_write(f"\n❌ Cannot delete protected system workspace '{target_ws_del}'.\n\n")
                    else:
                        deleted = store.remove_workspace(target_ws_del)
                        if deleted:
                            safe_stdout_write(f"\n🗑️ Successfully deleted workspace '\033[93m{target_ws_del}\033[0m'.\n\n")
                            if active_workspace.lower() == target_ws_del.lower():
                                active_workspace = "Default"
                                config["configurable"]["active_workspace"] = "Default"
                                agent_instance = None
                        else:
                            safe_stdout_write(f"\n❌ Workspace '{target_ws_del}' not found.\n\n")
                    continue

                if "--create" in parts or "-c" in parts or "--add" in parts:
                    idx = next(i for i, p in enumerate(parts) if p in ["--create", "-c", "--add"])
                    target_ws = parts[idx + 1] if len(parts) > idx + 1 else None
                elif len(parts) > 1 and parts[1].lower() in ["add", "create"] and len(parts) > 2:
                    target_ws = parts[2]
                elif len(parts) > 1 and parts[1].lower() in ["delete", "remove"] and len(parts) > 2:
                    target_ws_del = parts[2]
                    if target_ws_del.lower() in ["default", "global", "shared sources"]:
                        safe_stdout_write(f"\n❌ Cannot delete protected system workspace '{target_ws_del}'.\n\n")
                    else:
                        deleted = store.remove_workspace(target_ws_del)
                        if deleted:
                            safe_stdout_write(f"\n🗑️ Successfully deleted workspace '\033[93m{target_ws_del}\033[0m'.\n\n")
                            if active_workspace.lower() == target_ws_del.lower():
                                active_workspace = "Default"
                                config["configurable"]["active_workspace"] = "Default"
                                agent_instance = None
                        else:
                            safe_stdout_write(f"\n❌ Workspace '{target_ws_del}' not found.\n\n")
                    continue
                else:
                    target_ws = parts[1]

                if not target_ws:
                    safe_stdout_write("\n❌ Please specify a workspace name.\n\n")
                    continue

                meta = store.get_workspace_meta(target_ws)
                if not meta:
                    store.add_workspace(target_ws, paths=[])
                    safe_stdout_write(f"\n✅ Created and switched to new workspace '\033[93m{target_ws}\033[0m'.\n\n")
                else:
                    target_ws = meta["name"]
                    safe_stdout_write(f"\n🔄 Switched to workspace '\033[93m{target_ws}\033[0m'.\n\n")

                active_workspace = target_ws
                config["configurable"]["active_workspace"] = active_workspace
                from any_context.ingestion.local_folder_ingestor import check_workspace_changes, run_index_folder, BackgroundSyncManager
                diff = check_workspace_changes(active_workspace)
                if diff.get("is_virgin"):
                    with Spinner(f"Indexing new workspace '{active_workspace}'...", done_message=f"Workspace '{active_workspace}' ready"):
                        run_index_folder(workspace_name=active_workspace, verbose=False)
                elif diff.get("is_up_to_date"):
                    safe_stdout_write(f"✔ Workspace '\033[93m{active_workspace}\033[0m' ready (Up to date)\n")
                else:
                    safe_stdout_write(f"✔ Workspace '\033[93m{active_workspace}\033[0m' ready\n")
                    safe_stdout_write(f"\033[90m📦 Context update available ({diff.get('summary', '')}). Auto-syncing in background...\033[0m\n")
                    bg_mgr = BackgroundSyncManager()
                    bg_mgr.start_background_sync(active_workspace, verbose=False)
                agent_instance = None
                continue

            elif cmd == "/transfer" or cmd.startswith("/transfer ") or cmd.startswith("/workspace transfer") or cmd.startswith("/move-source"):
                parts = parse_command_args(user_input)
                if len(parts) < 4 or (len(parts) > 1 and parts[1].lower() == "transfer" and len(parts) < 5):
                    from any_context.cli.config_menu import _transfer_workspace_source
                    store = ConfigDBStore()
                    _transfer_workspace_source(store)
                    continue

                arg_offset = 2 if len(parts) > 1 and parts[1].lower() == "transfer" else 1
                source_ws = parts[arg_offset]
                target_ws = parts[arg_offset + 1]
                source_item = " ".join(parts[arg_offset + 2:]).strip().strip("'\"")

                store = ConfigDBStore()
                from any_context.ingestion.web_scheduler import WebSchedulerStore
                web_store = WebSchedulerStore()

                with Spinner(f"Moving '{source_item}' from '{source_ws}' to '{target_ws}'..."):
                    if source_item.startswith("http://") or source_item.startswith("https://"):
                        res = web_store.transfer_web_source(source_ws=source_ws, target_ws=target_ws, url_or_root=source_item)
                        if res.get("success"):
                            safe_stdout_write(f"\n✅ Transferred web portal '{source_item}' ({res.get('transferred_pages', 0)} pages, {res.get('transferred_chunks', 0)} chunks) to '{target_ws}' in < 50ms! (API Cost: $0.00)\n\n")
                        else:
                            safe_stdout_write(f"\n❌ Transfer error: {res.get('error')}\n\n")
                    else:
                        res = store.transfer_local_folder_source(source_ws=source_ws, target_ws=target_ws, folder_path=source_item)
                        if res.get("success"):
                            safe_stdout_write(f"\n✅ Transferred folder '{source_item}' ({res.get('transferred_chunks', 0)} vector chunks) to '{target_ws}' in < 50ms! (API Cost: $0.00)\n\n")
                        else:
                            safe_stdout_write(f"\n❌ Transfer error: {res.get('error')}\n\n")
                continue

            elif cmd in ["/link", "/unlink", "/shared"] or cmd.startswith("/link ") or cmd.startswith("/unlink ") or cmd.startswith("/shared ") or cmd.startswith("/workspace link") or cmd.startswith("/workspace unlink"):
                parts = parse_command_args(user_input)
                store = ConfigDBStore()
                is_unlink = cmd.startswith("/unlink") or "--unlink" in parts or "-u" in parts or (len(parts) > 1 and parts[1].lower() == "unlink")
                is_list = cmd.startswith("/shared") or "--list" in parts or "-l" in parts or (len(parts) > 1 and parts[1].lower() == "shared")

                if is_list:
                    sources = store.list_all_available_shared_sources()
                    safe_stdout_write("\n📚 \033[1mIndexed Shared Sources across Workspaces ($0.00 Reusable):\033[0m\n")
                    if not sources:
                        safe_stdout_write("  (No sources indexed yet. Add a local folder or web portal to any workspace first!)\n\n")
                    else:
                        for s in sources:
                            orig = s.get("origin_workspace", "Workspace")
                            stype = s.get("type", "folder").upper()
                            ident = s.get("identifier")
                            title = s.get("title") or ident
                            safe_stdout_write(f"  • [\033[96m{stype}\033[0m] \033[93m{title}\033[0m (Origin: {orig})\n    Path: {ident}\n")
                        safe_stdout_write("----------------------------------------------------\n\n")
                    continue

                if is_unlink:
                    if len(parts) == 1 or (len(parts) == 2 and parts[1].lower() in ["unlink", "--unlink", "-u"]):
                        from any_context.cli.config_menu import _unlink_shared_source
                        _unlink_shared_source(store)
                        continue

                    arg_offset = 2 if len(parts) > 1 and parts[1].lower() in ["unlink", "--unlink", "-u"] else 1
                    source_item = parts[arg_offset].strip().strip("'\"")
                    target_ws = parts[arg_offset + 1] if len(parts) > arg_offset + 1 else active_workspace

                    links = store.get_workspace_shared_links(target_ws)
                    matched_link = None
                    for l in links:
                        if (source_item.lower() == l["source_identifier"].lower() or 
                            source_item.lower() == (l.get("title") or "").lower() or 
                            source_item.lower() in l["source_identifier"].lower() or 
                            source_item.lower() in (l.get("title") or "").lower()):
                            matched_link = l
                            break

                    if matched_link:
                        source_identifier = matched_link["source_identifier"]
                        stype = matched_link["source_type"]
                    else:
                        source_identifier = source_item
                        stype = "web" if source_item.startswith("http://") or source_item.startswith("https://") else "folder"

                    unlinked = store.unlink_shared_source_from_workspace(workspace_name=target_ws, source_type=stype, source_identifier=source_identifier)
                    if unlinked:
                        safe_stdout_write(f"\n🗑️ Unlinked Shared Source '{source_identifier}' from workspace '\033[93m{target_ws}\033[0m'.\n\n")
                    else:
                        safe_stdout_write(f"\n❌ Shared Source link '{source_item}' not found in workspace '{target_ws}'.\n\n")
                    continue

                if len(parts) == 1:
                    from any_context.cli.config_menu import _link_shared_source
                    _link_shared_source(store)
                    continue

                arg_offset = 2 if len(parts) > 1 and parts[1].lower() == "link" else 1
                source_item = parts[arg_offset].strip().strip("'\"")
                target_ws = parts[arg_offset + 1] if len(parts) > arg_offset + 1 else active_workspace

                available_sources = store.list_all_available_shared_sources()
                matched_src = None
                for s in available_sources:
                    if (source_item.lower() == s["identifier"].lower() or 
                        source_item.lower() == (s.get("title") or "").lower() or 
                        source_item.lower() in s["identifier"].lower() or 
                        source_item.lower() in (s.get("title") or "").lower()):
                        matched_src = s
                        break

                if matched_src:
                    source_identifier = matched_src["identifier"]
                    stype = matched_src["type"]
                    title = matched_src.get("title")
                else:
                    source_identifier = source_item
                    stype = "web" if source_item.startswith("http://") or source_item.startswith("https://") else "folder"
                    title = None

                res = store.link_shared_source_to_workspace(workspace_name=target_ws, source_type=stype, source_identifier=source_identifier, title=title)
                safe_stdout_write(f"\n🔗 Successfully linked Shared Source '{title or source_identifier}' to workspace '\033[93m{target_ws}\033[0m' ($0.00 cost)!\n\n")
                continue

            elif cmd == "/rename" or cmd.startswith("/rename ") or cmd.startswith("/workspace rename"):
                parts = parse_command_args(user_input)
                store = ConfigDBStore()
                if len(parts) < 3 or (len(parts) > 1 and parts[1].lower() == "rename" and len(parts) < 4):
                    settings = store.get_app_settings()
                    known_workspaces = [w.name for w in settings.workspaces if w.name.lower() not in ["default", "global", "shared sources"]] if settings else []
                    if not known_workspaces:
                        safe_stdout_write("\n⚠️ No custom workspaces configured to rename ('Default', 'Global', and 'Shared Sources' are protected system workspaces).\n\n")
                        continue
                    old_ws = questionary.select("Select Workspace to rename:", choices=known_workspaces).ask()
                    if not old_ws:
                        continue
                    new_ws = questionary.text(f"Enter new name for workspace '{old_ws}':").ask()
                    if not new_ws or not new_ws.strip():
                        continue
                    clean_new_ws = new_ws.strip().strip("'\"")
                else:
                    arg_offset = 2 if len(parts) > 1 and parts[1].lower() == "rename" else 1
                    old_ws = parts[arg_offset]
                    clean_new_ws = parts[arg_offset + 1].strip().strip("'\"")

                if old_ws.lower() in ["default", "global", "shared sources"]:
                    safe_stdout_write(f"\n❌ Error renaming workspace: Workspace '{old_ws}' is a protected system workspace and cannot be renamed.\n\n")
                    continue
                if clean_new_ws.lower() in ["default", "global", "shared sources"]:
                    safe_stdout_write(f"\n❌ Error renaming workspace: Cannot rename to protected system workspace '{clean_new_ws}'.\n\n")
                    continue

                with Spinner(f"Renaming workspace '{old_ws}' to '{clean_new_ws}'..."):
                    res = store.rename_workspace(old_name=old_ws, new_name=clean_new_ws)
                if res.get("success"):
                    migrated = res.get("migrated_chunks", 0)
                    safe_stdout_write(f"\n✅ Renamed workspace '{old_ws}' to '{clean_new_ws}' ({migrated} vector chunks updated)! (API Cost: $0.00)\n\n")
                    if active_workspace == old_ws:
                        active_workspace = clean_new_ws
                        config["configurable"]["active_workspace"] = active_workspace
                        agent_instance = None
                else:
                    safe_stdout_write(f"\n❌ Error renaming workspace: {res.get('error')}\n\n")
                continue

            elif cmd in ["/sync", "/resync", "/index"] or cmd.startswith("/sync ") or cmd.startswith("/index "):
                parts = parse_command_args(user_input)
                is_verbose = "--verbose" in parts or "-v" in parts
                is_full = "--full" in parts or "--force" in parts
                is_status = "--status" in parts or "-s" in parts or "status" in parts
                is_all = "--all" in parts or "-a" in parts or "all" in parts
                is_bg = "--bg" in parts or "--background" in parts

                is_folder_only = "--folder" in parts or "--folders" in parts or "-f" in parts
                is_web_only = "--web" in parts or "-w" in parts or "--urls" in parts
                is_drive_only = "--drive" in parts or "--drives" in parts or "--cloud" in parts

                from any_context.ingestion.unified_sync import run_unified_sync
                from any_context.ingestion.local_folder_ingestor import check_workspace_changes, BackgroundSyncManager, format_sync_status_box

                if is_status:
                    if is_all:
                        from any_context.config.db_store import ConfigDBStore
                        store = ConfigDBStore()
                        all_ws = store.list_all_workspace_sources()
                        safe_stdout_write(f"\n📋 \033[1mAll Configured Workspaces Sync Status ({len(all_ws)} Workspaces):\033[0m\n\n")
                        for ws_entry in all_ws:
                            w_name = ws_entry.get("name")
                            diff = check_workspace_changes(w_name)
                            safe_stdout_write(format_sync_status_box(diff) + "\n\n")
                    else:
                        diff = check_workspace_changes(active_workspace)
                        safe_stdout_write("\n" + format_sync_status_box(diff) + "\n\n")
                    continue

                target_ws = None if is_all else active_workspace
                has_specific_flag = is_folder_only or is_web_only or is_drive_only

                sync_folders = is_folder_only if has_specific_flag else True
                sync_web = is_web_only if has_specific_flag else True
                sync_drives = is_drive_only if has_specific_flag else True

                scope_desc = "all sources (folders, web, drives)" if not has_specific_flag else (
                    "local folders only" if sync_folders else ("web sources only" if sync_web else "cloud drives only")
                )
                ws_label = "all workspaces" if is_all else f"workspace '{active_workspace}'"

                if is_verbose:
                    run_unified_sync(
                        workspace_name=target_ws,
                        sync_folders=sync_folders,
                        sync_web=sync_web,
                        sync_drives=sync_drives,
                        force_full=is_full,
                        verbose=True,
                        is_all=is_all
                    )
                else:
                    bg_mgr = BackgroundSyncManager()
                    bg_mgr.start_background_sync(
                        workspace_name=target_ws or active_workspace,
                        sync_folders=sync_folders,
                        sync_web=sync_web,
                        sync_drives=sync_drives,
                        force_full=is_full,
                        verbose=False,
                        is_all=is_all
                    )
                    safe_stdout_write(f"\n⚡ Background synchronization started for {ws_label} ({scope_desc}). You can continue chatting!\n\n")
                agent_instance = None
                continue

            elif cmd == "/model" or cmd == "/m" or cmd.startswith("/model ") or cmd.startswith("/m "):
                from any_context.core.models_catalog import get_available_models, validate_model_key_availability

                parts = parse_command_args(user_input)
                if "--list" in parts or "-l" in parts:
                    available = get_available_models()
                    print("\n🤖 Available Models with Configured API Keys:")
                    for m in available:
                        active_tag = " \033[92m[Active]\033[0m" if m["id"] == current_model else ""
                        print(f"  • \033[95m{m['name']}\033[0m (\033[93m{m['id']}\033[0m){active_tag}")
                    print()
                    continue

                if len(parts) > 1 and not parts[1].startswith("-"):
                    new_model = parts[1].strip()
                    is_valid, prov, err_msg = validate_model_key_availability(new_model)
                    if not is_valid:
                        print(f"\n{err_msg}\n")
                        continue
                    current_model = new_model
                    agent_instance = None
                    print(f"\n🔄 Switched active inference model to \033[95m{current_model}\033[0m ({prov.upper()}) for this session.\n")
                    continue

                available_models = get_available_models()
                choices = []
                for m in available_models:
                    prefix = "👉 " if m["id"] == current_model else "• "
                    choices.append(f"{prefix}{m['name']} ({m['id']})")

                choices.append("➕ Enter Custom Model ID")
                choices.append("🔑 Add API Key for Another Provider (/config)")
                choices.append("🔙 Cancel")

                selected = questionary.select(
                    f"Select Inference Model (Active: {current_model}):",
                    choices=choices
                ).ask()

                if not selected or selected.startswith("🔙"):
                    continue

                if selected.startswith("🔑"):
                    show_config_menu()
                    continue

                if selected.startswith("➕"):
                    custom_id = questionary.text("Enter Model Identifier (e.g. 'claude-3-5-sonnet-20241022', 'gpt-4o', 'deepseek-chat'):").ask()
                    if custom_id and custom_id.strip():
                        is_valid, prov, err_msg = validate_model_key_availability(custom_id.strip())
                        if not is_valid:
                            print(f"\n{err_msg}\n")
                            continue
                        current_model = custom_id.strip()
                        agent_instance = None
                        print(f"\n🔄 Switched active inference model to \033[95m{current_model}\033[0m for this session.\n")
                    continue

                if "(" in selected and selected.endswith(")"):
                    extracted_id = selected[selected.rfind("(") + 1 : -1].strip()
                    current_model = extracted_id
                    agent_instance = None
                    print(f"\n🔄 Switched active inference model to \033[95m{current_model}\033[0m for this session.\n")
                continue

            elif cmd.startswith("/mode") or cmd.startswith("/answer-mode") or cmd.startswith("/grounding") or cmd.startswith("/am"):
                parts = parse_command_args(user_input)
                store = ConfigDBStore()
                is_global = "--global" in parts or "-g" in parts
                action_parts = [p for p in parts if p not in ["--global", "-g"]]

                if len(action_parts) >= 2:
                    arg_mode = action_parts[1].lower().strip().lstrip("-")
                    if arg_mode in ["strict", "s", "audit", "legal", "1"]:
                        new_mode = "strict"
                    elif arg_mode in ["proactive", "p", "research", "creative", "strategy", "3"]:
                        new_mode = "proactive"
                    else:
                        new_mode = "hybrid"
                    current_grounding_mode = store.set_grounding_mode(new_mode, workspace_name=None if is_global else active_workspace, apply_global=is_global)
                    agent_instance = None
                    active_mode_for_agent = None
                    target_label = "all workspaces (global)" if is_global else f"workspace '\033[1m{active_workspace}\033[0m\033[96m'"
                    print(f"\n✅ AI Grounding Mode for {target_label} set to: \033[1m\033[96m{current_grounding_mode.capitalize()}\033[0m\n")
                    continue
                else:
                    choices = [
                        f"⚖️ Hybrid (Default - Workspace facts + clearly labeled external suggestions){'  [Active]' if current_grounding_mode == 'hybrid' else ''}",
                        f"🛡️ Strict (Audit & Legal - 100% grounded to indexed documents, zero speculation){'  [Active]' if current_grounding_mode == 'strict' else ''}",
                        f"🚀 Proactive (Research & Ideation - Broad synthesis, insights & web recommendations){'  [Active]' if current_grounding_mode == 'proactive' else ''}",
                        "🌐 Apply Grounding Mode Globally Across All Workspaces...",
                        "🔙 [Cancel]"
                    ]
                    mode_choice = questionary.select(
                        f"Select AI Grounding & Answer Mode for Workspace '{active_workspace}':",
                        choices=choices
                    ).ask()
                    if not mode_choice or mode_choice.startswith("🔙"):
                        continue
                    if mode_choice.startswith("🛡️"):
                        new_mode = "strict"
                    elif mode_choice.startswith("🚀"):
                        new_mode = "proactive"
                    elif mode_choice.startswith("🌐"):
                        g_choice = questionary.select(
                            "Select Grounding Mode to apply GLOBALLY across ALL workspaces:",
                            choices=["⚖️ Hybrid (Global)", "🛡️ Strict (Global)", "🚀 Proactive (Global)", "🔙 [Cancel]"]
                        ).ask()
                        if not g_choice or g_choice.startswith("🔙"):
                            continue
                        g_mode = "strict" if "Strict" in g_choice else ("proactive" if "Proactive" in g_choice else "hybrid")
                        current_grounding_mode = store.set_grounding_mode(g_mode, apply_global=True)
                        agent_instance = None
                        active_mode_for_agent = None
                        print(f"\n✅ AI Grounding Mode set GLOBALLY to: \033[1m\033[96m{current_grounding_mode.capitalize()}\033[0m across all workspaces!\n")
                        continue
                    else:
                        new_mode = "hybrid"
                    current_grounding_mode = store.set_grounding_mode(new_mode, workspace_name=active_workspace)
                    agent_instance = None
                    active_mode_for_agent = None
                    print(f"\n✅ AI Grounding Mode for workspace '\033[1m{active_workspace}\033[0m\033[96m' set to: \033[1m\033[96m{current_grounding_mode.capitalize()}\033[0m\n")
                    continue

            elif cmd.startswith("/web-search") or cmd.startswith("/websearch") or cmd.startswith("/search-web") or cmd == "/search" or cmd.startswith("/search ") or cmd == "/ws" or cmd.startswith("/ws "):
                parts = parse_command_args(user_input)
                store = ConfigDBStore()
                is_global = "--global" in parts or "-g" in parts
                action_parts = [p for p in parts if p not in ["--global", "-g"]]

                if len(action_parts) >= 2:
                    arg_action = action_parts[1].lower().strip().lstrip("-")
                    if arg_action in ["on", "1", "true", "enable", "enabled"]:
                        store.set_web_search_status(True, workspace_name=None if is_global else active_workspace, apply_global=is_global)
                        agent_instance = None
                        target_label = "all workspaces (global)" if is_global else f"workspace '\033[1m{active_workspace}\033[0m\033[92m'"
                        print(f"\n✅ \033[92mWeb Search enabled for {target_label}!\033[0m")
                        print("\033[93m⚠️ Cost & Transparency Notice:\033[0m The agent will perform real-time internet searches and consume external web tokens when answering.\n")
                        continue
                    elif arg_action in ["off", "0", "false", "disable", "disabled"]:
                        store.set_web_search_status(False, workspace_name=None if is_global else active_workspace, apply_global=is_global)
                        agent_instance = None
                        target_label = "all workspaces (global)" if is_global else f"workspace '\033[1m{active_workspace}\033[0m\033[90m'"
                        print(f"\n🔒 \033[90mWeb Search disabled for {target_label}. (100% offline local isolation)\033[0m\n")
                        continue
                    elif arg_action in ["status", "s", "info"]:
                        st = store.get_web_search_status(workspace_name=active_workspace)
                        status_str = "\033[92mENABLED (ON)\033[0m" if st else "\033[90mDISABLED (OFF)\033[0m"
                        print(f"\n🌐 Web Search Status for Workspace '\033[1m{active_workspace}\033[0m': {status_str}\n")
                        continue

                current_st = store.get_web_search_status(workspace_name=active_workspace)
                choices = [
                    f"🟢 Enable Web Search for Workspace '{active_workspace}' (Real-time online search){'  [Active]' if current_st else ''}",
                    f"🔴 Disable Web Search for Workspace '{active_workspace}' (100% offline local docs){'  [Active]' if not current_st else ''}",
                    "🌐 Apply Web Search Setting Globally Across All Workspaces...",
                    "🔙 [Cancel]"
                ]
                choice = questionary.select(
                    f"Configure Web Search for Workspace '{active_workspace}':",
                    choices=choices
                ).ask()
                if not choice or choice.startswith("🔙"):
                    continue
                if choice.startswith("🟢"):
                    store.set_web_search_status(True, workspace_name=active_workspace)
                    agent_instance = None
                    print(f"\n✅ \033[92mWeb Search enabled for workspace '\033[1m{active_workspace}\033[0m\033[92m'!\033[0m")
                    print("\033[93m⚠️ Cost & Transparency Notice:\033[0m The agent will perform real-time internet searches and consume external web tokens when answering.\n")
                elif choice.startswith("🔴"):
                    store.set_web_search_status(False, workspace_name=active_workspace)
                    agent_instance = None
                    print(f"\n🔒 \033[90mWeb Search disabled for workspace '\033[1m{active_workspace}\033[0m\033[90m'. (100% offline local isolation)\033[0m\n")
                elif choice.startswith("🌐"):
                    g_choice = questionary.select(
                        "Apply Web Search setting across ALL workspaces:",
                        choices=["Enable Web Search Globally (All Workspaces)", "Disable Web Search Globally (All Workspaces)", "🔙 [Cancel]"]
                    ).ask()
                    if g_choice and "Enable" in g_choice:
                        store.set_web_search_status(True, apply_global=True)
                        agent_instance = None
                        print("\n✅ \033[92mWeb Search enabled GLOBALLY across all workspaces!\033[0m")
                        print("\033[93m⚠️ Cost Notice:\033[0m Real-time web lookups are now active for all workspaces.\n")
                    elif g_choice and "Disable" in g_choice:
                        store.set_web_search_status(False, apply_global=True)
                        agent_instance = None
                        print("\n🔒 \033[90mWeb Search disabled GLOBALLY across all workspaces.\033[0m\n")
                continue

            elif cmd in ["/search-engine", "/searchengine", "/engine"] or cmd.startswith("/search-engine ") or cmd.startswith("/engine "):
                parts = parse_command_args(user_input)
                cur_eng = store.get_default_search_engine(workspace_name=active_workspace)
                tavily_key = store.get_api_key("tavily") or os.getenv("TAVILY_API_KEY")
                serper_key = store.get_api_key("serper") or os.getenv("SERPER_API_KEY")

                # Direct argument mode (e.g. /search-engine tavily, /engine serper, /search-engine --ddg)
                if len(parts) >= 2:
                    raw_arg = parts[1].lower().strip().lstrip("-")
                    if raw_arg in ["tavily", "tvly"]:
                        store.set_default_search_engine("tavily", workspace_name=active_workspace)
                        agent_instance = None
                        safe_stdout_write(f"\n✅ Web Search Engine for workspace '\033[1m{active_workspace}\033[0m' set to: \033[92mTAVILY (AI-Native Research)\033[0m\n")
                        if not tavily_key:
                            safe_stdout_write("\033[93m⚠️ Notice:\033[0m Tavily API key is not yet configured. Run '/key tavily <chave>' or configure in '/config'.\n\n")
                        else:
                            safe_stdout_write("\n")
                        continue
                    elif raw_arg in ["serper", "google", "serp"]:
                        store.set_default_search_engine("serper", workspace_name=active_workspace)
                        agent_instance = None
                        safe_stdout_write(f"\n✅ Web Search Engine for workspace '\033[1m{active_workspace}\033[0m' set to: \033[92mSERPER (Google Search API)\033[0m\n")
                        if not serper_key:
                            safe_stdout_write("\033[93m⚠️ Notice:\033[0m Serper API key is not yet configured. Run '/key serper <chave>' or configure in '/config'.\n\n")
                        else:
                            safe_stdout_write("\n")
                        continue
                    elif raw_arg in ["duckduckgo", "ddg", "duck", "free"]:
                        store.set_default_search_engine("duckduckgo", workspace_name=active_workspace)
                        agent_instance = None
                        safe_stdout_write(f"\n✅ Web Search Engine for workspace '\033[1m{active_workspace}\033[0m' set to: \033[92mDUCKDUCKGO (Free / No Key Required)\033[0m\n\n")
                        continue
                    elif raw_arg in ["auto", "smart", "default"]:
                        store.set_default_search_engine("auto", workspace_name=active_workspace)
                        agent_instance = None
                        safe_stdout_write(f"\n✅ Web Search Engine for workspace '\033[1m{active_workspace}\033[0m' set to: \033[92mAUTO (Tavily -> Serper -> DuckDuckGo)\033[0m\n\n")
                        continue

                # Interactive selection menu
                tav_status = f" [Key: {mask_key(tavily_key)}]" if tavily_key else " [No Key]"
                serp_status = f" [Key: {mask_key(serper_key)}]" if serper_key else " [No Key]"
                eng_choices = [
                    f"⭐ Auto (Tavily -> Serper -> DuckDuckGo){'  [Active]' if cur_eng == 'auto' else ''}",
                    f"🌐 Tavily Search API (AI-Native Research){tav_status}{'  [Active]' if cur_eng == 'tavily' else ''}",
                    f"🔍 Serper (Google Search API){serp_status}{'  [Active]' if cur_eng == 'serper' else ''}",
                    f"🦆 DuckDuckGo (Free / 100% No-Cost){'  [Active]' if cur_eng in ['duckduckgo', 'ddg'] else ''}",
                    "🔙 [Cancel]"
                ]
                choice = questionary.select(
                    f"Select Web Search Engine for Workspace '{active_workspace}':",
                    choices=eng_choices
                ).ask()
                if not choice or choice.startswith("🔙"):
                    continue

                if choice.startswith("⭐"):
                    new_eng = "auto"
                    label = "Auto (Tavily -> Serper -> DuckDuckGo)"
                elif choice.startswith("🌐"):
                    new_eng = "tavily"
                    label = "Tavily Search API"
                elif choice.startswith("🔍"):
                    new_eng = "serper"
                    label = "Serper (Google Search API)"
                else:
                    new_eng = "duckduckgo"
                    label = "DuckDuckGo (Free / No Key)"

                store.set_default_search_engine(new_eng, workspace_name=active_workspace)
                agent_instance = None
                safe_stdout_write(f"\n✅ Web Search Engine for workspace '\033[1m{active_workspace}\033[0m' set to: \033[92m{label}\033[0m!\n\n")
                continue

            elif cmd == "/update" or cmd.startswith("/update ") or cmd.startswith("/update@") or cmd in ["/check-update", "/checkupdate", "/check"]:
                parts = parse_command_args(user_input)
                is_force = "--force" in parts or "-f" in parts
                is_check_only = "--check" in parts or "-c" in parts or cmd in ["/check-update", "/checkupdate", "/check"]
                is_list = "--list" in parts or "-l" in parts or "list" in parts or "--releases" in parts or "releases" in parts
                is_rollback = "--rollback" in parts or "-r" in parts or "rollback" in parts
                from any_context.cli.updater import run_self_update, check_for_updates, display_available_releases

                if is_list or is_rollback:
                    picked = display_available_releases(interactive_select=True)
                    if picked:
                        run_self_update(target_version=picked, force=is_force)
                    continue

                if is_check_only:
                    has_up, new_tag = check_for_updates(quiet_if_latest=False)
                    if has_up:
                        try:
                            do_upgrade = questionary.confirm(
                                f"Would you like to download and install {new_tag} now?",
                                default=True
                            ).ask()
                            if do_upgrade:
                                run_self_update(force=is_force)
                        except Exception:
                            pass
                    continue

                # Extract targeted version if specified
                target_version = None
                if cmd.startswith("/update@"):
                    target_version = cmd.split("@", 1)[1].strip()
                elif "--to" in parts:
                    idx = parts.index("--to")
                    if idx + 1 < len(parts):
                        target_version = parts[idx + 1]
                elif "--version" in parts:
                    idx = parts.index("--version")
                    if idx + 1 < len(parts):
                        target_version = parts[idx + 1]
                else:
                    # Look for positional version argument (e.g. /update 0.15.2 or /update @0.15.2 or /update v0.15.2)
                    non_flags = [p for p in parts[1:] if not p.startswith("-")]
                    if non_flags:
                        target_version = non_flags[0]

                run_self_update(target_version=target_version, force=is_force)
                continue

            elif cmd in ["/reset-memory", "/reset"] or cmd.startswith("/reset-memory ") or cmd.startswith("/reset "):
                parts = parse_command_args(user_input)
                is_force = "--force" in parts or "-f" in parts
                is_all = "--all" in parts or "-a" in parts

                target_desc = "ALL workspaces" if is_all else f"workspace '{active_workspace}'"
                if not is_force:
                    confirm = questionary.confirm(
                        f"⚠️ Are you sure you want to reset long-term memory for {target_desc}?"
                    ).ask()
                    if not confirm:
                        continue
                from any_context.memory import MemoryManager
                memory_mgr = MemoryManager()
                target_ws_arg = None if is_all else active_workspace
                deleted = memory_mgr.reset_memory(workspace=target_ws_arg)
                print(f"🧹 Reset complete! Deleted {deleted} long-term memory entries for {target_desc}.")
                continue

            elif cmd in ["/factory-reset", "/reset-factory"]:
                confirm = questionary.confirm(
                    "⚠️ DANGER: Are you sure you want to reset AnyContext to Factory Defaults?\n  This will erase ALL workspaces, folders, API keys, configuration settings, and vector memory databases!"
                ).ask()
                if confirm:
                    store = ConfigDBStore()
                    store.factory_reset()
                    print("\n🎉 AnyContext has been completely reset to factory defaults!")
                    print("Run 'actx' again anytime to launch the first-time setup wizard.\n")
                    sys.exit(0)
                continue

            elif cmd == "/config":
                show_config_menu()
                continue

            elif cmd == "/key" or cmd.startswith("/key ") or cmd in ["/keys", "/api-keys", "/apikeys"] or cmd.startswith("/keys ") or cmd.startswith("/apikey "):
                parts = parse_command_args(user_input)
                store = ConfigDBStore()
                from any_context.cli.config_menu import mask_key, _manage_api_keys

                if len(parts) == 1:
                    _manage_api_keys(store)
                    continue

                if "--list" in parts or "-l" in parts or "list" in parts:
                    all_keys = store.get_all_api_keys()
                    safe_stdout_write("\n--- Saved API Keys ---\n")
                    if not all_keys:
                        safe_stdout_write("No API keys saved in database yet.\n")
                    else:
                        for provider, key in all_keys.items():
                            safe_stdout_write(f"• \033[93m{provider.upper()}\033[0m: {mask_key(key)}\n")
                    safe_stdout_write("----------------------\n\n")
                    continue

                action_parts = [p for p in parts if p not in ["--set", "-s", "set"]]
                # e.g. /key tavily tvly-xxx OR /key set tavily tvly-xxx
                if len(action_parts) >= 3:
                    prov = action_parts[1].lower().strip()
                    val = action_parts[2].strip()
                    store.set_api_key(prov, val)
                    safe_stdout_write(f"\n✅ Saved API Key for provider '\033[93m{prov.upper()}\033[0m' ({mask_key(val)}).\n\n")
                elif len(action_parts) == 2 and action_parts[1] in ["--help", "-h", "help", "guide"]:
                    from any_context.help.manager import display_help_page
                    from any_context.help.registry import get_help_page
                    page = get_help_page("api-keys")
                    if page:
                        display_help_page(page)
                else:
                    _manage_api_keys(store)
                continue

            elif cmd in ["/billing", "/plans"]:
                from any_context.cli.config_menu import _manage_subscription
                _manage_subscription()
                continue

            elif cmd == "/folder" or cmd.startswith("/folder ") or cmd == "/folders" or cmd.startswith("/folders "):
                parts = parse_command_args(user_input)
                is_sync = "--sync" in parts or "-s" in parts or "sync" in parts or "resync" in parts
                is_add = "--add" in parts or "-a" in parts or "add" in parts
                is_remove = "--remove" in parts or "-r" in parts or "remove" in parts or "rm" in parts or "delete" in parts
                is_full = "--full" in parts or "--force" in parts

                from any_context.config.db_store import ConfigDBStore
                store = ConfigDBStore()

                if is_sync:
                    is_verbose = "--verbose" in parts or "-v" in parts
                    if is_verbose:
                        from any_context.ingestion.unified_sync import run_unified_sync
                        run_unified_sync(workspace_name=active_workspace, sync_folders=True, sync_web=False, sync_drives=False, force_full=is_full, verbose=True)
                    else:
                        from any_context.ingestion.local_folder_ingestor import BackgroundSyncManager
                        bg_mgr = BackgroundSyncManager()
                        bg_mgr.start_background_sync(active_workspace, sync_folders=True, sync_web=False, sync_drives=False, force_full=is_full)
                        safe_stdout_write(f"\n⚡ Background folder synchronization started for workspace '\033[93m{active_workspace}\033[0m'. You can continue chatting!\n\n")
                    agent_instance = None
                    continue

                if is_add:
                    folder_to_add = None
                    for i, p in enumerate(parts):
                        if p in ["--add", "-a", "add"] and len(parts) > i + 1:
                            folder_to_add = parts[i + 1]
                            break
                        elif os.path.exists(p) and p not in ["/folder", "/folders"]:
                            folder_to_add = p
                            break
                    if folder_to_add and os.path.exists(folder_to_add):
                        abs_p = os.path.abspath(folder_to_add)
                        store.add_folder_to_workspace(active_workspace, abs_p)
                        print(f"\n📁 Added folder '\033[96m{abs_p}\033[0m' to workspace '\033[93m{active_workspace}\033[0m'!")
                        with Spinner(f"Indexing new folder...", done_message=f"Folder indexed"):
                            from any_context.ingestion.unified_sync import run_unified_sync
                            run_unified_sync(workspace_name=active_workspace, sync_folders=True, sync_web=False, sync_drives=False)
                        agent_instance = None
                    else:
                        print("\n⚠️ Folder path does not exist or was not specified. Usage: /folder --add <path>\n")
                    continue

                if is_remove:
                    folder_to_rm = None
                    for i, p in enumerate(parts):
                        if p in ["--remove", "-r", "remove", "rm", "delete"] and len(parts) > i + 1:
                            folder_to_rm = parts[i + 1]
                            break
                    if folder_to_rm:
                        abs_p = os.path.abspath(folder_to_rm)
                        removed = store.remove_folder_from_workspace(active_workspace, abs_p)
                        if removed:
                            print(f"\n🗑️ Removed folder '\033[96m{abs_p}\033[0m' from workspace '\033[93m{active_workspace}\033[0m'!\n")
                        else:
                            print(f"\n⚠️ Folder '{folder_to_rm}' not found in workspace '{active_workspace}'.\n")
                    else:
                        print("\n⚠️ Please specify a folder path to remove. Usage: /folder --remove <path>\n")
                    continue

                # Default: list folders
                folders = store.get_workspace_folders(active_workspace)
                print(f"\n📁 Local Folders for Workspace '\033[93m{active_workspace}\033[0m':")
                if not folders:
                    print("  (No local folders configured. Type '/folder --add <path>' to add one)")
                for f in folders:
                    print(f"  • \033[96m{f}\033[0m")
                print()
                continue

            elif cmd == "/drive" or cmd.startswith("/drive ") or cmd == "/drives" or cmd.startswith("/drives ") or cmd == "/cloud" or cmd.startswith("/cloud "):
                parts = parse_command_args(user_input)
                is_sync = "--sync" in parts or "-s" in parts or "sync" in parts or "resync" in parts
                is_add = "--add" in parts or "-a" in parts or "add" in parts
                is_remove = "--remove" in parts or "-r" in parts or "remove" in parts or "delete" in parts
                is_full = "--full" in parts or "--force" in parts

                from any_context.config.db_store import ConfigDBStore
                store = ConfigDBStore()

                if is_sync:
                    is_verbose = "--verbose" in parts or "-v" in parts
                    if is_verbose:
                        from any_context.ingestion.unified_sync import run_unified_sync
                        run_unified_sync(workspace_name=active_workspace, sync_folders=False, sync_web=False, sync_drives=True, force_full=is_full, verbose=True)
                    else:
                        from any_context.ingestion.local_folder_ingestor import BackgroundSyncManager
                        bg_mgr = BackgroundSyncManager()
                        bg_mgr.start_background_sync(active_workspace, sync_folders=False, sync_web=False, sync_drives=True, force_full=is_full)
                        safe_stdout_write(f"\n⚡ Background cloud drive synchronization started for workspace '\033[93m{active_workspace}\033[0m'. You can continue chatting!\n\n")
                    agent_instance = None
                    continue

                if is_add or is_remove:
                    from any_context.cli.config_menu import _manage_cloud_drives
                    _manage_cloud_drives(workspace_name=active_workspace)
                    continue

                # Default: list cloud drives
                drives = store.get_workspace_cloud_drives(active_workspace) if hasattr(store, "get_workspace_cloud_drives") else []
                print(f"\n☁️ Cloud Drives for Workspace '\033[93m{active_workspace}\033[0m':")
                if not drives:
                    print("  (No cloud drives connected. Type '/drive --add' or use /config to connect Google Drive / OneDrive)")
                for d in drives:
                    title = d.get("title") or d.get("provider", "Cloud Drive")
                    print(f"  • \033[96m{title}\033[0m ({d.get('provider')}) - Status: {d.get('auth_status', 'connected')}")
                print()
                continue

            elif cmd == "/web" or cmd.startswith("/web ") or cmd == "/urls" or cmd.startswith("/urls "):
                parts = parse_command_args(user_input)
                if len(parts) == 1:
                    from any_context.ingestion.web_scheduler import WebSchedulerStore
                    web_store = WebSchedulerStore()
                    urls = web_store.get_workspace_web_urls(active_workspace)
                    print(f"\n🌐 Web Sources for Workspace '{active_workspace}':")
                    if not urls:
                        print("  (No web URLs configured yet. Type '/web --add <url>' to add one)")
                    for u in urls:
                        pages_info = f" • {u.get('page_count')} pages" if u.get('page_count', 1) > 1 else ""
                        print(f"  • \033[96m{u.get('title') or u['url']}\033[0m{pages_info} ({u['url']}) - Interval: {u.get('polling_interval_hours', 24)}h | Last Scraped: {u.get('last_scraped_at') or 'Pending'}")
                    print()
                    continue

                is_list = "--list" in parts or "-l" in parts or "list" in parts or "urls" in parts
                is_sync = "--sync" in parts or "-s" in parts or "sync" in parts or "resync" in parts
                is_add = "--add" in parts or "-a" in parts or "add" in parts
                is_remove = "--remove" in parts or "-r" in parts or "remove" in parts or "delete" in parts or "rm" in parts
                is_full = "--full" in parts or "--force" in parts

                from any_context.ingestion.web_scheduler import WebSchedulerStore, remove_web_url_from_chromadb
                web_store = WebSchedulerStore()

                if is_sync:
                    is_verbose = "--verbose" in parts or "-v" in parts
                    if is_verbose:
                        from any_context.ingestion.unified_sync import run_unified_sync
                        run_unified_sync(workspace_name=active_workspace, sync_folders=False, sync_web=True, sync_drives=False, force_full=is_full, verbose=True)
                    else:
                        from any_context.ingestion.local_folder_ingestor import BackgroundSyncManager
                        bg_mgr = BackgroundSyncManager()
                        bg_mgr.start_background_sync(active_workspace, sync_folders=False, sync_web=True, sync_drives=False, force_full=is_full)
                        safe_stdout_write(f"\n⚡ Background web synchronization started for workspace '\033[93m{active_workspace}\033[0m'. You can continue chatting!\n\n")
                    agent_instance = None
                    continue

                if is_remove:
                    url_to_rm = None
                    for i, p in enumerate(parts):
                        if p in ["--remove", "-r", "remove", "rm", "delete"] and len(parts) > i + 1:
                            url_to_rm = parts[i + 1]
                            break
                    if url_to_rm:
                        removed = web_store.remove_web_url(active_workspace, url_to_rm)
                        remove_web_url_from_chromadb(active_workspace, url_to_rm)
                        if removed:
                            print(f"\n🗑️ Removed web source '\033[96m{url_to_rm}\033[0m' from workspace '\033[93m{active_workspace}\033[0m'!\n")
                        else:
                            print(f"\n⚠️ Web URL '{url_to_rm}' not found in workspace '{active_workspace}'.\n")
                    else:
                        print("\n⚠️ Please specify a web URL to remove. Usage: /web --remove <url>\n")
                    continue

                if is_add:
                    from any_context.cli.formatters import run_interactive_web_crawler
                    url_to_add = None
                    for i, p in enumerate(parts):
                        if p in ["--add", "-a", "add"] and len(parts) > i + 1:
                            url_to_add = parts[i + 1]
                            break
                        elif p.startswith("http://") or p.startswith("https://"):
                            url_to_add = p
                            break
                    run_interactive_web_crawler(workspace_name=active_workspace, start_url=url_to_add)
                    agent_instance = None
                    continue

                if len(parts) > 1 and (parts[1].startswith("http://") or parts[1].startswith("https://")):
                    from any_context.cli.formatters import run_interactive_web_crawler
                    run_interactive_web_crawler(workspace_name=active_workspace, start_url=parts[1])
                    agent_instance = None
                    continue

                if is_list:
                    urls = web_store.get_workspace_web_urls(active_workspace)
                    print(f"\n🌐 Web Sources for Workspace '{active_workspace}':")
                    if not urls:
                        print("  (No web URLs configured yet. Type '/web --add <url>' to add one)")
                    for u in urls:
                        pages_info = f" • {u.get('page_count')} pages" if u.get('page_count', 1) > 1 else ""
                        print(f"  • \033[96m{u.get('title') or u['url']}\033[0m{pages_info} ({u['url']}) - Interval: {u.get('polling_interval_hours', 24)}h | Last Scraped: {u.get('last_scraped_at') or 'Pending'}")
                    print()
                    continue

                from any_context.cli.config_menu import _manage_workspace_web_urls
                _manage_workspace_web_urls(workspace_name=active_workspace)
                continue

            elif cmd in ["/history", "/hist"] or cmd.startswith("/history ") or cmd.startswith("/hist ") or cmd in ["/clear-history", "/clearhistory", "/reset-history"]:
                parts = parse_command_args(user_input)
                is_clear = "--clear" in parts or "-c" in parts or cmd in ["/clear-history", "/clearhistory", "/reset-history"]

                if is_clear:
                    from any_context.cli.history import clear_workspace_history
                    cleared = clear_workspace_history(active_workspace)
                    if cleared:
                        print(f"\n🧹 Input history cleared for workspace '\033[93m{active_workspace or 'Global'}\033[0m'!\n")
                    else:
                        print(f"\n⚠️ Could not clear history for workspace '{active_workspace or 'Global'}'.\n")
                    continue

                limit = 20
                for i, p in enumerate(parts):
                    if p in ["--limit", "-n"] and len(parts) > i + 1:
                        try:
                            limit = int(parts[i + 1])
                        except ValueError:
                            pass
                        break

                from any_context.cli.history import get_workspace_history_entries
                entries = get_workspace_history_entries(active_workspace, limit=limit)
                print(f"\n📜 Recent Input History for Workspace '\033[93m{active_workspace or 'Global'}\033[0m' ({len(entries)} entries):")
                if not entries:
                    print("  (No previous inputs recorded for this workspace. Use ↑ / ↓ arrow keys as you chat)")
                else:
                    for idx, h_entry in enumerate(entries, 1):
                        print(f"  {idx:2d}. \033[96m{h_entry}\033[0m")
                print("  \033[90mTip: Press [↑] Up Arrow / [↓] Down Arrow anytime to cycle through past inputs.\033[0m\n")
                continue

            elif cmd in ["/inspect", "/chunks", "/lance", "/status-chunks", "/db-status"] or cmd.startswith("/inspect ") or cmd.startswith("/chunks "):
                parts = parse_command_args(user_input)
                limit = 5
                for i, p in enumerate(parts):
                    if p in ["--limit", "-n"] and len(parts) > i + 1:
                        try:
                            limit = int(parts[i + 1])
                        except ValueError:
                            pass
                        break

                from any_context.config.app_settings import AppSettings
                from any_context.vector_engine.store import LanceDBStore
                import chromadb

                settings = AppSettings.load()
                db_save_path = settings.context.db_path if settings else "./context_db"
                coll_name = settings.context.collection_name if settings else "context_docs"

                print(f"\n🔍 \033[1mVector Database Inspection for Workspace '\033[93m{active_workspace}\033[0m\033[1m':\033[0m")
                print(f"  • Storage Engine: \033[92m⚡ LanceDB Columnar (Apache Arrow / Rust)\033[0m")
                print(f"  • Storage Path  : \033[90m{os.path.abspath(os.path.join(db_save_path, 'lancedb'))}\033[0m")

                # Document chunks in LanceDB
                lance_store = LanceDBStore.get_instance(db_path=os.path.join(db_save_path, "lancedb"))
                lance_count = lance_store.count_records(table_name="workspace_chunks")
                ws_count = lance_store.count_records(workspace_name=active_workspace, table_name="workspace_chunks")
                print(f"  • \033[96m📁 Workspace Chunks:\033[0m \033[1m{ws_count}\033[0m (Total in DB: {lance_count})")

                # Session memory in LanceDB
                session_db_path = settings.session.db_path if settings else "./memory"
                mem_store = LanceDBStore.get_instance(db_path=os.path.join(session_db_path, "lancedb"))
                mem_count = mem_store.count_records(table_name="session_memory")
                ws_mem_count = mem_store.count_records(workspace_name=active_workspace, table_name="session_memory")
                print(f"  • \033[95m🧠 Session Memory:\033[0m \033[1m{ws_mem_count}\033[0m (Total in DB: {mem_count})\n")

                if ws_count > 0 or lance_count > 0:
                    try:
                        table = lance_store._db.open_table(lance_store._default_table_name)
                        clean_ws = active_workspace.replace("'", "''")
                        results = table.search().where(f"workspace = '{clean_ws}'").limit(limit).to_list()
                        if not results:
                            results = table.search().limit(limit).to_list()

                        print(f"📄 \033[1mSample Indexed Chunks ({len(results)} shown):\033[0m")
                        for idx, r in enumerate(results, 1):
                            src_name = r.get("file_name") or r.get("file_path") or "Unknown"
                            ctype = r.get("content_type") or "Document"
                            text_snip = (r.get("text") or "").strip().replace("\n", " ")[:140]
                            print(f"  {idx}. [\033[96m{ctype}\033[0m] \033[1m{src_name}\033[0m")
                            print(f"     \033[90mPath: {r.get('file_path')}\033[0m")
                            print(f"     \033[37mSnippet: \"{text_snip}...\"\033[0m\n")
                    except Exception as e:
                        print(f"  ⚠️ Could not read sample records from LanceDB: {e}\n")
                else:
                    print("  ⚠️ \033[93mDatabase is currently empty (0 chunks).\033[0m")
                    print("  👉 Type '\033[96m/sync\033[0m' or '\033[96m/web --add <url>\033[0m' to index content into LanceDB.\n")
                continue

            elif cmd == "/sources" or cmd.startswith("/sources "):
                parts = parse_command_args(user_input)
                store = ConfigDBStore()
                is_all = "--all" in parts or "-a" in parts or "all" in parts

                if is_all:
                    detailed = store.list_workspaces_detailed()
                    print("\n📂 All Configured Workspaces & Sources:")
                    for ws_d in detailed:
                        src_count = f" ({ws_d['total_sources']} sources)" if ws_d.get('total_sources', 0) > 0 else " (Empty)"
                        print(f"• \033[93m{ws_d['name']}\033[0m{src_count}:")
                        for s in ws_d.get("sources", []):
                            stype = s.get("type", "")
                            if stype == "folder":
                                print(f"    - [Folder] {s.get('identifier')}")
                            elif stype == "web":
                                p_cnt = s.get("details", {}).get("page_count", 1)
                                pages_badge = f" • {p_cnt} pages" if p_cnt > 1 else ""
                                print(f"    - [Web] {s.get('identifier')} ({s.get('title') or 'Web Source'}{pages_badge})")
                            elif stype == "cloud_drive":
                                auth_st = s.get("details", {}).get("auth_status", "")
                                prov = s.get("details", {}).get("provider", "drive")
                                auth_badge = f" • {auth_st}" if auth_st else ""
                                print(f"    - [Drive] {prov}://{s.get('identifier')} ({s.get('title') or 'Cloud Drive'}{auth_badge})")
                        if not ws_d.get("sources"):
                            print("    - (No sources configured)")
                    print()
                else:
                    ws_detail = store.get_workspace_sources(active_workspace)
                    src_count = f" ({ws_detail['total_sources']} sources)" if ws_detail.get('total_sources', 0) > 0 else " (Empty)"
                    print(f"\n📂 Sources for Active Workspace '\033[93m{active_workspace or 'Global'}\033[0m'{src_count}:")
                    for s in ws_detail.get("sources", []):
                        stype = s.get("type", "")
                        if stype == "folder":
                            print(f"  • [Folder] {s.get('identifier')}")
                        elif stype == "web":
                            p_cnt = s.get("details", {}).get("page_count", 1)
                            pages_badge = f" • {p_cnt} pages" if p_cnt > 1 else ""
                            print(f"  • [Web] \033[96m{s.get('title') or s.get('identifier')}\033[0m{pages_badge} ({s.get('identifier')})")
                        elif stype == "cloud_drive":
                            auth_st = s.get("details", {}).get("auth_status", "")
                            prov = s.get("details", {}).get("provider", "drive")
                            auth_badge = f" • {auth_st}" if auth_st else ""
                            print(f"  • [Drive] \033[95m{s.get('title') or s.get('identifier')}\033[0m ({prov}://{s.get('identifier')}{auth_badge})")
                    if not ws_detail.get("sources"):
                        print("  (No sources configured yet. Type '/web --add <url>' or '/config' to add folders/websites)")
                    print()
                continue

            elif cmd.startswith("/"):
                import difflib
                known_commands = [
                    "/help", "/exit", "/quit", "/q", "/version", "/v",
                    "/clear", "/cls",
                    "/switch", "/model", "/m", "/sync", "/resync", "/index", "/update", "/check-update",
                    "/folder", "/folders", "/web", "/urls", "/drive", "/drives", "/cloud",
                    "/inspect", "/chunks", "/lance",
                    "/reset-memory", "/reset", "/factory-reset", "/config",
                    "/keys", "/billing", "/plans", "/history", "/clear-history",
                    "/paste", "/multiline", "/mline", "/transfer", "/move-source",
                    "/sources", "/density", "/web-search", "/websearch", "/search", "/ws"
                ]
                typed_cmd = user_input.split()[0]
                matches = difflib.get_close_matches(typed_cmd.lower(), known_commands, n=1, cutoff=0.45)
                if matches:
                    print(f"\n\033[91m⚠️ Unknown command '\033[1m{typed_cmd}\033[0m\033[91m'.\033[0m Did you mean '\033[93m{matches[0]}\033[0m'?")
                else:
                    print(f"\n\033[91m⚠️ Unknown command '\033[1m{typed_cmd}\033[0m\033[91m'.\033[0m")
                print("👉 Type '\033[96m/help\033[0m' to view all available commands.\n")
                continue

            # Check for one-shot model prefix (e.g. '@gpt-4o summarize this file')
            effective_model = current_model
            effective_prompt = user_input

            if user_input.startswith("@") and " " in user_input:
                target_model, actual_msg = user_input[1:].split(" ", 1)
                target_model = target_model.strip()
                actual_msg = actual_msg.strip()
                if target_model and actual_msg:
                    from any_context.core.models_catalog import validate_model_key_availability
                    is_valid, prov, err_msg = validate_model_key_availability(target_model)
                    if not is_valid:
                        print(f"\n{err_msg}\n")
                        continue
                    effective_model = target_model
                    effective_prompt = actual_msg

            try:
                store = ConfigDBStore()
                current_web_search_enabled = store.get_web_search_status(workspace_name=active_workspace)
                if (
                    agent_instance is None
                    or active_workspace_for_agent != active_workspace
                    or active_model_for_agent != effective_model
                    or active_mode_for_agent != current_grounding_mode
                    or getattr(agent_instance, "_web_search_enabled", None) != current_web_search_enabled
                ):
                    search_spin = " • Web:ON" if current_web_search_enabled else ""
                    with Spinner(f"Initializing AI Agent ({effective_model} • {current_grounding_mode.capitalize()}{search_spin})..."):
                        from any_context.core.agent import create_anycontext_agent, saver
                        agent_instance = create_anycontext_agent(
                            active_workspace=active_workspace, 
                            checkpointer=saver,
                            model_override=effective_model,
                            grounding_mode=current_grounding_mode,
                            web_search_enabled=current_web_search_enabled
                        )
                        setattr(agent_instance, "_web_search_enabled", current_web_search_enabled)
                        active_workspace_for_agent = active_workspace
                        active_model_for_agent = effective_model
                        active_mode_for_agent = current_grounding_mode

                from any_context.cli.viewport import PinnedBottomDock

                with PinnedBottomDock(
                    workspace_name=active_workspace,
                    model_name=effective_model,
                    grounding_mode=current_grounding_mode,
                    web_search_enabled=current_web_search_enabled
                ) as dock:
                    has_printed_ai_header = False

                    for token, metadata in agent_instance.stream(
                        {
                            "messages": [effective_prompt]
                        },
                        stream_mode="messages",
                        config=config
                    ):
                        if hasattr(token, "type") and token.type in ["ai", "AIMessageChunk", "AIMessage"]:
                            content_str = ""
                            if isinstance(token.content, str) and token.content:
                                content_str = token.content
                            elif isinstance(token.content, list):
                                parts = []
                                for part in token.content:
                                    if isinstance(part, str):
                                        parts.append(part)
                                    elif isinstance(part, dict) and "text" in part:
                                        parts.append(part["text"])
                                content_str = "".join(parts)

                            if content_str:
                                if not has_printed_ai_header:
                                    dock.write(f"\r\033[K\033[93m🤖 AI [\033[95m{effective_model}\033[93m]:\033[0m ")
                                    has_printed_ai_header = True
                                dock.write(content_str)

                        elif hasattr(token, "type") and token.type in ["tool", "ToolMessage", "ToolMessageChunk"]:
                            tool_name = str(getattr(token, "name", "") or "")
                            if "web" in tool_name.lower():
                                from any_context.tools.web_search_tool import get_active_web_search_engine
                                eng = get_active_web_search_engine()
                                status_msg = f"🌐 [{eng}] Pesquisando na internet em tempo real..."
                            else:
                                status_msg = "📚 [RAG] Reading retrieved context documents for AI analysis..."

                            if not has_printed_ai_header:
                                dock.write(f"\r\033[K{status_msg}")
                            dock.update_status(status_msg)

                    if not has_printed_ai_header:
                        dock.write(f"\r\033[K\033[93m🤖 AI [\033[95m{effective_model}\033[93m]:\033[0m ")
                    dock.write("\n")

            except KeyboardInterrupt:
                print("\n\n⏹️ Generation interrupted by user.\n")
                continue
            except Exception as e:
                from any_context.core.models_catalog import format_inference_error
                err_info = format_inference_error(e, effective_model)
                print(err_info["formatted_box"])

                # Invalidate agent instance so next prompt re-initializes cleanly
                agent_instance = None
                active_model_for_agent = None

                # If the failed model was set as current_model, offer automatic fallback
                if current_model == effective_model and current_model != "gpt-4o-mini":
                    current_model = "gpt-4o-mini"
                    print(f"🔄 Automatically reverted active session model to \033[95m{current_model}\033[0m.\n")

        except (KeyboardInterrupt, EOFError, StopIteration):
            safe_stdout_write("\n")
            try:
                safe_stdout_write("\033[93m❓ Are you sure you want to exit AnyContext? [y/N]:\033[0m ")
                confirm_ans = input().strip().lower()
                should_exit = confirm_ans in ["y", "yes", "s", "sim"]
            except (KeyboardInterrupt, EOFError, StopIteration):
                should_exit = True

            if should_exit:
                safe_stdout_write("\n👋 Saving session memory and exiting AnyContext. See you soon!\n\n")
                try:
                    from any_context.memory import run_session_summarizer_async
                    run_session_summarizer_async(thread_id, active_workspace)
                except Exception:
                    pass
                break
            else:
                safe_stdout_write("↩️ Resuming session...\n\n")
                continue
        except Exception as e:
            safe_stdout_write(format_session_error(e))
            continue


def launch_opentui(workspace: str = "Default") -> bool:
    """Attempts to launch the OpenTUI frontend using bun if installed."""
    import shutil
    import subprocess
    bun_bin = shutil.which("bun")
    if not bun_bin:
        home_bun = os.path.expanduser("~/.bun/bin/bun.exe")
        if os.path.exists(home_bun):
            bun_bin = home_bun

    if not bun_bin:
        return False

    tui_index = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "tui", "index.tsx"))
    if not os.path.exists(tui_index):
        return False

    try:
        env = dict(os.environ)
        # Clean PyInstaller bootloader internal variables to prevent child process security validation failure
        env.pop("_MEIPASS2", None)
        env.pop("_MEIPASS", None)
        env.pop("PYI_PARENT_PID", None)
        env["ACTX_EXECUTABLE"] = sys.executable or "actx"
        res = subprocess.run([bun_bin, "run", tui_index, workspace], cwd=os.path.dirname(tui_index), env=env)
        return res.returncode == 0
    except Exception:
        return False


def main_cli():
    try:
        workspace = get_active_workspace()
        print_startup_update_notice()

        # Check for --tui flag
        if "--tui" in sys.argv:
            if launch_opentui(workspace):
                return

        # Check for direct prompt passed via arguments (e.g. actx "my question" or actx -p "my question")
        non_flag_args = [a for a in sys.argv[1:] if not a.startswith("-") and a.lower() not in ["serve", "api", "mcp"]]
        p_flag_idx = -1
        for flag in ["-p", "--prompt", "-q", "--query"]:
            if flag in sys.argv:
                p_flag_idx = sys.argv.index(flag)
                break

        direct_prompt = None
        if p_flag_idx != -1 and p_flag_idx + 1 < len(sys.argv):
            direct_prompt = sys.argv[p_flag_idx + 1].strip()
        elif non_flag_args and not any(arg.lower() in ["--help", "-h", "--version", "-v", "--update", "-u", "--mcp", "--server", "serve", "api"] for arg in sys.argv[1:]):
            direct_prompt = " ".join(non_flag_args).strip()

        # If direct prompt provided, execute one-shot query cleanly and exit
        if direct_prompt:
            from any_context.core.agent import create_anycontext_agent
            from any_context.config.db_store import ConfigDBStore
            store = ConfigDBStore()
            settings = store.get_app_settings()
            model = settings.models.inference_model if settings and settings.models else "gpt-4o-mini"
            mode = store.get_grounding_mode(workspace_name=workspace) or "strict"
            search_enabled = store.get_web_search_status(workspace_name=workspace) or False

            agent = create_anycontext_agent(
                model_name=model,
                workspace_name=workspace,
                grounding_mode=mode,
                web_search_enabled=search_enabled
            )
            config = {
                "configurable": {
                    "thread_id": f"batch_{workspace}",
                    "active_workspace": workspace,
                    "grounding_mode": mode,
                    "web_search_enabled": search_enabled
                }
            }
            for token, meta in agent.stream({"messages": [direct_prompt]}, stream_mode="messages", config=config):
                if hasattr(token, "type") and token.type in ["ai", "AIMessageChunk", "AIMessage"] and token.content:
                    if isinstance(token.content, str):
                        safe_stdout_write(token.content)
            safe_stdout_write("\n")
            return

        run_chat_loop(active_workspace=workspace)
    except (KeyboardInterrupt, EOFError):
        print("\n\n👋 AnyContext closed.\n")
        sys.exit(0)


def main():
    if "--mcp" not in sys.argv:
        print_banner()
    main_cli()


if __name__ == "__main__":
    main()


