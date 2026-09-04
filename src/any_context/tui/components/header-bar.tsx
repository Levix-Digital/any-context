import React from "react";
import type { AnyContextState } from "../bridge-client";
import { getInitialVersion } from "../bridge-client";
import { anyContextTheme } from "../themes";

interface HeaderBarProps {
  state?: AnyContextState;
  hasMessages?: boolean;
}

export const HeaderBar = ({ state, hasMessages }: HeaderBarProps): any => {
  // If there are no messages yet, do not show top dock bar (banner is visible inside the chat window)
  if (!hasMessages) {
    return null;
  }

  const versionStr = state?.version || getInitialVersion();
  const displayTier = state?.tier_name || "🌿 Community Edition";
  const wsName = state?.workspace || "Default";

  return (
    <box
      flexDirection="column"
      flexShrink={0}
      paddingTop={0}
      paddingBottom={0}
      border={["bottom"]}
      borderStyle="single"
      borderColor={anyContextTheme.ruleColor}
    >
      <box flexDirection="row" paddingLeft={1} paddingRight={1} paddingTop={0} paddingBottom={0}>
        <text fg={anyContextTheme.accentWarning}>
          <b>🚀 AnyContext (actx) v{versionStr}</b>
        </text>
        <text fg={anyContextTheme.ruleColor}>  │  </text>
        <text fg={anyContextTheme.accentSecondary}>
          <b>Levix Digital</b>
        </text>
        <text fg={anyContextTheme.ruleColor}>  │  </text>
        <text fg={anyContextTheme.accentSuccess}>
          <b>{displayTier}</b>
        </text>
        <text fg={anyContextTheme.ruleColor}>  │  </text>
        <text fg={anyContextTheme.foreground}>
          📂 <b>{wsName}</b>
        </text>
      </box>
    </box>
  );
};
