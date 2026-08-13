import uuid
import questionary
from any_context.core.agent import cli_agent
from any_context.ingestion.local_folder_ingestor import index_folder
from any_context.cli.workspace_selector import show_workspace_menu, get_active_workspace
from any_context.cli.config_menu import show_config_menu
from any_context.cli.banner import print_banner
from any_context.cli.updater import print_startup_update_notice, check_for_updates, run_self_update
from any_context.memory import MemoryManager
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

            if cmd == "/help":
                help_text = f"""
\033[93m🤖 AnyContext Agent Help (v{__version__})\033[0m

\033[1mUSAGE:\033[0m
  Just type your question to chat with the AI. The agent will automatically
  search the vector database for documents in your current active workspace
  and will also remember previous messages from this session.

\033[1mCOMMANDS:\033[0m
  \033[96m/switch\033[0m       Change the active workspace. Opens an interactive menu to select
                a workspace and resynchronizes the vector database instantly.

  \033[96m/version\033[0m      Display AnyContext version information. (Alias: \033[96m/v\033[0m)

  \033[96m/update\033[0m       Check for and install the latest AnyContext release automatically.

  \033[96m/check-update\033[0m Check if a newer version of AnyContext is available.

  \033[96m/reset-memory\033[0m Reset all long-term memories saved for the current workspace.
                (Alias: \033[96m/reset\033[0m)

  \033[96m/config\033[0m       Open the interactive configuration menu to manage workspaces,
                AI models, base URLs, and memory limits.

  \033[96m/help\033[0m         Show this detailed help message.

\033[1mSERVER MODES & ENTERPRISE VPC DEPLOYMENT:\033[0m
  • \033[96mactx --serve\033[0m           Start REST API Server on localhost (default port: 8000).
  • \033[96mactx --serve --host 0.0.0.0\033[0m Launch in VPC Enterprise Mode listening on all
                           internal network interfaces for company-wide APIs.
                           Interactive Swagger docs: http://127.0.0.1:8000/docs.
  • \033[96mactx --mcp\033[0m             Start Model Context Protocol (MCP) Server for native
                           integration with Claude Desktop, Cursor, and AI sidecars.

\033[1mTIPS:\033[0m
  • \033[90mSyncing:\033[0m If you add new files to the workspace folder, type `/switch`
    and select the current workspace again to force a fast resync!
  • \033[90mUpdates:\033[0m Run `actx --update` from your terminal to update anytime.
  • \033[90mExiting:\033[0m Press \033[91mCtrl+C\033[0m to exit and trigger long-term memory summary.
"""
                print(help_text)
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
            elif cmd == "/config":
                show_config_menu()
                continue
    
            print("\033[93m🤖 AI:\033[0m ", end="", flush=True)
    
            for token, metadata in cli_agent.stream(
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
