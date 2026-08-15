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

    print("\n=======================================================")
    print("💬 Chat started! Press Ctrl+C to exit.")
    print("=======================================================\n")

    agent_instance = None
    active_workspace_for_agent = None

    while True:
        try:
            prompt_name = f"You [\033[93m{active_workspace}\033[96m]" if active_workspace else "You"
            user_input = input(f"\n\033[96m👤 {prompt_name}:\033[0m ")
            cmd = user_input.strip().lower()
            if not cmd:
                continue

            # Intercept command help flags and /help commands
            if handle_command_help_interception(user_input):
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
            elif cmd == "/update":
                run_self_update()
                continue
            elif cmd == "/check-update":
                check_for_updates(quiet_if_latest=False)
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

            if agent_instance is None or active_workspace_for_agent != active_workspace:
                with Spinner("Initializing AI Agent & Tools..."):
                    from any_context.core.agent import create_anycontext_agent, saver
                    agent_instance = create_anycontext_agent(active_workspace=active_workspace, checkpointer=saver)
                    active_workspace_for_agent = active_workspace

            print("\033[93m🤖 AI:\033[0m ", end="", flush=True)

            for token, metadata in agent_instance.stream(
                {
                    "messages": [user_input]
                },
                stream_mode="messages",
                config=config
            ):
                if hasattr(token, "type") and token.type in ["ai", "AIMessageChunk", "AIMessage"]:
                    if isinstance(token.content, str) and token.content:
                        print(token.content, end="", flush=True)
                elif hasattr(token, "type") and token.type in ["tool", "ToolMessage", "ToolMessageChunk"]:
                    print("\n📚 Reading retrieved documents... Please wait for AI analysis.")
                    print("\033[93m🤖 AI:\033[0m ", end="", flush=True)

            print()

        except KeyboardInterrupt:
            print("\nExiting...")
            from any_context.memory import run_session_summarizer_async
            run_session_summarizer_async(thread_id, active_workspace)
            break


def main():
    if "--mcp" not in sys.argv:
        print_banner()
        print_startup_update_notice()
    workspace = get_active_workspace()
    run_chat_loop(active_workspace=workspace)


if __name__ == "__main__":
    main()

