import sys
import os
from typing import Optional, List, Dict, Any
import questionary
from any_context.config.db_store import ConfigDBStore, safe_print

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
                    settings.models.embedding_model = "text-embedding-3-small"
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
                "🤝 Workspace Sharing & Collaboration (Google Drive Style)",
                "🤖 AI Models, Base URL & API Keys",
                "🔑 Manage Saved API Keys",
                "🧠 Memory Compression & Reset Settings",
                "💳 Subscription & Payment Plans (Tiers, Pricing & Licensing)",
                "🛡️ User Accounts & Security Access Control (RBAC & Tokens)",
                "💥 Factory Reset (Reset all settings, workspaces, API keys, and memory)",
                "🔙 Return / Exit Menu"
            ]
        ).ask()

        if not choice or choice.startswith("🔙"):
            break

        if choice.startswith("📂"):
            _manage_workspaces(store)
        elif choice.startswith("🤝"):
            _manage_workspace_sharing(store)
        elif choice.startswith("🤖"):
            _manage_models(store)
        elif choice.startswith("🔑"):
            _manage_api_keys(store)
        elif choice.startswith("🧠"):
            _manage_memory(store)
        elif choice.startswith("💳"):
            _manage_subscription()
        elif choice.startswith("🛡️"):
            _manage_users_and_security(store)

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
            "🌐 Manage Web URLs & Scraping Sources",
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
                print(f"    - [Folder] {p}")
            from any_context.ingestion.web_scheduler import WebSchedulerStore
            web_urls = WebSchedulerStore().get_workspace_web_urls(ws.name)
            for w in web_urls:
                pages_badge = f" • {w.get('page_count')} pages" if w.get('page_count', 1) > 1 else ""
                print(f"    - [Web Portal] {w['url']} ({w.get('title') or 'Web Source'}{pages_badge})")
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

    elif ws_action.startswith("🌐"):
        _manage_workspace_web_urls(store=store)

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

def _manage_workspace_web_urls(workspace_name: Optional[str] = None, store: Optional[ConfigDBStore] = None):
    """Interactive management of Web URLs and Documentation Site Ingestors for a Workspace."""
    from any_context.ingestion.web_scheduler import WebSchedulerStore, sync_workspace_web_urls
    from any_context.ingestion.web_crawler import run_interactive_web_crawler

    # Handle polymorphic call if first argument was passed as store
    if isinstance(workspace_name, ConfigDBStore):
        store = workspace_name
        workspace_name = None

    store = store or ConfigDBStore()
    settings = store.get_app_settings()
    workspaces = settings.workspaces if settings else []
    
    target_ws = workspace_name
    if not target_ws:
        ws_names = [ws.name for ws in workspaces]
        if not ws_names:
            print("No workspaces configured.")
            return
        target_ws = questionary.select("Select Workspace to manage Web URLs:", choices=ws_names).ask()
        if not target_ws:
            return

    web_store = WebSchedulerStore()

    while True:
        urls = web_store.get_workspace_web_urls(target_ws)
        print(f"\n🌐 Workspace \033[93m{target_ws}\033[0m Configured Web Sources ({len(urls)} registered):")
        if not urls:
            print("  (No web sources configured yet)")
        for u in urls:
            pages_info = f" | Indexed Pages: {u.get('page_count', 1)}" if u.get('page_count', 1) > 1 else ""
            print(f"  • \033[96m{u.get('title') or u['url']}\033[0m")
            print(f"    URL: {u['url']}{pages_info} | Interval: {u.get('polling_interval_hours', 24)}h | Last Scraped: {u.get('last_scraped_at') or 'Pending'}")
        print()

        action = questionary.select(
            f"Web Sources Action for '{target_ws}':",
            choices=[
                "➕ Add Website / Documentation Portal (Interactive Discovery & Deep Crawl)",
                "🔄 Force Re-sync / Scrape All Web Sources",
                "🗑️ Remove Web Source & Purge Vectors",
                "🔙 Back"
            ]
        ).ask()

        if not action or action.startswith("🔙"):
            break

        if action.startswith("➕"):
            run_interactive_web_crawler(workspace_name=target_ws)

        elif action.startswith("🔄"):
            if not urls:
                print("No web sources registered to re-sync.")
                continue
            print(f"\n⏳ Synchronizing {len(urls)} web sources for workspace '{target_ws}'...")
            sync_res = sync_workspace_web_urls(target_ws)
            print(f"✅ Synced {sync_res.get('total_urls', 0)} web sources successfully!\n")

        elif action.startswith("🗑️"):
            if not urls:
                print("No web sources in this workspace.")
                continue
            url_choices = []
            for u in urls:
                pages_badge = f" ({u.get('page_count')} pages)" if u.get('page_count', 1) > 1 else ""
                url_choices.append(f"{u['url']}{pages_badge} — {u.get('title') or 'Web Source'}")
            url_choices.append("🔙 Back")
            selected_choice = questionary.select("Select Web Source to remove:", choices=url_choices).ask()
            if selected_choice and not selected_choice.startswith("🔙"):
                target_url = selected_choice.split(" ")[0]
                web_store.delete_web_url_by_url(target_ws, target_url)
                remove_web_url_from_chromadb(target_ws, target_url)
                print(f"🗑️ Removed '{target_url}' and purged all associated indexed vectors from workspace '{target_ws}'.\n")

