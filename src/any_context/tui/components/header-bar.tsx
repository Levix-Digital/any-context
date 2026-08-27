import React from "react";
import type { AnyContextState } from "../bridge-client";
import { anyContextTheme } from "../themes";

const ASCII_BANNER = `  ___               ____ ___  _   _ _____ _____ _  _______ 
 / _ \\ _ __  _   _ / ___/ _ \\| \\ | |_   _| ____\\ \\/ /_   _|
| |_| | '_ \\| | | | |  | | | |  \| | | | |  _|  \\  /  | |  
|  _  | | | | |_| | |__| |_| | |\\  | | | | |___ /  \\  | |  
|_| |_|_| |_|\\__, |\\____\\___/|_| \\_| |_| |_____/_/\\_\\ |_|  
             |___/                                          `;

interface HeaderBarProps {
  state?: AnyContextState;
  hasMessages: boolean;
}

export const HeaderBar = ({ state, hasMessages }: HeaderBarProps): any => {
  const versionStr = state?.version || "0.27.6";
  const tierStr = state?.tier_name || "Community Edition";
  const tierIcon = tierStr.includes("Enterprise") ? "🏢" : tierStr.includes("Pro") ? "⭐" : "🌿";

  // Compact Top Bar Mode during active conversation (1+ messages)
  if (hasMessages) {
    return (
      <box flexDirection="column" flexShrink={0} paddingTop={0} paddingBottom={0}>
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
            <b>{tierIcon} {tierStr}</b>
          </text>
        </box>
        <box flexDirection="row" marginTop={0} marginBottom={0}>
          <text fg={anyContextTheme.ruleColor}>─────────────────────────────────────────────────────────────────</text>
        </box>
      </box>
    );
  }

  // Full Glory Mode at startup and after /clear (0 messages)
  return (
    <box flexDirection="column" flexShrink={0} paddingLeft={1} paddingRight={1} paddingTop={1} paddingBottom={0}>
      <text fg={anyContextTheme.accent}>
        <b>{ASCII_BANNER}</b>
      </text>
      <box flexDirection="row" paddingTop={0} paddingBottom={0}>
        <text fg={anyContextTheme.accentWarning}>
          <b>  🚀 AnyContext (actx) v{versionStr}</b>
        </text>
        <text fg={anyContextTheme.ruleColor}>  │  </text>
        <text fg={anyContextTheme.accentSecondary}>
          <b>Levix Digital</b>
        </text>
        <text fg={anyContextTheme.ruleColor}>  │  </text>
        <text fg={anyContextTheme.accentSuccess}>
          <b>{tierIcon} {tierStr}</b>
        </text>
      </box>
      <text fg={anyContextTheme.foregroundMuted}>
        {"  ⚡ Transform any file, folder, website, or drive into a living, real-time AI context."}
      </text>
      <text fg={anyContextTheme.foregroundMuted}>
        {"  🔒 100% Local & Offline-First Privacy"}
      </text>

      <box
        borderStyle="single"
        borderColor={anyContextTheme.ruleColor}
        paddingLeft={1}
        paddingRight={1}
        paddingTop={0}
        paddingBottom={0}
        marginTop={1}
        marginBottom={1}
        flexDirection="row"
      >
        <text fg={anyContextTheme.foreground}>
          💬 Chat started! Type <b>'/'</b> for quick commands, <b>'/menu'</b> for interactive menu, or <b>'/exit'</b> to quit.
        </text>
      </box>
    </box>
  );
};
