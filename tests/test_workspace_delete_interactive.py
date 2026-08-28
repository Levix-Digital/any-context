import unittest
from any_context.core.interaction.options_engine import OptionsEngine
from any_context.core.interaction.config_engine import ConfigEngine
from any_context.config.db_store import ConfigDBStore


class TestWorkspaceDeleteInteractive(unittest.TestCase):
    def setUp(self):
        self.store = ConfigDBStore()
        self.opts_engine = OptionsEngine()
        self.config_engine = ConfigEngine()

    def test_01_config_engine_ws_delete_action(self):
        res = self.config_engine.execute_action("ws_delete", {}, workspace="Default")
        self.assertTrue(res.success)
        self.assertEqual(res.action, "open_delete_workspace_modal")

    def test_02_delete_workspace_options(self):
        # Create a temp workspace to ensure one exists to delete
        self.store.add_workspace("TempDeleteWS_Test", paths=[])
        opts = self.opts_engine.get_delete_workspace_options(current_workspace="Default")
        self.assertEqual(opts.type, "delete_workspace")
        self.assertTrue(len(opts.items) >= 2)  # at least the workspace and Cancel

        target_item = next((it for it in opts.items if it.id == "delete_ws_TempDeleteWS_Test"), None)
        self.assertIsNotNone(target_item)
        self.assertIn("TempDeleteWS_Test", target_item.title)

    def test_03_selection_opens_confirmation(self):
        res = self.opts_engine.execute_delete_workspace_option(
            "delete_ws_TempDeleteWS_Test",
            current_workspace="Default"
        )
        self.assertTrue(res.success)
        self.assertEqual(res.action, "open_confirm_delete_workspace_modal")
        self.assertEqual(res.state_updates.get("target_workspace"), "TempDeleteWS_Test")

    def test_04_confirmation_options(self):
        opts = self.opts_engine.get_confirm_delete_workspace_options(
            workspace_to_delete="TempDeleteWS_Test",
            is_active=True
        )
        self.assertEqual(opts.type, "confirm_delete_workspace")
        self.assertEqual(len(opts.items), 2)
        yes_opt = next(it for it in opts.items if it.id.startswith("confirm_delete_yes_"))
        cancel_opt = next(it for it in opts.items if it.id == "confirm_delete_cancel")
        self.assertIsNotNone(yes_opt)
        self.assertIsNotNone(cancel_opt)
        self.assertTrue(cancel_opt.is_active)  # Safe default

    def test_05_cancel_deletion(self):
        res = self.opts_engine.execute_delete_workspace_option(
            "confirm_delete_cancel",
            current_workspace="Default"
        )
        self.assertTrue(res.success)
        self.assertIn("cancelled", res.message.lower())

    def test_06_confirm_deletion_execution(self):
        res = self.opts_engine.execute_delete_workspace_option(
            "confirm_delete_yes_TempDeleteWS_Test",
            current_workspace="TempDeleteWS_Test"
        )
        self.assertTrue(res.success)
        self.assertEqual(res.action, "delete_workspace_success")
        self.assertEqual(res.state_updates.get("workspace"), "Default")

        # Verify workspace is gone from SQLite
        all_ws = [w["name"] for w in self.store.list_workspaces_detailed()]
        self.assertNotIn("TempDeleteWS_Test", all_ws)


if __name__ == "__main__":
    unittest.main()
