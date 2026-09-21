import os
import re
from dataclasses import dataclass, field
from typing import Dict, List, Optional
import yaml


@dataclass
class Skill:
    """Represents a modular behavioral skill for the AnyContext AI agent."""
    name: str
    description: str
    instructions: str
    path: str
    caller_types: List[str] = field(default_factory=lambda: ["human", "mcp"])

    @classmethod
    def from_file(cls, skill_md_path: str) -> Optional["Skill"]:
        """Parses a SKILL.md file with YAML frontmatter (name, description, caller_types) and markdown body."""
        if not os.path.exists(skill_md_path):
            return None

        try:
            with open(skill_md_path, "r", encoding="utf-8") as f:
                content = f.read()

            name = os.path.basename(os.path.dirname(skill_md_path))
            description = ""
            caller_types = ["human", "mcp"]
            instructions = content

            # Parse YAML frontmatter between --- and ---
            frontmatter_match = re.match(r"^---\s*\n(.*?)\n---\s*\n(.*)$", content, re.DOTALL)
            if frontmatter_match:
                fm_text = frontmatter_match.group(1)
                instructions = frontmatter_match.group(2).strip()

                try:
                    data = yaml.safe_load(fm_text) or {}
                    if isinstance(data, dict):
                        name = str(data.get("name", name)).strip()
                        description = str(data.get("description", "")).strip()
                        raw_callers = data.get("caller_types")
                        if isinstance(raw_callers, list):
                            caller_types = [str(c).strip().lower() for c in raw_callers if str(c).strip()]
                        elif isinstance(raw_callers, str):
                            caller_types = [str(raw_callers).strip().lower()]
                except Exception:
                    # Fallback line-by-line parsing
                    for line in fm_text.splitlines():
                        if line.startswith("name:"):
                            name = line.split("name:", 1)[1].strip().strip("\"'")
                        elif line.startswith("description:"):
                            description = line.split("description:", 1)[1].strip().strip("\"'")

            return cls(
                name=name,
                description=description,
                instructions=instructions,
                path=skill_md_path,
                caller_types=caller_types
            )
        except Exception:
            return None


class SkillRegistry:
    """
    Central registry for discovering, loading, and formatting skills into the system prompt.
    Supports built-in skills and caller-aware selective filtering (human vs. mcp).
    """
    _instance: Optional["SkillRegistry"] = None

    def __init__(self, base_dirs: Optional[List[str]] = None):
        self._skills: Dict[str, Skill] = {}
        self._base_dirs: List[str] = base_dirs or []

        default_dir = os.path.dirname(os.path.abspath(__file__))
        if default_dir not in self._base_dirs:
            self._base_dirs.append(default_dir)

        self.reload()

    @classmethod
    def get_instance(cls) -> "SkillRegistry":
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

    def reload(self):
        """Scans base directories for subfolders containing SKILL.md."""
        self._skills.clear()
        for bdir in self._base_dirs:
            if not os.path.isdir(bdir):
                continue
            for entry in os.listdir(bdir):
                subpath = os.path.join(bdir, entry)
                if os.path.isdir(subpath):
                    skill_file = os.path.join(subpath, "SKILL.md")
                    if os.path.isfile(skill_file):
                        skill = Skill.from_file(skill_file)
                        if skill:
                            self._skills[skill.name] = skill

    def get_skill(self, name: str) -> Optional[Skill]:
        return self._skills.get(name)

    def list_skills(self, caller_type: Optional[str] = None) -> List[Skill]:
        if not caller_type:
            return list(self._skills.values())
        c_type = caller_type.strip().lower()
        return [s for s in self._skills.values() if c_type in [c.lower() for c in s.caller_types]]

    def format_skills_for_system_prompt(self, caller_type: str = "human", active_skills: Optional[List[str]] = None) -> str:
        """
        Builds a structured skills section for the AI agent's system prompt.
        Selectively filters skills based on caller_type ('human' vs. 'mcp') and optional active_skills list.
        """
        applicable_skills = self.list_skills(caller_type=caller_type)
        if active_skills:
            applicable_skills = [s for s in applicable_skills if s.name in active_skills]

        if not applicable_skills:
            return ""

        sections = [
            f"### 🧩 MODULAR AGENT SKILLS & SPECIALIZED CAPABILITIES (CALLER: {caller_type.upper()})",
            "You are equipped with specialized modular behavioral skills. Follow the instructions of each active skill dynamically:\n"
        ]

        for skill in applicable_skills:
            sections.append(f"#### Skill: `{skill.name}`")
            if skill.description:
                sections.append(f"> **Purpose:** {skill.description}\n")
            sections.append(skill.instructions)
            sections.append("\n---\n")

        return "\n".join(sections)
