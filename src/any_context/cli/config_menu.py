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
    sep = "=" * 82
    print(f"\n{sep}")
    print("🎉 Welcome to AnyContext (actx) Initial Setup!")
    print("No workspaces were found in your configuration database.")
    print(f"{sep}\n")

    ws_name = questionary.text(
        "1. Enter a name for your first workspace (or press Enter for 'Default'):",
        default="Default"
    ).ask()

    if not ws_name or not ws_name.strip():
        ws_name = "Default"
    ws_name = ws_name.strip()

    store = ConfigDBStore()
    store.add_workspace(name=ws_name, paths=[])
    print(f"✅ Workspace '{ws_name}' created successfully.\n")

    # Offer Quick AI Provider Setup
    setup_ai = questionary.confirm("2. Do you want to configure your AI Provider & API Key now?").ask()
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
                "🎛️ AI Grounding & Answer Modes (Hybrid / Strict / Proactive)",
                "🌐 Live Web Search & External Intelligence (ON / OFF / Per-Workspace)",
                "🔍 Context Retrieval Density & RAG Presets (Balanced / Turbo / Deep Research)",
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
        elif choice.startswith("🎛️"):
            _manage_grounding_mode(store)
        elif choice.startswith("🌐"):
            _manage_web_search(store)
        elif choice.startswith("🔍"):
            _manage_retrieval_density(store)
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
            "✏️ Rename Workspace",
            "📁 Manage Folders in Existing Workspace",
            "🌐 Manage Web URLs & Scraping Sources",
            "🔗 Link Shared Source (Reusable Library)",
            "🔓 Unlink Shared Source",
            "🔄 Transfer Source (Folder/Web) to Another Workspace",
            "🗑️ Delete Workspace Entirely",
            "🔙 Back"
        ]
    ).ask()

    if not ws_action or ws_action.startswith("🔙"):
        return

    if ws_action.startswith("📋"):
        detailed_workspaces = store.list_workspaces_detailed()
        print("\n--- Configured Workspaces ---")
        if not detailed_workspaces:
            print("  (No workspaces configured)")
        for ws in detailed_workspaces:
            src_count_badge = f" ({ws['total_sources']} sources)" if ws.get('total_sources', 0) > 0 else " (Empty)"
            print(f"• \033[93m{ws['name']}\033[0m{src_count_badge}:")
            for s in ws.get("sources", []):
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
            if not ws.get("sources"):
                print("    - (No sources configured. Use /web add, /sync, or /config)")
        print("-----------------------------\n")

    elif ws_action.startswith("➕"):
        name = questionary.text("New Workspace Name:").ask()
        if name and name.strip():
            clean_name = name.strip()
            store.add_workspace(clean_name, paths=[])
            print(f"✅ Created workspace '{clean_name}'.")

    elif ws_action.startswith("✏️"):
        settings = store.get_app_settings()
        workspaces = settings.workspaces if settings else []
        names = [ws.name for ws in workspaces if ws.name.lower() not in ["default", "global", "shared sources"]]
        if not names:
            print("\n⚠️ No custom workspaces configured to rename ('Default', 'Global', and 'Shared Sources' are protected system workspaces).\n")
            return
        target_ws = questionary.select("Select Workspace to rename:", choices=names).ask()
        if not target_ws:
            return
        new_name = questionary.text(f"Enter new name for workspace '{target_ws}':").ask()
        if new_name and new_name.strip():
            clean_new_name = new_name.strip()
            from any_context.cli.spinner import Spinner
            with Spinner(f"Renaming workspace and migrating vector records to '{clean_new_name}'..."):
                res = store.rename_workspace(old_name=target_ws, new_name=clean_new_name)
            if res.get("success"):
                migrated = res.get("migrated_chunks", 0)
                print(f"\n✅ Successfully renamed workspace '{target_ws}' to '{clean_new_name}' ({migrated} vector chunks updated)! (API Cost: $0.00)\n")
            else:
                print(f"\n❌ Error renaming workspace: {res.get('error')}\n")


    elif ws_action.startswith("📁"):
        settings = store.get_app_settings()
        workspaces = settings.workspaces if settings else []
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
            if new_path and new_path.strip():
                clean_path = new_path.strip().strip("'\"")
                other_workspaces = [w.name for w in workspaces if w.name.lower() != selected_ws.lower()]
                link_targets = []
                if other_workspaces:
                    try:
                        ask_link = questionary.confirm(f"🔗 Would you like to link '{clean_path}' to other workspaces as well?").ask()
                        if ask_link:
                            link_targets = questionary.checkbox(
                                "Select workspaces to link this folder to (Space to select, Enter to confirm):",
                                choices=other_workspaces
                            ).ask() or []
                    except Exception:
                        link_targets = []

                res = store.attach_and_broadcast_source(
                    primary_workspace=selected_ws,
                    source_type="folder",
                    source_identifier=clean_path,
                    link_to_workspaces=link_targets
                )
                print(f"✅ Added folder '{clean_path}' to workspace '{selected_ws}'.")
                if link_targets:
                    print(f"🔗 Also linked to: {', '.join(link_targets)} ($0.00 cost)!\n")

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

    elif ws_action.startswith("🔗"):
        _link_shared_source(store=store)

    elif ws_action.startswith("🔓"):
        _unlink_shared_source(store=store)

    elif ws_action.startswith("🔄"):
        _transfer_workspace_source(store=store)

    elif ws_action.startswith("🗑️"):
        settings = store.get_app_settings()
        workspaces = settings.workspaces if settings else []
        names = [ws.name for ws in workspaces if ws.name.lower() not in ["default", "global", "shared sources"]]
        if not names:
            print("\n⚠️ No custom workspaces available to delete ('Default', 'Global', and 'Shared Sources' are protected system workspaces).\n")
            return
        to_remove = questionary.select("Select workspace to delete entirely:", choices=names).ask()
        if to_remove:
            confirm = questionary.confirm(f"⚠️ Are you sure you want to delete workspace '{to_remove}' and all its folder links?").ask()
            if confirm:
                store.remove_workspace(to_remove)
                print(f"🗑️ Deleted workspace '{to_remove}'.")


