import sys
import os
import questionary
from any_context.config.db_store import ConfigDBStore, safe_print
from any_context.memory import MemoryManager
from any_context.ingestion.local_folder_ingestor import clear_context_vector_db

def mask_key(key: str) -> str:
    if not key or len(key) < 8:
        return "*****"
    return f"{key[:5]}...{key[-4:]}"

def run_first_time_wizard():
    """
    Interactive first-time onboarding wizard if no settings/workspaces exist.
    """
    print("\n=======================================================")
    print("🎉 Welcome to AnyContext (actx) Initial Setup!")
    print("No workspaces were found in your configuration database.")
    print("=======================================================\n")

    ws_name = questionary.text(
        "1. Enter a name for your first workspace (e.g. MyProject):",
        default="MyWorkspace"
    ).ask()

    if not ws_name:
        ws_name = "MyWorkspace"

    folder_path = questionary.text(
        "2. Enter the absolute folder path containing your documents:",
        default=os.getcwd()
    ).ask()

    if not folder_path or not os.path.exists(folder_path):
        print(f"⚠️ Warning: Directory '{folder_path}' does not exist right now, but saving configuration.")

    store = ConfigDBStore()
    store.add_workspace(name=ws_name, paths=[folder_path])
    print(f"✅ Workspace '{ws_name}' created successfully with path: {folder_path}\n")

    # Offer Quick AI Provider Setup
    setup_ai = questionary.confirm("3. Do you want to configure your AI Provider & API Key now?").ask()
    if setup_ai:
        provider_choice = questionary.select(
            "Select your preferred AI Provider:",
            choices=[
                "⚡ OpenAI Cloud (Automatic Quick-Setup)",
                "🏠 Local LLM Server (LM Studio / Ollama - 100% Free & Offline)",
                "🛠️ Custom / Other Provider"
            ]
        ).ask()

        if provider_choice and provider_choice.startswith("⚡"):
            api_key = questionary.password("Enter your OpenAI API Key (sk-...):").ask()
            if api_key:
                store.set_api_key("openai", api_key)
                settings = store.get_app_settings()
                if settings and settings.models:
                    settings.models.model_provider = "openai"
                    settings.models.inference_model = "gpt-4o-mini"
                    settings.models.summary_model = "gpt-4o-mini"
                    settings.models.local_embedding_model = "text-embedding-3-small"
                    settings.models.local_openai_embedding_model = "text-embedding-3-small"
                    settings.models.local_base_url = "https://api.openai.com/v1"
                    store.save_app_settings(settings)
                print("✅ OpenAI Provider & API Key configured successfully!")
        elif provider_choice and provider_choice.startswith("🏠"):
            base_url = questionary.text("Enter Local Server Base URL:", default="http://localhost:1234/v1").ask()
            if base_url:
                store.set_api_key("openai", "lm-studio")
                settings = store.get_app_settings()
                if settings and settings.models:
                    settings.models.local_base_url = base_url
                    settings.models.summary_model = "google/gemma-4-e2b"
                    store.save_app_settings(settings)
                print("✅ Local Server configured successfully!")

    print("\n🎉 Setup complete! You are ready to start chatting.\n")

def show_config_menu():
    """
    Interactive CLI Configuration Management Menu (/config or --config)
    """
    store = ConfigDBStore()
    
    while True:
        choice = questionary.select(
            "⚙️ AnyContext Configuration Menu:",
            choices=[
                "📂 Workspaces & Folders Management (List / Add / Delete Folders)",
                "🤖 AI Models, Base URL & API Keys",
                "🔑 Manage Saved API Keys",
                "🧠 Memory Compression & Reset Settings",
                "❓ How to Get API Keys (Guide & Links)",
                "💥 Factory Reset (Reset all settings, workspaces, API keys, and memory)",
                "🔙 Return / Exit Menu"
            ]
        ).ask()

        if not choice or choice.startswith("🔙"):
            break

        if choice.startswith("📂"):
            _manage_workspaces(store)
        elif choice.startswith("🤖"):
            _manage_models(store)
        elif choice.startswith("🔑"):
            _manage_api_keys(store)
        elif choice.startswith("🧠"):
            _manage_memory(store)
        elif choice.startswith("❓"):
            _show_api_keys_guide()
        elif choice.startswith("💥"):
            confirm = questionary.confirm(
                "⚠️ DANGER: Are you sure you want to reset AnyContext to Factory Defaults?\n  This will erase ALL workspaces, folders, API keys, configuration settings, and vector memory databases!"
            ).ask()
            if confirm:
                store.factory_reset()
                print("\n🎉 AnyContext has been completely reset to factory defaults!")
                print("Run 'actx' again anytime to launch the first-time setup wizard.\n")
                sys.exit(0)


