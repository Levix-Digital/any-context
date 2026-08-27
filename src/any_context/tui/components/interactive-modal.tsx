import React from "react";
import type {
  AnyContextState,
  MenuTreeSchema,
  MenuItemSchema,
  OptionsGroupSchema,
  OptionItemSchema,
} from "../bridge-client";
import { anyContextTheme } from "../themes";

export interface InteractiveModalProps {
  isOpen: boolean;
  mode: "options" | "menu";
  optionsGroup?: OptionsGroupSchema | null;
  menuTree?: MenuTreeSchema | null;
  selectedIndex: number;
  state?: AnyContextState;
}

export const InteractiveModal = ({
  isOpen,
  mode,
  optionsGroup,
  menuTree,
  selectedIndex,
  state,
}: InteractiveModalProps): any => {
  if (!isOpen) return null;

  // 1. Render Options Group Selection (e.g. /mode, /model, /density, /switch)
  if (mode === "options" && optionsGroup) {
    const items = optionsGroup.items || [];
    const wsName = state?.workspace || "Default";

    return (
      <box
        flexDirection="column"
        backgroundColor={anyContextTheme.surface}
        borderStyle="rounded"
        borderColor={anyContextTheme.accentWarning}
        paddingLeft={1}
        paddingRight={1}
        paddingTop={0}
        paddingBottom={0}
        marginBottom={1}
        flexShrink={0}
      >
        {/* Header */}
        <box
          flexDirection="column"
          paddingBottom={0}
          border={["bottom"]}
          borderStyle="single"
          borderColor={anyContextTheme.ruleColor}
        >
          <text fg={anyContextTheme.accentWarning}>
            <b>{optionsGroup.title}</b>  <span fg={anyContextTheme.ruleColor}>│</span>  <span fg={anyContextTheme.accentSecondary}>Workspace: <b>{wsName}</b></span>
          </text>

          {optionsGroup.description ? (
            <text fg={anyContextTheme.foregroundMuted}>
              {optionsGroup.description}
            </text>
          ) : null}
        </box>

        {/* Options List */}
        {items.map((item: OptionItemSchema, idx: number) => {
          const isSelected = idx === selectedIndex;
          const prefix = isSelected ? "▸ " : "  ";
          const bgColor = isSelected ? anyContextTheme.surfaceHighlight : undefined;
          const titleColor = isSelected ? anyContextTheme.accentWarning : anyContextTheme.foreground;

          const badgeStr = item.badge ? `  ${item.badge}` : "";
          const descStr = item.description ? ` - ${item.description}` : "";

          return (
            <box
              key={item.id}
              flexDirection="column"
              backgroundColor={bgColor}
              paddingLeft={1}
              paddingRight={1}
            >
              <text fg={titleColor}>
                <b>{prefix}{item.icon} {item.title}</b>
                {badgeStr ? <span fg={anyContextTheme.accentSuccess}>{badgeStr}</span> : ""}
                {descStr ? <span fg={anyContextTheme.foregroundMuted}>{descStr}</span> : ""}
              </text>
            </box>
          );
        })}

        {/* Footer Navigation */}
        <box
          flexDirection="row"
          paddingLeft={1}
          paddingRight={1}
          paddingTop={0}
          border={["top"]}
          borderStyle="single"
          borderColor={anyContextTheme.ruleColor}
        >
          <text fg={anyContextTheme.foregroundMuted}>
            💡 <b>[↑/↓]</b> Select Option  •  <b>[Enter/Tab]</b> Confirm  •  <b>[Esc]</b> Close
          </text>
        </box>
      </box>
    );
  }

  // 2. Render Hierarchical Menu Tree (e.g. /menu, /config)
  if (mode === "menu" && menuTree) {
    const items = menuTree.items || [];
    const breadcrumbStr =
      menuTree.breadcrumbs && menuTree.breadcrumbs.length > 0
        ? menuTree.breadcrumbs.join(" ➔ ")
        : menuTree.title;
    const wsName = menuTree.workspace || state?.workspace || "Default";

    return (
      <box
        flexDirection="column"
        backgroundColor={anyContextTheme.surface}
        borderStyle="rounded"
        borderColor={anyContextTheme.accentWarning}
        paddingLeft={1}
        paddingRight={1}
        paddingTop={0}
        paddingBottom={0}
        marginBottom={1}
        flexShrink={0}
      >
        {/* Header with Breadcrumbs */}
        <box
          flexDirection="column"
          paddingBottom={0}
          border={["bottom"]}
          borderStyle="single"
          borderColor={anyContextTheme.ruleColor}
        >
          <text fg={anyContextTheme.accentWarning}>
            <b>{breadcrumbStr}</b>  <span fg={anyContextTheme.ruleColor}>│</span>  <span fg={anyContextTheme.accentSecondary}>Workspace: <b>{wsName}</b></span>
          </text>

          {menuTree.subtitle ? (
            <text fg={anyContextTheme.foregroundMuted}>
              {menuTree.subtitle}
            </text>
          ) : null}
        </box>

        {/* Menu Items List */}
        {items.map((item: MenuItemSchema, idx: number) => {
          const isSelected = idx === selectedIndex;
          const prefix = isSelected ? "▸ " : "  ";
          const bgColor = isSelected ? anyContextTheme.surfaceHighlight : undefined;
          const titleColor = isSelected ? anyContextTheme.accentWarning : anyContextTheme.foreground;
          const typeIndicator = item.type === "submenu" ? " ▶" : "";

          const badgeStr = item.badge ? `  ${item.badge}` : "";
          const shortcutStr = item.command_shortcut ? `  (${item.command_shortcut})` : "";
          const descStr = item.description ? ` - ${item.description}` : "";

          return (
            <box
              key={item.id}
              flexDirection="column"
              backgroundColor={bgColor}
              paddingLeft={1}
              paddingRight={1}
            >
              <text fg={titleColor}>
                <b>{prefix}{item.icon} {item.title}{typeIndicator}</b>
                {badgeStr ? <span fg={anyContextTheme.accentSuccess}>{badgeStr}</span> : ""}
                {shortcutStr ? <span fg={anyContextTheme.accentSecondary}>{shortcutStr}</span> : ""}
                {descStr ? <span fg={anyContextTheme.foregroundMuted}>{descStr}</span> : ""}
              </text>
            </box>
          );
        })}

        {/* Footer Navigation */}
        <box
          flexDirection="row"
          paddingLeft={1}
          paddingRight={1}
          paddingTop={0}
          border={["top"]}
          borderStyle="single"
          borderColor={anyContextTheme.ruleColor}
        >
          <text fg={anyContextTheme.foregroundMuted}>
            💡 <b>[↑/↓]</b> Navigate  •  <b>[Enter/Tab]</b> Open / Execute  •  <b>[Esc]</b> {menuTree.menu_id === "main" ? "Close" : "Back"}
          </text>
        </box>
      </box>
    );
  }

  return null;
};
