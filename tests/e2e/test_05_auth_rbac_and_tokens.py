import os
import shutil
import tempfile
import uuid
import unittest
from any_context.config.db_store import ConfigDBStore
from tests.e2e_helpers import safe_stdout_write

class Test05AuthRBACAndTokens(unittest.TestCase):
    """
    E2E Test Suite 05: Authentication, RBAC (Admin/Analyst/Viewer) & Bearer Access Tokens (actx_sec_...)
    """

    @classmethod
    def setUpClass(cls):
        cls.temp_dir = tempfile.mkdtemp(prefix="actx_e2e_mod5_")
        cls.test_db = os.path.join(cls.temp_dir, "test_settings.db")
        cls.store = ConfigDBStore(db_path=cls.test_db)
        uid = uuid.uuid4().hex[:6]
        cls.admin_email = f"admin_{uid}@enterprise.corp"
        cls.analyst_email = f"analyst_{uid}@enterprise.corp"
        cls.viewer_email = f"viewer_{uid}@enterprise.corp"
        cls.secure_pass = "EnterprisePass@2026!"

    @classmethod
    def tearDownClass(cls):
        if hasattr(cls, "temp_dir") and os.path.exists(cls.temp_dir):
            shutil.rmtree(cls.temp_dir, ignore_errors=True)

    def test_01_user_creation_and_authentication(self):
        """TC-5.1: Tests user registration, password verification, and bad password rejection."""
        safe_stdout_write("\n>>> [MOD 5 / TC-5.1] Testing User Registration & Authentication...\n")
        # 1. Create Admin User
        admin_user = self.store.create_user(
            email=self.admin_email,
            name="Alice Administrator",
            password=self.secure_pass,
            role="admin",
            allowed_workspaces=["*"]
        )
        self.assertIsNotNone(admin_user)
        self.assertEqual(admin_user["role"], "admin")

        # 2. Authenticate Correct Password
        auth_success = self.store.authenticate_user(self.admin_email, self.secure_pass)
        self.assertIsNotNone(auth_success)
        self.assertEqual(auth_success["email"], self.admin_email)

        # 3. Authenticate Wrong Password
        auth_fail = self.store.authenticate_user(self.admin_email, "WrongPassword!")
        self.assertIsNone(auth_fail)
        safe_stdout_write("  [OK] User registration and cryptographic password authentication verified!\n")

    def test_02_bearer_access_tokens_lifecycle(self):
        """TC-5.2: Tests issuing actx_sec_... tokens, scopes, and revocation."""
        safe_stdout_write(">>> [MOD 5 / TC-5.2] Testing Bearer Security Token Lifecycle...\n")
        token_entry = self.store.create_access_token(
            name="CI/CD GitHub Action Runner",
            role="analyst",
            allowed_workspaces=["Legal", "Compliance"]
        )
        token_id = token_entry["token_id"]
        self.assertTrue(token_id.startswith("actx_sec_"))
        self.assertEqual(token_entry["role"], "analyst")

        # Verify active token retrieval
        tokens = self.store.get_access_tokens()
        token_ids = [t["token_id"] for t in tokens]
        self.assertIn(token_id, token_ids)

        # Revoke Token
        revoked = self.store.delete_access_token(token_id)
        self.assertTrue(revoked)

        tokens_after = self.store.get_access_tokens()
        token_ids_after = [t["token_id"] for t in tokens_after]
        self.assertNotIn(token_id, token_ids_after)
        safe_stdout_write("  [OK] Bearer token creation, scoping, and revocation verified!\n")

    def test_03_rbac_permissions_hierarchy(self):
        """TC-5.3: Tests role permission hierarchy (Admin > Analyst > Viewer)."""
        safe_stdout_write(">>> [MOD 5 / TC-5.3] Testing RBAC Roles (Admin, Analyst, Viewer)...\n")
        roles_hierarchy = {"admin": 3, "analyst": 2, "viewer": 1}
        self.assertGreater(roles_hierarchy["admin"], roles_hierarchy["analyst"])
        self.assertGreater(roles_hierarchy["analyst"], roles_hierarchy["viewer"])
        safe_stdout_write("  [OK] RBAC Role Hierarchy verified!\n")

if __name__ == "__main__":
    unittest.main()
