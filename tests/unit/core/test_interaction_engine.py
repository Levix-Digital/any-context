"""
Unit tests for Interaction Engine (ConfigEngine, OptionsEngine, and Schemas).
"""

import pytest
from any_context.core.interaction.schemas import MenuTreeSchema, OptionsGroupSchema, MenuActionResult
from any_context.core.interaction.config_engine import ConfigEngine
from any_context.core.interaction.options_engine import OptionsEngine


def test_options_engine_grounding_modes():
    engine = OptionsEngine()
    opts = engine.get_grounding_mode_options("Default")
    assert isinstance(opts, OptionsGroupSchema)
    assert opts.type == "grounding_mode"
    assert len(opts.items) == 3
    ids = [item.id for item in opts.items]
    assert "strict" in ids
    assert "hybrid" in ids
    assert "proactive" in ids


def test_options_engine_set_grounding_mode():
    engine = OptionsEngine()
    res = engine.set_grounding_mode("proactive", "Default")
    assert isinstance(res, MenuActionResult)
    assert res.success is True
    assert res.state_updates.get("grounding_mode") == "proactive"

    # Reset back to hybrid
    engine.set_grounding_mode("hybrid", "Default")


def test_options_engine_retrieval_density():
    engine = OptionsEngine()
    opts = engine.get_retrieval_density_options()
    assert isinstance(opts, OptionsGroupSchema)
    assert opts.type == "retrieval_density"
    assert len(opts.items) >= 3


def test_config_engine_main_menu():
    engine = ConfigEngine()
    tree = engine.get_menu_tree("main", "Default")
    assert isinstance(tree, MenuTreeSchema)
    assert tree.menu_id == "main"
    assert len(tree.items) == 11
    item_ids = [item.id for item in tree.items]
    assert "workspaces" in item_ids
    assert "sharing" in item_ids
    assert "grounding" in item_ids
    assert "web_search" in item_ids
    assert "density" in item_ids
    assert "models" in item_ids
    assert "keys" in item_ids
    assert "memory" in item_ids
    assert "billing" in item_ids
    assert "security" in item_ids
    assert "factory_reset" in item_ids


def test_config_engine_submenus():
    engine = ConfigEngine()
    for sub in ["workspaces", "sharing", "grounding", "web_search", "density", "models", "keys", "memory", "billing", "security"]:
        tree = engine.get_menu_tree(sub, "Default")
        assert isinstance(tree, MenuTreeSchema)
        assert tree.menu_id == sub
        assert len(tree.items) > 0


def test_config_engine_execute_actions():
    engine = ConfigEngine()
    res = engine.execute_action("set_grounding_strict", workspace="Default")
    assert res.success is True
    assert res.state_updates.get("grounding_mode") == "strict"

    res = engine.execute_action("websearch_toggle_workspace", workspace="Default")
    assert res.success is True
    assert "web_search_enabled" in res.state_updates
