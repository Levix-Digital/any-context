"""
Unit tests for Interaction Engine (ConfigEngine, OptionsEngine, and Schemas).
"""

import unittest
from any_context.core.interaction.schemas import MenuTreeSchema, OptionsGroupSchema, MenuActionResult
from any_context.core.interaction.config_engine import ConfigEngine
from any_context.core.interaction.options_engine import OptionsEngine


class TestInteractionEngine(unittest.TestCase):

    def test_options_engine_grounding_modes(self):
        engine = OptionsEngine()
        opts = engine.get_grounding_mode_options("Default")
        self.assertIsInstance(opts, OptionsGroupSchema)
        self.assertEqual(opts.type, "grounding_mode")
        self.assertEqual(len(opts.items), 3)
        ids = [item.id for item in opts.items]
        self.assertIn("strict", ids)
        self.assertIn("hybrid", ids)
        self.assertIn("proactive", ids)

    def test_options_engine_set_grounding_mode(self):
        engine = OptionsEngine()
        res = engine.set_grounding_mode("proactive", "Default")
        self.assertIsInstance(res, MenuActionResult)
        self.assertTrue(res.success)
        self.assertEqual(res.state_updates.get("grounding_mode"), "proactive")

        # Reset back to hybrid
        engine.set_grounding_mode("hybrid", "Default")

    def test_options_engine_retrieval_density(self):
        engine = OptionsEngine()
        opts = engine.get_retrieval_density_options()
        self.assertIsInstance(opts, OptionsGroupSchema)
        self.assertEqual(opts.type, "retrieval_density")
        self.assertGreaterEqual(len(opts.items), 3)

    def test_options_engine_workspaces(self):
        engine = OptionsEngine()
        opts = engine.get_workspace_options("Default")
        self.assertIsInstance(opts, OptionsGroupSchema)
        self.assertEqual(opts.type, "workspace")
        self.assertGreaterEqual(len(opts.items), 1)
        titles = [i.title for i in opts.items]
        self.assertIn("Default", titles)

        res = engine.set_workspace("Default")
        self.assertTrue(res.success)
        self.assertEqual(res.state_updates.get("workspace"), "Default")

    def test_config_engine_main_menu(self):
        engine = ConfigEngine()
        tree = engine.get_menu_tree("main", "Default")
        self.assertIsInstance(tree, MenuTreeSchema)
        self.assertEqual(tree.menu_id, "main")
        self.assertEqual(len(tree.items), 11)
        item_ids = [item.id for item in tree.items]
        self.assertIn("workspaces", item_ids)
        self.assertIn("sharing", item_ids)
        self.assertIn("grounding", item_ids)
        self.assertIn("web_search", item_ids)
        self.assertIn("density", item_ids)
        self.assertIn("models", item_ids)
        self.assertIn("keys", item_ids)
        self.assertIn("memory", item_ids)
        self.assertIn("billing", item_ids)
        self.assertIn("security", item_ids)
        self.assertIn("factory_reset", item_ids)

    def test_config_engine_submenus(self):
        engine = ConfigEngine()
        for sub in ["workspaces", "sharing", "grounding", "web_search", "density", "models", "keys", "memory", "billing", "security"]:
            tree = engine.get_menu_tree(sub, "Default")
            self.assertIsInstance(tree, MenuTreeSchema)
            self.assertEqual(tree.menu_id, sub)
            self.assertGreater(len(tree.items), 0)

    def test_config_engine_execute_actions(self):
        engine = ConfigEngine()
        res = engine.execute_action("set_grounding_strict", workspace="Default")
        self.assertTrue(res.success)
        self.assertEqual(res.state_updates.get("grounding_mode"), "strict")

        res = engine.execute_action("websearch_toggle_workspace", workspace="Default")
        self.assertTrue(res.success)
        self.assertIn("web_search_enabled", res.state_updates)

        # Test workspace delete source action
        res_src = engine.execute_action("ws_sources_delete", workspace="Default")
        self.assertTrue(res_src.success)
        self.assertEqual(res_src.action, "open_delete_source_modal")

    def test_options_engine_delete_source_flow(self):
        engine = OptionsEngine()
        opts = engine.get_delete_source_options("Default")
        self.assertIsInstance(opts, OptionsGroupSchema)
        self.assertEqual(opts.type, "delete_source")
        self.assertGreaterEqual(len(opts.items), 1)

        # Test cancel option
        res_cancel = engine.execute_delete_source_option("cancel_delete_source", "Default")
        self.assertTrue(res_cancel.success)

        # Test confirmation options builder
        dummy_info = {"source_type": "folder", "source_val": "C:/dummy/docs", "workspace": "Default"}
        confirm_opts = engine.get_confirm_delete_source_options(dummy_info, "Default")
        self.assertIsInstance(confirm_opts, OptionsGroupSchema)
        self.assertEqual(confirm_opts.type, "confirm_delete_source")
        self.assertEqual(len(confirm_opts.items), 2)


if __name__ == "__main__":
    unittest.main()
