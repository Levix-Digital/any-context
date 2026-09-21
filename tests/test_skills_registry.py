"""
Unit tests for the AnyContext Modular Skills Registry, caller_type filtering, and individual skills.
"""
import pytest
from any_context.skills.registry import SkillRegistry, Skill
from any_context.core.utils import get_system_prompt


def test_skills_registry_discovery():
    registry = SkillRegistry.get_instance()
    skills = registry.list_skills()
    assert len(skills) >= 4

    skill_names = [s.name for s in skills]
    assert "clarification-dialogue" in skill_names
    assert "mcp-direct-response" in skill_names
    assert "temporal-grounding" in skill_names
    assert "panoramic-synthesis" in skill_names


def test_clarification_dialogue_metadata():
    registry = SkillRegistry.get_instance()
    skill = registry.get_skill("clarification-dialogue")
    assert skill is not None
    assert skill.name == "clarification-dialogue"
    assert "clarification" in skill.description.lower()
    assert "human" in skill.caller_types
    assert "mcp" not in skill.caller_types


def test_mcp_direct_response_metadata():
    registry = SkillRegistry.get_instance()
    skill = registry.get_skill("mcp-direct-response")
    assert skill is not None
    assert skill.name == "mcp-direct-response"
    assert "mcp" in skill.caller_types
    assert "human" not in skill.caller_types


def test_caller_type_filtering():
    registry = SkillRegistry.get_instance()
    human_skills = [s.name for s in registry.list_skills(caller_type="human")]
    mcp_skills = [s.name for s in registry.list_skills(caller_type="mcp")]

    assert "clarification-dialogue" in human_skills
    assert "mcp-direct-response" not in human_skills

    assert "mcp-direct-response" in mcp_skills
    assert "clarification-dialogue" not in mcp_skills

    # Shared skills
    assert "temporal-grounding" in human_skills
    assert "temporal-grounding" in mcp_skills
    assert "panoramic-synthesis" in human_skills
    assert "panoramic-synthesis" in mcp_skills


def test_system_prompt_caller_modulation():
    prompt_human = get_system_prompt(active_workspace="Default", caller_type="human")
    assert "clarification-dialogue" in prompt_human
    assert "mcp-direct-response" not in prompt_human
    assert "collaborative dialogue" in prompt_human or "clarification" in prompt_human

    prompt_mcp = get_system_prompt(active_workspace="Default", caller_type="mcp")
    assert "mcp-direct-response" in prompt_mcp
    assert "clarification-dialogue" not in prompt_mcp
    assert "CALLER: MCP" in prompt_mcp
