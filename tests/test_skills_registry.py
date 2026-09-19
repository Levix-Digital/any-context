"""
Unit tests for the AnyContext Modular Skills Registry and clarification-dialogue skill.
"""
import pytest
from any_context.skills.registry import SkillRegistry, Skill
from any_context.core.utils import get_system_prompt


def test_skills_registry_discovery():
    registry = SkillRegistry.get_instance()
    skills = registry.list_skills()
    assert len(skills) >= 1

    skill_names = [s.name for s in skills]
    assert "clarification-dialogue" in skill_names


def test_clarification_dialogue_metadata():
    registry = SkillRegistry.get_instance()
    skill = registry.get_skill("clarification-dialogue")
    assert skill is not None
    assert skill.name == "clarification-dialogue"
    assert "clarification dialogue" in skill.description.lower()
    assert "Prohibition of \"Silent Assumptions\"" in skill.instructions or "Silent Assumptions" in skill.instructions


def test_skills_format_for_system_prompt():
    registry = SkillRegistry.get_instance()
    formatted = registry.format_skills_for_system_prompt()
    assert "MODULAR AGENT SKILLS" in formatted
    assert "clarification-dialogue" in formatted


def test_system_prompt_includes_skills():
    prompt = get_system_prompt(active_workspace="Default")
    assert "MODULAR AGENT SKILLS" in prompt
    assert "clarification-dialogue" in prompt
