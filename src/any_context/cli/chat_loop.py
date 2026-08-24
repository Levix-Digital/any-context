import os
import sys
import uuid
from typing import Optional, List, Dict, Any
import questionary
from any_context.cli.workspace_selector import show_workspace_menu, get_active_workspace
from any_context.cli.config_menu import show_config_menu, mask_key
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
                        active_tag = " \0