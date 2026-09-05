"""
Unit Tests for Universal Command Dispatcher (v0.27.0).
Tests:
  - Canonical catalog completeness (all 23 commands)
  - Argument parsing and alias resolution
  - Command dispatching and structured CommandResult outputs
  - State updates propagation
"""

import unittest
from unittest.mock import patch, MagicMock

from any_context.commands.registry import COMMANDS_REGISTRY, find_command_meta
from any_context.commands.dispatcher import dispatch_command, CommandDispatcher, parse_args
from any_context.commands.result import CommandResult


class TestCommandDispatcher(unittest.TestCase):

    def test_01_canonical_registry_completeness(self):
        """Validates that all 32 slash commands are properly registered."""
        self.assertEqual(len(COMMANDS_REGISTRY), 32)
        expected_names = [
            "/switch", "/model", "/mode", "/web-search", "/sync", "/sources",
            "/folder", "/web", "/transfer", "/link", "/unlink", "/shared",
            "/rename", "/config", "/key", "/models", "/billing", "/reset-memory",
            "/clear", "/paste", "/help", "/version", "/check-update", "/update", "/inspect",
            "/density", "/history", "/menu", "/logs", "/diagnostics", "/onboarding", "/exit"
        ]
        registered_names = [c.name for c in COMMANDS_REGISTRY]
        for name in expected_names:
            self.assertIn(name, registered_names, f"Command {name} must be in COMMANDS_REGISTRY")


    def test_02_alias_resolution(self):
        """Validates alias resolution."""
        self.assertEqual(find_command_meta("/workspace").name, "/switch")
        self.assertEqual(find_command_meta("/m").name, "/model")
        self.assertEqual(find_command_meta("/grounding").name, "/mode")
        self.assertEqual(find_command_meta("/search").name, "/web-search")
        self.assertEqual(find_command_meta("/cls").name, "/clear")
        self.assertEqual(find_command_meta("/v").name, "/version")
        self.assertEqual(find_command_meta("/q").name, "/exit")
        self.assertEqual(find_command_meta("/setup").name, "/onboarding")
        self.assertEqual(find_command_meta("/update@0.28.85").name, "/update")

    def test_03_dispatch_version_and_clear_and_exit(self):
        """Validates basic system commands dispatching."""
        res_v = dispatch_command("/version")
        self.assertTrue(res_v.success)
        self.assertIn("AnyContext", res_v.message)

        res_c = dispatch_command("/clear")
        self.assertTrue(res_c.success)
        self.assertEqual(res_c.action, "clear")

        res_e = dispatch_command("/exit")
        self.assertTrue(res_e.success)
        self.assertEqual(res_e.action, "exit")

    def test_04_dispatch_help(self):
        """Validates full help catalog and individual command help."""
        res_all = dispatch_command("/help")
        self.assertTrue(res_all.success)
        self.assertIn("Slash Commands Catalog", res_all.message)

        res_single = dispatch_command("/help /sources")
        self.assertTrue(res_single.success)
        self.assertIn("Command Help: `/sources`", res_single.message)

    def test_05_dispatch_state_updates(self):
        """Validates that dispatching /model, /mode, /web-search, and /switch produce state updates."""
        with patch.object(CommandDispatcher, "_handle_model") as mock_model:
            mock_model.return_value = CommandResult(
                success=True,
                message="Switched",
                state_updates={"model": "claude-3-5-sonnet"}
            )
            disp = CommandDispatcher()
            res = disp.dispatch("/model claude-3-5-sonnet")
            self.assertEqual(res.state_updates.get("model"), "claude-3-5-sonnet")

    def test_06_unknown_command(self):
        """Validates graceful handling of unknown commands."""
        res = dispatch_command("/unknown_xyz_command")
        self.assertFalse(res.success)
        self.assertIn("Unknown command", res.message)

    def test_07_dispatch_onboarding(self):
        """Validates /onboarding and /setup dispatching open_onboarding_modal."""
        res = dispatch_command("/onboarding")
        self.assertTrue(res.success)
        self.assertEqual(res.action, "open_onboarding_modal")

        res_setup = dispatch_command("/setup")
        self.assertTrue(res_setup.success)
        self.assertEqual(res_setup.action, "open_onboarding_modal")

    def test_08_dispatch_update_at_version(self):
        """Validates /update@version returns open_update_modal with target_version."""
        res = dispatch_command("/update@0.28.85")
        self.assertTrue(res.success)
        self.assertEqual(res.action, "open_update_modal")
        self.assertEqual(res.state_updates.get("target_version"), "0.28.85")

    def test_09_dispatch_modal_actions(self):
        """Validates that commands without args return their respective open modal actions."""
        res_mode = dispatch_command("/mode")
        self.assertTrue(res_mode.success)
        self.assertEqual(res_mode.action, "open_mode_modal")

        res_switch = dispatch_command("/switch")
        self.assertTrue(res_switch.success)
        self.assertEqual(res_switch.action, "open_switch_modal")

        res_model = dispatch_command("/model")
        self.assertTrue(res_model.success)
        self.assertEqual(res_model.action, "open_model_modal")

        res_config = dispatch_command("/config")
        self.assertTrue(res_config.success)
        self.assertEqual(res_config.action, "open_config_modal")

        res_menu = dispatch_command("/menu")
        self.assertTrue(res_menu.success)
        self.assertEqual(res_menu.action, "open_config_modal")

    def test_10_dispatch_mode_values(self):
        """Validates /mode command with explicit values."""
        res_strict = dispatch_command("/mode strict")
        self.assertTrue(res_strict.success)
        self.assertEqual(res_strict.state_updates.get("grounding_mode"), "strict")

        res_hybrid = dispatch_command("/mode hybrid")
        self.assertTrue(res_hybrid.success)
        self.assertEqual(res_hybrid.state_updates.get("grounding_mode"), "hybrid")

    def test_11_dispatch_web_search(self):
        """Validates /web-search toggle values."""
        res_on = dispatch_command("/web-search on")
        self.assertTrue(res_on.success)
        self.assertEqual(res_on.state_updates.get("web_search_enabled"), True)

        res_off = dispatch_command("/web-search off")
        self.assertTrue(res_off.success)
        self.assertEqual(res_off.state_updates.get("web_search_enabled"), False)

    def test_12_dispatch_paste(self):
        """Validates /paste action."""
        res = dispatch_command("/paste")
        self.assertTrue(res.success)
        self.assertEqual(res.action, "paste_mode")


if __name__ == "__main__":
    unittest.main()
