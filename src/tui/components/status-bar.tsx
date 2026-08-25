import React from "react";
import { AnyContextState } from "../bridge-client";

interface StatusBarProps {
  state: AnyContextState;
}

export const StatusBar = ({ state }: StatusBarProps): any => {
  const modeCap = (state.grounding_mode || "strict").toUpperCase();
  const searchBadge = state.web_search_enabled ? "🌐 Search: ON" : "🌐 Search: OFF";
  const searchColor = state.web_search_enabled ? "#73daca" : "#565f89";

  return (
    <box
      flexDirection="row"
      backgroundColor="#16161e"
      borderStyle="single"
      borderColor="#3b4261"
      paddingLeft={1}
      paddingRight={1}
      height={3}
      alignItems="center"
    >
      <text fg="#e0af68">
        <b>📂 {state.workspace}</b>
      </text>
      <text fg="#565f89"> │ </text>

      <text fg="#bb9af7">
        <b>🤖 {state.model}</b>
      </text>
      <text fg="#565f89"> │ </text>

      <text fg="#7aa2f7">
        <b>🛡️ {modeCap}</b>
      </text>
      <text fg="#565f89"> │ </text>

      <text fg={searchColor}>
        <b>{searchBadge}</b>
      </text>

      {state.is_syncing && (
        <>
          <text fg="#565f89"> │ </text>
          <text fg="#ff9e64">
            <b>⚡ Syncing {state.sync_info}</b>
          </text>
        </>
      )}

      <text fg="#565f89"> │ </text>
      <text fg="#565f89">💡 /help │ 🚪 /exit</text>
    </box>
  );
};


