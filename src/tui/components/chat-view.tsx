import React from "react";
import { SyntaxStyle } from "@opentui/core";

export interface MessageItem {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  ticker?: string;
  model?: string;
}

interface ChatViewProps {
  messages: MessageItem[];
}

const defaultSyntaxStyle = (SyntaxStyle as any).create ? (SyntaxStyle as any).create() : new (SyntaxStyle as any)();

export const ChatView = ({ messages }: ChatViewProps): any => {
  return (
    <scrollbox flexGrow={1} flexDirection="column" paddingLeft={1} paddingRight={1}>
      {messages.length === 0 ? (
        <box
          borderStyle="rounded"
          borderColor="#3b4261"
          backgroundColor="#1f2335"
          padding={1}
          margin={1}
          flexDirection="column"
        >
          <text fg="#e0af68">
            <b>🚀 Welcome to AnyContext OpenTUI!</b>
          </text>
          <text fg="#c0caf5">
            • Type your query to chat with your local documents, web portals, and knowledge base.
          </text>
          <text fg="#c0caf5">
            • Type <b>/</b> to open the interactive Slash Command Palette (/switch, /model, /sync, /help).
          </text>
          <text fg="#7aa2f7">
            • Native mouse text selection, smooth mouse wheel scrolling, and Copy/Paste enabled.
          </text>
        </box>
      ) : (
        messages.map((msg) => {
          if (msg.role === "user") {
            return (
              <box
                key={msg.id}
                flexDirection="column"
                backgroundColor="#24283b"
                borderStyle="rounded"
                borderColor="#7aa2f7"
                paddingLeft={1}
                paddingRight={1}
                marginTop={1}
                marginBottom={1}
              >
                <text fg="#7dcfff">
                  <b>👤 You</b>
                </text>
                <text fg="#c0caf5">{msg.content}</text>
              </box>
            );
          }

          if (msg.role === "system") {
            return (
              <box
                key={msg.id}
                flexDirection="column"
                backgroundColor="#16161e"
                borderStyle="rounded"
                borderColor="#9ece6a"
                paddingLeft={1}
                paddingRight={1}
                marginTop={1}
                marginBottom={1}
              >
                <text fg="#9ece6a">
                  <b>💡 System:</b> {msg.content}
                </text>
              </box>
            );
          }

          return (
            <box
              key={msg.id}
              flexDirection="column"
              backgroundColor="#1f2335"
              borderStyle="rounded"
              borderColor="#bb9af7"
              paddingLeft={1}
              paddingRight={1}
              marginTop={1}
              marginBottom={1}
            >
              <text fg="#e0af68">
                <b>🤖 AI [{msg.model || "gpt-4o-mini"}]</b>
              </text>

              {msg.ticker ? (
                <text fg="#ff9e64">
                  <i>⚡ {msg.ticker}</i>
                </text>
              ) : null}

              {msg.content ? (
                <markdown content={msg.content} syntaxStyle={defaultSyntaxStyle} />
              ) : (
                <text fg="#565f89">Thinking...</text>
              )}
            </box>
          );
        })
      )}
    </scrollbox>
  );
};


