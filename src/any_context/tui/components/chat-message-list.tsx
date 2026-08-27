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

const ASCII_BANNER = `  ___               ____ ___  _   _ _____ _____ _  _______ 
 / _ \\ _ __  _   _ / ___/ _ \\| \\ | |_   _| ____\\ \\/ /_   _|
| |_| | '_ \\| | | | |  | | | |  \\| | | | |  _|  \\  /  | |  
|  _  | | | | |_| | |__| |_| | |\\  | | | | |___ /  \\  | |  
|_| |_|_| |_|\\__, |\\____\\___/|_| \\_| |_| |_____/_/\\_\\ |_|  
             |___/                                          `;

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
      {/* Initial Welcome Banner inside scrollbox */}
      {messages.length === 0 ? (
        <box flexDirection="column" paddingLeft={1} paddingRight={1} paddingTop={1}>
          <text fg={anyContextTheme.accent}>
            <b>{ASCII_BANNER}</b>
          </text>
          <text fg={anyContextTheme.accentWarning}>
            <b>  🚀 AnyContext (actx) v{state?.version || "0.28.5"}</b>  <span fg={anyContextTheme.ruleColor}>│</span>  <span fg={anyContextTheme.accentSecondary}>Levix Digital</span>  <span fg={anyContextTheme.ruleColor}>│</span>  <span fg={anyContextTheme.accentSuccess}>{state?.tier_name || "Community Edition"}</span>
          </text>
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
            flexDirection="column"
          >
            <text fg={anyContextTheme.foreground}>
              💬 Chat started! Type <b>'/'</b> for quick commands, <b>'/switch'</b> to change workspace, <b>'/menu'</b> for config, or <b>'/exit'</b> to quit.
            </text>
          </box>
        </box>
      ) : null}

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
