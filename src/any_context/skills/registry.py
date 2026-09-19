import os
import re
from dataclasses import dataclass
from typing import Dict, List, Optional


@dataclass
class Skill:
    """Represents a modular behavioral skill for the AnyContext AI agent."""
    name: str
    description: str
    instructions: str
    path: str

    @classmethod
    def from_file(cls, skill_md_path: str) -> Optional["Skill"]:
        """Parses a SKILL.md file with YAML frontmatter (name, description) and markdown body."""
        if not os.path.exists(skill_md_path):
            return None

        try:
            with open(skill_md_path, "r", encoding="utf-8") as f:
                content = f.read()

            name = os.path.basename(os.path.dirname(skill_md_path))
            description = ""
            instructions = content

            # Parse YAML frontmatter between --- and ---
            frontmatter_match = re.match(r"^---\s*\n(.*?)\n---\s*\n(.*)$", content, re.DOTALL)
            if frontmatter_match:
                fm_text = frontmatter_match.group(1)
                instructions = frontmatter_match.group(2).strip()

                for line in fm_text.splitlines():
                    if line.startswith("name:"):
                        name = line.split("name:", 1)[1].strip().strip("\"'")
                    elif line.startswith("description:"):
                        description = line.split("description:", 1)[1].strip().strip("\"'")

            return cls(
                name=name,
                description=description,
                instructions=instructions,
                path=skill_md_path
            )
        except Exception:
            return None


class SkillRegistry:
    """
    Central registry for discovering, loading, and formatting skills into the system prompt.
    Supports built-in skills and future custom user/workspace skills.
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

    def list_skills(self) -> List[Skill]:
        return list(self._skills.values())

    def format_skills_for_system_prompt(self) -> str:
        """
        Builds a structured skills section for the AI agent's system prompt.
        Presents available capabilities and detailed behavioral instructions.
        """
        if not self._skills:
            return ""

        sections = [
            "### 🧩 MODULAR AGENT SKILLS & SPECIALIZED CAPABILITIES",
            "You are equipped with specialized modular behavioral skills. Follow the instructions of each active skill dynamically:\n"
        ]

        for skill in self._skills.values():
            sections.append(f"#### Skill: `{skill.name}`")
            if skill.description:
                sections.append(f"> **Purpose:** {skill.description}\n")
            sections.append(skill.instructions)
            sections.append("\n---\n")

        return "\n".join(sections)
