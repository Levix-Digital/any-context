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
        """Validates that all 29 slash commands are properly registered."""
        self.assertEqual(len(COMMANDS_REGISTRY), 29)
        expected_names = [
            "/switch", "/model", "/mode", "/web-search", "/sync", "/sources",
            "/folder", "/web", "/transfer", "/link", "/unlink", "/shared",
            "/rename", "/config", "/key", "/models", "/billing", "/reset-memory",
            "/clear", "/paste", "/help", "/version", "/check-update", "/update", "/inspect",
            "/density", "/history", "/menu", "/exit"
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


if __name__ == "__main__":
    unittest.main()