def _link_shared_source(store: ConfigDBStore):
    """Interactive guided linking of existing indexed sources across workspaces ($0.00 cost)."""
    settings = store.get_app_settings()
    workspaces = settings.workspaces if settings else []
    if not workspaces:
        print("\n⚠️ No workspaces configured.\n")
        return

    available_sources = store.list_all_available_shared_sources()
    if not available_sources:
        print("\n⚠️ No indexed sources found across any workspaces. Add a folder or web portal first!\n")
        return

    ws_names = [w.name for w in workspaces]
    target_ws = questionary.select("1. Select Destination Workspace to attach shared source:", choices=ws_names).ask()
    if not target_ws:
        return

    source_choices = []
    for s in available_sources:
        orig = s.get("origin_workspace", "Workspace")
        stype = s.get("type", "folder")
        title = s.get("title", s.get("identifier"))
        badge = f"[{stype.upper()}] {title} (from '{orig}')"
        source_choices.append(badge)

    selected_choice = questionary.select("2. Select Shared Source to link:", choices=source_choices).ask()
    if not selected_choice:
        return

    idx = source_choices.index(selected_choice)
    chosen_source = available_sources[idx]

    from any_context.cli.spinner import Spinner
    with Spinner(f"Linking shared source to '{target_ws}'..."):
        res = store.link_shared_source_to_workspace(
            workspace_name=target_ws,
            source_type=chosen_source["type"],
            source_identifier=chosen_source["identifier"],
            title=chosen_source.get("title")
        )
    print(f"\n✅ Successfully linked shared source '{chosen_source.get('title')}' to workspace '{target_ws}'! (Cost: $0.00)\n")


