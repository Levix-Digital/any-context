import React from "react";
import { SlashCommandMeta } from "../bridge-client";

interface PaletteProps {
  isOpen: boolean;
  filterText: string;
  commands: SlashCommandMeta[];
  selectedIndex: number;
}

export const SlashCommandPalette = ({
  isOpen,
  filterText,
  commands,
  selectedIndex,
}: PaletteProps): any => {
  if (!isOpen) return null;

  const query = filterText.startsWith("/") ? filterText.slice(1).toLowerCase().trim() : filterText.toLowerCase().trim();
  const filtered = commands.filter(
    (c) =>
      c.command.toLowerCase().includes(query) ||
      c.description.toLowerCase().includes(query) ||
      c.category.toLowerCase().includes(query)
  );

  const displayList = filtered.slice(0, 7);

  return (
    <box
      borderStyle="rounded"
      borderColor="#7aa2f7"
      backgroundColor="#1f2335"
      paddingLeft={1}
      paddingRight={1}
      paddingTop={0}
      paddingBottom={0}
      flexDirection="column"
      marginBottom={1}
    >
      <box flexDirection="row" marginBottom={0}>
        <text fg="#e0af68">
          <b>📚 Slash Commands Palette</b>
        </text>
        {query.length > 0 && (
          <text fg="#565f89"> (Filtering: "{query}")</text>
        )}
      </box>

      {displayList.length === 0 ? (
        <text fg="#f7768e">No matching slash command found.</text>
      ) : (
        displayList.map((cmd, idx) => {
          const isSelected = idx === selectedIndex;
          const prefix = isSelected ? "▸ " : "  ";
          const bgColor = isSelected ? "#24283b" : undefined;
          const cmdColor = isSelected ? "#7dcfff" : "#7aa2f7";

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
              <text fg="#9ece6a"> {cmd.args} </text>
              <text fg="#c0caf5"> - {cmd.description} </text>
              <text fg="#bb9af7">[{cmd.category}]</text>
            </box>
          );
        })
      )}
      <text fg="#565f89">
        [↑/↓ Navigate • Tab/Enter Select • Esc Close]
      </text>
    </box>
  );
};