def _manage_models(store: ConfigDBStore):
    settings = store.get_app_settings()
    m = settings.models if settings else None

    old_emb_model = m.embedding_model if m else ""

    print(f"\n--- Current Model Settings ---")
    print(f"• Inference Model : {m.inference_model if m else 'gpt-4o-mini'} (High Reasoning)")
    print(f"• Summary Model   : {m.summary_model if m else 'gpt-4o-mini'} (Fast & Efficient)")
    print(f"• Embedding Model : {m.embedding_model if m else 'text-embedding-3-small'}")
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
                m.embedding_model = "text-embedding-3-small"
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
                m.embedding_model = "text-embedding-3-small"
                new_emb_model = "text-embedding-3-small"
                settings.models = m
                store.save_app_settings(settings)
            print("✅ Local Server Provider configured successfully!")

    elif setup_mode.startswith("🛠️"):
        inf = questionary.text("Inference Model (High reasoning):", default=m.inference_model if m else "gpt-4o-mini").ask()
        sum_m = questionary.text("Summary Model (Fast & efficient):", default=m.summary_model if m else "gpt-4o-mini").ask()
        emb_m = questionary.text("Embedding Model:", default=m.embedding_model if m else "text-embedding-3-small").ask()
        prov = questionary.text("Provider (openai/local):", default=m.model_provider if m else "openai").ask()
        url = questionary.text("Base URL:", default=m.local_base_url if m else "http://localhost:1234/v1").ask()

        if m:
            m.inference_model = inf
            m.summary_model = sum_m
            m.embedding_model = emb_m
            m.model_provider = prov
            m.local_base_url = url
            new_emb_model = emb_m
            settings.models = m
            store.save_app_settings(settings)
            print("✅ Custom Model settings updated successfully!")

    # Check if embedding model changed to purge vector DB & prevent dimension mismatch
    if new_emb_model and old_emb_model and new_emb_model != old_emb_model:
        print(f"\n⚠️ Notice: Embedding model changed from '{old_emb_model}' to '{new_emb_model}'.")
        print("🧹 Clearing vector database to force clean re-indexing and prevent dimension mismatch errors...")
        from any_context.ingestion.local_folder_ingestor import clear_context_vector_db, run_index_folder
        clear_context_vector_db()
        print("⚡ Re-indexing workspace documents with new embedding model...")
        run_index_folder()
    elif new_emb_model:
        from any_context.ingestion.local_folder_ingestor import run_index_folder
        run_index_folder()


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
            "📖 How to Get API Keys (Guide & Links)",
            "🔙 Back"
        ]
    ).ask()

    if not action or action.startswith("🔙"):
        return

    if action.startswith("📖"):
        from any_context.help.manager import display_help_page
        from any_context.help.registry import get_help_page
        page = get_help_page("api-keys")
        if page:
            display_help_page(page)
        return

    if action.startswith("➕"):
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

    from any_context.memory import MemoryManager
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