def _unlink_shared_source(store: ConfigDBStore):
    """Interactive unlinking of a shared source from a workspace."""
    settings = store.get_app_settings()
    workspaces = settings.workspaces if settings else []
    if not workspaces:
        print("\n⚠️ No workspaces configured.\n")
        return

    ws_names = [w.name for w in workspaces]
    target_ws = questionary.select("1. Select Workspace to manage shared links:", choices=ws_names).ask()
    if not target_ws:
        return

    links = store.get_workspace_shared_links(target_ws)
    if not links:
        print(f"\n⚠️ Workspace '{target_ws}' has no linked shared sources.\n")
        return

    link_choices = [f"[{l.get('source_type', '').upper()}] {l.get('title', l.get('source_identifier'))}" for l in links]
    chosen = questionary.select("2. Select Shared Link to remove:", choices=link_choices).ask()
    if not chosen:
        return

    idx = link_choices.index(chosen)
    target_link = links[idx]

    store.unlink_shared_source_from_workspace(
        workspace_name=target_ws,
        source_type=target_link["source_type"],
        source_identifier=target_link["source_identifier"]
    )
    print(f"\n🗑️ Unlinked shared source from '{target_ws}'.\n")


def _transfer_workspace_source(store: ConfigDBStore):
    """Interactive guided transfer of local folders or web sources between workspaces."""
    settings = store.get_app_settings()
    workspaces = settings.workspaces if settings else []
    if len(workspaces) < 2:
        print("\n⚠️ You need at least 2 workspaces to transfer data sources. Please create a target workspace first!\n")
        return

    from any_context.ingestion.web_scheduler import WebSchedulerStore
    web_store = WebSchedulerStore()

    ws_names = [w.name for w in workspaces]
    source_ws = questionary.select("1. Select Source Workspace (Origem):", choices=ws_names).ask()
    if not source_ws:
        return

    src_obj = next((w for w in workspaces if w.name == source_ws), None)
    folders = src_obj.paths if src_obj else []
    web_urls = web_store.get_workspace_web_urls(source_ws)

    if not folders and not web_urls:
        print(f"\n⚠️ Workspace '{source_ws}' has no data sources (no folders or web URLs) to transfer.\n")
        return

    source_choices = []
    for f in folders:
        source_choices.append(f"📁 [Folder] {f}")
    for w in web_urls:
        pages_info = f" • {w.get('page_count')} pages" if w.get('page_count', 1) > 1 else ""
        source_choices.append(f"🌐 [Web Source] {w['url']} ({w.get('title') or 'Web Source'}{pages_info})")
    source_choices.append("🔙 Cancel")

    selected_source = questionary.select(
        f"2. Select Data Source to move from '{source_ws}':",
        choices=source_choices
    ).ask()

    if not selected_source or selected_source.startswith("🔙"):
        return

    target_ws_candidates = [w for w in ws_names if w != source_ws]
    target_ws = questionary.select("3. Select Target Workspace (Destino):", choices=target_ws_candidates).ask()
    if not target_ws:
        return

    # Check RBAC permissions for shared workspaces
    try:
        from any_context.workspace_sharing.store import WorkspaceSharingStore
        sharing_store = WorkspaceSharingStore()
        user_perms_src = sharing_store.get_workspace_permissions(source_ws)
        user_perms_tgt = sharing_store.get_workspace_permissions(target_ws)
        if user_perms_src and user_perms_tgt:
            # User is in shared collaboration mode
            pass
    except Exception:
        pass

    confirm = questionary.confirm(
        f"❓ Move '{selected_source}' from '{source_ws}' to '{target_ws}'?"
    ).ask()
    if not confirm:
        print("↩️ Transfer cancelled.\n")
        return

    from any_context.cli.spinner import Spinner
    with Spinner(f"Transferring source and migrating vector metadata to '{target_ws}'..."):
        if selected_source.startswith("📁"):
            raw_path = selected_source.split("[Folder] ", 1)[1].strip()
            res = store.transfer_local_folder_source(source_ws=source_ws, target_ws=target_ws, folder_path=raw_path)
            if res.get("success"):
                chunks = res.get("transferred_chunks", 0)
                print(f"\n✅ Successfully moved folder '{raw_path}' ({chunks} vector chunks) to '{target_ws}'! (API Cost: $0.00)\n")
            else:
                print(f"\n❌ Error transferring folder: {res.get('error')}\n")
        elif selected_source.startswith("🌐"):
            raw_url = selected_source.split("[Web Source] ", 1)[1].split(" (")[0].strip()
            res = web_store.transfer_web_source(source_ws=source_ws, target_ws=target_ws, url_or_root=raw_url)
            if res.get("success"):
                chunks = res.get("transferred_chunks", 0)
                pages = res.get("transferred_pages", 0)
                print(f"\n✅ Successfully moved web portal '{raw_url}' ({pages} pages, {chunks} vector chunks) to '{target_ws}'! (API Cost: $0.00)\n")
            else:
                print(f"\n❌ Error transferring web source: {res.get('error')}\n")


