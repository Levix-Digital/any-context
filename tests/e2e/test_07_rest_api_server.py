import os
import unittest
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

        # 2. List Workspaces
        res_list = self.client.get("/v1/workspaces", headers=self.headers)
        self.assertEqual(res_list.status_code, 200)
        data = res_list.json()
        workspaces = data.get("workspaces", [])
        ws_names = [w["name"] if isinstance(w, dict) else w for w in workspaces]
        self.assertIn(self.test_ws, ws_names)
        self.assertIn("E2E_Empty_Workspace", ws_names)

        # 3. Trigger Indexing
        res_idx = self.client.post("/v1/index", json={"workspace": self.test_ws}, headers=self.headers)
        self.assertEqual(res_idx.status_code, 200)
        idx_data = res_idx.json()
        self.assertEqual(idx_data["status"], "accepted")
        safe_stdout_write("  [OK] Workspace creation, listing and background index REST endpoints verified!\n")

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

if __name__ == "__main__":
    unittest.main()
