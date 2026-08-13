import uuid
import questionary
from any_context.core.agent import create_anycontext_agent, saver

from any_context.ingestion.local_folder_ingestor import index_folder
from any_context.cli.workspace_selector import show_workspace_menu, get_active_workspace
from any_context.cli.config_menu import show_config_menu
from any_context.cli.banner import print_banner
from any_context.cli.updater import print_startup_update_notice, check_for_updates, run_self_update
from any_context.memory import MemoryManager
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

    print("\n🔄 Synchronizing file database...")
    index_folder.invoke({"workspace_name": active_workspace})
    
    print("\n=======================================================")
    print("💬 Chat started! Press Ctrl+C to exit.")
    print("=======================================================\n")
    
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
                    print("\n🔄 Re-synchronizing file database for new workspace...")
                    index_folder.invoke({"workspace_name": active_workspace})
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
                    import sys
                    store = ConfigDBStore()
                    store.factory_reset()
                    print("\n🎉 AnyContext has been completely reset to factory defaults!")
                    print("Run 'actx' again anytime to launch the first-time setup wizard.\n")
                    sys.exit(0)
                continue
            elif cmd == "/config":
                show_config_menu()
                continue
    
            active_agent = create_anycontext_agent(active_workspace=active_workspace, checkpointer=saver)
            print("\033[93m🤖 AI:\033[0m ", end="", flush=True)
    
            for token, metadata in active_agent.stream(
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
    print_banner()
    print_startup_update_notice()
    workspace = get_active_workspace()
    run_chat_loop(active_workspace=workspace)

if __name__ == "__main__":
    main()
