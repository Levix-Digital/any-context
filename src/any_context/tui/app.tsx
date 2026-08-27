import React, { useState, useEffect } from "react";
import { useKeyboard } from "@opentui/react";
import { BridgeClient } from "./bridge-client";
import type { AnyContextState } from "./bridge-client";
import { ChatMessage } from "./components/chat-message-list";
import { ChatView } from "./views/chat-view";
import {
  filterSlashCommands,
  isDirectExecutionCommand,
  MAX_PALETTE_ITEMS,
} from "./commands";

interface AppProps {
  initialWorkspace?: string;
  onExit?: () => void;
}

export const App = ({ initialWorkspace = "Default", onExit }: AppProps): any => {
  const [client] = useState(() => new BridgeClient(initialWorkspace));
  const [state, setState] = useState<AnyContextState>(client.state);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteIndex, setPaletteIndex] = useState(0);
  const [isGenerating, setIsGenerating] = useState(false);

  useEffect(() => {
    client.onStateChange = (newState) => setState(newState);
    client.start().catch((err) => {
      setMessages((prev) => [
        ...prev,
        {
          id: `err_${Date.now()}`,
          role: "system",
          content: `Failed to connect to AnyContext Python backend: ${err.message}`,
        },
      ]);
    });

    const interval = setInterval(() => {
      client.refreshState();
    }, 2000);

    return () => {
      clearInterval(interval);
      client.stop();
    };
  }, [client]);

  const handleInputChange = (val: string) => {
    setInputValue(val);
    if (val.startsWith("/")) {
      setPaletteOpen(true);
      setPaletteIndex(0);
    } else {
      setPaletteOpen(false);
    }
  };

  useKeyboard((event) => {
    if (event.name === "escape") {
      if (paletteOpen) {
        setPaletteOpen(false);
      }
      return;
    }

    if (paletteOpen) {
      const filtered = filterSlashCommands(client.commands, inputValue);
      const displayCount = Math.min(filtered.length, MAX_PALETTE_ITEMS);

      if (event.name === "up" && displayCount > 0) {
        setPaletteIndex((prev) => (prev > 0 ? prev - 1 : displayCount - 1));
      } else if (event.name === "down" && displayCount > 0) {
        setPaletteIndex((prev) => (prev < displayCount - 1 ? prev + 1 : 0));
      } else if (event.name === "tab" && displayCount > 0) {
        const safeIdx = Math.min(paletteIndex, displayCount - 1);
        const selectedCmd = filtered[safeIdx];
        if (selectedCmd) {
          if (isDirectExecutionCommand(selectedCmd)) {
            setInputValue(selectedCmd.command);
          } else {
            setInputValue(`${selectedCmd.command} `);
          }
          setPaletteOpen(false);
        }
      }
    }
  });

  const handleSubmit = async (text?: string) => {
    const raw = (text !== undefined ? text : inputValue).trim();
    if (!raw) return;

    if (paletteOpen) {
      const filtered = filterSlashCommands(client.commands, raw);
      const displayCount = Math.min(filtered.length, MAX_PALETTE_ITEMS);

      // If user submitted '/' or a partial command name without arguments
      if (raw === "/" || (!raw.includes(" ") && !client.commands.some((c) => c.command.toLowerCase() === raw.toLowerCase()))) {
        if (displayCount > 0) {
          const safeIdx = Math.min(paletteIndex, displayCount - 1);
          const selectedCmd = filtered[safeIdx];
          if (selectedCmd) {
            setPaletteOpen(false);
            if (isDirectExecutionCommand(selectedCmd)) {
              setInputValue("");
              await handleSlashCommand(selectedCmd.command);
              return;
            } else {
              setInputValue(`${selectedCmd.command} `);
              return;
            }
          }
        }
      }
    }

    setInputValue("");
    setPaletteOpen(false);

    if (raw.startsWith("/")) {
      await handleSlashCommand(raw);
      return;
    }

    if (isGenerating) {
      setMessages((prev) => [
        ...prev,
        {
          id: `warn_${Date.now()}`,
          role: "system",
          content: "Please wait for current AI response to complete or press Esc.",
        },
      ]);
      return;
    }

    const userMsgId = `user_${Date.now()}`;
    const aiMsgId = `ai_${Date.now()}`;

    setMessages((prev) => [
      ...prev,
      { id: userMsgId, role: "user", content: raw },
      { id: aiMsgId, role: "assistant", content: "", model: state.model },
    ]);

    setIsGenerating(true);

    client.streamChat(raw, {
      onToken: (chunk) => {
        setMessages((prev) =>
          prev.map((m) => (m.id === aiMsgId ? { ...m, content: m.content + chunk, ticker: undefined } : m))
        );
      },
      onTicker: (ticker) => {
        setMessages((prev) =>
          prev.map((m) => (m.id === aiMsgId ? { ...m, ticker } : m))
        );
      },
      onDone: (fullReply) => {
        setIsGenerating(false);
        setMessages((prev) =>
          prev.map((m) => (m.id === aiMsgId ? { ...m, content: fullReply, ticker: undefined } : m))
        );
      },
      onError: (error) => {
        setIsGenerating(false);
        setMessages((prev) =>
          prev.map((m) => (m.id === aiMsgId ? { ...m, content: `❌ Error: ${error}`, ticker: undefined } : m))
        );
      },
    });
  };

  const handleSlashCommand = async (cmdText: string) => {
    const parts = cmdText.split(" ");
    const cmd = parts[0].toLowerCase();

    if (cmd === "/exit" || cmd === "/quit" || cmd === "/q") {
      client.stop();
      if (onExit) {
        onExit();
      } else {
        process.exit(0);
      }
      return;
    }

    if (cmd === "/clear" || cmd === "/cls") {
      setMessages([]);
      return;
    }

    try {
      const res = await client.executeCommand(cmdText);
      if (res) {
        if (res.action === "exit") {
          client.stop();
          if (onExit) {
            onExit();
          } else {
            process.exit(0);
          }
          return;
        }
        if (res.action === "clear") {
          setMessages([]);
          return;
        }

        setMessages((prev) => [
          ...prev,
          {
            id: `sys_${Date.now()}`,
            role: "system",
            content: res.message || (res.success ? "Command executed successfully." : `❌ Error: ${res.error}`),
          },
        ]);
      }
    } catch (err: any) {
      setMessages((prev) => [
        ...prev,
        {
          id: `err_${Date.now()}`,
          role: "system",
          content: `❌ Command failed: ${err.message || err}`,
        },
      ]);
    }
  };

  return (
    <ChatView
      state={state}
      messages={messages}
      inputValue={inputValue}
      paletteOpen={paletteOpen}
      paletteIndex={paletteIndex}
      commands={client.commands}
      isGenerating={isGenerating}
      onInputChange={handleInputChange}
      onSubmit={(text?: string) => handleSubmit(text)}
    />
  );
};

