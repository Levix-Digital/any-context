import sys
import os
import questionary
from any_context.config.db_store import ConfigDBStore
from any_context.memory import MemoryManager

def run_first_time_wizard():
    """
    Interactive first-time onboarding wizard if no settings/workspaces exist.
    """
    print("\n=======================================================")
    print("🎉 Welcome to AnyContext (actx) Initial Setup!")
    print("No workspaces were found in your configuration database.")
    print("=======================================================\n")

    ws_name = questionary.text(
        "Enter a name for your first workspace (e.g. MyProject):",
        default="MyWorkspace"
    ).ask()

    if not ws_name:
        ws_name = "MyWorkspace"

    folder_path = questionary.text(
        "Enter the absolute folder path containing your documents:",
        default=os.getcwd()
    ).ask()

    if not folder_path or not os.path.exists(folder_path):
        print(f"⚠️ Warning: Directory '{folder_path}' does not exist right now, but saving configuration.")

    store = ConfigDBStore()
    store.add_workspace(name=ws_name, paths=[folder_path])
    print(f"\n✅ Workspace '{ws_name}' created successfully with path: {folder_path}\n")

def show_config_menu():
    """
    Interactive CLI Configuration Management Menu (/config or --config)
    """
    store = ConfigDBStore()
    
    while True:
        choice = questionary.select(
            "⚙️ AnyContext Configuration Menu:",
            choices=[
                "📂 Workspaces Management (List / Add / Remove)",
                "🤖 AI Models & Provider Settings",
                "🧠 Memory Compression & Reset Settings",
                "🔙 Return / Exit Menu"
            ]
        ).ask()

        if not choice or choice.startswith("🔙"):
            break

        if choice.startswith("📂"):
            _manage_workspaces(store)
        elif choice.startswith("🤖"):
            _manage_models(store)
        elif choice.startswith("🧠"):
            _manage_memory(store)

def _manage_workspaces(store: ConfigDBStore):
    settings = store.get_app_settings()
    workspaces = settings.workspaces if settings else []
    
    ws_action = questionary.select(
        "📂 Workspaces Action:",
        choices=[
            "📋 List Workspaces",
            "➕ Add New Workspace",
            "🗑️ Remove Workspace",
            "🔙 Back"
        ]
    ).ask()

    if not ws_action or ws_action.startswith("🔙"):
        return

    if ws_action.startswith("📋"):
        print("\n--- Configured Workspaces ---")
        for ws in workspaces:
            print(f"• \033[93m{ws.name}\033[0m: {', '.join(ws.paths)}")
        print("-----------------------------\n")
    elif ws_action.startswith("➕"):
        name = questionary.text("Workspace Name:").ask()
        if name:
            path = questionary.text("Folder Path:").ask()
            if path:
                store.add_workspace(name.strip(), [path.strip()])
                print(f"✅ Added workspace '{name}'.")
    elif ws_action.startswith("🗑️"):
        names = [ws.name for ws in workspaces]
        if not names:
            print("No workspaces to remove.")
            return
        to_remove = questionary.select("Select workspace to remove:", choices=names).ask()
        if to_remove:
            store.remove_workspace(to_remove)
            print(f"🗑️ Removed workspace '{to_remove}'.")

def _manage_models(store: ConfigDBStore):
    settings = store.get_app_settings()
    m = settings.models if settings else None

    print(f"\n--- Current Model Settings ---")
    print(f"• Inference Model: {m.inference_model if m else 'gpt-4o-mini'}")
    print(f"• Summary Model  : {m.summary_model if m else 'google/gemma-4-e2b'}")
    print(f"• Provider       : {m.model_provider if m else 'openai'}")
    print(f"• Local Base URL : {m.local_base_url if m else 'http://localhost:1234/v1'}")
    print("------------------------------\n")

    update = questionary.confirm("Do you want to update model settings?").ask()
    if update:
        inf = questionary.text("Inference Model:", default=m.inference_model if m else "gpt-4o-mini").ask()
        sum_m = questionary.text("Summary Model:", default=m.summary_model if m else "google/gemma-4-e2b").ask()
        prov = questionary.text("Provider (openai/local):", default=m.model_provider if m else "openai").ask()
        url = questionary.text("Local Base URL:", default=m.local_base_url if m else "http://localhost:1234/v1").ask()

        if m:
            m.inference_model = inf
            m.summary_model = sum_m
            m.model_provider = prov
            m.local_base_url = url
            settings.models = m
            store.save_app_settings(settings)
            print("✅ Model settings updated successfully!")

def _manage_memory(store: ConfigDBStore):
    settings = store.get_app_settings()
    mem = settings.memory if settings else None
    workspaces = [ws.name for ws in settings.workspaces] if settings else []

    print(f"\n--- Memory Compression Settings ---")
    print(f"• Short-Term Buffer Size : {mem.short_term_buffer_size if mem else 20} messages")
    print(f"• Active Rolling Window   : {mem.rolling_window_messages if mem else 10} messages")
    print(f"• Meta-Summary Threshold : {mem.meta_summary_threshold if mem else 30} summaries")
    print("------------------------------------\n")

    action = questionary.select(
        "🧠 Memory Action:",
        choices=[
            "⚙️ View / Info",
            "🧹 Reset Long-Term Memory (Specific Workspace)",
            "🔥 Reset ALL Long-Term Memory (Global)",
            "🔙 Back"
        ]
    ).ask()

    if not action or action.startswith("🔙") or action.startswith("⚙️"):
        return

    memory_mgr = MemoryManager(settings=settings)

    if action.startswith("🧹"):
        if not workspaces:
            print("No workspaces found.")
            return
        ws_choice = questionary.select("Select workspace memory to reset:", choices=workspaces).ask()
        if ws_choice:
            confirm = questionary.confirm(f"⚠️ Are you sure you want to delete all long-term memories for workspace '{ws_choice}'?").ask()
            if confirm:
                deleted = memory_mgr.reset_memory(workspace=ws_choice)
                print(f"🧹 Reset complete! Removed {deleted} memory entries for workspace '{ws_choice}'.")

    elif action.startswith("🔥"):
        confirm = questionary.confirm("⚠️ DANGER: Are you sure you want to reset ALL long-term memories across all workspaces?").ask()
        if confirm:
            deleted = memory_mgr.reset_memory(workspace=None)
            print(f"🔥 Global memory reset complete! Removed {deleted} total memory entries.")
