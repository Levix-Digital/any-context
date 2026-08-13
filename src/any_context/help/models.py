from typing import List, Optional
from pydantic import BaseModel, Field

class HelpPage(BaseModel):
    command: str = Field(..., description="Primary command name (e.g. '/switch', '--serve')")
    aliases: List[str] = Field(default_factory=list, description="Alternative names or flags")
    title: str = Field(..., description="Short title and summary")
    description: str = Field(..., description="Detailed description of what the command does")
    syntax: str = Field(..., description="Usage syntax (e.g. actx --serve --port 8000)")
    parameters: List[str] = Field(default_factory=list, description="Supported flags or parameters")
    examples: List[str] = Field(default_factory=list, description="Real-world usage examples")
    tips: List[str] = Field(default_factory=list, description="Best practices and tips")
