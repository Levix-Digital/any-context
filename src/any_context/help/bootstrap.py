"""
AnyContext System Knowledge Auto-Bootstrap Engine (v0.30.26).
Permanently indexes the Complete Command Registry (HELP_REGISTRY) and the user-facing README.md
into the 'Global' workspace inside LanceDB, ensuring that the AI agent has instant,
first-class self-awareness of all AnyContext commands, options, and workflows across every chat.

STRICT SECURITY RULE:
Only user-facing documentation (HELP_REGISTRY and README.md) is indexed. Internal architecture/secrets
(such as TECDOC.md) are strictly excluded from the user knowledge base to protect proprietary engineering.
"""
import os
import json
import hashlib
import time
import threading
from typing import List, Optional

from any_context import __version__
from any_context.config.app_settings import AppSettings
from any_context.help.registry import HELP_REGISTRY
from any_context.vector_engine.store import LanceDBStore
from any_context.vector_engine.indexer import ParallelIndexer
from any_context.vector_engine.models import IngestionConfig
from llama_index.core import Document


def _find_readme_path() -> Optional[str]:
    """Locates the project README.md across package roots and working directory."""
    candidates = [
        os.path.abspath("README.md"),
        os.path.join(os.path.dirname(__file__), "..", "..", "..", "README.md"),
        os.path.join(os.path.dirname(__file__), "..", "..", "README.md"),
        os.path.join(os.path.dirname(__file__), "..", "README.md"),
    ]
    for c in candidates:
        if os.path.exists(c) and os.path.isfile(c):
            return os.path.abspath(c)
    return None


def build_system_help_document() -> Document:
    """
    Constructs a comprehensive synthetic Markdown Document compiling the complete HELP_REGISTRY.
    Covers all CLI commands, parameters, aliases, syntax, and usage examples.
    """
    text_blocks = [
        f"# 📖 AnyContext Complete Commands, Options & Usage Manual (v{__version__})\n",
        "> This is the official and authoritative reference for all AnyContext commands, parameters, options, and workflows.\n\n"
    ]

    for key, page in HELP_REGISTRY.items():
        text_blocks.append(f"## Command: {page.command} ({page.title})\n")
        if page.aliases:
            text_blocks.append(f"**Aliases & Shortcuts**: {', '.join(page.aliases)}\n")
        text_blocks.append(f"**Description**: {page.description}\n")
        text_blocks.append(f"**Syntax & Usage**:\n```text\n{page.syntax}\n```\n")
        if page.parameters:
            text_blocks.append("**Parameters & Options**:\n" + "\n".join([f"- {p}" for p in page.parameters]) + "\n")
        if page.examples:
            text_blocks.append("**Usage Examples**:\n" + "\n".join([f"- {e}" for e in page.examples]) + "\n")
        if page.tips:
            text_blocks.append("**Best Practice Tips**:\n" + "\n".join([f"- {t}" for t in page.tips]) + "\n")
        text_blocks.append("\n---\n")

    full_text = "\n".join(text_blocks)
    return Document(
        text=full_text,
        metadata={
            "file_name": "AnyContext Command Manual & Help Registry (HELP_REGISTRY)",
            "file_path": "system://help_registry",
            "workspace": "Global",
            "source_type": "system_help",
            "content_type": "System Documentation",
            "is_system_help": True,
            "version": __version__,
            "last_modified": time.strftime("%Y-%m-%d"),
            "keywords": "anycontext, help, commands, transfer, move, switch, sync, web, source, workspace, config, inspect, share, link, backup, restore, density, model, api-keys"
        },
        id_="system_help_registry_global"
    )


def build_system_readme_document(readme_path: str) -> Optional[Document]:
    """
    Reads the user-facing README.md and creates a Document for permanent Global system context.
    Excludes TECDOC.md strictly.
    """
    if not os.path.exists(readme_path):
        return None

    try:
        with open(readme_path, "r", encoding="utf-8", errors="ignore") as f:
            content = f.read()

        return Document(
            text=f"# AnyContext Official User Guide & Overview (v{__version__})\n\n{content}",
            metadata={
                "file_name": "AnyContext System Documentation (README.md)",
                "file_path": "system://readme",
                "workspace": "Global",
                "source_type": "system_help",
                "content_type": "System Documentation",
                "is_system_help": True,
                "version": __version__,
                "last_modified": time.strftime("%Y-%m-%d"),
                "keywords": "anycontext, guide, readme, architecture, workspaces, local ai, privacy, offline models, vector search, rag"
            },
            id_="system_readme_global"
        )
    except Exception:
        return None


