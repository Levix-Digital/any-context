"""
Unit and Integration Tests for AnyContext Interactive /update & Auto-Restart Architecture.
100% native unittest.TestCase without pytest dependencies.
"""
import os
import sys
import unittest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.core.services.update_service import UpdateService
from any_context.core.interaction.options_engine import OptionsEngine
from any_context.commands.dispatcher import dispatch_command


class TestUpdateInteractive(unittest.TestCase):
    def test_update_service_query_and_instances(self):
        svc = UpdateService()
        current = svc.get_current_version()
        self.assertIsNotNone(current)
        instances = svc.find_active_instances()
        self.assertIsInstance(instances, list)

    def test_options_engine_update_options(self):
        engine = OptionsEngine()
        opts = engine.get_update_options(target_version="0.28.16")
        self.assertEqual(opts.type, "update")
        self.assertIn("0.28.16", opts.title)
        self.assertEqual(len(opts.items), 3)
        self.assertEqual(opts.items[0].id, "background")
        self.assertEqual(opts.items[1].id, "close")
        self.assertEqual(opts.items[2].id, "cancel")

    def test_options_engine_cancel_update(self):
        engine = OptionsEngine()
        res = engine.execute_update_option("cancel")
        self.assertTrue(res.success)
        self.assertIn("cancelled", res.message.lower())

    def test_dispatcher_check_update_command(self):
        res = dispatch_command("/check-update", active_workspace="Default")
        self.assertTrue(res.success)
        self.assertIn("AnyContext", res.message)

    def test_dispatcher_update_command_modal(self):
        res = dispatch_command("/update@0.28.99", active_workspace="Default")
        self.assertTrue(res.success)
        self.assertEqual(res.action, "open_update_modal")


if __name__ == "__main__":
    unittest.main()
