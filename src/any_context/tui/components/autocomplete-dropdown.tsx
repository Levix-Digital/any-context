import React from "react";
import { SlashCommandMeta } from "../bridge-client";
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

  const query = filterText.startsWith("/") ? filterText.slice(1).toLowerCase().trim() : filterText.toLowerCase().trim();
  const filtered = commands.filter(
    (c) =>
      c.command.toLowerCase().includes(query) ||
      c.description.toLowerCase().includes(query) ||
      c.category.toLowerCase().includes(query)
  );

  const displayList = filtered.slice(0, 6);

  return (
    <box
      flexDirection="column"
      backgroundColor={anyContextTheme.surface}
      borderStyle="rounded"
      borderColor={anyContextTheme.accent}
      paddingLeft={1}
      paddingRight={1}
      paddingTop={0}
      paddingBottom={0}
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
      <text fg={anyContextTheme.foregroundMuted}>
        [↑/↓ Navigate • Tab Select • Esc Close]
      </text>
    </box>
  );
};
