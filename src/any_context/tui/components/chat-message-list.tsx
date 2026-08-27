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
  scrollOffset?: number;
  windowSize?: number;
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
  scrollOffset = 0,
  windowSize = 8,
}, ref): any => {
  const total = messages.length;
  const maxOffset = Math.max(0, total - windowSize);
  const clampedOffset = Math.min(Math.max(0, scrollOffset), maxOffset);

  const endIdx = total <= windowSize ? total : total - clampedOffset;
  const startIdx = total <= windowSize ? 0 : Math.max(0, endIdx - windowSize);
  const visibleMessages = messages.slice(startIdx, endIdx);

  const hasOlder = startIdx > 0;
  const hasNewer = clampedOffset > 0;
  const showBanner = startIdx === 0;

  return (
    <box
      ref={ref}
      flexGrow={1}
      flexShrink={1}
      minHeight={0}
      flexDirection="column"
      paddingLeft={1}
      paddingRight={1}
      overflow="hidden"
    >
      {/* Top Indicator if older messages exist */}
      {hasOlder && (
        <box flexDirection="row" justifyContent="center" paddingTop={1} paddingBottom={0} flexShrink={0}>
          <text fg={anyContextTheme.accentSecondary}>
            <b>▲ [{startIdx} mensagens anteriores - pressione PageUp para subir]</b>
          </text>
        </box>
      )}

      {/* Welcome Banner when at the very beginning of history */}
      {showBanner && (
        <box flexDirection="column" paddingLeft={1} paddingRight={1} paddingTop={1} paddingBottom={0} flexShrink={0}>
          <text fg={anyContextTheme.accent}>
            <b>{ASCII_BANNER}</b>
          </text>
          <text fg={anyContextTheme.accentWarning}>
            <b>  🚀 AnyContext (actx) v{state?.version || "0.28.11"}</b>  <span fg={anyContextTheme.ruleColor}>│</span>  <span fg={anyContextTheme.accentSecondary}>Levix Digital</span>  <span fg={anyContextTheme.ruleColor}>│</span>  <span fg={anyContextTheme.accentSuccess}>{state?.tier_name || "Community Edition"}</span>
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
              💬 Chat started! Type <b>'/'</b> for quick commands, <b>'/switch'</b> to change workspace, <b>'/menu'</b> for config, or <b>'/exit'</b> to quit.
            </text>
          </box>
        </box>
      )}

      {/* Render Sliced Conversation Messages */}
      {visibleMessages.map((msg) => {
        if (msg.role === "user") {
          return (
            <box
              key={msg.id}
              flexDirection="column"
              paddingTop={1}
              paddingBottom={0}
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
              flexShrink={0}
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
            flexShrink={0}
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
              <markdown content={msg.content} syntaxStyle={defaultSyntaxStyle} flexShrink={0} />
            ) : (
              <text fg={anyContextTheme.foregroundMuted}>Thinking...</text>
            )}
          </box>
        );
      })}

      {/* Bottom Indicator if newer messages exist below */}
      {hasNewer && (
        <box flexDirection="row" justifyContent="center" paddingTop={1} paddingBottom={0} flexShrink={0}>
          <text fg={anyContextTheme.accentWarning}>
            <b>▼ [{clampedOffset} mensagens mais recentes abaixo - pressione PageDown ou End para descer]</b>
          </text>
        </box>
      )}
    </box>
  );
});
