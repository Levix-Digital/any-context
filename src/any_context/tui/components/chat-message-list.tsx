import React from "react";
import { SyntaxStyle } from "@opentui/core";
import { AnyContextState } from "../bridge-client";
import { anyContextTheme } from "../themes";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  ticker?: string;
  model?: string;
  timestamp?: string;
}

interface ChatMessageListProps {
  messages: ChatMessage[];
  state?: AnyContextState;
}

const defaultSyntaxStyle = (SyntaxStyle as any).create ? (SyntaxStyle as any).create() : new (SyntaxStyle as any)();

const ASCII_BANNER = `  ___               ____ ___  _   _ _____ _____ _  _______ 
 / _ \\ _ __  _   _ / ___/ _ \\| \\ | |_   _| ____\\ \\/ /_   _|
| |_| | '_ \\| | | | |  | | | |  \\| | | | |  _|  \\  /  | |  
|  _  | | | | |_| | |__| |_| | |\\  | | | | |___ /  \\  | |  
|_| |_|_| |_|\\__, |\\____\\___/|_| \\_| |_| |_____/_/\\_\\ |_|  
             |___/                                          `;

export const ChatMessageList = ({ messages, state }: ChatMessageListProps): any => {
  const versionStr = state?.version || "0.26.8";

  return (
    <scrollbox flexGrow={1} flexDirection="column" paddingLeft={1} paddingRight={1}>
      {/* Signature ASCII Art & Startup Banner in the Scroll View */}
      <box flexDirection="column" paddingTop={1} paddingBottom={1}>
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
            <b>🌿 Community Edition</b>
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
          marginBottom={0}
          flexDirection="row"
        >
          <text fg={anyContextTheme.foreground}>
            💬 Chat started! Type <b>'/'</b> for command palette or <b>'/exit'</b> to quit.
          </text>
        </box>
      </box>

      {/* Render Conversation Messages */}
      {messages.map((msg) => {
        if (msg.role === "user") {
          return (
            <box
              key={msg.id}
              flexDirection="row"
              paddingTop={1}
              paddingBottom={0}
            >
              <text fg={anyContextTheme.accent}>
                <b>👤 You: </b>
              </text>
              <text fg={anyContextTheme.foreground}>
                <b>{msg.content}</b>
              </text>
            </box>
          );
        }

        if (msg.role === "system") {
          return (
            <box
              key={msg.id}
              flexDirection="column"
              backgroundColor={anyContextTheme.inputBackground}
              borderStyle="rounded"
              borderColor={anyContextTheme.accentSuccess}
              paddingLeft={1}
              paddingRight={1}
              paddingTop={0}
              paddingBottom={0}
              marginTop={1}
              marginBottom={1}
            >
              <text fg={anyContextTheme.accentSuccess}>
                <b>💡 System:</b> {msg.content}
              </text>
            </box>
          );
        }

        // Assistant message entry
        return (
          <box
            key={msg.id}
            flexDirection="column"
            paddingTop={1}
            paddingBottom={0}
          >
            <box flexDirection="row" marginBottom={0}>
              <text fg={anyContextTheme.accentWarning}>
                <b>🤖 AI [</b>
              </text>
              <text fg={anyContextTheme.accentSecondary}>
                <b>{msg.model || state?.model || "gpt-4o-mini"}</b>
              </text>
              <text fg={anyContextTheme.accentWarning}>
                <b>]:</b>
              </text>
            </box>

            {msg.ticker ? (
              <box paddingTop={0} paddingBottom={0}>
                <text fg={anyContextTheme.accentWarning}>
                  <b>⚡ {msg.ticker}</b>
                </text>
              </box>
            ) : null}

            {msg.content ? (
              <markdown content={msg.content} syntaxStyle={defaultSyntaxStyle} />
            ) : (
              <text fg={anyContextTheme.foregroundMuted}>Thinking...</text>
            )}
          </box>
        );
      })}
    </scrollbox>
  );
};
