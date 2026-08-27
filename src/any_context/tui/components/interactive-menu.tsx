import React from "react";
import type { AnyContextState } from "../bridge-client";
import { anyContextTheme } from "../themes";

export interface MenuItem {
  id: string;
  icon: string;
  title: string;
  command: string;
  description: string;
}

export const MENU_ITEMS: MenuItem[] = [
  {
    id: "workspaces",
    icon: "📂",
    title: "Workspaces & Context",
    command: "/switch",
    description: "Switch active workspace or create a new isolated context",
  },
  {
    id: "models",
    icon: "🤖",
    title: "AI Inference Models",
    command: "/model",
    description: "Select active model (GPT-4o, Claude 3.5, Gemini, DeepSeek, Local)",
  },
  {
    id: "web-search",
    icon: "🌐",
    title: "Real-Time Web Search",
    command: "/web-search",
    description: "Toggle real-time workspace Web Search grounding (ON / OFF)",
  },
  {
    id: "grounding",
    icon: "🛡️",
    title: "Grounding Strategy Mode",
    command: "/mode",
    description: "Configure Strict, Hybrid, or Proactive grounding mode",
  },
  {
    id: "sources",
    icon: "📁",
    title: "Workspace Sources",
    command: "/sources",
    description: "View all indexed folders, web portals, and cloud drives",
  },
  {
    id: "sync",
    icon: "⚡",
    title: "Synchronize Sources",
    command: "/sync",
    description: "Re-index all documents and web portals in workspace",
  },
  {
    id: "config",
    icon: "⚙️",
    title: "System Settings & Keys",
    command: "/config",
    description: "Configure system settings, API keys, and environment",
  },
  {
    id: "billing",
    icon: "💳",
    title: "Billing & Subscription",
    command: "/billing",
    description: "Inspect subscription plan and license capabilities",
  },
  {
    id: "memory",
    icon: "🧠",
    title: "Session Memory & Reset",
    command: "/reset-memory",
    description: "Reset long-term session memory database for workspace",
  },
  {
    id: "clear",
    icon: "🧹",
    title: "Clear Chat View",
    command: "/clear",
    description: "Clear current visual chat messages",
  },
  {
    id: "help",
    icon: "❓",
    title: "Help & Documentation",
    command: "/help",
    description: "Browse comprehensive documentation and command reference",
  },
  {
    id: "exit",
    icon: "🚪",
    title: "Exit AnyContext",
    command: "/exit",
    description: "Save session memory and exit safely",
  },
];

interface InteractiveMenuProps {
  isOpen: boolean;
  selectedIndex: number;
  state?: AnyContextState;
}

export const InteractiveMenu = ({
  isOpen,
  selectedIndex,
  state,
}: InteractiveMenuProps): any => {
  if (!isOpen) return null;

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
    >
      {/* Header */}
      <box flexDirection="row" paddingTop={0} paddingBottom={0} marginBottom={0}>
        <text fg={anyContextTheme.accentWarning}>
          <b>📋 AnyContext Interactive Menu</b>
        </text>
        <text fg={anyContextTheme.ruleColor}>  │  </text>
        <text fg={anyContextTheme.accentSecondary}>
          Workspace: <b>{state?.workspace || "Default"}</b>
        </text>
      </box>

      {/* Separator */}
      <box flexDirection="row" marginTop={0} marginBottom={0}>
        <text fg={anyContextTheme.ruleColor}>─────────────────────────────────────────────────────────────────</text>
      </box>

      {/* Menu Options List */}
      {MENU_ITEMS.map((item, idx) => {
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
            <text fg={anyContextTheme.accentSuccess}> ({item.command}) </text>
            <text fg={anyContextTheme.foregroundMuted}> - {item.description}</text>
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
          💡 <b>[↑/↓]</b> Select Option  •  <b>[Enter/Tab]</b> Execute  •  <b>[Esc]</b> Close Menu
        </text>
      </box>
    </box>
  );
};