def _manage_workspaces(store: ConfigDBStore):
    settings = store.get_app_settings()
    workspaces = settings.workspaces if settings else []
    
    ws_action = questionary.select(
        "📂 Workspaces Action:",
        choices=[
            "📋 List Workspaces & Folders",
            "➕ Create New Workspace",
            "📁 Manage Folders in Existing Workspace",
            "🗑️ Delete Workspace Entirely",
            "🔙 Back"
        ]
    ).ask()

    if not ws_action or ws_action.startswith("🔙"):
        return

    if ws_action.startswith("📋"):
        print("\n--- Configured Workspaces ---")
        for ws in workspaces:
            print(f"• \033[93m{ws.name}\033[0m:")
            for p in ws.paths:
                print(f"    - {p}")
        print("-----------------------------\n")

    elif ws_action.startswith("➕"):
        name = questionary.text("New Workspace Name:").ask()
        if name:
            path = questionary.text("First Folder Path:").ask()
            if path:
                store.add_workspace(name.strip(), [path.strip()])
                print(f"✅ Created workspace '{name}' with folder '{path}'.")

    elif ws_action.startswith("📁"):
        ws_names = [ws.name for ws in workspaces]
        if not ws_names:
            print("No workspaces configured.")
            return
        selected_ws = questionary.select("Select Workspace to manage folders:", choices=ws_names).ask()
        if not selected_ws:
            return

        curr_ws = next((w for w in workspaces if w.name == selected_ws), None)
        curr_paths = curr_ws.paths if curr_ws else []

        print(f"\n📂 Workspace \033[93m{selected_ws}\033[0m current folders:")
        for p in curr_paths:
            print(f"  - {p}")
        print()

        folder_action = questionary.select(
            f"Folder Action for '{selected_ws}':",
            choices=[
                "➕ Add Folder Path to Workspace",
                "🗑️ Remove Folder Path from Workspace",
                "🔙 Back"
            ]
        ).ask()

        if folder_action and folder_action.startswith("➕"):
            new_path = questionary.text("Enter absolute folder path to add:").ask()
            if new_path:
                if store.add_folder_to_workspace(selected_ws, new_path):
                    print(f"✅ Added folder '{new_path}' to workspace '{selected_ws}'.")
                else:
                    print("❌ Error adding folder.")

        elif folder_action and folder_action.startswith("🗑️"):
            if not curr_paths:
                print("No folders in this workspace.")
                return
            path_to_remove = questionary.select("Select folder path to remove:", choices=curr_paths).ask()
            if path_to_remove:
                if store.remove_folder_from_workspace(selected_ws, path_to_remove):
                    print(f"🗑️ Removed folder '{path_to_remove}' from workspace '{selected_ws}'.")
                else:
                    print("❌ Error removing folder.")

    elif ws_action.startswith("🗑️"):
        names = [ws.name for ws in workspaces]
        if not names:
            print("No workspaces to delete.")
            return
        to_remove = questionary.select("Select workspace to delete entirely:", choices=names).ask()
        if to_remove:
            confirm = questionary.confirm(f"⚠️ Are you sure you want to delete workspace '{to_remove}' and all its folder links?").ask()
            if confirm:
                store.remove_workspace(to_remove)
                print(f"🗑️ Deleted workspace '{to_remove}'.")

