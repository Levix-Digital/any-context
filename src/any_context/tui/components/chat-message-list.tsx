import React, { forwardRef } from "react";
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

export const ChatMessageList = forwardRef<any, ChatMessageListProps>(({ messages, state }, ref): any => {
  return (
    <scrollbox
      ref={ref}
      flexGrow={1}
      flexShrink={1}
      minHeight={0}
      flexDirection="column"
      paddingLeft={1}
      paddingRight={1}
      stickyScroll={true}
    >
      {/* Render Conversation Messages */}
      {messages.map((msg) => {
        if (msg.role === "user") {
          return (
            <box
              key={msg.id}
              flexDirection="column"
              paddingTop={1}
              paddingBottom={0}
            >
              <text fg={anyContextTheme.accent}>
                <b>👤 You: </b>
                <span fg={anyContextTheme.foreground}><b>{msg.content}</b></span>
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
            <text fg={anyContextTheme.accentWarning}>
              <b>🤖 AI [</b>
              <span fg={anyContextTheme.accentSecondary}><b>{msg.model || state?.model || "gpt-4o-mini"}</b></span>
              <b>]:</b>
            </text>

            {msg.ticker ? (
              <text fg={anyContextTheme.accentWarning}>
                <b>⚡ {msg.ticker}</b>
              </text>
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
});
