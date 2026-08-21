import os
import sys
import unittest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from fastapi.testclient import TestClient
from any_context.server.api import create_app
from any_context.config.db_store import ConfigDBStore
from tests.e2e_helpers import safe_stdout_write, setup_mock_embeddings_if_needed

class Test07RestApiServer(unittest.TestCase):
    """
    E2E Test Suite 07: REST API Server Endpoints (FastAPI actx --serve), Swagger UI & Workspace Endpoints
    """

    @classmethod
    def setUpClass(cls):
        import tempfile
        import shutil
        from any_context.config.app_settings import ContextSettings
        setup_mock_embeddings_if_needed()
        cls.test_dir = tempfile.mkdtemp(prefix="anycontext_e2e_rest_")
        cls.db_dir = os.path.join(cls.test_dir, "context_db")
        os.makedirs(cls.db_dir, exist_ok=True)
        cls.store = ConfigDBStore()
        cls.orig_settings = cls.store.get_app_settings()
        cls.store.update_context_settings(ContextSettings(db_path=cls.db_dir, collection_name="rest_api_docs"))
        cls.app = create_app()
        cls.client = TestClient(cls.app)
        cls.test_ws = "E2E_Mod7_RestApi"
        cls.store.add_workspace(cls.test_ws, [])

        token_entry = cls.store.create_access_token(
            name="E2E REST API Admin Token",
            role="admin",
            allowed_workspaces=["*"]
        )
        cls.admin_token = token_entry["token_id"]
        cls.headers = {"Authorization": f"Bearer {cls.admin_token}"}

    @classmethod
    def tearDownClass(cls):
        import shutil
        try:
            cls.store.remove_workspace(cls.test_ws)
            cls.store.delete_access_token(cls.admin_token)
            if hasattr(cls, "orig_settings") and cls.orig_settings and cls.orig_settings.context:
                cls.store.update_context_settings(cls.orig_settings.context)
            if hasattr(cls, "test_dir") and os.path.exists(cls.test_dir):
                shutil.rmtree(cls.test_dir, ignore_errors=True)
        except Exception:
            pass

    def test_01_health_and_docs_endpoints(self):
        """TC-7.1: Tests /v1/health and Swagger UI redirect."""
        safe_stdout_write("\n>>> [MOD 7 / TC-7.1] Testing Health & Documentation Endpoints...\n")
        res_health = self.client.get("/v1/health")
        self.assertEqual(res_health.status_code, 200)
        data = res_health.json()
        self.assertEqual(data["status"], "ok")
        self.assertIn("version", data)

        res_docs = self.client.get("/docs")
        self.assertEqual(res_docs.status_code, 200)
        safe_stdout_write("  [OK] /v1/health & /docs Swagger UI endpoints verified!\n")

    def test_02_auth_status_endpoint(self):
        """TC-7.1: Tests /v1/auth/status endpoint reporting security mode."""
        safe_stdout_write(">>> [MOD 7 / TC-7.1] Testing Auth Status Endpoint...\n")
        res = self.client.get("/v1/auth/status")
        self.assertEqual(res.status_code, 200)
        data = res.json()
        self.assertIn("admin_configured", data)
        self.assertIn("security_enforced", data)
        self.assertIn("mode", data)
        safe_stdout_write("  [OK] /v1/auth/status endpoint verified!\n")

    def test_03_workspaces_crud_endpoints(self):
        """TC-7.2: Tests creating workspaces (with or without folders), listing, and triggering background indexing via REST API."""
        safe_stdout_write(">>> [MOD 7 / TC-7.2] Testing Workspace REST API Endpoints...\n")
        # 1. Create Empty Workspace
        res_create = self.client.post("/v1/workspaces?name=E2E_Empty_Workspace", headers=self.headers)
        self.assertEqual(res_create.status_code, 200)
        create_data = res_create.json()
        self.assertEqual(create_data["name"], "E2E_Empty_Workspace")
        self.assertIn("id", create_data)
        self.assertEqual(create_data["sources"], [])
        self.assertEqual(create_data["total_sources"], 0)

        # 2. Add web source via WebSchedulerStore
        from any_context.ingestion.web_scheduler import WebSchedulerStore
        web_store = WebSchedulerStore()
        web_store.add_or_update_root_web_source(
            workspace_name=self.test_ws,
            root_url="https://canada.ca/en/immigration",
            title="Canada Immigration",
            page_count=10,
            scope="section"
        )

        # 3. Add cloud drive via REST API endpoint
        res_cd = self.client.post(
            f"/v1/workspaces/{self.test_ws}/cloud-drives",
            json={"provider": "google_drive", "mount_path_or_id": "gdrive://folder-test-123", "title": "Google Drive Docs"},
            headers=self.headers
        )
        self.assertEqual(res_cd.status_code, 200)
        cd_data = res_cd.json()
        self.assertEqual(cd_data["status"], "success")
        drive_id = cd_data["cloud_drive"]["id"]

        # 4. List Workspaces (verifying multi-source parity)
        res_list = self.client.get("/v1/workspaces", headers=self.headers)
        self.assertEqual(res_list.status_code, 200)
        data = res_list.json()
        workspaces = data.get("workspaces", [])
        ws_names = [w["name"] if isinstance(w, dict) else w for w in workspaces]
        self.assertIn(self.test_ws, ws_names)
        self.assertIn("E2E_Empty_Workspace", ws_names)

        target_ws_dto = next((w for w in workspaces if w["name"] == self.test_ws), None)
        self.assertIsNotNone(target_ws_dto)
        self.assertIn("id", target_ws_dto)
        self.assertGreaterEqual(target_ws_dto["total_sources"], 2)
        source_types = {s["type"] for s in target_ws_dto["sources"]}
        self.assertIn("web", source_types)
        self.assertIn("cloud_drive", source_types)

        # 5. Get Workspace Detail & Sources endpoints
        res_single = self.client.get(f"/v1/workspaces/{self.test_ws}", headers=self.headers)
        self.assertEqual(res_single.status_code, 200)
        single_data = res_single.json()
        self.assertEqual(single_data["name"], self.test_ws)
        self.assertIn("id", single_data)
        self.assertGreaterEqual(len(single_data["sources"]), 2)

        res_sources = self.client.get(f"/v1/workspaces/{self.test_ws}/sources", headers=self.headers)
        self.assertEqual(res_sources.status_code, 200)
        src_breakdown = res_sources.json()
        self.assertEqual(src_breakdown["name"], self.test_ws)
        self.assertIn("id", src_breakdown)
        self.assertGreaterEqual(src_breakdown["total_sources"], 2)
        self.assertGreaterEqual(len(src_breakdown["sources"]), 2)

        # 6. Delete cloud drive
        res_del_cd = self.client.delete(f"/v1/workspaces/{self.test_ws}/cloud-drives/{drive_id}", headers=self.headers)
        self.assertEqual(res_del_cd.status_code, 200)

        # 7. Trigger Indexing
        res_idx = self.client.post("/v1/index", json={"workspace": self.test_ws}, headers=self.headers)
        self.assertEqual(res_idx.status_code, 200)
        idx_data = res_idx.json()
        self.assertEqual(idx_data["status"], "accepted")
        safe_stdout_write("  [OK] Workspace creation, multi-source listing, cloud drive CRUD and background index REST endpoints verified!\n")

    def test_04_billing_and_plans_endpoints(self):
        """TC-7.6: Tests /v1/billing/status and /v1/billing/plans licensing endpoints."""
        safe_stdout_write(">>> [MOD 7 / TC-7.6] Testing Billing & Licensing Endpoints...\n")
        res_status = self.client.get("/v1/billing/status")
        self.assertEqual(res_status.status_code, 200)
        status_data = res_status.json()
        self.assertIn("active_tier_id", status_data)

        res_plans = self.client.get("/v1/billing/plans")
        self.assertEqual(res_plans.status_code, 200)
        data = res_plans.json()
        plans = data["plans"] if isinstance(data, dict) and "plans" in data else data
        self.assertIsInstance(plans, list)
        self.assertGreaterEqual(len(plans), 1)
        safe_stdout_write("  [OK] Billing status and licensing plans endpoints verified!\n")

    def test_05_transfer_source_endpoint(self):
        """TC-7.7: Tests POST /v1/workspaces/transfer REST API endpoint."""
        safe_stdout_write(">>> [MOD 7 / TC-7.7] Testing /v1/workspaces/transfer REST Endpoint...\n")
        src_ws = "E2E_Api_Transfer_Src"
        tgt_ws = "E2E_Api_Transfer_Tgt"
        test_folder = os.path.abspath(os.path.join(os.getcwd(), "test_api_transfer_folder"))
        os.makedirs(test_folder, exist_ok=True)

        self.store.add_workspace(src_ws, paths=[test_folder])
        self.store.add_workspace(tgt_ws, paths=[])

        try:
            payload = {
                "source_workspace": src_ws,
                "target_workspace": tgt_ws,
                "source_type": "folder",
                "source_path_or_url": test_folder
            }
            res = self.client.post("/v1/workspaces/transfer", json=payload, headers=self.headers)
            self.assertEqual(res.status_code, 200, f"API Transfer failed: {res.text}")
            data = res.json()
            self.assertEqual(data["status"], "success")
            self.assertEqual(data["source_workspace"], src_ws)
            self.assertEqual(data["target_workspace"], tgt_ws)
            self.assertEqual(data["api_embedding_cost"], "$0.00")

            settings = self.store.get_app_settings()
            src_obj = next((w for w in settings.workspaces if w.name == src_ws), None)
            tgt_obj = next((w for w in settings.workspaces if w.name == tgt_ws), None)
            self.assertNotIn(test_folder, [os.path.abspath(p) for p in src_obj.paths])
            self.assertIn(test_folder, [os.path.abspath(p) for p in tgt_obj.paths])
            safe_stdout_write("  [OK] POST /v1/workspaces/transfer REST API verified!\n")
        finally:
            self.store.remove_workspace(src_ws)
            self.store.remove_workspace(tgt_ws)
            try:
                os.rmdir(test_folder)
            except Exception:
                pass

    def test_06_context_settings_and_preset_endpoints(self):
        """TC-7.8: Tests GET and POST /v1/context/settings for retrieval density presets and parameters."""
        safe_stdout_write(">>> [MOD 7 / TC-7.8] Testing /v1/context/settings REST Endpoints...\n")
        
        # 1. GET /v1/context/settings
        res = self.client.get("/v1/context/settings", headers=self.headers)
        self.assertEqual(res.status_code, 200, f"GET /v1/context/settings failed: {res.text}")
        data = res.json()
        self.assertIn("retrieval_preset", data)
        self.assertIn("top_k", data)
        self.assertIn("candidate_pool_size", data)
        self.assertIn("max_chunks_per_source", data)

        # 2. POST /v1/context/settings (Apply Turbo Preset)
        payload = {"preset": "turbo"}
        res = self.client.post("/v1/context/settings", json=payload, headers=self.headers)
        self.assertEqual(res.status_code, 200, f"POST /v1/context/settings failed: {res.text}")
        updated = res.json()
        self.assertEqual(updated["retrieval_preset"], "turbo")
        self.assertEqual(updated["top_k"], 20)
        self.assertEqual(updated["candidate_pool_size"], 50)
        self.assertEqual(updated["max_chunks_per_source"], 2)

        # 3. POST /v1/context/settings (Restore Balanced Preset)
        payload_balanced = {"preset": "balanced"}
        res_balanced = self.client.post("/v1/context/settings", json=payload_balanced, headers=self.headers)
        self.assertEqual(res_balanced.status_code, 200)
        self.assertEqual(res_balanced.json()["retrieval_preset"], "balanced")
        self.assertEqual(res_balanced.json()["top_k"], 40)
        safe_stdout_write("  [OK] /v1/context/settings GET and POST preset endpoints verified!\n")

    def test_07_rename_workspace_endpoint(self):
        """TC-7.9: Tests POST /v1/workspaces/rename endpoint for zero-cost workspace renaming."""
        safe_stdout_write(">>> [MOD 7 / TC-7.9] Testing /v1/workspaces/rename REST Endpoint...\n")
        orig_name = "api_rename_src"
        new_name = "api_rename_tgt"

        # Create workspace first
        self.store.add_workspace(orig_name, paths=[])

        try:
            payload = {
                "old_name": orig_name,
                "new_name": new_name
            }
            res = self.client.post("/v1/workspaces/rename", json=payload, headers=self.headers)
            self.assertEqual(res.status_code, 200, f"POST /v1/workspaces/rename failed: {res.text}")
            data = res.json()
            self.assertEqual(data["status"], "success")
            self.assertEqual(data["old_workspace"], orig_name)
            self.assertEqual(data["new_workspace"], new_name)
            self.assertEqual(data["api_cost"], "$0.00")

            # Verify in DB
            settings = self.store.get_app_settings()
            ws_names = [w.name for w in settings.workspaces]
            self.assertNotIn(orig_name, ws_names)
            self.assertIn(new_name, ws_names)

            # Test Analyst Token Rename on Assigned Workspace
            import uuid
            rand_suffix = uuid.uuid4().hex[:6]
            analyst_email = f"analyst_{rand_suffix}@firm.com"
            analyst_ws = f"analyst_ws_{rand_suffix}"
            analyst_ws_new = f"analyst_ws_renamed_{rand_suffix}"
            self.store.add_workspace(analyst_ws, paths=[])
            u_info = self.store.create_user("AnalystTest", analyst_email, "pass12345", role="analyst", allowed_workspaces=["Default", analyst_ws])
            auth_info = self.store.authenticate_user(analyst_email, "pass12345")
            analyst_headers = {"Authorization": f"Bearer {auth_info['token_id']}"}

            analyst_payload = {"old_name": analyst_ws, "new_name": analyst_ws_new}
            analyst_res = self.client.post("/v1/workspaces/rename", json=analyst_payload, headers=analyst_headers)
            self.assertEqual(analyst_res.status_code, 200, f"Analyst rename failed: {analyst_res.text}")

            # Verify that list_users now reflects the new name for the analyst
            u_list = self.store.list_users()
            matching_u = next((u for u in u_list if u["email"] == analyst_email), None)
            self.assertIsNotNone(matching_u)
            self.assertIn(analyst_ws_new, matching_u["allowed_workspaces"])

            safe_stdout_write("  [OK] POST /v1/workspaces/rename REST API verified for Admin and Analyst!\n")
        finally:
            self.store.remove_workspace(orig_name)
            self.store.remove_workspace(new_name)
            try:
                self.store.remove_workspace(analyst_ws)
                self.store.remove_workspace(analyst_ws_new)
                if 'u_info' in locals():
                    self.store.delete_user(u_info["user_id"])
            except Exception:
                pass

    def test_08_grounding_mode_endpoints(self):
        """TC-7.10: Tests GET and POST /v1/context/mode endpoints for AI Grounding Mode switching."""
        safe_stdout_write(">>> [MOD 7 / TC-7.10] Testing /v1/context/mode REST Endpoints...\n")

        # 1. GET /v1/context/mode
        res = self.client.get("/v1/context/mode", headers=self.headers)
        self.assertEqual(res.status_code, 200, f"GET /v1/context/mode failed: {res.text}")
        data = res.json()
        self.assertIn("mode", data)
        self.assertIn("available_modes", data)
        self.assertIn("hybrid", data["available_modes"])
        self.assertIn("strict", data["available_modes"])
        self.assertIn("proactive", data["available_modes"])

        # 2. POST /v1/context/mode (Switch to Strict)
        res_post = self.client.post("/v1/context/mode", json={"mode": "strict"}, headers=self.headers)
        self.assertEqual(res_post.status_code, 200)
        self.assertEqual(res_post.json()["mode"], "strict")
        self.assertEqual(self.store.get_grounding_mode(), "strict")

        # 3. POST /v1/context/mode (Switch back to Hybrid)
        res_post_hyb = self.client.post("/v1/context/mode", json={"mode": "hybrid"}, headers=self.headers)
        self.assertEqual(res_post_hyb.status_code, 200)
        self.assertEqual(res_post_hyb.json()["mode"], "hybrid")
        self.assertEqual(self.store.get_grounding_mode(), "hybrid")
        safe_stdout_write("  [OK] /v1/context/mode GET and POST endpoints verified!\n")

    def test_09_shared_sources_endpoints(self):
        """TC-7.11: Tests GET/POST endpoints for Shared Sources linking and unlinking across workspaces."""
        safe_stdout_write(">>> [MOD 7 / TC-7.11] Testing Shared Sources REST Endpoints...\n")
        import tempfile
        import shutil
        temp_d = tempfile.mkdtemp()
        ws_target = "REST_Shared_Target"
        test_folder = os.path.abspath(os.path.join(temp_d, "rest_shared_docs"))
        os.makedirs(test_folder, exist_ok=True)

        try:
            self.store.add_folder_to_workspace("Shared Sources", test_folder)
            self.store.add_workspace(ws_target, paths=[])

            # 1. GET /v1/workspaces/shared-sources/available
            res_avail = self.client.get("/v1/workspaces/shared-sources/available", headers=self.headers)
            self.assertEqual(res_avail.status_code, 200)
            avail_data = res_avail.json()
            self.assertIn("sources", avail_data)
            matching = next((s for s in avail_data["sources"] if s["identifier"] == test_folder), None)
            self.assertIsNotNone(matching)

            # 2. POST /v1/workspaces/{target}/shared-sources/link
            link_payload = {
                "source_type": "folder",
                "source_identifier": test_folder,
                "title": "REST Shared Docs"
            }
            res_link = self.client.post(f"/v1/workspaces/{ws_target}/shared-sources/link", json=link_payload, headers=self.headers)
            self.assertEqual(res_link.status_code, 200, f"Link failed: {res_link.text}")
            self.assertEqual(res_link.json()["status"], "success")

            # 3. GET /v1/workspaces/{target}/shared-sources
            res_list = self.client.get(f"/v1/workspaces/{ws_target}/shared-sources", headers=self.headers)
            self.assertEqual(res_list.status_code, 200)
            self.assertEqual(len(res_list.json()["shared_links"]), 1)

            # 4. POST /v1/workspaces/{target}/shared-sources/unlink
            unlink_payload = {
                "source_type": "folder",
                "source_identifier": test_folder
            }
            res_unlink = self.client.post(f"/v1/workspaces/{ws_target}/shared-sources/unlink", json=unlink_payload, headers=self.headers)
            self.assertEqual(res_unlink.status_code, 200)
            self.assertEqual(res_unlink.json()["status"], "success")

        finally:
            self.store.remove_folder_from_workspace("Shared Sources", test_folder)
            self.store.remove_workspace(ws_target)
            try:
                shutil.rmtree(temp_d, ignore_errors=True)
            except Exception:
                pass

    def test_10_broadcast_source_linking_endpoint(self):
        """TC-7.12: Tests POST /v1/workspaces/{workspace_name}/folders endpoint with link_to_workspaces broadcast."""
        safe_stdout_write(">>> [MOD 7 / TC-7.12] Testing POST /v1/workspaces/{workspace_name}/folders Broadcast Endpoint...\n")
        import tempfile
        import shutil
        temp_d = tempfile.mkdtemp()
        ws_prim = "REST_Broadcast_Prim"
        ws_sub = "REST_Broadcast_Sub"
        test_folder = os.path.abspath(os.path.join(temp_d, "rest_broadcast_folder"))
        os.makedirs(test_folder, exist_ok=True)

        try:
            self.store.add_workspace(ws_prim, paths=[])
            self.store.add_workspace(ws_sub, paths=[])

            payload = {
                "folder_path": test_folder,
                "link_to_workspaces": [ws_sub]
            }
            res = self.client.post(f"/v1/workspaces/{ws_prim}/folders", json=payload, headers=self.headers)
            self.assertEqual(res.status_code, 200, f"Folder add failed: {res.text}")
            data = res.json()
            self.assertEqual(data["status"], "success")
            self.assertEqual(data["total_linked"], 1)

            # Verify sub workspace has linked source
            res_sub = self.client.get(f"/v1/workspaces/{ws_sub}/shared-sources", headers=self.headers)
            self.assertEqual(res_sub.status_code, 200)
            self.assertEqual(len(res_sub.json()["shared_links"]), 1)

            safe_stdout_write("  [OK] POST /v1/workspaces/{workspace_name}/folders broadcast endpoint verified!\n")
        finally:
            self.store.remove_workspace(ws_prim)
            self.store.remove_workspace(ws_sub)
            try:
                shutil.rmtree(temp_d, ignore_errors=True)
            except Exception:
                pass

    def test_11_sync_status_and_trigger_endpoints(self):
        """TC-7.13: Tests GET /v1/workspaces/{name}/sync/status and POST /v1/workspaces/{name}/sync REST endpoints."""
        safe_stdout_write(">>> [MOD 7 / TC-7.13] Testing Workspace Sync Status & Trigger REST Endpoints...\n")
        import tempfile
        import shutil
        temp_d = tempfile.mkdtemp()
        ws_test = "REST_Sync_Test"
        test_folder = os.path.abspath(os.path.join(temp_d, "rest_sync_folder"))
        os.makedirs(test_folder, exist_ok=True)
        doc_f = os.path.join(test_folder, "test_doc.md")
        with open(doc_f, "w", encoding="utf-8") as f:
            f.write("# REST API Sync Document\nTesting hybrid stat cache synchronization.")

        try:
            self.store.add_workspace(ws_test, paths=[test_folder])

            # 1. GET /v1/workspaces/{name}/sync/status
            res_status = self.client.get(f"/v1/workspaces/{ws_test}/sync/status", headers=self.headers)
            self.assertEqual(res_status.status_code, 200, f"GET sync/status failed: {res_status.text}")
            data = res_status.json()
            self.assertEqual(data["workspace_name"], ws_test)
            self.assertIn("is_up_to_date", data)
            self.assertIn("summary", data)
            self.assertEqual(data["total_sources"], 1)
            self.assertEqual(data["local_folders_count"], 1)
            self.assertEqual(len(data["folders"]), 1)

            # 2. POST /v1/workspaces/{name}/sync (Synchronous)
            sync_payload = {"force_full": False, "background": False}
            res_sync = self.client.post(f"/v1/workspaces/{ws_test}/sync", json=sync_payload, headers=self.headers)
            self.assertEqual(res_sync.status_code, 200, f"POST sync failed: {res_sync.text}")
            sync_data = res_sync.json()
            self.assertEqual(sync_data["status"], "success")

            # 3. POST /v1/workspaces/{name}/sync (Background)
            bg_payload = {"force_full": False, "background": True}
            res_bg = self.client.post(f"/v1/workspaces/{ws_test}/sync", json=bg_payload, headers=self.headers)
            self.assertEqual(res_bg.status_code, 200)
            self.assertEqual(res_bg.json()["mode"], "background")

            safe_stdout_write("  [OK] Workspace sync status and trigger REST endpoints verified!\n")
        finally:
            self.store.remove_workspace(ws_test)
            try:
                shutil.rmtree(temp_d, ignore_errors=True)
            except Exception:
                pass

    def test_12_workspace_web_search_and_settings_endpoints(self):
        """TC-7.14: Tests GET/POST /v1/context/web-search and GET/POST /v1/workspaces/{name}/settings endpoints."""
        safe_stdout_write("\n>>> [TEST 12] Testing Web Search & Workspace-Isolated Settings REST Endpoints...\n")
        ws_test = "E2E_WebSearch_WS"
        self.store.add_workspace(ws_test, paths=[])

        try:
            # 1. GET /v1/context/web-search (default global)
            res1 = self.client.get("/v1/context/web-search", headers=self.headers)
            self.assertEqual(res1.status_code, 200)
            self.assertFalse(res1.json()["web_search_enabled"])

            # 2. POST /v1/context/web-search for specific workspace
            res2 = self.client.post(
                "/v1/context/web-search",
                json={"enabled": True, "workspace": ws_test, "apply_global": False},
                headers=self.headers
            )
            self.assertEqual(res2.status_code, 200)
            self.assertTrue(res2.json()["web_search_enabled"])
            self.assertEqual(res2.json()["workspace"], ws_test)

            # 3. GET /v1/workspaces/{name}/settings
            res3 = self.client.get(f"/v1/workspaces/{ws_test}/settings", headers=self.headers)
            self.assertEqual(res3.status_code, 200)
            data3 = res3.json()
            self.assertEqual(data3["workspace_name"], ws_test)
            self.assertTrue(data3["web_search_enabled"])
            self.assertEqual(data3["grounding_mode"], "strict")

            # 4. POST /v1/workspaces/{name}/settings to update mode and search
            res4 = self.client.post(
                f"/v1/workspaces/{ws_test}/settings",
                json={"grounding_mode": "proactive", "web_search_enabled": False},
                headers=self.headers
            )
            self.assertEqual(res4.status_code, 200)
            data4 = res4.json()
            self.assertEqual(data4["grounding_mode"], "proactive")
            self.assertFalse(data4["web_search_enabled"])

            safe_stdout_write("  [OK] Web search and workspace settings REST endpoints verified!\n")
        finally:
            self.store.remove_workspace(ws_test)

if __name__ == "__main__":
    unittest.main()

