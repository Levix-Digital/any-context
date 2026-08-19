import os
import sys
import chromadb
from typing import List
from any_context.config.app_settings import AppSettings
from any_context.core.utils import get_api_key
from llama_index.core import Settings, SimpleDirectoryReader, Document
from llama_index.core.ingestion import IngestionPipeline, DocstoreStrategy
from llama_index.core.storage.docstore import SimpleDocumentStore
from llama_index.core.node_parser import SentenceSplitter
from llama_index.vector_stores.chroma import ChromaVectorStore
from llama_index.embeddings.openai import OpenAIEmbedding

from any_context.help.registry import HELP_REGISTRY
from langchain.tools import tool

def safe_print(msg: str):
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))

LOCAL_API_KEY = get_api_key()

settings = AppSettings.load()

db_save_path = settings.context.db_path if settings else "./context_db"
collection_name = settings.context.collection_name if settings else "context_docs"
embedding_model = settings.models.embedding_model if settings else "text-embedding-3-small"
local_base_url = settings.models.local_base_url if settings else "http://localhost:1234/v1"

from any_context.tools.search_tools import configure_embedding_model



SUPPORTED_EXTENSIONS = {
    # Documents & Text
    ".pdf", ".docx", ".doc", ".txt", ".md", ".rtf", ".odt", ".pages", ".epub", ".eml", ".msg",
    # Data & Spreadsheets
    ".csv", ".tsv", ".json", ".jsonl", ".xlsx", ".xls", ".ods",
    # Presentations
    ".pptx", ".ppt", ".key",
    # Code & Tech
    ".py", ".js", ".ts", ".tsx", ".jsx", ".html", ".htm", ".css", ".xml", ".yaml", ".yml", ".toml", ".sql",
    ".c", ".cpp", ".cs", ".java", ".go", ".rs", ".sh", ".ps1", ".bat", ".cmd",
    # Images
    ".png", ".jpg", ".jpeg", ".webp"
}

IGNORED_DIRS = {
    ".git", ".svn", "node_modules", "__pycache__", ".venv", "venv", "env",
    ".vs", ".idea", ".vscode", "$recycle.bin", ".tmp"
}

def discover_workspace_files(root_folder: str) -> List[str]:
    """
    Recursively crawls all subfolders starting from root_folder using os.walk.
    Finds ALL supported files, handling case-insensitive extensions and ignoring lock/temp files.
    """
    valid_file_paths = []
    for root, dirs, files in os.walk(root_folder):
        dirs[:] = [d for d in dirs if d.lower() not in IGNORED_DIRS and not d.startswith(".")]
        
        for file_name in files:
            if file_name.startswith("~$") or file_name.startswith("._"):
                continue  # Skip Microsoft Office temporary lock files & macOS metadata files
            
            ext = os.path.splitext(file_name)[1].lower()
            if ext in SUPPORTED_EXTENSIONS:
                full_path = os.path.abspath(os.path.join(root, file_name))
                valid_file_paths.append(full_path)
                
    return valid_file_paths

def build_help_registry_document() -> Document:
    """Constructs a comprehensive synthetic Markdown Document from the Help Module Registry."""
    text_blocks = ["# 📖 AnyContext Complete Commands & Architectural Manual\n"]
    for key, page in HELP_REGISTRY.items():
        text_blocks.append(f"## Command: {page.command} ({page.title})\n")
        text_blocks.append(f"**Aliases**: {', '.join(page.aliases)}\n")
        text_blocks.append(f"**Description**: {page.description}\n")
        text_blocks.append(f"**Syntax**: {page.syntax}\n")
        if page.parameters:
            text_blocks.append("**Parameters & Options**:\n" + "\n".join([f"- {p}" for p in page.parameters]) + "\n")
        if page.examples:
            text_blocks.append("**Usage Examples**:\n" + "\n".join([f"- {e}" for e in page.examples]) + "\n")
        if page.tips:
            text_blocks.append("**Best Practice Tips**:\n" + "\n".join([f"- {t}" for t in page.tips]) + "\n")
        text_blocks.append("\n---\n")

    full_text = "\n".join(text_blocks)
    return Document(text=full_text, metadata={"file_name": "AnyContext Command Manual & Help Registry (HELP_REGISTRY)"})

def clear_context_vector_db(verbose: bool = False):
    """
    Purges ChromaDB vector collection and docstore file to prevent dimension mismatch errors
    when embedding models are changed.
    """
    try:
        current_settings = AppSettings.load() or settings
        db_path = current_settings.context.db_path if current_settings else "./context_db"
        coll_name = current_settings.context.collection_name if current_settings else "context_docs"

        db = chromadb.PersistentClient(path=db_path)
        try:
            db.delete_collection(coll_name)
        except Exception:
            pass

        docstore_path = os.path.join(db_path, "docstore.json")
        if os.path.exists(docstore_path):
            os.remove(docstore_path)
        if verbose:
            safe_print("│ ├─ 🧹 Context vector collection and docstore cleared for re-indexing")
    except Exception as e:
        if verbose:
            safe_print(f"│ ├─ ⚠️ Warning during vector db clear: {e}")