def _manage_users_and_security(store: ConfigDBStore):
    admin_cfg = store.is_admin_configured()
    users = store.list_users()
    tokens = store.get_access_tokens()

    print("\n--- 🛡️ User Accounts & Access Control (RBAC) ---")
    print(f"• Security Status  : {'Protected (Multi-User RBAC Mode)' if admin_cfg else 'Open Local Mode (Friction-Free Personal Use)'}")
    print(f"• Total Users      : {len(users)}")
    print(f"• Total Bearer Tokens: {len(tokens)}")
    print("--------------------------------------------------\n")

    sec_action = questionary.select(
        "🛡️ Security Action:",
        choices=[
            "👑 Setup Administrator Account (Enable Enterprise Security)",
            "➕ Create Team User (Analyst / Viewer)",
            "👥 List Users",
            "🔑 Create Security Bearer Token",
            "📜 List Active Security Tokens",
            "🔙 Back"
        ]
    ).ask()

    if not sec_action or sec_action.startswith("🔙"):
        return

    if sec_action.startswith("👑"):
        if store.is_admin_configured():
            print("⚠️ Admin user is already configured!")
            return
        name = questionary.text("Enter Administrator Full Name (e.g. Dr. Silva):").ask()
        email = questionary.text("Enter Administrator Email:").ask()
        password = questionary.password("Enter Administrator Password:").ask()
        if name and email and password:
            try:
                admin_info = store.setup_admin_user(name=name, email=email, password=password)
                print(f"✅ Admin Account created for '{admin_info['name']}' ({admin_info['email']})!")
                print(f"🔑 Master Admin Bearer Token: {admin_info['token']['token_id']}")
            except Exception as e:
                print(f"❌ Error creating Admin: {e}")

    elif sec_action.startswith("➕"):
        if not store.is_admin_configured():
            print("⚠️ Please configure an Administrator account first (Option 👑).")
            return
        name = questionary.text("Enter User Full Name (e.g. Dra. Amanda):").ask()
        email = questionary.text("Enter User Email:").ask()
        password = questionary.password("Enter User Password:").ask()
        role = questionary.select("Select Role Level:", choices=["analyst", "viewer", "admin"]).ask()
        
        all_ws = [ws.name for ws in store.get_app_settings().workspaces]
        ws_choices = questionary.checkbox("Select Allowed Workspaces:", choices=all_ws).ask()
        if not ws_choices:
            ws_choices = ["Default"]

        if name and email and password and role:
            try:
                new_u = store.create_user(name=name, email=email, password=password, role=role, allowed_workspaces=ws_choices)
                print(f"✅ User '{new_u['name']}' ({new_u['email']}) created with role '{new_u['role']}'!")
            except Exception as e:
                print(f"❌ Error creating user: {e}")

    elif sec_action.startswith("👥"):
        if not users:
            print("No team users created yet.")
        else:
            print("\n--- Configured Users ---")
            for u in users:
                print(f"• \033[93m{u['name']}\033[0m ({u['email']}) - Role: {u['role'].upper()} | Workspaces: {u['allowed_workspaces']}")
            print("------------------------\n")

    elif sec_action.startswith("🔑"):
        token_name = questionary.text("Enter Token Name (e.g. HR Bot, Dev Token):").ask()
        role = questionary.select("Select Token Role:", choices=["viewer", "analyst", "admin"]).ask()
        if token_name and role:
            t_info = store.create_access_token(name=token_name, role=role)
            print(f"✅ Generated Security Token for '{t_info['name']}':")
            print(f"   \033[92m{t_info['token_id']}\033[0m")

    elif sec_action.startswith("📜"):
        if not tokens:
            print("No active security tokens found.")
        else:
            print("\n--- Active Security Tokens ---")
            for t in tokens:
                print(f"• Token: \033[93m{t['token_id']}\033[0m | Name: {t['name']} | Role: {t['role'].upper()} | Workspaces: {t['allowed_workspaces']}")
            print("------------------------------\n")


