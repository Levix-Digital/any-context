import React from "react";
import { SyntaxStyle } from "@opentui/core";
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
}

const defaultSyntaxStyle = (SyntaxStyle as any).create ? (SyntaxStyle as any).create() : new (SyntaxStyle as any)();

export const ChatMessageList = ({ messages }: ChatMessageListProps): any => {
  return (
    <scrollbox flexGrow={1} flexDirection="column" paddingLeft={1} paddingRight={1}>
      {messages.length === 0 ? (
        <box
          borderStyle="rounded"
          borderColor={anyContextTheme.ruleColor}
          backgroundColor={anyContextTheme.surface}
          padding={1}
          marginTop={1}
          marginBottom={1}
          flexDirection="column"
        >
          <text fg={anyContextTheme.accentWarning}>
            <b>🚀 AnyContext OpenTUI</b>
          </text>
          <text fg={anyContextTheme.foreground}>
            • Universal Multi-Context RAG Assistant & Engine.
          </text>
          <text fg={anyContextTheme.foreground}>
            • Type your question below or type <b>/</b> to open the Slash Command Palette.
          </text>
          <text fg={anyContextTheme.accent}>
            • 100% native mouse selection and clipboard (Ctrl+C / Ctrl+V) enabled.
          </text>
        </box>
      ) : (
        messages.map((msg) => {
          if (msg.role === "user") {
            return (
              <box
                key={msg.id}
                flexDirection="row"
                paddingTop={1}
                paddingBottom={1}
              >
                <box paddingRight={1}>
                  <text fg={anyContextTheme.accent}>
                    <b>❯</b>
                  </text>
                </box>
                <box flexGrow={1}>
                  <text fg={anyContextTheme.foreground}>
                    <b>{msg.content}</b>
                  </text>
                </box>
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
              paddingBottom={1}
              border={["bottom"]}
              borderStyle="single"
              borderColor={anyContextTheme.ruleColor}
            >
              <box flexDirection="row" marginBottom={0}>
                <text fg={anyContextTheme.accentSecondary}>
                  <b>🤖 AnyContext</b>
                </text>
                {msg.model ? (
                  <text fg={anyContextTheme.foregroundMuted}> ({msg.model})</text>
                ) : null}
              </box>

              {msg.ticker ? (
                <box paddingTop={0} paddingBottom={0}>
                  <text fg={anyContextTheme.accentWarning}>
                    <i>⚡ {msg.ticker}</i>
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
        })
      )}
    </scrollbox>
  );
};
