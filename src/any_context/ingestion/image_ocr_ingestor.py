import os
import hashlib
from typing import Dict, Any, Optional
from any_context.billing import BillingManager

def extract_text_from_image(image_path: str) -> Dict[str, Any]:
    """
    Extracts text content from image files (.png, .jpg, .jpeg, .webp, .tiff, .bmp) using OCR engines.
    Uses pytesseract if available, or falls back to basic image metadata text extraction.
    """
    if not os.path.exists(image_path):
        raise FileNotFoundError(f"Image file not found at: {image_path}")

    filename = os.path.basename(image_path)
    extracted_text = ""

    # Attempt pytesseract OCR extraction
    try:
        from PIL import Image
        import pytesseract
        img = Image.open(image_path)
        extracted_text = pytesseract.image_to_string(img).strip()
    except Exception:
        # Fallback metadata representation if pytesseract binaries are not on system PATH
        try:
            from PIL import Image
            img = Image.open(image_path)
            w, h = img.size
            mode = img.mode
            extracted_text = f"[Image Metadata File: {filename} | Resolution: {w}x{h} px | Format: {img.format} | Color Mode: {mode}]"
        except Exception:
            extracted_text = f"[Image File: {filename}]"

    content_hash = hashlib.sha256(extracted_text.encode("utf-8")).hexdigest()

    return {
        "file_path": image_path,
        "filename": filename,
        "content": extracted_text,
        "hash": content_hash,
        "char_count": len(extracted_text)
    }

def index_image_file_to_chromadb(workspace_name: str, image_path: str) -> bool:
    """
    Extracts text from an image file and indexes it into the workspace vector store (LanceDB)
    if the active plan tier supports OCR. Enforces feature gates via BillingManager.
    """
    b_mgr = BillingManager()
    if not b_mgr.can_use_ocr():
        print("⚠️ Billing Gate: Image & Scanned PDF OCR requires 'Starter', 'Pro', 'Team', or 'Enterprise' tier.")
        return False

    try:
        data = extract_text_from_image(image_path)
        if not data["content"]:
            return False

        from llama_index.core import Document
        from any_context.vector_engine.store import LanceDBStore
        from any_context.vector_engine.indexer import ParallelIndexer, IngestionConfig
        from any_context.config.app_settings import AppSettings

        settings = AppSettings.load()
        db_path = settings.database.db_path if settings and settings.database else "./context_db"
        lance_store = LanceDBStore.get_instance(db_path=os.path.join(db_path, "lancedb"))
        indexer = ParallelIndexer(store=lance_store)

        doc = Document(
            text=f"Image Document: {data['filename']}\nFilePath: {data['file_path']}\n\n{data['content']}",
            metadata={"source": data["file_path"], "type": "image_ocr", "filename": data["filename"], "file_path": data["file_path"]}
        )
        cfg = IngestionConfig(chunk_size=512, chunk_overlap=50, max_workers=1)
        indexer.index_documents(documents=[doc], workspace_name=workspace_name, config=cfg)
        return True
    except Exception as e:
        print(f"❌ Error indexing image file '{image_path}': {e}")
        return False


# Backward-compatible alias
index_image_file = index_image_file_to_chromadb
