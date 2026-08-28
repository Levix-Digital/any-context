import React from "react";
import { AnyContextState } from "../bridge-client";
import { anyContextTheme } from "../themes";

interface StatusBarProps {
  state: AnyContextState;
  onToggleMode?: () => void;
}

export const StatusBar = ({ state }: StatusBarProps): any => {
  const currentMode = (state.grounding_mode || "strict").toLowerCase();
  const cleanMode = currentMode.charAt(0).toUpperCase() + currentMode.slice(1);
  const searchBadge = state.web_search_enabled ? "🌐 Search: ON" : "🌐 Search: OFF";
  const searchColor = state.web_search_enabled ? anyContextTheme.accentSuccess : anyContextTheme.foregroundMuted;

  return (
    <box
      flexDirection="row"
      justifyContent="space-between"
      alignItems="center"
      backgroundColor={anyContextTheme.background}
      border={["top"]}
      borderStyle="single"
      borderColor={anyContextTheme.ruleColor}
      paddingLeft={1}
      paddingRight={1}
      flexShrink={0}
      minHeight={2}
    >
      {/* Left Dock Items: Workspace | Model | Mode | Web Search | /menu | Syncing */}
      <box flexDirection="row" alignItems="center">
        <text fg={anyContextTheme.accentWarning}>
          <b>📂 {state.workspace}</b>
        </text>
        <text fg={anyContextTheme.ruleColor}> │ </text>
        <text fg={anyContextTheme.accentSecondary}>
          <b>🤖 {state.model}</b>
        </text>
        <text fg={anyContextTheme.ruleColor}> │ </text>
        <text fg={anyContextTheme.accent}>
          <b>🛡️ {cleanMode}</b>
        </text>
        <text fg={anyContextTheme.ruleColor}> │ </text>
        <text fg={searchColor}>
          <b>{searchBadge}</b>
        </text>
        <text fg={anyContextTheme.ruleColor}> │ </text>
        <text fg={anyContextTheme.accentWarning}>
          <b>💡 /menu</b>
        </text>
        {state.is_syncing ? (
          <>
            <text fg={anyContextTheme.ruleColor}> │ </text>
            <text fg={anyContextTheme.accentWarning}>
              <b>⚡ Syncing {state.sync_info}</b>
            </text>
          </>
        ) : state.sync_info ? (
          <>
            <text fg={anyContextTheme.ruleColor}> │ </text>
            <text fg={anyContextTheme.accentSuccess}>
              <b>✔ {state.sync_info}</b>
            </text>
          </>
        ) : null}
      </box>

      {/* Right Dock Items: Terminal Scroll Guide & /exit */}
      <box flexDirection="row" alignItems="center">
        <text fg={anyContextTheme.foregroundMuted}>
          📜 <span fg={anyContextTheme.accent}>Shift+PgUp/PgDn</span> (scroll)
        </text>
        <text fg={anyContextTheme.ruleColor}> │ </text>
        <text fg={anyContextTheme.accentError}>
          <b>🚪 /exit</b>
        </text>
      </box>
    </box>
  );
};