def _manage_workspace_web_urls(workspace_name: Optional[str] = None, store: Optional[ConfigDBStore] = None):
    """Interactive management of Web URLs and Documentation Site Ingestors for a Workspace."""
    from any_context.ingestion.web_scheduler import WebSchedulerStore, sync_workspace_web_urls, remove_web_url_from_chromadb
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
                "Tavily (Web Search Engine)",
                "Serper (Google Search API)",
                "Azure OpenAI",
                "xAI (Grok)",
                "DeepSeek",
                "Groq Cloud",
                "OpenRouter",
                "Other"
            ]
        ).ask()
        if provider:
            clean_provider = provider
            if "Tavily" in provider:
                clean_provider = "tavily"
            elif "Serper" in provider:
                clean_provider = "serper"
            elif "Gemini" in provider:
                clean_provider = "gemini"
            elif "Azure" in provider:
                clean_provider = "azure"
            elif "xAI" in provider:
                clean_provider = "xai"
            elif "Groq" in provider:
                clean_provider = "groq"
            elif "OpenRouter" in provider:
                clean_provider = "openrouter"
            elif provider == "Other":
                clean_provider = questionary.text("Enter Provider Name:").ask()

            if clean_provider:
                key = questionary.password(f"Enter API Key for {clean_provider}:").ask()
                if key:
                    store.set_api_key(clean_provider, key)
                    print(f"✅ Saved API Key for provider '{clean_provider}'.")


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


def _manage_grounding_mode(store: ConfigDBStore):
    current_mode = store.get_grounding_mode()
    mode_titles = {
        "strict": "🛡️ Strict (Default - 100% grounded to indexed documents, zero speculation)",
        "hybrid": "⚖️ Hybrid (Balanced - Facts from workspace + labeled suggestions)",
        "proactive": "🚀 Proactive (Research - Broad synthesis, insights & web recommendations)"
    }

    sep = "=" * 82
    print(f"\n{sep}")
    print("🎛️ AI Grounding & Answer Modes")
    print(sep)
    print(f"Current Active Mode : \033[93m{current_mode.upper()}\033[0m")
    print(f"Description         : {mode_titles.get(current_mode, current_mode)}")
    print(f"{sep}\n")

    choices = [
        f"🛡️ Strict (Default - 100% grounded to indexed documents, zero speculation){'  [Active]' if current_mode == 'strict' else ''}",
        f"⚖️ Hybrid (Balanced - Facts from workspace + labeled suggestions){'  [Active]' if current_mode == 'hybrid' else ''}",
        f"🚀 Proactive (Research & Ideation - Broad synthesis, insights & web recommendations){'  [Active]' if current_mode == 'proactive' else ''}",
        "🔙 Back"
    ]

    choice = questionary.select("Select AI Grounding & Answer Mode:", choices=choices).ask()
    if not choice or choice.startswith("🔙"):
        return

    if choice.startswith("🛡️"):
        new_mode = "strict"
    elif choice.startswith("🚀"):
        new_mode = "proactive"
    else:
        new_mode = "hybrid"

    saved = store.set_grounding_mode(new_mode)
    print(f"\n✅ AI Grounding Mode updated to: \033[92m{saved.capitalize()}\033[0m!\n")