def _manage_models(store: ConfigDBStore):
    settings = store.get_app_settings()
    m = settings.models if settings else None

    old_emb_model = m.local_openai_embedding_model if (m and m.model_provider == "openai") else (m.local_embedding_model if m else "")

    print(f"\n--- Current Model Settings ---")
    print(f"• Inference Model : {m.inference_model if m else 'gpt-4o-mini'} (High Reasoning)")
    print(f"• Summary Model   : {m.summary_model if m else 'gpt-4o-mini'} (Fast & Efficient)")
    print(f"• Embedding Model : {m.local_openai_embedding_model if m else 'text-embedding-3-small'}")
    print(f"• Provider        : {m.model_provider if m else 'openai'}")
    print(f"• Local Base URL  : {m.local_base_url if m else 'http://localhost:1234/v1'}")
    print("------------------------------\n")

    setup_mode = questionary.select(
        "🤖 Select Model Setup Mode:",
        choices=[
            "⚡ Quick-Setup: OpenAI Cloud (gpt-4o-mini + OpenAI Embeddings)",
            "🏠 Quick-Setup: Local Server (LM Studio / Ollama + Gemma Summary)",
            "🛠️ Custom Model & Base URL Setup",
            "🔙 Back"
        ]
    ).ask()

    if not setup_mode or setup_mode.startswith("🔙"):
        return

    new_emb_model = ""

    if setup_mode.startswith("⚡"):
        api_key = questionary.password("Enter your OpenAI API Key (sk-...):").ask()
        if api_key:
            store.set_api_key("openai", api_key)
            if m:
                m.model_provider = "openai"
                m.inference_model = "gpt-4o-mini"
                m.summary_model = "gpt-4o-mini"
                m.local_embedding_model = "text-embedding-3-small"
                m.local_openai_embedding_model = "text-embedding-3-small"
                m.local_base_url = "https://api.openai.com/v1"
                new_emb_model = "text-embedding-3-small"
                settings.models = m
                store.save_app_settings(settings)
            print("✅ OpenAI Cloud Provider configured successfully!")

    elif setup_mode.startswith("🏠"):
        url = questionary.text("Local Server Base URL:", default="http://localhost:1234/v1").ask()
        if url:
            store.set_api_key("openai", "lm-studio")
            if m:
                m.local_base_url = url
                m.summary_model = "google/gemma-4-e2b"
                m.local_embedding_model = "text-embedding-multilingual-e5-small"
                new_emb_model = "text-embedding-multilingual-e5-small"
                settings.models = m
                store.save_app_settings(settings)
            print("✅ Local Server Provider configured successfully!")

    elif setup_mode.startswith("🛠️"):
        inf = questionary.text("Inference Model (High reasoning):", default=m.inference_model if m else "gpt-4o-mini").ask()
        sum_m = questionary.text("Summary Model (Fast & efficient):", default=m.summary_model if m else "gpt-4o-mini").ask()
        emb_m = questionary.text("Embedding Model:", default=m.local_openai_embedding_model if m else "text-embedding-3-small").ask()
        prov = questionary.text("Provider (openai/local):", default=m.model_provider if m else "openai").ask()
        url = questionary.text("Base URL:", default=m.local_base_url if m else "http://localhost:1234/v1").ask()

        if m:
            m.inference_model = inf
            m.summary_model = sum_m
            m.local_embedding_model = emb_m
            m.local_openai_embedding_model = emb_m
            m.model_provider = prov
            m.local_base_url = url
            new_emb_model = emb_m
            settings.models = m
            store.save_app_settings(settings)
            print("✅ Custom Model settings updated successfully!")

    # Check if embedding model changed to purge vector DB & prevent dimension mismatch
    if new_emb_model and old_emb_model and new_emb_model != old_emb_model:
        print(f"\n⚠️ Notice: Embedding model changed from '{old_emb_model}' to '{new_emb_model}'.")
        print("🧹 Clearing vector database (ChromaDB) to force clean re-indexing and prevent dimension mismatch errors...")
        clear_context_vector_db()

