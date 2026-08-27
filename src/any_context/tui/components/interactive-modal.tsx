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

  // 1. Render Options Group Selection (e.g. /mode, /model, /density)
  if (mode === "options" && optionsGroup) {
    const items = optionsGroup.items || [];
    return (
      <box
        flexDirection="column"
        backgroundColor={anyContextTheme.surface}
        borderStyle="rounded"
        borderColor={anyContextTheme.accentWarning}
        paddingLeft={1}
        paddingRight={1}
        paddingTop={0}
        paddingBottom={1}
        marginBottom={1}
      >
        {/* Header */}
        <box flexDirection="row" paddingTop={0} paddingBottom={0} marginBottom={0}>
          <text fg={anyContextTheme.accentWarning}>
            <b>{optionsGroup.title}</b>
          </text>
          <text fg={anyContextTheme.ruleColor}>  │  </text>
          <text fg={anyContextTheme.accentSecondary}>
            Workspace: <b>{state?.workspace || "Default"}</b>
          </text>
        </box>

        {optionsGroup.description ? (
          <text fg={anyContextTheme.foregroundMuted}>
            {optionsGroup.description}
          </text>
        ) : null}

        {/* Separator */}
        <box flexDirection="row" marginTop={0} marginBottom={0}>
          <text fg={anyContextTheme.ruleColor}>─────────────────────────────────────────────────────────────────</text>
        </box>

        {/* Options List */}
        {items.map((item: OptionItemSchema, idx: number) => {
          const isSelected = idx === selectedIndex;
          const prefix = isSelected ? "▸ " : "  ";
          const bgColor = isSelected ? anyContextTheme.surfaceHighlight : undefined;
          const titleColor = isSelected ? anyContextTheme.accentWarning : anyContextTheme.foreground;

          return (
            <box
              key={item.id}
              flexDirection="row"
              backgroundColor={bgColor}
              paddingLeft={1}
              paddingRight={1}
            >
              <text fg={titleColor}>
                <b>{prefix}{item.icon} {item.title}</b>
              </text>
              {item.badge ? (
                <text fg={anyContextTheme.accentSuccess}> {item.badge} </text>
              ) : null}
              {item.description ? (
                <text fg={anyContextTheme.foregroundMuted}> - {item.description}</text>
              ) : null}
            </box>
          );
        })}

        {/* Separator */}
        <box flexDirection="row" marginTop={0} marginBottom={0}>
          <text fg={anyContextTheme.ruleColor}>─────────────────────────────────────────────────────────────────</text>
        </box>

        {/* Footer Navigation */}
        <box flexDirection="row" paddingLeft={1} paddingRight={1} marginTop={0}>
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
    const breadcrumbStr = (menuTree.breadcrumbs && menuTree.breadcrumbs.length > 0)
      ? menuTree.breadcrumbs.join(" ➔ ")
      : menuTree.title;

    return (
      <box
        flexDirection="column"
        backgroundColor={anyContextTheme.surface}
        borderStyle="rounded"
        borderColor={anyContextTheme.accentWarning}
        paddingLeft={1}
        paddingRight={1}
        paddingTop={0}
        paddingBottom={1}
        marginBottom={1}
      >
        {/* Header with Breadcrumbs */}
        <box flexDirection="row" paddingTop={0} paddingBottom={0} marginBottom={0}>
          <text fg={anyContextTheme.accentWarning}>
            <b>{breadcrumbStr}</b>
          </text>
          <text fg={anyContextTheme.ruleColor}>  │  </text>
          <text fg={anyContextTheme.accentSecondary}>
            Workspace: <b>{menuTree.workspace || state?.workspace || "Default"}</b>
          </text>
        </box>

        {menuTree.subtitle ? (
          <text fg={anyContextTheme.foregroundMuted}>
            {menuTree.subtitle}
          </text>
        ) : null}

        {/* Separator */}
        <box flexDirection="row" marginTop={0} marginBottom={0}>
          <text fg={anyContextTheme.ruleColor}>─────────────────────────────────────────────────────────────────</text>
        </box>

        {/* Menu Items List */}
        {items.map((item: MenuItemSchema, idx: number) => {
          const isSelected = idx === selectedIndex;
          const prefix = isSelected ? "▸ " : "  ";
          const bgColor = isSelected ? anyContextTheme.surfaceHighlight : undefined;
          const titleColor = isSelected ? anyContextTheme.accentWarning : anyContextTheme.foreground;

          const typeIndicator = item.type === "submenu" ? " ▶" : "";

          return (
            <box
              key={item.id}
              flexDirection="row"
              backgroundColor={bgColor}
              paddingLeft={1}
              paddingRight={1}
            >
              <text fg={titleColor}>
                <b>{prefix}{item.icon} {item.title}{typeIndicator}</b>
              </text>
              {item.badge ? (
                <text fg={anyContextTheme.accentSuccess}> {item.badge} </text>
              ) : null}
              {item.command_shortcut ? (
                <text fg={anyContextTheme.accentSecondary}> ({item.command_shortcut}) </text>
              ) : null}
              {item.description ? (
                <text fg={anyContextTheme.foregroundMuted}> - {item.description}</text>
              ) : null}
            </box>
          );
        })}

        {/* Separator */}
        <box flexDirection="row" marginTop={0} marginBottom={0}>
          <text fg={anyContextTheme.ruleColor}>─────────────────────────────────────────────────────────────────</text>
        </box>

        {/* Footer Navigation */}
        <box flexDirection="row" paddingLeft={1} paddingRight={1} marginTop={0}>
          <text fg={anyContextTheme.foregroundMuted}>
            💡 <b>[↑/↓]</b> Navigate  •  <b>[Enter/Tab]</b> Open / Execute  •  <b>[Esc]</b> {menuTree.menu_id === "main" ? "Close" : "Back"}
          </text>
        </box>
      </box>
    );
  }

  return null;
};
