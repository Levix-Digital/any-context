import React from "react";
import { AnyContextState } from "../bridge-client";
import { anyContextTheme } from "../themes";

interface StatusBarProps {
  state: AnyContextState;
  onToggleMode?: () => void;
}

export const StatusBar = ({ state, onToggleMode }: StatusBarProps): any => {
  const currentMode = (state.grounding_mode || "strict").toLowerCase();
  const searchBadge = state.web_search_enabled ? "🌐 Web Search: ON" : "🌐 Web Search: OFF";
  const searchColor = state.web_search_enabled ? anyContextTheme.accentSuccess : anyContextTheme.foregroundMuted;

  return (
    <box
      flexDirection="column"
      backgroundColor={anyContextTheme.background}
      paddingLeft={1}
      paddingRight={1}
      paddingTop={0}
      paddingBottom={0}
    >
      {/* Row 1: Model & Grounding Mode Pills */}
      <box flexDirection="row" justifyContent="space-between" alignItems="center">
        <box flexDirection="row">
          <text fg={anyContextTheme.accentSecondary}>
            <b>🤖 {state.model}</b>
          </text>
          <text fg={anyContextTheme.foregroundMuted}> │ </text>
          <text fg={searchColor}>
            <b>{searchBadge}</b>
          </text>
        </box>

        {/* Mode Pills (Cline Style: Strict / Hybrid / Proactive) */}
        <box flexDirection="row" gap={1}>
          <text fg={currentMode === "strict" ? anyContextTheme.accent : anyContextTheme.foregroundMuted}>
            {currentMode === "strict" ? "●" : "○"} Strict
          </text>
          <text fg={currentMode === "hybrid" ? anyContextTheme.accentWarning : anyContextTheme.foregroundMuted}>
            {currentMode === "hybrid" ? "●" : "○"} Hybrid
          </text>
          <text fg={currentMode === "proactive" ? anyContextTheme.accentSuccess : anyContextTheme.foregroundMuted}>
            {currentMode === "proactive" ? "●" : "○"} Proactive
          </text>
          <text fg={anyContextTheme.foregroundMuted}>(/mode)</text>
        </box>
      </box>

      {/* Row 2: Workspace Info & Sync Status & Exit Shortcut */}
      <box flexDirection="row" justifyContent="space-between" alignItems="center">
        <box flexDirection="row">
          <text fg={anyContextTheme.accentWarning}>
            <b>📂 {state.workspace}</b>
          </text>
          {state.is_syncing ? (
            <>
              <text fg={anyContextTheme.foregroundMuted}> │ </text>
              <text fg={anyContextTheme.accentWarning}>
                <b>⚡ Syncing {state.sync_info}</b>
              </text>
            </>
          ) : (
            <>
              <text fg={anyContextTheme.foregroundMuted}> │ </text>
              <text fg={anyContextTheme.foregroundMuted}>⚡ Ready</text>
            </>
          )}
        </box>

        <box flexDirection="row">
          <text fg={anyContextTheme.foregroundMuted}>💡 /help │ 🚪 /exit</text>
        </box>
      </box>
    </box>
  );
};