def _manage_web_search(store: ConfigDBStore):
    settings = store.get_app_settings()
    global_status = store.get_web_search_status()
    tavily_key = store.get_api_key("tavily") or os.getenv("TAVILY_API_KEY")
    serper_key = store.get_api_key("serper") or os.getenv("SERPER_API_KEY")

    tavily_status = f"\033[92mConfigured ({mask_key(tavily_key)})\033[0m" if tavily_key else "\033[90mNot Configured (Free DuckDuckGo fallback)\033[0m"
    serper_status = f"\033[92mConfigured ({mask_key(serper_key)})\033[0m" if serper_key else "\033[90mNot Configured\033[0m"

    sep = "=" * 82
    print(f"\n{sep}")
    print("🌐 Live Web Search & External Intelligence")
    print(sep)
    status_label = "\033[92mENABLED (ON)\033[0m" if global_status else "\033[90mDISABLED (OFF)\033[0m"
    print(f"Global Default Status : {status_label}")
    print(f"• DuckDuckGo Engine   : \033[92mFree / Active (No Key Required)\033[0m")
    print(f"• Tavily Search API   : {tavily_status}")
    print(f"• Serper Search API   : {serper_status}")
    print(f"{sep}\n")

    workspaces = settings.workspaces if settings else []
    choices = [
        f"🟢 Enable Web Search Globally (All Workspaces){'  [Active]' if global_status else ''}",
        f"🔴 Disable Web Search Globally (100% Offline Local){'  [Active]' if not global_status else ''}",
        "📂 Configure Web Search for a Specific Workspace...",
        "🔑 Set / Update Tavily API Key (Premium Web Intelligence)...",
        "🔑 Set / Update Serper API Key (Google Search API)...",
        "🔙 Back"
    ]

    choice = questionary.select("Select Web Search Action:", choices=choices).ask()
    if not choice or choice.startswith("🔙"):
        return

    if choice.startswith("🟢"):
        store.set_web_search_status(True, apply_global=True)
        print("\n✅ \033[92mWeb Search ENABLED globally for all workspaces!\033[0m")
        print("\033[93m⚠️ Cost Notice:\033[0m Real-time internet searches consume external web tokens.\n")
    elif choice.startswith("🔴"):
        store.set_web_search_status(False, apply_global=True)
        print("\n🔒 \033[90mWeb Search DISABLED globally. (100% offline local isolation)\033[0m\n")
    elif choice.startswith("🔑 Set / Update Tavily"):
        cur_tvly = store.get_api_key("tavily") or ""
        new_key = questionary.password("Enter Tavily API Key (tvly-...):", default=cur_tvly).ask()
        if new_key is not None:
            if new_key.strip():
                store.set_api_key("tavily", new_key.strip())
                print(f"\n✅ \033[92mSaved Tavily API Key ({mask_key(new_key.strip())}) successfully!\033[0m\n")
            else:
                print("\n⚠️ No Tavily API key entered.\n")
    elif choice.startswith("🔑 Set / Update Serper"):
        cur_serp = store.get_api_key("serper") or ""
        new_key = questionary.password("Enter Serper API Key:", default=cur_serp).ask()
        if new_key is not None:
            if new_key.strip():
                store.set_api_key("serper", new_key.strip())
                print(f"\n✅ \033[92mSaved Serper API Key ({mask_key(new_key.strip())}) successfully!\033[0m\n")
            else:
                print("\n⚠️ No Serper API key entered.\n")
    elif choice.startswith("📂"):
        if not workspaces:
            print("⚠️ No workspaces found.")
            return
        ws_choices = [f"{w.name} (Search: {'ON' if getattr(w, 'web_search_enabled', False) else 'OFF'})" for w in workspaces]
        ws_choices.append("🔙 Back")
        ws_pick = questionary.select("Select Workspace to Configure:", choices=ws_choices).ask()
        if not ws_pick or ws_pick.startswith("🔙"):
            return
        picked_name = ws_pick.split("(")[0].strip()
        cur_ws_st = store.get_web_search_status(workspace_name=picked_name)
        toggle_choices = [
            f"🟢 Enable Web Search for '{picked_name}'{'  [Active]' if cur_ws_st else ''}",
            f"🔴 Disable Web Search for '{picked_name}'{'  [Active]' if not cur_ws_st else ''}",
            "🔙 Cancel"
        ]
        t_pick = questionary.select(f"Set Web Search for '{picked_name}':", choices=toggle_choices).ask()
        if t_pick and t_pick.startswith("🟢"):
            store.set_web_search_status(True, workspace_name=picked_name)
            print(f"\n✅ \033[92mWeb Search ENABLED for workspace '{picked_name}'!\033[0m\n")
        elif t_pick and t_pick.startswith("🔴"):
            store.set_web_search_status(False, workspace_name=picked_name)
            print(f"\n🔒 \033[90mWeb Search DISABLED for workspace '{picked_name}'.\033[0m\n")


