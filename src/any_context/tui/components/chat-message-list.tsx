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

export const ChatMessageList = ({ messages, state }: ChatMessageListProps): any => {
  return (
    <scrollbox flexGrow={1} flexDirection="column" paddingLeft={1} paddingRight={1} stickyScroll={true}>
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
