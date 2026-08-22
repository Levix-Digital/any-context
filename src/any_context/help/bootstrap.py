"""
AnyContext System Knowledge Auto-Bootstrap Engine (v0.23.1).
Permanently indexes the Complete Command Registry (HELP_REGISTRY) and the user-facing README.md
into the 'Global' workspace inside LanceDB, ensuring that the AI agent has instant,
first-class self-awareness of all AnyContext commands, options, and workflows across every chat.

STRICT SECURITY RULE:
Only user-facing documentation (HELP_REGISTRY and README.md) is indexed. Internal architecture/secrets
(such as TECDOC.md) are strictly excluded from the user knowledge base.
"""
import os
import hashlib
import time
from typing import List, Optional

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
    Covers all 28 CLI commands, parameters, aliases, syntax, and usage examples.
    """
    text_blocks = [
        "# 📖 AnyContext Complete Commands, Options & Usage Manual\n",
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
            "last_modified": time.strftime("%Y-%m-%d"),
            "keywords": "anycontext, help, commands, transfer, move, switch, sync, web, source, workspace, config, inspect, share, link, backup, restore, density, model, api-keys"
        },
        id_="system_help_registry_global"
    )


def build_system_readme_document(readme_path: str) -> Optional[Document]:
    """
    Reads the user-facing README.md and creates a Document for permanent Global system context.
    """
    if not os.path.exists(readme_path):
        return None

    try:
        with open(readme_path, "r", encoding="utf-8", errors="ignore") as f:
            content = f.read()

        return Document(
            text=f"# AnyContext Official User Guide & Overview\n\n{content}",
            metadata={
                "file_name": "AnyContext System Documentation (README.md)",
                "file_path": "system://readme",
                "workspace": "Global",
                "source_type": "system_help",
                "content_type": "System Documentation",
                "is_system_help": True,
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
    Uses SHA-256 content hashing to bypass indexing in < 2ms if already up-to-date.
    """
    settings = AppSettings.load()
    base_db_path = db_path or (settings.context.db_path if settings and settings.context else "./context_db")
    lance_dir = os.path.join(base_db_path, "lancedb")
    os.makedirs(lance_dir, exist_ok=True)

    lance_store = LanceDBStore.get_instance(db_path=lance_dir)

    # 1. Build synthetic documents
    help_doc = build_system_help_document()
    readme_path = _find_readme_path()
    readme_doc = build_system_readme_document(readme_path) if readme_path else None

    docs_to_index: List[Document] = [help_doc]
    if readme_doc:
        docs_to_index.append(readme_doc)

    # 2. Compute composite hash
    combined_content = "".join([d.text for d in docs_to_index])
    composite_hash = hashlib.sha256(combined_content.encode("utf-8")).hexdigest()

    # 3. Check if already indexed in LanceDB with same hash
    if not force and lance_store._has_table("workspace_chunks"):
        try:
            existing_records = lance_store.search_vector(
                query_vector=[0.0] * 1536,
                limit=20,
                workspace="Global",
                filter_expr="workspace = 'Global' AND source_type = 'system_help'"
            )
            if existing_records:
                # Check if hash matches
                existing_hashes = {r.metadata.get("content_hash") for r in existing_records if r.metadata}
                if composite_hash in existing_hashes:
                    return True
        except Exception:
            pass

    # 4. Stamp composite hash on all docs
    for doc in docs_to_index:
        doc.metadata["content_hash"] = composite_hash

    # 5. Remove any older Global system help chunks before inserting updated ones
    try:
        lance_store.delete_by_file("system://help_registry", workspace_name="Global")
        lance_store.delete_by_file("system://readme", workspace_name="Global")
    except Exception:
        pass

    # 6. Index into LanceDB under workspace='Global'
    try:
        indexer = ParallelIndexer(store=lance_store)
        cfg = IngestionConfig(chunk_size=1024, chunk_overlap=150, max_workers=4)
        indexer.index_documents(documents=docs_to_index, workspace_name="Global", config=cfg)
        return True
    except Exception as e:
        print(f"⚠️ Warning: Could not bootstrap system knowledge into LanceDB: {e}")
        return False