def _manage_retrieval_density(store: ConfigDBStore):
    settings = store.get_app_settings()
    ctx = settings.context if settings else None

    if not ctx:
        print("⚠️ Could not load context settings.")
        return

    sep = "=" * 82
    print(f"\n{sep}")
    print("🔍 Context Retrieval Density & Multi-Source RAG Presets")
    print(sep)
    print(f"Current Preset      : \033[93m{ctx.retrieval_preset.upper()}\033[0m")
    print(f"Top-K Chunks to AI  : \033[92m{ctx.top_k}\033[0m chunks (~{ctx.top_k * 130} tokens)")
    print(f"ChromaDB Candidate  : \033[96m{ctx.candidate_pool_size}\033[0m candidate chunks")
    print(f"Max Chunks per Doc  : \033[95m{ctx.max_chunks_per_source}\033[0m chunks per unique source")
    print(f"{sep}\n")

    preset_choice = questionary.select(
        "Select Retrieval Density Preset:",
        choices=[
            "⚡ Balanced (Default: Top-40, Pool-100, Max-3) - Recommended for Cloud models and 20+ sources",
            "🚀 Turbo (Top-20, Pool-50, Max-2) - Ultra-fast TTFT, ideal for LM Studio/Ollama offline",
            "🔬 Deep Research (Top-60, Pool-150, Max-4) - Maximum density for massive dossiers & 50+ websites",
            "🛠️ Custom (Enter exact numbers manually)",
            "🔙 Back"
        ]
    ).ask()

    if not preset_choice or preset_choice.startswith("🔙"):
        return

    if preset_choice.startswith("⚡"):
        ctx.apply_preset("balanced")
        store.update_context_settings(ctx)
        print("\n✅ Applied \033[92mBalanced Preset\033[0m: Top-K 40 chunks, Candidate Pool 100, Max 3 per source!\n")

    elif preset_choice.startswith("🚀"):
        ctx.apply_preset("turbo")
        store.update_context_settings(ctx)
        print("\n✅ Applied \033[92mTurbo Preset\033[0m: Top-K 20 chunks, Candidate Pool 50, Max 2 per source!\n")

    elif preset_choice.startswith("🔬"):
        ctx.apply_preset("deep_research")
        store.update_context_settings(ctx)
        print("\n✅ Applied \033[92mDeep Research Preset\033[0m: Top-K 60 chunks, Candidate Pool 150, Max 4 per source!\n")

    elif preset_choice.startswith("🛠️"):
        new_top_k = questionary.text("Enter Target Top-K Chunks to deliver to AI (e.g. 40):", default=str(ctx.top_k)).ask()
        new_pool = questionary.text("Enter Initial ChromaDB Candidate Pool (e.g. 100):", default=str(ctx.candidate_pool_size)).ask()
        new_max_src = questionary.text("Enter Max Chunks per unique Document/URL (e.g. 3):", default=str(ctx.max_chunks_per_source)).ask()
        try:
            ctx.top_k = int(new_top_k)
            ctx.candidate_pool_size = int(new_pool)
            ctx.max_chunks_per_source = int(new_max_src)
            ctx.retrieval_preset = "custom"
            store.update_context_settings(ctx)
            print("\n✅ Saved custom retrieval density parameters!\n")
        except (ValueError, TypeError):
            print("\n⚠️ Invalid integer values entered. Changes not saved.\n")


def _manage_subscription():
    from any_context.billing import BillingManager, get_all_plans
    mgr = BillingManager()
    status = mgr.get_status()

    sep = "=" * 82
    print(f"\n{sep}")
    print("💳 AnyContext Subscription & Payment Plans")
    print(sep)
    print(f"Active Tier : \033[93m{status.active_tier_name}\033[0m (ID: {status.active_tier_id})")
    print(f"License Key : {status.license_key or 'None'}")
    print(f"{sep}\n")

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
        from any_context.tools.search_tools import safe_stdout_write
        safe_stdout_write("\n" + mgr.format_pricing_cards_cli() + "\n\n")

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



