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

export const ChatMessageList = forwardRef<any, ChatMessageListProps>(({
  messages,
  state,
}, ref): any => {
  return (
    <scrollbox
      ref={ref}
      flexGrow={1}
      flexShrink={1}
      minHeight={0}
      width="100%"
      height="100%"
      scrollY={true}
      stickyScroll={true}
      stickyStart="bottom"
    >
      {/* Welcome Banner */}
      <box flexDirection="column" paddingLeft={1} paddingRight={1} paddingTop={1} paddingBottom={0} flexShrink={0}>
        <text fg={anyContextTheme.accent}>
          <b>{ASCII_BANNER}</b>
        </text>
        <text fg={anyContextTheme.accentWarning}>
          <b>  🚀 AnyContext (actx) v{state?.version || "0.28.43"}</b>  <span fg={anyContextTheme.ruleColor}>│</span>  <span fg={anyContextTheme.accentSecondary}>Levix Digital</span>  <span fg={anyContextTheme.ruleColor}>│</span>  <span fg={anyContextTheme.accentSuccess}>{state?.tier_name || "🌿 Community Edition"}</span>
        </text>
        <text fg={anyContextTheme.foregroundMuted}>
          {"  ⚡ Transform any file, folder, website, or drive into a living, real-time AI context."}
        </text>
        <text fg={anyContextTheme.foregroundMuted}>
          {"  🔒 100% Local & Offline-First Privacy"}
        </text>

        <box
          borderStyle="rounded"
          borderColor={anyContextTheme.ruleColor}
          paddingLeft={1}
          paddingRight={1}
          paddingTop={0}
          paddingBottom={0}
          marginTop={1}
          marginBottom={1}
          flexDirection="column"
          flexShrink={0}
        >
          <text fg={anyContextTheme.foreground}>
            💬 Chat started! Type <b>'/'</b> for quick commands, <b>'/switch'</b> to change workspace, <b>'/model'</b> for LLM, <b>'/menu'</b> for config, or <b>'/exit'</b> to quit.
          </text>
          <text fg={anyContextTheme.foregroundMuted}>
            📜 <i>Scroll Hint:</i> Use <b>PgUp / PgDn</b>, <b>Shift + ↑ / ↓</b>, or mouse wheel to scroll.
          </text>
        </box>
      </box>

      {/* Render All Conversation Messages */}
      {messages.map((msg) => {
        if (msg.role === "user") {
          return (
            <box
              key={msg.id}
              flexDirection="column"
              paddingTop={1}
              paddingBottom={0}
              paddingLeft={1}
              paddingRight={1}
              flexShrink={0}
            >
              <text fg={anyContextTheme.accent}>
                <b>👤 You: </b>
                <span fg={anyContextTheme.foreground}><b>{msg.content}</b></span>
              </text>
            </box>
          );
        }

        if (msg.role === "system") {
          const isError = msg.content.startsWith("❌") || msg.content.toLowerCase().includes("error:");
          const isSuccess = msg.content.startsWith("✔") || msg.content.startsWith("✅") || msg.content.toLowerCase().includes("success");
          const headerColor = isError ? anyContextTheme.accentError : isSuccess ? anyContextTheme.accentSuccess : anyContextTheme.accentSecondary;
          const headerTitle = isError ? "❌ System Error" : isSuccess ? "✅ System Notification" : "💡 System Info";

          return (
            <box
              key={msg.id}
              flexDirection="column"
              paddingTop={1}
              paddingBottom={0}
              paddingLeft={1}
              paddingRight={1}
              flexShrink={0}
            >
              <text fg={headerColor}>
                <b>{headerTitle}:</b>
              </text>
              <markdown content={msg.content} syntaxStyle={defaultSyntaxStyle} flexShrink={0} />
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
            paddingLeft={1}
            paddingRight={1}
            flexShrink={0}
          >
            <text fg={anyContextTheme.accentWarning}>
              <b>🤖 AI [</b>
              <span fg={anyContextTheme.accentSecondary}><b>{msg.model || state?.model_display || state?.model || "GPT-4o Mini"}</b></span>
              <b>]:</b>
            </text>

            {msg.ticker ? (
              <text fg={anyContextTheme.accentWarning}>
                <b>⚡ {msg.ticker}</b>
              </text>
            ) : null}

            {msg.content ? (
              <markdown content={msg.content} syntaxStyle={defaultSyntaxStyle} flexShrink={0} />
            ) : (
              <text fg={anyContextTheme.foregroundMuted}>Thinking...</text>
            )}
          </box>
        );
      })}
    </scrollbox>
  );
});
