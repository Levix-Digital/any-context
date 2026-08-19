import os
import unittest
from any_context.config.db_store import ConfigDBStore
from any_context.workspace_sharing.store import WorkspaceSharingStore
from tests.e2e_helpers import safe_stdout_write

class Test06WorkspaceCollaboration(unittest.TestCase):
    """
    E2E Test Suite 06: Workspace Sharing, Google Drive-style Folder Ownership & Invite Codes
    """

    @classmethod
    def setUpClass(cls):
        cls.ws = "E2E_Mod6_SharedProject"
        cls.config_store = ConfigDBStore()
        cls.config_store.add_workspace(cls.ws, [])
        cls.share_store = WorkspaceSharingStore()

        cls.owner_email = "alice_owner@company.com"
        cls.editor_email = "bob_editor@company.com"

    @classmethod
    def tearDownClass(cls):
        try:
            cls.config_store.remove_workspace(cls.ws)
        except Exception:
            pass

    def test_01_invite_generation_and_acceptance(self):
        """TC-6.1: Tests creating a share invite code and accepting it as a collaborator."""
        safe_stdout_write("\n>>> [MOD 6 / TC-6.1] Testing Share Invite Creation & Acceptance...\n")
        # 1. Alice creates an Editor invite
        invite = self.share_store.create_share_invite(
            workspace_name=self.ws,
            access_level="editor",
            created_by_email=self.owner_email,
            max_uses=2
        )
        self.assertIsNotNone(invite)
        self.assertTrue(invite.invite_code.startswith("SHARE-"))
        self.assertEqual(invite.access_level, "editor")

        # 2. Bob accepts the invite
        permission = self.share_store.accept_share_invite(
            invite_code=invite.invite_code,
            user_email=self.editor_email
        )
        self.assertIsNotNone(permission)
        self.assertEqual(permission.workspace_name, self.ws)
        self.assertEqual(permission.access_level, "editor")
        self.assertEqual(permission.user_email, self.editor_email)
        safe_stdout_write("  [OK] Share invite creation and acceptance verified!\n")

    def test_02_folder_ownership_and_permission_isolation(self):
        """TC-6.2: Tests folder tagging and verifying that collaborators cannot delete owner folders."""
        safe_stdout_write(">>> [MOD 6 / TC-6.2] Testing Folder Ownership & Permission Guardrails...\n")
        alice_folder = "C:/Docs/Alice/Confidential"
        bob_folder = "C:/Docs/Bob/Notes"

        # Alice adds her folder as owner
        fld_alice = self.share_store.add_workspace_folder(
            workspace_name=self.ws,
            folder_path=alice_folder,
            added_by_email=self.owner_email
        )

        # Bob adds his folder
        fld_bob = self.share_store.add_workspace_folder(
            workspace_name=self.ws,
            folder_path=bob_folder,
            added_by_email=self.editor_email
        )

        folders = self.share_store.get_workspace_folders(self.ws)
        self.assertEqual(len(folders), 2)

        # Bob CAN delete his own folder
        bob_deleted = self.share_store.delete_workspace_folder(fld_bob.folder_id, user_email=self.editor_email)
        self.assertTrue(bob_deleted)

        # Bob CANNOT delete Alice's folder
        bob_deleted_alice = self.share_store.delete_workspace_folder(fld_alice.folder_id, user_email=self.editor_email)
        self.assertFalse(bob_deleted_alice)

        # Admin CAN delete Alice's folder
        admin_deleted_alice = self.share_store.delete_workspace_folder(fld_alice.folder_id, user_email="admin@system.local")
        self.assertTrue(admin_deleted_alice)
        safe_stdout_write("  [OK] Folder ownership and delete permission guardrails verified!\n")

if __name__ == "__main__":
    unittest.main()
