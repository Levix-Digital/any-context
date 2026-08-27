import React from "react";
import type { SlashCommandMeta } from "../commands";
import { filterSlashCommands, MAX_PALETTE_ITEMS } from "../commands";
import { anyContextTheme } from "../themes";

interface AutocompleteDropdownProps {
  isOpen: boolean;
  filterText: string;
  commands: SlashCommandMeta[];
  selectedIndex: number;
}

export const AutocompleteDropdown = ({
  isOpen,
  filterText,
  commands,
  selectedIndex,
}: AutocompleteDropdownProps): any => {
  if (!isOpen) return null;

  const raw = filterText || "";
  const query = raw.startsWith("/") ? raw.slice(1).toLowerCase().trim() : raw.toLowerCase().trim();
  const filtered = filterSlashCommands(commands, filterText);
  const displayList = filtered.slice(0, MAX_PALETTE_ITEMS);

  return (
    <box
      flexDirection="column"
      backgroundColor={anyContextTheme.surface}
      borderStyle="rounded"
      borderColor={anyContextTheme.accent}
      paddingLeft={1}
      paddingRight={1}
      paddingTop={0}
      paddingBottom={1}
      marginBottom={0}
    >
      <box flexDirection="row" marginBottom={0}>
        <text fg={anyContextTheme.accentWarning}>
          <b>📚 Slash Commands Palette</b>
        </text>
        {query.length > 0 && (
          <text fg={anyContextTheme.foregroundMuted}> (Filtering: "{query}")</text>
        )}
      </box>

      {displayList.length === 0 ? (
        <text fg={anyContextTheme.accentError}>No matching command found.</text>
      ) : (
        displayList.map((cmd, idx) => {
          const isSelected = idx === selectedIndex;
          const prefix = isSelected ? "▸ " : "  ";
          const bgColor = isSelected ? anyContextTheme.surfaceHighlight : undefined;
          const cmdColor = isSelected ? anyContextTheme.accent : anyContextTheme.foreground;

          return (
            <box
              key={cmd.command}
              flexDirection="row"
              backgroundColor={bgColor}
              paddingLeft={1}
              paddingRight={1}
            >
              <text fg={cmdColor}>
                <b>{prefix}{cmd.command}</b>
              </text>
              <text fg={anyContextTheme.accentSuccess}> {cmd.args} </text>
              <text fg={anyContextTheme.foreground}> - {cmd.description} </text>
              <text fg={anyContextTheme.accentSecondary}>[{cmd.category}]</text>
            </box>
          );
        })
      )}

      {/* Dedicated Separator */}
      <box flexDirection="row" marginTop={0} marginBottom={0}>
        <text fg={anyContextTheme.ruleColor}>─────────────────────────────────────────────────────────────────</text>
      </box>

      {/* Dedicated Navigation Footer */}
      <box flexDirection="row" paddingLeft={1} paddingRight={1} marginTop={0}>
        <text fg={anyContextTheme.foregroundMuted}>
          💡 <b>[↑/↓]</b> Navigate  •  <b>[↹ Tab]</b> Select  •  <b>[Esc]</b> Close
        </text>
      </box>
    </box>
  );
};
