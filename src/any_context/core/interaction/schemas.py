"""
Interaction Schemas - Universal dataclasses and models for menus, choices, forms and actions.
Used by CLI, TUI (OpenTUI), REST API and MCP adapters to render native user interfaces.
"""

from typing import List, Dict, Any, Optional, Literal
from pydantic import BaseModel, Field


class OptionItemSchema(BaseModel):
    """Represents a single selectable option in a list (e.g. Grounding Mode, Model)."""
    id: str
    title: str
    description: Optional[str] = ""
    icon: Optional[str] = ""
    badge: Optional[str] = ""
    is_active: bool = False
    metadata: Dict[str, Any] = Field(default_factory=dict)


class OptionsGroupSchema(BaseModel):
    """Group of selectable options with context metadata."""
    type: str  # "grounding_mode", "inference_model", "retrieval_density", "search_engine", etc.
    title: str
    description: Optional[str] = ""
    active_id: Optional[str] = None
    items: List[OptionItemSchema] = Field(default_factory=list)


class MenuItemSchema(BaseModel):
    """Represents a menu item or submenu in the configuration tree."""
    id: str
    title: str
    description: Optional[str] = ""
    icon: Optional[str] = ""
    badge: Optional[str] = ""
    type: Literal["submenu", "action", "toggle", "select", "input"] = "action"
    command_shortcut: Optional[str] = None
    is_active: Optional[bool] = None
    current_value: Optional[str] = None
    options: Optional[List[OptionItemSchema]] = None
    subitems: Optional[List["MenuItemSchema"]] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


MenuItemSchema.model_rebuild()


class MenuTreeSchema(BaseModel):
    """Represents a full hierarchical menu tree with breadcrumb and active state."""
    menu_id: str
    title: str
    subtitle: Optional[str] = None
    workspace: str
    breadcrumbs: List[str] = Field(default_factory=list)
    items: List[MenuItemSchema] = Field(default_factory=list)


class MenuActionResult(BaseModel):
    """Result of executing an action from a menu or modal."""
    success: bool
    message: str
    error: Optional[str] = None
    state_updates: Dict[str, Any] = Field(default_factory=dict)
    next_menu_id: Optional[str] = None
    action: Optional[str] = None
