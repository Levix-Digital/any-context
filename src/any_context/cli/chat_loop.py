import sys
import uuid
from typing import Optional
import questionary
from any_context.cli.workspace_selector import show_workspace_menu, get_active_workspace
from any_context.cli.config_menu import show_config_menu
from any_context.cli.banner import print_banner
from any_context.cli.updater import print_startup_update_notice, check_for_updates, run_self_update
from any_context.cli.spinner import Spinner
from any_context.help import handle_command_help_interception
from any_context import __version__


def safe_prompt_input(prompt_text: str) -> Optional[str]:
    """
    Safely reads input from terminal with complete immunity to Windows signal corruption & EOF.
    Returns:
      - str: user input (or '/exit' if user confirmed exit)
      - None: if user cancelled exit with 'No'
    """
    try:
        return input(prompt_text)
    except (KeyboardInterrupt, EOFError):
        print()
        try:
            confirm_ans = input("\033[93m❓ Are you sure you want to exit AnyContext? [y/N]:\033[0m ").strip().lower()
            if confirm_ans in ["y", "yes", "s", "sim"]:
                return "/exit"
            print("↩️ Resuming session...\n")
            return None
        except (KeyboardInterrupt, EOFError):
            return "/exit"


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


def run_chat_loop(active_workspace: str = None):
    thread_id = f"chat_{uuid.uuid4()}"
    config = {
        "configurable": {
            "thread_id": thread_id,
            "active_workspace": active_workspace
        },
        "recursion_limit": 50
    }

    with Spinner(f"Synchronizing workspace '{active_workspace}'...", done_message=f"Workspace '{active_workspace}' ready"):
        from any_context.ingestion.local_folder_ingestor import run_index_folder
        run_index_folder(workspace_name=active_workspace, verbose=False)

    from any_context.config.app_settings import AppSettings
    settings = AppSettings.load()
    current_model = settings.models.inference_model if (settings and settings.models and settings.models.inference_model) else "gpt-4o-mini"

    print("\n=======================================================")
    print("💬 Chat started! Type '/exit' or press Ctrl+C to quit.")
    print("=======================================================\n")

    agent_instance = None
    active_workspace_for_agent = None
    active_model_for_agent = None

    while True:
        try:
            prompt_ws = f"\033[93m{active_workspace}\033[96m" if active_workspace else "Global"
            prompt_str = f"You [{prompt_ws} | \033[95m{current_model}\033[96m]"
            raw_input = safe_prompt_input(f"\n\033[96m👤 {prompt_str}:\033[0m ")
            if raw_input is None:
                continue

            user_input = raw_input.strip()
            cmd = user_input.lower()
            if not cmd:
                continue

            # Intercept command help flags and /help commands
            if handle_command_help_interception(user_input):
                continue

            elif cmd in ["/exit", "/quit", "/q", "exit", "quit"]:
                print("\n👋 Saving session memory and exiting AnyContext. See you soon!\n")
                try:
                    from any_context.memory import run_session_summarizer_async
                    run_session_summarizer_async(thread_id, active_workspace)
                except Exception:
                    pass
                break

            elif cmd in ["/version", "/v"]:
                print(f"\033[93m🤖 AnyContext (actx) v{__version__}\033[0m - Levix Digital")
                continue
            elif cmd == "/switch":
                new_workspace = show_workspace_menu()
                if new_workspace:
                    active_workspace = new_workspace
                    config["configurable"]["active_workspace"] = active_workspace
                    with Spinner(f"Synchronizing workspace '{active_workspace}'...", done_message=f"Workspace '{active_workspace}' ready"):
                        from any_context.ingestion.local_folder_ingestor import run_index_folder
                        run_index_folder(workspace_name=active_workspace, verbose=False)
                    agent_instance = None
                continue
            elif cmd in ["/sync", "/resync", "/index"] or cmd.startswith("/sync ") or cmd.startswith("/index "):
                is_verbose = "--verbose" in user_input or "-v" in user_input
                from any_context.ingestion.local_folder_ingestor import run_index_folder
                if is_verbose:
                    run_index_folder(workspace_name=active_workspace, verbose=True)
                else:
                    with Spinner(f"Synchronizing workspace '{active_workspace}'...", done_message=f"Workspace '{active_workspace}' ready"):
                        run_index_folder(workspace_name=active_workspace, verbose=False)
                agent_instance = None
                continue
            elif cmd == "/model" or cmd == "/m" or cmd.startswith("/model ") or cmd.startswith("/m "):
                from any_context.core.models_catalog import get_available_models, validate_model_key_availability

                parts = user_input.strip().split(maxsplit=1)
                if len(parts) > 1:
                    new_model = parts[1].strip()
                    is_valid, prov, err_msg = validate_model_key_availability(new_model)
                    if not is_valid:
                        print(f"\n{err_msg}\n")
                        continue
                    current_model = new_model
                    agent_instance = None
                    print(f"\n🔄 Switched active inference model to \033[95m{current_model}\033[0m ({prov.upper()}) for this session.\n")
                    continue

                # Interactive selection menu (strictly key-aware)
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
            elif cmd == "/update":
                run_self_update()
                continue
            elif cmd in ["/check-update", "/checkupdate", "/check"]:
                has_up, new_tag = check_for_updates(quiet_if_latest=False)
                if has_up:
                    try:
                        do_upgrade = questionary.confirm(
                            f"Would you like to download and install {new_tag} now?",
                            default=True
                        ).ask()
                        if do_upgrade:
                            run_self_update()
                    except Exception:
                        pass
                continue
            elif cmd in ["/reset-memory", "/reset"]:
                confirm = questionary.confirm(
                    f"⚠️ Are you sure you want to reset long-term memory for workspace '{active_workspace}'?"
                ).ask()
                if confirm:
                    from any_context.memory import MemoryManager
                    memory_mgr = MemoryManager()
                    deleted = memory_mgr.reset_memory(workspace=active_workspace)
                    print(f"🧹 Reset complete! Deleted {deleted} long-term memory entries for workspace '{active_workspace}'.")
                continue
            elif cmd in ["/factory-reset", "/reset-factory"]:
                confirm = questionary.confirm(
                    "⚠️ DANGER: Are you sure you want to reset AnyContext to Factory Defaults?\n  This will erase ALL workspaces, folders, API keys, configuration settings, and vector memory databases!"
                ).ask()
                if confirm:
                    from any_context.config.db_store import ConfigDBStore
                    store = ConfigDBStore()
                    store.factory_reset()
                    print("\n🎉 AnyContext has been completely reset to factory defaults!")
                    print("Run 'actx' again anytime to launch the first-time setup wizard.\n")
                    sys.exit(0)
                continue
            elif cmd == "/config":
                show_config_menu()
                continue
            elif cmd in ["/keys", "/api-keys", "/apikeys"]:
                from any_context.help.manager import display_help_page
                from any_context.help.registry import get_help_page
                page = get_help_page("api-keys")
                if page:
                    display_help_page(page)
                continue
            elif cmd in ["/billing", "/plans"]:
                from any_context.cli.config_menu import _manage_subscription
                _manage_subscription()
                continue
            elif cmd == "/web":
                from any_context.cli.config_menu import _manage_workspace_web_urls
                _manage_workspace_web_urls(workspace_name=active_workspace)
                continue
            elif cmd.startswith("/web add ") or cmd == "/web add":
                from any_context.ingestion.web_crawler import run_interactive_web_crawler
                url_to_add = user_input.strip()[9:].strip() if len(user_input.strip()) > 9 else None
                run_interactive_web_crawler(workspace_name=active_workspace, start_url=url_to_add)
                continue
            elif cmd in ["/web list", "/web urls"]:
                from any_context.ingestion.web_scheduler import WebSchedulerStore
                web_store = WebSchedulerStore()
                urls = web_store.get_workspace_web_urls(active_workspace)
                print(f"\n🌐 Web Sources for Workspace '{active_workspace}':")
                if not urls:
                    print("  (No web URLs configured yet. Type '/web' to add one)")
                for u in urls:
                    pages_info = f" • {u.get('page_count')} pages" if u.get('page_count', 1) > 1 else ""
                    print(f"  • \033[96m{u.get('title') or u['url']}\033[0m{pages_info} ({u['url']}) - Interval: {u.get('polling_interval_hours', 24)}h | Last Scraped: {u.get('last_scraped_at') or 'Pending'}")
                print()
                continue
            elif cmd in ["/web sync", "/web resync"]:
                from any_context.ingestion.web_scheduler import sync_workspace_web_urls
                with Spinner(f"Re-scraping and synchronizing all web URLs for workspace '{active_workspace}'..."):
                    sync_res = sync_workspace_web_urls(active_workspace)
                print(f"✅ Synced {sync_res.get('total_urls', 0)} web URLs successfully!\n")
                continue

            elif cmd.startswith("/"):
                import difflib
                known_commands = [
                    "/help", "/exit", "/quit", "/q", "/version", "/v",
                    "/switch", "/model", "/m", "/sync", "/index", "/update", "/check-update",
                    "/reset-memory", "/reset", "/factory-reset", "/config",
                    "/keys", "/billing", "/plans", "/web"
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
                if agent_instance is None or active_workspace_for_agent != active_workspace or active_model_for_agent != effective_model:
                    with Spinner(f"Initializing AI Agent ({effective_model})..."):
                        from any_context.core.agent import create_anycontext_agent, saver
                        agent_instance = create_anycontext_agent(
                            active_workspace=active_workspace, 
                            checkpointer=saver,
                            model_override=effective_model
                        )
                        active_workspace_for_agent = active_workspace
                        active_model_for_agent = effective_model

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
                                safe_stdout_write(f"\r\033[K\033[93m🤖 AI [\033[95m{effective_model}\033[93m]:\033[0m ")
                                has_printed_ai_header = True
                            safe_stdout_write(content_str)

                    elif hasattr(token, "type") and token.type in ["tool", "ToolMessage", "ToolMessageChunk"]:
                        if not has_printed_ai_header:
                            safe_stdout_write(f"\r\033[K📚 [RAG] Reading retrieved context documents for AI analysis...")

                if not has_printed_ai_header:
                    safe_stdout_write(f"\r\033[K\033[93m🤖 AI [\033[95m{effective_model}\033[93m]:\033[0m ")
                print()

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

        except (KeyboardInterrupt, EOFError):
            print()
            try:
                confirm_ans = input("\033[93m❓ Are you sure you want to exit AnyContext? [y/N]:\033[0m ").strip().lower()
                should_exit = confirm_ans in ["y", "yes", "s", "sim"]
            except (KeyboardInterrupt, EOFError):
                should_exit = True

            if should_exit:
                print("\n👋 Saving session memory and exiting AnyContext. See you soon!\n")
                try:
                    from any_context.memory import run_session_summarizer_async
                    run_session_summarizer_async(thread_id, active_workspace)
                except Exception:
                    pass
                break
            else:
                print("↩️ Resuming session...\n")
                continue


def main_cli():
    try:
        workspace = get_active_workspace()
        print_startup_update_notice()
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


