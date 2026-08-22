import os
import sys
import unittest
import tempfile
import chromadb

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.db_store import ConfigDBStore
from any_context.ingestion.web_scheduler import WebSchedulerStore
from tests.e2e_helpers import safe_stdout_write

class TestSourceTransfer(unittest.TestCase):
    """
    Core Unit Test Suite: Validates Instant Zero-Cost Data Source Transfers between Workspaces.
    Tests moving local folders and crawled web portals (SQLite + ChromaDB metadata migration).
    """

    @classmethod
    def setUpClass(cls):
        from tests.e2e_helpers import setup_mock_embeddings_if_needed
        setup_mock_embeddings_if_needed()

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="actx_test_transfer_")
        self.db_path = os.path.join(self.temp_dir, "test_settings.db")
        self.store = ConfigDBStore(db_path=self.db_path)
        self.web_store = WebSchedulerStore(db_path=self.db_path)

        self.source_ws = "Transfer_Source_WS"
        self.target_ws = "Transfer_Target_WS"
        self.test_folder = os.path.join(self.temp_dir, "legal_docs")
        os.makedirs(self.test_folder, exist_ok=True)

        self.store.add_workspace(self.source_ws, paths=[self.test_folder])
        self.store.add_workspace(self.target_ws, paths=[])

    def tearDown(self):
        try:
            self.store.remove_workspace(self.source_ws)
            self.store.remove_workspace(self.target_ws)
        except Exception:
            pass

    def test_01_transfer_local_folder_source(self):
        """Tests transferring a local folder from source_ws to target_ws with ChromaDB vector migration."""
        safe_stdout_write("\n>>> [CORE UNIT] Testing Local Folder Source Transfer...\n")
        
        # 1. Setup mock vector chunk in ChromaDB
        from any_context.config.app_settings import AppSettings
        settings = AppSettings.load()
        chroma_dir = settings.context.db_path if (settings and settings.context) else "./context_db"
        coll_name = settings.context.collection_name if (settings and settings.context) else "context_docs"
        
        client = chromadb.PersistentClient(path=chroma_dir)
        coll = client.get_or_create_collection(coll_name)
        mock_id = f"test_transfer_chunk_{self.source_ws}"
        coll.upsert(
            ids=[mock_id],
            documents=["Sample legal contract clause 101"],
            metadatas=[{
                "workspace": self.source_ws,
                "file_path": os.path.join(self.test_folder, "contract.pdf"),
                "source": os.path.join(self.test_folder, "contract.pdf")
            }],
            embeddings=[[0.01] * 1536]
        )

        from any_context.vector_engine.store import LanceDBStore
        l_store = LanceDBStore.get_instance(db_path=os.path.join(chroma_dir, "lancedb"))
        l_store.upsert_records([{
            "id": mock_id,
            "vector": [0.01] * 1536,
            "text": "Sample legal contract clause 101",
            "file_name": "contract.pdf",
            "file_path": os.path.join(self.test_folder, "contract.pdf"),
            "workspace": self.source_ws,
            "last_modified": "2026-08-22",
            "content_type": "Local Document",
            "document_summary": "",
            "keywords": "",
            "content_hash": ""
        }])

        # 2. Execute transfer
        res = self.store.transfer_local_folder_source(
            source_ws=self.source_ws,
            target_ws=self.target_ws,
            folder_path=self.test_folder
        )

        self.assertTrue(res["success"], f"Transfer failed: {res.get('error')}")
        self.assertGreaterEqual(res["transferred_chunks"], 1)

        # 3. Verify SQLite paths
        src_obj = next((w for w in self.store.get_app_settings().workspaces if w.name == self.source_ws), None)
        tgt_obj = next((w for w in self.store.get_app_settings().workspaces if w.name == self.target_ws), None)

        self.assertNotIn(os.path.abspath(self.test_folder), [os.path.abspath(p) for p in src_obj.paths])
        self.assertIn(os.path.abspath(self.test_folder), [os.path.abspath(p) for p in tgt_obj.paths])

        # 4. Verify LanceDB metadata
        table = l_store._db.open_table(l_store._default_table_name)
        chunk_data = table.search().where(f"id = '{mock_id}'").to_list()
        self.assertEqual(chunk_data[0]["workspace"], self.target_ws)

        # Clean up mock chunk
        try:
            l_store.delete_by_id(mock_id)
        except Exception:
            pass

        safe_stdout_write("  [OK] Local folder transfer & LanceDB metadata migration verified!\n")

    def test_02_transfer_web_source(self):
        """Tests transferring a web source and indexed sub-pages with ChromaDB vector migration."""
        safe_stdout_write(">>> [CORE UNIT] Testing Web Source Transfer...\n")
        test_url = "https://example.com/docs"
        
        # 1. Setup web source & pages in SQLite
        self.web_store.add_web_url(workspace_name=self.source_ws, url=test_url, title="Example Docs")
        self.web_store.record_indexed_web_pages(
            workspace_name=self.source_ws,
            root_url=test_url,
            pages=[
                {"url": "https://example.com/docs/page1", "title": "Page 1", "content_hash": "hash1", "char_count": 100},
                {"url": "https://example.com/docs/page2", "title": "Page 2", "content_hash": "hash2", "char_count": 200}
            ]
        )

        # Setup mock vector chunk in ChromaDB
        from any_context.config.app_settings import AppSettings
        settings = AppSettings.load()
        chroma_dir = settings.context.db_path if (settings and settings.context) else "./context_db"
        coll_name = settings.context.collection_name if (settings and settings.context) else "context_docs"

        client = chromadb.PersistentClient(path=chroma_dir)
        coll = client.get_or_create_collection(coll_name)
        mock_web_id = f"test_web_chunk_{self.source_ws}"
        coll.upsert(
            ids=[mock_web_id],
            documents=["Sample documentation content"],
            metadatas=[{
                "workspace": self.source_ws,
                "root_url": test_url,
                "url": "https://example.com/docs/page1",
                "source_type": "web"
            }],
            embeddings=[[0.01] * 1536]
        )

        from any_context.vector_engine.store import LanceDBStore
        l_store = LanceDBStore.get_instance(db_path=os.path.join(chroma_dir, "lancedb"))
        l_store.upsert_records([{
            "id": mock_web_id,
            "vector": [0.01] * 1536,
            "text": "Sample documentation content",
            "file_name": "Example Docs",
            "file_path": test_url,
            "workspace": self.source_ws,
            "last_modified": "2026-08-22",
            "content_type": "Web Documentation",
            "document_summary": "",
            "keywords": "",
            "content_hash": ""
        }])

        # 2. Execute transfer
        res = self.web_store.transfer_web_source(
            source_ws=self.source_ws,
            target_ws=self.target_ws,
            url_or_root=test_url
        )

        self.assertTrue(res["success"], f"Web transfer failed: {res.get('error')}")
        self.assertGreaterEqual(res["transferred_pages"], 1)
        self.assertGreaterEqual(res["transferred_chunks"], 1)

        # 3. Verify SQLite records
        src_urls = self.web_store.get_workspace_web_urls(self.source_ws)
        tgt_urls = self.web_store.get_workspace_web_urls(self.target_ws)
        self.assertEqual(len(src_urls), 0)
        self.assertEqual(len(tgt_urls), 1)
        self.assertEqual(tgt_urls[0]["url"], test_url)

        # 4. Verify LanceDB metadata
        table = l_store._db.open_table(l_store._default_table_name)
        chunk_data = table.search().where(f"id = '{mock_web_id}'").to_list()
        self.assertEqual(chunk_data[0]["workspace"], self.target_ws)

        # Clean up
        try:
            l_store.delete_by_id(mock_web_id)
        except Exception:
            pass

        safe_stdout_write("  [OK] Web source transfer & LanceDB metadata migration verified!\n")

    def test_03_transfer_validation_guards(self):
        """Tests guardrails for invalid source/target parameters."""
        safe_stdout_write(">>> [CORE UNIT] Testing Transfer Guardrails & Validations...\n")
        
        # Same workspace
        res1 = self.store.transfer_local_folder_source(self.source_ws, self.source_ws, self.test_folder)
        self.assertFalse(res1["success"])

        # Non-existent target
        res2 = self.store.transfer_local_folder_source(self.source_ws, "NonExistent_WS", self.test_folder)
        self.assertFalse(res2["success"])

        safe_stdout_write("  [OK] Transfer validation guardrails verified!\n")

    def test_04_workspace_sharing_get_permissions(self):
        """Tests that WorkspaceSharingStore.get_workspace_permissions returns collaborators list properly."""
        safe_stdout_write(">>> [CORE UNIT] Testing WorkspaceSharingStore.get_workspace_permissions...\n")
        from any_context.workspace_sharing.store import WorkspaceSharingStore
        sharing_store = WorkspaceSharingStore(db_path=self.db_path)
        
        # Should return empty list when no collaborators
        perms = sharing_store.get_workspace_permissions(self.source_ws)
        self.assertIsInstance(perms, list)
        self.assertEqual(len(perms), 0)

        # Grant direct permission and verify
        sharing_store.grant_direct_permission(self.source_ws, "user@example.com", "editor", "admin@local")
        perms_after = sharing_store.get_workspace_permissions(self.source_ws)
        self.assertEqual(len(perms_after), 1)
        self.assertEqual(perms_after[0].user_email, "user@example.com")
        self.assertEqual(perms_after[0].access_level, "editor")
        safe_stdout_write("  [OK] WorkspaceSharingStore.get_workspace_permissions verified!\n")

    def test_05_transfer_folder_with_quotes_and_spaces(self):
        """Tests that transferring a folder path wrapped in literal quotes with spaces resolves and migrates properly."""
        safe_stdout_write(">>> [CORE UNIT] Testing Folder Transfer with Quotes & Spaces...\n")
        space_folder = os.path.join(self.temp_dir, "My Drive", "Levix Digital", "VentureHub")
        os.makedirs(space_folder, exist_ok=True)

        self.store.add_workspace("WS_With_Spaces", paths=[space_folder])
        self.store.add_workspace("WS_Target_Spaces", paths=[])

        try:
            # Transfer using path wrapped in quotes like '"G:\My Drive\..."'
            quoted_input = f'"{space_folder}"'
            res = self.store.transfer_local_folder_source(
                source_ws="WS_With_Spaces",
                target_ws="WS_Target_Spaces",
                folder_path=quoted_input
            )
            self.assertTrue(res["success"], f"Transfer failed: {res.get('error')}")
            
            # Verify source has 0 paths
            settings = self.store.get_app_settings()
            src_ws_obj = next((w for w in settings.workspaces if w.name == "WS_With_Spaces"), None)
            self.assertEqual(len(src_ws_obj.paths), 0)

            # Verify target has the clean absolute path without quotes or prepended cwd
            tgt_ws_obj = next((w for w in settings.workspaces if w.name == "WS_Target_Spaces"), None)
            self.assertEqual(len(tgt_ws_obj.paths), 1)
            self.assertEqual(os.path.abspath(tgt_ws_obj.paths[0]), os.path.abspath(space_folder))
            self.assertFalse(tgt_ws_obj.paths[0].startswith('"'))
            self.assertFalse('"' in tgt_ws_obj.paths[0])
            safe_stdout_write("  [OK] Folder transfer with quotes & spaces verified!\n")
        finally:
            self.store.remove_workspace("WS_With_Spaces")
            self.store.remove_workspace("WS_Target_Spaces")


if __name__ == "__main__":
    unittest.main()