def ensure_system_knowledge_indexed(db_path: Optional[str] = None, force: bool = False) -> bool:
    """
    Ensures that HELP_REGISTRY and README.md are indexed into the 'Global' workspace in LanceDB.
    Uses version checking and SHA-256 content hashing to bypass re-indexing in < 1ms if already up-to-date.
    Automatically re-indexes on every new version release or document update.
    """
    settings = AppSettings.load()
    base_db_path = db_path or (settings.context.db_path if settings and settings.context else "./context_db")
    lance_dir = os.path.join(base_db_path, "lancedb")
    os.makedirs(lance_dir, exist_ok=True)

    metadata_cache_path = os.path.join(lance_dir, "system_help_cache.json")

    # 1. Build synthetic documents
    help_doc = build_system_help_document()
    readme_path = _find_readme_path()
    readme_doc = build_system_readme_document(readme_path) if readme_path else None

    docs_to_index: List[Document] = [help_doc]
    if readme_doc:
        docs_to_index.append(readme_doc)

    # 2. Compute composite hash including version and text
    combined_content = "".join([d.text for d in docs_to_index])
    composite_hash = hashlib.sha256(f"{__version__}:{combined_content}".encode("utf-8")).hexdigest()

    # 3. Fast cache check (< 1ms)
    if not force and os.path.exists(metadata_cache_path):
        try:
            with open(metadata_cache_path, "r", encoding="utf-8") as f:
                cache_data = json.load(f)
            if cache_data.get("version") == __version__ and cache_data.get("hash") == composite_hash:
                return True
        except Exception:
            pass

    # 4. Pre-flight credential check: if cloud embedding provider is configured but API key is missing or placeholder,
    # skip indexing silently to avoid throwing exceptions and polluting stdout/stderr.
    from any_context.core.utils import get_api_key, load_env
    from any_context.observability import obs
    load_env()
    model_provider = settings.models.model_provider if settings and settings.models else "openai"
    api_key = get_api_key(provider=model_provider)
    is_local_provider = model_provider in ["local", "lm-studio", "ollama"]
    if not is_local_provider:
        from llama_index.core import Settings
        existing_embed = getattr(Settings, "_embed_model", None)
        is_mock_env = (
            (existing_embed is not None and "mock" in type(existing_embed).__name__.lower())
            or (api_key is not None and (api_key.startswith("mock_") or "test" in api_key.lower() or "mock" in api_key.lower()))
        )
        if not api_key and not is_mock_env:
            obs.debug("HELP:BOOTSTRAP_SKIP", "Skipping system help vector indexing: no API key configured yet.")
            return False
        if api_key in ["placeholder", "sk-placeholder", "lm-studio"] and not is_mock_env:
            obs.debug("HELP:BOOTSTRAP_SKIP", "Skipping system help vector indexing: placeholder API key.")
            return False

    lance_store = LanceDBStore.get_instance(db_path=lance_dir)

    # 5. Stamp composite hash on all docs
    for doc in docs_to_index:
        doc.metadata["content_hash"] = composite_hash
        doc.metadata["version"] = __version__

    # 6. Remove any older Global system help chunks before inserting updated ones
    try:
        if lance_store._has_table("workspace_chunks"):
            lance_store.delete_by_file("system://help_registry", workspace_name="Global")
            lance_store.delete_by_file("system://readme", workspace_name="Global")
    except Exception:
        pass

    # 7. Index into LanceDB under workspace='Global'
    try:
        from any_context.tools.search_tools import configure_embedding_model
        configure_embedding_model()
        indexer = ParallelIndexer(store=lance_store)
        cfg = IngestionConfig(chunk_size=1024, chunk_overlap=150, max_workers=4)
        indexer.index_documents(documents=docs_to_index, workspace_name="Global", config=cfg)

        # 8. Write atomic cache marker
        try:
            with open(metadata_cache_path, "w", encoding="utf-8") as f:
                json.dump({"version": __version__, "hash": composite_hash, "updated_at": time.time()}, f)
        except Exception:
            pass

        return True
    except Exception as e:
        if "interpreter shutdown" not in str(e).lower():
            obs.debug("HELP:BOOTSTRAP_ERROR", f"Could not bootstrap system knowledge into LanceDB: {e}")
        return False


def async_ensure_system_knowledge_indexed(db_path: Optional[str] = None, force: bool = False) -> threading.Thread:
    """Spawns a non-blocking background thread to ensure system knowledge is indexed."""
    t = threading.Thread(
        target=ensure_system_knowledge_indexed,
        kwargs={"db_path": db_path, "force": force},
        daemon=True,
        name="bootstrap-system-help"
    )
    t.start()
    return t