def _manage_workspace_sharing(store: ConfigDBStore):
    from any_context.workspace_sharing import WorkspaceSharingStore, WorkspaceSharingManager

    s_store = WorkspaceSharingStore()
    mgr = WorkspaceSharingManager(store=s_store)
    all_ws = [ws.name for ws in store.get_app_settings().workspaces]

    print("\n--- 🤝 Workspace Sharing & Collaboration (Google Drive Style) ---")
    print("Share workspace context and AI RAG intelligence with team collaborators.")
    print("-------------------------------------------------------------------\n")

    action = questionary.select(
        "🤝 Sharing Action:",
        choices=[
            "🔗 Generate Workspace Share Invite Code (Share Workspace)",
            "📩 Accept Workspace Share Invite Code (Join Workspace)",
            "👥 List Workspace Collaborators",
            "📁 View Workspace Transparent Folders & Ownership",
            "🔙 Back"
        ]
    ).ask()

    if not action or action.startswith("🔙"):
        return

    if action.startswith("🔗"):
        if not all_ws:
            print("No workspaces available to share.")
            return
        target_ws = questionary.select("Select Workspace to Share:", choices=all_ws).ask()
        role = questionary.select("Select Access Level for Invitees:", choices=[
            "👁️ Viewer (Chat & Search only - Read-Only)",
            "✏️ Editor (Chat & Search + Can add own local folders)"
        ]).ask()
        role_code = "editor" if role and role.startswith("✏️") else "viewer"
        max_uses_str = questionary.text("Enter max uses limit (1 for single use, 0 for unlimited):", default="1").ask()
        try:
            max_u = int(max_uses_str)
        except ValueError:
            max_u = 1

        if target_ws:
            invite = s_store.create_share_invite(
                workspace_name=target_ws,
                access_level=role_code,
                created_by_email="owner@local",
                max_uses=max_u
            )
            print(f"\n✅ Workspace Share Invite Code generated for '\033[93m{target_ws}\033[0m'!")
            print(f"🔑 Share Code: \033[92m{invite.invite_code}\033[0m")
            print(f"📋 Access Level: {invite.access_level.upper()} | Max Uses: {invite.max_uses}\n")

    elif action.startswith("📩"):
        inv_code = questionary.text("Enter Workspace Share Invite Code (e.g. SHARE-WKS-1234):").ask()
        user_email = questionary.text("Enter Your Email Address:").ask()
        if inv_code and user_email:
            try:
                perm = s_store.accept_share_invite(invite_code=inv_code, user_email=user_email)
                print(f"\n🎉 Successfully joined workspace '\033[93m{perm.workspace_name}\033[0m' as '\033[92m{perm.access_level.upper()}\033[0m'!")
            except Exception as e:
                print(f"❌ Error accepting invite: {e}")

    elif action.startswith("👥"):
        if not all_ws:
            print("No workspaces available.")
            return
        target_ws = questionary.select("Select Workspace to View Collaborators:", choices=all_ws).ask()
        if target_ws:
            collabs = s_store.list_workspace_collaborators(target_ws)
            if not collabs:
                print(f"No external collaborators joined workspace '{target_ws}' yet.")
            else:
                print(f"\n--- Collaborators for '{target_ws}' ---")
                for c in collabs:
                    print(f"• \033[93m{c.user_email}\033[0m - Access: {c.access_level.upper()} | Invited by: {c.granted_by_email}")
                print("---------------------------------------\n")

    elif action.startswith("📁"):
        if not all_ws:
            print("No workspaces available.")
            return
        target_ws = questionary.select("Select Workspace to View Folder Ownership:", choices=all_ws).ask()
        user_email = questionary.text("Enter Your Email (to check your folder permissions):", default="owner@local").ask()
        if target_ws:
            t_folders = mgr.get_transparent_folders_view(workspace_name=target_ws, current_user_email=user_email)
            print(f"\n--- Transparent Folder View for '{target_ws}' ---")
            if not t_folders:
                print("No registered folders in SQLite workspace_folders table.")
            else:
                for tf in t_folders:
                    status_color = "\033[92m" if tf["is_owner"] else "\033[90m"
                    print(f"• Path: {tf['folder_path']}")
                    print(f"  Status: {status_color}{tf['tag']}\033[0m")
            print("--------------------------------------------------\n")


def _manage_subscription():
    from any_context.billing import BillingManager, get_all_plans
    mgr = BillingManager()
    status = mgr.get_status()

    print("\n=======================================================")
    print("💳 AnyContext Subscription & Payment Plans")
    print(f"Active Tier : \033[93m{status.active_tier_name}\033[0m (ID: {status.active_tier_id})")
    print(f"License Key : {status.license_key or 'None'}")
    print("=======================================================\n")

    action = questionary.select(
        "Subscription Action:",
        choices=[
            "📊 View Complete Pricing & Capability Matrix Table",
            "🔑 Activate / Change Subscription Plan Tier",
            "🔙 Back"
        ]
    ).ask()

    if not action or action.startswith("🔙"):
        return

    if action.startswith("📊"):
        print("\n" + mgr.format_pricing_table_markdown() + "\n")

    elif action.startswith("🔑"):
        plans = get_all_plans()
        choices = [f"{p.name} (${p.monthly_price_usd:.0f}/mo) - {p.tier_id}" for p in plans]
        sel = questionary.select("Select Plan Tier to Activate:", choices=choices).ask()
        if sel:
            selected_tier = sel.split("-")[-1].strip()
            l_key = questionary.text("Enter License Key (Leave empty to auto-generate):").ask()
            new_status = mgr.store.set_active_tier(tier_id=selected_tier, license_key=l_key if l_key else None)
            print(f"\n🎉 Successfully activated tier '\033[92m{new_status.active_tier_name}\033[0m'!")
            print(f"License Key: {new_status.license_key}\n")