def run_index_folder(workspace_name: str = None, verbose: bool = False):
    """
    Index documents in the vector database incrementally across all configured workspaces,
    or a specific workspace if provided. Performs deep recursive scanning across all subdirectories.
    Automatically embeds application README and Help Module Registry as permanent system self-help context.
    """
    current_settings = AppSettings.load()
    if not current_settings or not current_settings.workspaces:
        if verbose:
            safe_print("❌ Error: No workspaces configured in settings.")
        return

    configure_embedding_model()

    target_ws_name = workspace_name or (current_settings.workspaces[0].name if current_settings.workspaces else "Global")
    db_save_path = current_settings.context.db_path if (current_settings and current_settings.context) else "./context_db"
    collection_name = current_settings.context.collection_name if (current_settings and current_settings.context) else "context_docs"

    if verbose:
        safe_print(f"\n┌ 📦 \033[1mIngestion Pipeline: {target_ws_name}\033[0m")
        safe_print(f"│ ├─ 📂 Storage     : Vector Store ({db_save_path}/{collection_name})")

    db = chromadb.PersistentClient(path=db_save_path)
    collection = db.get_or_create_collection(collection_name)
    vector_store = ChromaVectorStore(chroma_collection=collection)

    docstore_path = os.path.join(db_save_path, "docstore.json")
    if os.path.exists(docstore_path):
        docstore = SimpleDocumentStore.from_persist_path(docstore_path)
    else:
        docstore = SimpleDocumentStore()

    chunk_size = current_settings.context.chunk_size if (current_settings and current_settings.context) else 1024
    chunk_overlap = current_settings.context.chunk_overlap if (current_settings and current_settings.context) else 200

    pipeline = IngestionPipeline(
        transformations = [
            SentenceSplitter(chunk_size=chunk_size, chunk_overlap=chunk_overlap),
            Settings.embed_model
        ],
        vector_store = vector_store,
        docstore = docstore,
        docstore_strategy = DocstoreStrategy.UPSERTS
    )

    all_documents = []
    total_discovered_files = 0
    scanned_file_samples = []

    # Locate application README.md for permanent system help context
    readme_path = None
    readme_candidates = [
        os.path.join(os.getcwd(), "README.md"),
        os.path.join(os.path.dirname(__file__), "..", "..", "..", "README.md"),
        os.path.join(os.path.dirname(__file__), "..", "config", "README.md")
    ]
    for cand in readme_candidates:
        if os.path.exists(cand):
            readme_path = os.path.abspath(cand)
            break

    workspaces_to_process = []
    for ws in current_settings.workspaces:
        if workspace_name and ws.name != workspace_name:
            continue
        workspaces_to_process.append(ws)
        
        ws_file_paths = []
        for folder_path in ws.paths:
            if not os.path.exists(folder_path):
                if verbose:
                    safe_print(f"│ ├─ ⚠️ Directory missing: {folder_path}")
                continue
                
            discovered_files = discover_workspace_files(folder_path)
            ws_file_paths.extend(discovered_files)
            total_discovered_files += len(discovered_files)
            scanned_file_samples.extend(discovered_files[:4])

        # Load discovered files safely
        if ws_file_paths:
            try:
                reader = SimpleDirectoryReader(input_files=ws_file_paths)
                docs = reader.load_data()
                for d in docs:
                    d.metadata["workspace"] = ws.name
                    if "file_path" in d.metadata:
                        fp = d.metadata["file_path"]
                        d.id_ = fp
                        try:
                            mtime = os.path.getmtime(fp)
                            ctime = os.path.getctime(fp)
                            d.metadata["last_modified_date"] = time.strftime("%Y-%m-%d", time.localtime(mtime))
                            d.metadata["creation_date"] = time.strftime("%Y-%m-%d", time.localtime(ctime))
                            d.metadata["content_type"] = "Local Document"
                            d.metadata["date_confidence"] = "filesystem_timestamp"
                        except Exception:
                            pass
                all_documents.extend(docs)
            except Exception:
                # Fallback: file-by-file loading if a batch contains a corrupted or locked file
                for single_file in ws_file_paths:
                    try:
                        single_reader = SimpleDirectoryReader(input_files=[single_file])
                        s_docs = single_reader.load_data()
                        for d in s_docs:
                            d.metadata["workspace"] = ws.name
                            if "file_path" in d.metadata:
                                fp = d.metadata["file_path"]
                                d.id_ = fp
                                try:
                                    mtime = os.path.getmtime(fp)
                                    ctime = os.path.getctime(fp)
                                    d.metadata["last_modified_date"] = time.strftime("%Y-%m-%d", time.localtime(mtime))
                                    d.metadata["creation_date"] = time.strftime("%Y-%m-%d", time.localtime(ctime))
                                    d.metadata["content_type"] = "Local Document"
                                    d.metadata["date_confidence"] = "filesystem_timestamp"
                                except Exception:
                                    pass
                        all_documents.extend(s_docs)
                    except Exception:
                        pass

        # Auto-inject application README.md as permanent system context for Default/Global workspace
        if ws.name in ["Default", "Global"] and readme_path:
            try:
                readme_reader = SimpleDirectoryReader(input_files=[readme_path])
                readme_docs = readme_reader.load_data()
                for rd in readme_docs:
                    rd.metadata["workspace"] = ws.name
                    rd.metadata["is_system_help"] = True
                    rd.metadata["file_name"] = "AnyContext System Documentation (README.md)"
                    rd.id_ = f"system_readme_{ws.name}"
                all_documents.extend(readme_docs)
            except Exception:
                pass

        # Auto-inject Help Module Registry as permanent system self-help context for Default/Global workspace
        if ws.name in ["Default", "Global"]:
            try:
                help_doc = build_help_registry_document()
                help_doc.metadata["workspace"] = ws.name
                help_doc.metadata["is_system_help"] = True
                help_doc.id_ = f"system_help_registry_{ws.name}"
                all_documents.append(help_doc)
            except Exception:
                pass

    if verbose:
        safe_print(f"│ ├─ 🔍 Discovery   : {total_discovered_files} files scanned across configured paths")
        for sample in scanned_file_samples[:3]:
            safe_print(f"│ │    • 📄 {os.path.basename(sample)}")
        if total_discovered_files > 3:
            safe_print(f"│ │    • ... (+ {total_discovered_files - 3} more files)")
        safe_print(f"│ ├─ 📚 Chunks      : {len(all_documents)} document nodes parsed")
        safe_print(f"│ ├─ 📖 System Help : Auto-injected README.md & Command Manual (HELP_REGISTRY)")
        curr_settings = AppSettings.load() or settings
        embed_label = curr_settings.models.embedding_model if curr_settings and curr_settings.models else "text-embedding-3-small"
        safe_print(f"│ ├─ ⚡ Embeddings  : {embed_label} (incremental check)")

    if not all_documents:
        if verbose:
            safe_print("└ ❌ No valid documents found across any workspace.\n")
        return
        
    try:
        nodes = pipeline.run(documents=all_documents, show_progress=False)
    except Exception as e:
        err_str = str(e).lower()
        if "dimension" in err_str or "invalidargumenterror" in err_str or "expecting embedding" in err_str:
            if verbose:
                safe_print("│ ├─ 🧹 Auto-clearing incompatible vector database for fresh re-indexing...")
            clear_context_vector_db(verbose=verbose)

            db = chromadb.PersistentClient(path=db_save_path)
            collection = db.get_or_create_collection(collection_name)
            vector_store = ChromaVectorStore(chroma_collection=collection)
            docstore = SimpleDocumentStore()

            pipeline = IngestionPipeline(
                transformations = [
                    SentenceSplitter(chunk_size=chunk_size, chunk_overlap=chunk_overlap),
                    Settings.embed_model
                ],
                vector_store = vector_store,
                docstore = docstore,
                docstore_strategy = DocstoreStrategy.UPSERTS
            )
            nodes = pipeline.run(documents=all_documents, show_progress=False)
        elif "no embedding data" in err_str or "connection" in err_str:
            if verbose:
                safe_print("└ ❌ Error generating embeddings: endpoint did not return embedding data.\n")
            return
        else:
            raise e

    current_doc_ids = {doc.doc_id for doc in all_documents}
    processed_workspace_names = {ws.name for ws in workspaces_to_process}
    
    deleted_count = 0
    for node_id, node in list(docstore.docs.items()):
        node_workspace = node.metadata.get("workspace") if node.metadata else None
        
        if not node_workspace or node_workspace in processed_workspace_names:
            if getattr(node, "ref_doc_id", None) not in current_doc_ids and node.id_ not in current_doc_ids:
                try:
                    docstore.delete_document(node_id)
                    vector_store.delete(node_id)
                    deleted_count += 1
                except Exception:
                    pass

    if verbose:
        safe_print(f"│ └─ 🧹 Maintenance : {deleted_count} outdated chunks purged")
        safe_print(f"└ \033[92m✔ Ingestion completed successfully!\033[0m\n")

    docstore.persist(persist_path=docstore_path)


@tool()
def index_folder(workspace_name: str = None, verbose: bool = False):
    """
    Index documents in the vector database incrementally across all configured workspaces,
    or a specific workspace if provided. Performs deep recursive scanning across all subdirectories.
    Automatically embeds application README and Help Module Registry as permanent system self-help context.
    """
    return run_index_folder(workspace_name=workspace_name, verbose=verbose)


if __name__ == "__main__":
    run_index_folder(verbose=True)

