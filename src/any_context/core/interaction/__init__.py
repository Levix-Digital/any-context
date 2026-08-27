"""
Interaction Engine Package - Centralized, presentation-agnostic configuration and interaction core.
Provides canonical schemas, hierarchical menu trees, and option selection engines.
"""

from any_context.core.interaction.schemas import (
    OptionItemSchema,
    OptionsGroupSchema,
    MenuItemSchema,
    MenuTreeSchema,
    MenuActionResult,
)
from any_context.core.interaction.options_engine import OptionsEngine
from any_context.core.interaction.config_engine import ConfigEngine

__all__ = [
    "OptionItemSchema",
    "OptionsGroupSchema",
    "MenuItemSchema",
    "MenuTreeSchema",
    "MenuActionResult",
    "OptionsEngine",
    "ConfigEngine",
]
