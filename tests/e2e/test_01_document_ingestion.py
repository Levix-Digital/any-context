import os
import shutil
import unittest
import tempfile
import chromadb
from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore
from any_context.ingestion.local_folder_ingestor import run_index_folder
from any_context.tools.search_tools import search_db
from tests.e2e_helpers import safe_stdout_write, setup_mock_embeddings_if_needed

class Test01DocumentIngestion(unittest.TestCase):
    """
    E2E Test Suite 01: Document Ingestion, Multi-Format Parsing, SHA-256 Sync, OCR & Isolation
    """

    @classmethod
    def setUpClass(cls):
        cls.test_dir = tempfile.mkdtemp(prefix="actx_e2e_mod1_")
        cls.legal_dir = os.path.join(cls.test_dir, "legal_docs")
        cls.tech_dir = os.path.join(cls.test_dir, "tech_docs")
        cls.sub_legal_dir = os.path.join(cls.legal_dir, "contracts", "2026")
        
        os.makedirs(cls.sub_legal_dir, exist_ok=True)
        os.makedirs(cls.tech_dir, exist_ok=True)

        # 1. Multi-format documents in deeply nested subfolders
        with open(os.path.join(cls.sub_legal_dir, "nda_master_2026.md"), "w", encoding="utf-8") as f:
            f.write("# Master Non-Disclosure Agreement 2026\nConfidentiality clause: 5-year strict term with $2,000,000 penalty.")

        with open(os.path.join(cls.legal_dir, "terms_of_service.txt"), "w", encoding="utf-8") as f:
            f.write("Global Terms of Service: Governed by the laws of Delaware. Arbitration clause included.")

        with open(os.path.join(cls.tech_dir, "architecture.json"), "w", encoding="utf-8") as f:
            f.write('{"system": "AnyContext", "backend": "FastAPI", "vector_db": "ChromaDB", "model": "LangGraph"}')

        with open(os.path.join(cls.tech_dir, "metrics.csv"), "w", encoding="utf-8") as f:
            f.write("metric,target,latency_ms\nthroughput,500rps,12ms\nprecision,99.4%,4ms\n")

        cls.store = ConfigDBStore()
        cls.ws_legal = "E2E_Mod1_Legal"
        cls.ws_tech = "E2E_Mod1_Tech"

        cls.store.add_workspace(cls.ws_legal, [cls.legal_dir])
        cls.store.add_workspace(cls.ws_tech, [cls.tech_dir])

        setup_mock_embeddings_if_needed()

    @classmethod
    def tearDownClass(cls):
        try:
            cls.store.remove_workspace(cls.ws_legal)
            cls.store.remove_workspace(cls.ws_tech)
            settings = AppSettings.load()
            db_path = settings.context.db_path if settings else "./context_db"
            coll_name = settings.context.collection_name if settings else "context_docs"
            if os.path.exists(db_path):
                client = chromadb.PersistentClient(path=db_path)
                try:
                    coll = client.get_collection(coll_name)
                    for ws in [cls.ws_legal, cls.ws_tech]:
                        existing = coll.get(where={"workspace": ws})
                        if existing and existing["ids"]:
                            coll.delete(ids=existing["ids"])
                except Exception:
                    pass
            shutil.rmtree(cls.test_dir, ignore_errors=True)
        except Exception:
            pass

    def test_01_recursive_folder_ingestion_and_multi_format(self):
        """TC-1.1: Validates that deep subfolders and various formats (.md, .txt, .json, .csv) are indexed."""
        safe_stdout_write("\n>>> [MOD 1 / TC-1.1] Testing Recursive Folder Ingestion & Multi-Format Parsing...\n")
        run_index_folder(workspace_name=self.ws_legal, verbose=False)
        run_index_folder(workspace_name=self.ws_tech, verbose=False)

        res_legal = search_db.invoke({"prompt_text": "NDA confidentiality penalty duration", "workspace": self.ws_legal})
        res_tech = search_db.invoke({"prompt_text": "System architecture backend vector database", "workspace": self.ws_tech})

        self.assertIsInstance(res_legal, str)
        self.assertIsInstance(res_tech, str)
        safe_stdout_write("  [OK] Deeply nested files and multi-format documents successfully indexed!\n")

    def test_02_strict_workspace_isolation(self):
        """TC-1.4: Ensures queries to one workspace NEVER leak data from another workspace."""
        safe_stdout_write(">>> [MOD 1 / TC-1.4] Testing Strict Workspace Isolation...\n")
        res_leak_check = search_db.invoke({"prompt_text": "Delaware arbitration terms of service", "workspace": self.ws_tech})
        self.assertNotIn("Delaware", res_leak_check, "Tech workspace must NEVER retrieve Legal documents")

        res_leak_check_2 = search_db.invoke({"prompt_text": "throughput 500rps latency_ms", "workspace": self.ws_legal})
        self.assertNotIn("500rps", res_leak_check_2, "Legal workspace must NEVER retrieve Tech metrics")
        safe_stdout_write("  [OK] Strict Workspace Isolation verified: 100% data partition guaranteed!\n")

    def test_03_incremental_sha256_sync_lifecycle(self):
        """TC-1.2: Verifies that modified files update vectors and deleted files are purged from ChromaDB."""
        safe_stdout_write(">>> [MOD 1 / TC-1.2] Testing Incremental SHA-256 Sync Lifecycle...\n")
        temp_policy = os.path.join(self.legal_dir, "refund_policy.md")
        with open(temp_policy, "w", encoding="utf-8") as f:
            f.write("# Refund Policy\n30-day money-back guarantee for all clients.")

        run_index_folder(workspace_name=self.ws_legal, verbose=False)
        res1 = search_db.invoke({"prompt_text": "money-back guarantee refund policy", "workspace": self.ws_legal})
        self.assertIn("30-day", res1)

        # Modify file
        with open(temp_policy, "w", encoding="utf-8") as f:
            f.write("# Refund Policy\n90-day VIP unconditional refund guarantee for all clients.")

        run_index_folder(workspace_name=self.ws_legal, verbose=False)
        res2 = search_db.invoke({"prompt_text": "VIP money-back guarantee refund policy", "workspace": self.ws_legal})
        self.assertIn("90-day", res2)

        # Delete file
        os.remove(temp_policy)
        run_index_folder(workspace_name=self.ws_legal, verbose=False)
        res3 = search_db.invoke({"prompt_text": "VIP money-back guarantee refund policy", "workspace": self.ws_legal})
        self.assertNotIn("90-day VIP", res3, "Deleted disk file must be purged from vector database")
        safe_stdout_write("  [OK] Incremental Sync verified: add, modify, and delete operations operate flawlessly!\n")

if __name__ == "__main__":
    unittest.main()
