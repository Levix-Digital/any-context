import sys
import uuid
import questionary
from any_context.cli.workspace_selector import show_workspace_menu, get_active_workspace
from any_context.cli.config_menu import show_config_menu
from any_context.cli.banner import print_banner
from any_context.cli.updater import print_startup_update_notice, check_for_updates, run_self_update
from any_context.cli.spinner import Spinner
from any_context.help import handle_command_help_interception
from any_context import __version__


def run_chat_loop(active_workspace: str = None):
    thread_id = f"chat_{uuid.uuid4()}"
    config = {
        "configurable": {
            "thread_id": thread_id,
            "active_workspace": active_workspace
        }
    }

    with Spinner(f"Synchronizing workspace file database for '{active_workspace}'...", done_message=f"Workspace '{active_workspace}' synchronized"):
        from any_context.ingestion.local_folder_ingestor import index_folder
        index_folder.invoke({"workspace_name": active_workspace})

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
            user_input = input(f"\n\033[96m👤 {prompt_str}:\033[0m ")
            cmd = user_input.strip().lower()
            if not cmd:
                continue

            # Intercept command help flags and /help commands
            if handle_command_help_interception(user_input):
                continue

            elif cmd in ["/exit", "/quit", "/q", "exit", "quit"]:
                try:
                    confirm = questionary.confirm(
                        "❓ Are you sure you want to exit AnyContext?",
                        default=True
                    ).ask()
                except Exception:
                    confirm = True

                if confirm:
                    print("\n👋 Saving session memory and exiting AnyContext. See you soon!\n")
                    from any_context.memory import run_session_summarizer_async
                    run_session_summarizer_async(thread_id, active_workspace)
                    break
                else:
                    print("↩️ Resuming session...\n")
                    continue

            elif cmd in ["/version", "/v"]:
                print(f"\033[93m🤖 AnyContext (actx) v{__version__}\033[0m - Levix Digital")
                continue
            elif cmd == "/switch":
                new_workspace = show_workspace_menu()
                if new_workspace:
                    active_workspace = new_workspace
                    config["configurable"]["active_workspace"] = active_workspace
                    with Spinner(f"Re-synchronizing file database for '{active_workspace}'...", done_message=f"Workspace '{active_workspace}' synchronized"):
                        from any_context.ingestion.local_folder_ingestor import index_folder
                        index_folder.invoke({"workspace_name": active_workspace})
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
            elif cmd.startswith("/web add "):
                from any_context.ingestion.web_scheduler import index_web_url_to_chromadb
                url_to_add = user_input.strip()[9:].strip()
                if url_to_add:
                    with Spinner(f"Scraping and indexing '{url_to_add}' into '{active_workspace}'..."):
                        res = index_web_url_to_chromadb(workspace_name=active_workspace, url=url_to_add, force=True)
                    if res.get("status") == "success":
                        print(f"✅ {res.get('message')}\n")
                    elif res.get("status") == "unchanged":
                        print(f"ℹ️ {res.get('message')}\n")
                    else:
                        print(f"❌ {res.get('message')}\n")
                continue
            elif cmd in ["/web list", "/web urls"]:
                from any_context.ingestion.web_scheduler import WebSchedulerStore
                web_store = WebSchedulerStore()
                urls = web_store.get_workspace_web_urls(active_workspace)
                print(f"\n🌐 Web Sources for Workspace '{active_workspace}':")
                if not urls:
                    print("  (No web URLs configured yet. Type '/web' to add one)")
                for u in urls:
                    print(f"  • \033[96m{u.get('title') or u['url']}\033[0m ({u['url']}) - Interval: {u.get('polling_interval_hours', 24)}h | Last Scraped: {u.get('last_scraped_at') or 'Pending'}")
                print()
                continue
            elif cmd in ["/web sync", "/web resync"]:
                from any_context.ingestion.web_scheduler import sync_workspace_web_urls
                with Spinner(f"Re-scraping and synchronizing all web URLs for workspace '{active_workspace}'..."):
                    sync_res = sync_workspace_web_urls(active_workspace)
                print(f"✅ Synced {sync_res.get('total_urls', 0)} web URLs successfully!\n")
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

                print(f"\033[93m🤖 AI [\033[95m{effective_model}\033[93m]:\033[0m ", end="", flush=True)

                for token, metadata in agent_instance.stream(
                    {
                        "messages": [effective_prompt]
                    },
                    stream_mode="messages",
                    config=config
                ):
                    if hasattr(token, "type") and token.type in ["ai", "AIMessageChunk", "AIMessage"]:
                        if isinstance(token.content, str) and token.content:
                            print(token.content, end="", flush=True)
                    elif hasattr(token, "type") and token.type in ["tool", "ToolMessage", "ToolMessageChunk"]:
                        print("\n📚 Reading retrieved documents... Please wait for AI analysis.")
                        print(f"\033[93m🤖 AI [\033[95m{effective_model}\033[93m]:\033[0m ", end="", flush=True)

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

        except KeyboardInterrupt:
            print()
            try:
                confirm = questionary.confirm(
                    "❓ Are you sure you want to exit AnyContext?",
                    default=False
                ).ask()
            except (KeyboardInterrupt, Exception):
                confirm = True

            if confirm:
                print("\n👋 Saving session memory and exiting AnyContext. See you soon!\n")
                from any_context.memory import run_session_summarizer_async
                run_session_summarizer_async(thread_id, active_workspace)
                break
            else:
                print("↩️ Resuming session...\n")
                continue


def main_cli():
    print_startup_update_notice()
    workspace = get_active_workspace()
    run_chat_loop(active_workspace=workspace)


def main():
    if "--mcp" not in sys.argv:
        print_banner()
    main_cli()


if __name__ == "__main__":
    main()


