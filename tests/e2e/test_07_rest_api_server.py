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
        setup_mock_embeddings_if_needed()
        cls.app = create_app()
        cls.client = TestClient(cls.app)
        cls.store = ConfigDBStore()
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
        try:
            cls.store.remove_workspace(cls.test_ws)
            cls.store.delete_access_token(cls.admin_token)
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
        self.assertEqual(create_data["paths"], [])
        self.assertIn("web_sources", create_data)
        self.assertIn("cloud_drives", create_data)
        self.assertIn("sources", create_data)

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
        self.assertGreaterEqual(len(target_ws_dto["web_sources"]), 1)
        self.assertEqual(target_ws_dto["web_sources"][0]["url"], "https://canada.ca/en/immigration")
        self.assertGreaterEqual(len(target_ws_dto["cloud_drives"]), 1)
        self.assertEqual(target_ws_dto["cloud_drives"][0]["provider"], "google_drive")
        self.assertGreaterEqual(target_ws_dto["total_sources"], 2)

        # 5. Get Workspace Detail & Sources endpoints
        res_single = self.client.get(f"/v1/workspaces/{self.test_ws}", headers=self.headers)
        self.assertEqual(res_single.status_code, 200)
        single_data = res_single.json()
        self.assertEqual(single_data["name"], self.test_ws)
        self.assertGreaterEqual(len(single_data["web_sources"]), 1)

        res_sources = self.client.get(f"/v1/workspaces/{self.test_ws}/sources", headers=self.headers)
        self.assertEqual(res_sources.status_code, 200)
        src_breakdown = res_sources.json()
        self.assertEqual(src_breakdown["workspace"], self.test_ws)
        self.assertGreaterEqual(src_breakdown["total_sources"], 2)

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
            safe_stdout_write("  [OK] POST /v1/workspaces/rename REST API verified!\n")
        finally:
            self.store.remove_workspace(orig_name)
            self.store.remove_workspace(new_name)


if __name__ == "__main__":
    unittest.main()


