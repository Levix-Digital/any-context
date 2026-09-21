"""
Unit and integration tests for Caller-Aware Output Modulation (human vs mcp),
ContextVar isolation, and Global system workspace protection.
"""
import pytest
from unittest.mock import patch, MagicMock

from any_context.core.agent import (
    set_caller_type_context,
    get_caller_type_context,
    create_anycontext_agent
)
from any_context.core.utils import get_system_prompt
from any_context.core.services.workspace_service import WorkspaceService
from any_context.config.db_store import ConfigDBStore


def test_caller_type_contextvar():
    set_caller_type_context("mcp")
    assert get_caller_type_context() == "mcp"

    set_caller_type_context("human")
    assert get_caller_type_context() == "human"

    # Default fallback
    set_caller_type_context(None)
    assert get_caller_type_context() == "human"


def test_system_prompt_modulation_human():
    prompt = get_system_prompt(caller_type="human", active_workspace="Default")
    assert "clarification-dialogue" in prompt
    assert "mcp-direct-response" not in prompt
    assert "CALLER: HUMAN" in prompt
    assert "collaborative dialogue" in prompt or "ask how the user prefers" in prompt


def test_system_prompt_modulation_mcp():
    prompt = get_system_prompt(caller_type="mcp", active_workspace="Default")
    assert "mcp-direct-response" in prompt
    assert "clarification-dialogue" not in prompt
    assert "CALLER: MCP" in prompt
    assert "dense, direct, structured answer for the calling tool/machine consumer" in prompt


def test_global_workspace_protection_in_service():
    service = WorkspaceService()
    workspaces = service.list_workspaces()
    names = [w["name"].lower() for w in workspaces]
    assert "global" not in names

    # Attempting to delete Global should raise ValueError
    with pytest.raises(ValueError) as exc:
        service.delete_workspace("Global")
    assert "protected" in str(exc.value).lower()