def _manage_api_keys(store: ConfigDBStore):
    all_keys = store.get_all_api_keys()
    print("\n--- Saved API Keys ---")
    if not all_keys:
        print("No API keys saved in database yet.")
    else:
        for provider, key in all_keys.items():
            print(f"• \033[93m{provider.upper()}\033[0m: {mask_key(key)}")
    print("----------------------\n")

    action = questionary.select(
        "🔑 API Key Action:",
        choices=[
            "➕ Save / Update API Key",
            "🔙 Back"
        ]
    ).ask()

    if action and action.startswith("➕"):
        provider = questionary.select(
            "Select Provider:",
            choices=[
                "OpenAI",
                "Anthropic",
                "Gemini (Google)",
                "Azure OpenAI",
                "xAI (Grok)",
                "DeepSeek",
                "Groq Cloud",
                "OpenRouter",
                "Other"
            ]
        ).ask()
        if provider:
            if provider == "Other":
                provider = questionary.text("Enter Provider Name:").ask()
            if provider:
                key = questionary.password(f"Enter API Key for {provider}:").ask()
                if key:
                    store.set_api_key(provider, key)
                    print(f"✅ Saved API Key for provider '{provider}'.")

def _show_api_keys_guide():
    guide_text = """
================================================================================
🔑 COMPREHENSIVE GUIDE: HOW TO OBTAIN API KEYS FOR ALL AI PROVIDERS
================================================================================

1. ⚡ OpenAI (ChatGPT, GPT-4o, o1 & Official Embeddings):
   • Sign up or log in at: https://platform.openai.com
   • Navigate to API Keys: https://platform.openai.com/api-keys
   • Click 'Create new secret key' (Key format: sk-proj-... or sk-...)
   • Base URL: https://api.openai.com/v1

2. 🧠 Anthropic (Claude 3.5 Sonnet, Claude 3 Opus & Haiku):
   • Sign up or log in at: https://console.anthropic.com
   • Navigate to API Keys: https://console.anthropic.com/settings/keys
   • Create a key (Key format: sk-ant-...)
   • Base URL: https://api.anthropic.com/v1

3. ♊ Google Gemini (Gemini 1.5 Pro, Flash & Text Embedding 004):
   • Visit Google AI Studio: https://aistudio.google.com
   • Navigate to Get API Key: https://aistudio.google.com/app/apikey
   • Click 'Create API key' (Key format: AIzaSy...)
   • Base URL: https://generativelanguage.googleapis.com/v1beta/openai/

4. 🪟 Microsoft Azure OpenAI Service (Enterprise Cloud):
   • Access Azure Portal: https://portal.azure.com or https://oai.azure.com
   • Go to your Azure OpenAI Resource -> Keys and Endpoint
   • Copy Key 1 or Key 2
   • Base URL: https://<your-resource-name>.openai.azure.com/openai/deployments/<your-deployment-name>

5. 🚀 xAI Grok (Grok-2 & Grok-beta):
   • Visit xAI Console: https://console.x.ai
   • Navigate to API Keys tab and create a new key (Key format: xai-...)
   • Base URL: https://api.x.ai/v1

6. 🐉 DeepSeek (DeepSeek V3 & R1 Reasoning):
   • Visit DeepSeek Platform: https://platform.deepseek.com
   • Go to API Keys: https://platform.deepseek.com/api_keys
   • Create a secret key (Key format: sk-...)
   • Base URL: https://api.deepseek.com/v1

7. ⚡ Groq Cloud (Ultra-Fast Inference for Llama 3.3, Mixtral & Gemma):
   • Visit Groq Console: https://console.groq.com
   • Navigate to API Keys: https://console.groq.com/keys
   • Create an API key (Key format: gsk_...)
   • Base URL: https://api.groq.com/openai/v1

8. 🌐 OpenRouter (Unified API Aggregator for 200+ AI Models):
   • Visit OpenRouter: https://openrouter.ai
   • Go to Keys: https://openrouter.ai/keys
   • Create a key (Key format: sk-or-v1-...)
   • Base URL: https://openrouter.ai/api/v1

9. 🏠 LM Studio & Ollama (100% Free & Local Private Offline LLMs):
   • LM Studio: Download from https://lmstudio.ai -> Base URL: http://localhost:1234/v1
   • Ollama: Download from https://ollama.com -> Base URL: http://localhost:11434/v1
   • No API Key required for local offline servers!

================================================================================
"""
    safe_print(guide_text)



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
