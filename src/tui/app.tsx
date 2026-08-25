import React, { useState, useEffect } from "react";
import { useKeyboard } from "@opentui/react";
import { BridgeClient, AnyContextState } from "./bridge-client";
import { ChatView, MessageItem } from "./components/chat-view";
import { SlashCommandPalette } from "./components/palette";
import { PromptInput } from "./components/prompt-input";
import { StatusBar } from "./components/status-bar";

interface AppProps {
  initialWorkspace?: string;
}

export const App = ({ initialWorkspace = "Default" }: AppProps): any => {
  const [client] = useState(() => new BridgeClient(initialWorkspace));
  const [state, setState] = useState<AnyContextState>(client.state);
  const [messages, setMessages] = useState<MessageItem[]>([]);
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
      const query = inputValue.startsWith("/") ? inputValue.slice(1).toLowerCase().trim() : "";
      const filtered = client.commands.filter((c) =>
        c.command.toLowerCase().includes(query) || c.description.toLowerCase().includes(query)
      );
      const count = Math.min(filtered.length, 7);

      if (event.name === "up" && count > 0) {
        setPaletteIndex((prev) => (prev > 0 ? prev - 1 : count - 1));
      } else if (event.name === "down" && count > 0) {
        setPaletteIndex((prev) => (prev < count - 1 ? prev + 1 : 0));
      } else if (event.name === "tab" && count > 0) {
        const selectedCmd = filtered[paletteIndex];
        if (selectedCmd) {
          setInputValue(`${selectedCmd.command} `);
          setPaletteOpen(false);
        }
      }
    }
  });

  const handleSubmit = async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) return;

    setInputValue("");
    setPaletteOpen(false);

    if (trimmed.startsWith("/")) {
      await handleSlashCommand(trimmed);
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
      { id: userMsgId, role: "user", content: trimmed },
      { id: aiMsgId, role: "assistant", content: "", model: state.model },
    ]);

    setIsGenerating(true);

    client.streamChat(trimmed, {
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

    if (cmd === "/exit" || cmd === "/quit") {
      client.stop();
      process.exit(0);
    }

    if (cmd === "/clear" || cmd === "/cls") {
      setMessages([]);
      return;
    }

    if (cmd === "/version" || cmd === "/v") {
      setMessages((prev) => [
        ...prev,
        { id: `sys_${Date.now()}`, role: "system", content: `🤖 AnyContext (actx) v${state.version} - Levix Digital` },
      ]);
      return;
    }

    if (cmd === "/help" || cmd === "/menu") {
      const helpLines = client.commands.map((c) => `• \`${c.command} ${c.args}\` : ${c.description}`);
      setMessages((prev) => [
        ...prev,
        {
          id: `sys_${Date.now()}`,
          role: "system",
          content: `**Available Slash Commands:**\n\n${helpLines.join("\n")}`,
        },
      ]);
      return;
    }

    if (cmd === "/switch") {
      const target = parts[1]?.trim();
      if (target) {
        const newState = await client.switchWorkspace(target);
        setMessages((prev) => [
          ...prev,
          { id: `sys_${Date.now()}`, role: "system", content: `Switched active workspace to '${newState.workspace}'.` },
        ]);
      } else {
        setMessages((prev) => [
          ...prev,
          { id: `sys_${Date.now()}`, role: "system", content: "Usage: `/switch <workspace_name>`" },
        ]);
      }
      return;
    }

    if (cmd === "/model") {
      const targetModel = parts[1]?.trim();
      if (targetModel) {
        const newState = await client.setModel(targetModel);
        setMessages((prev) => [
          ...prev,
          { id: `sys_${Date.now()}`, role: "system", content: `Inference model switched to '${newState.model}'.` },
        ]);
      } else {
        setMessages((prev) => [
          ...prev,
          { id: `sys_${Date.now()}`, role: "system", content: "Usage: `/model <model_name>` (e.g. `gpt-4o-mini`, `claude-3-5-sonnet`)" },
        ]);
      }
      return;
    }

    if (cmd === "/mode") {
      const targetMode = parts[1]?.trim()?.toLowerCase();
      if (targetMode && ["strict", "hybrid", "proactive"].includes(targetMode)) {
        const newState = await client.setMode(targetMode);
        setMessages((prev) => [
          ...prev,
          { id: `sys_${Date.now()}`, role: "system", content: `Grounding mode updated to '${newState.grounding_mode.toUpperCase()}'.` },
        ]);
      } else {
        setMessages((prev) => [
          ...prev,
          { id: `sys_${Date.now()}`, role: "system", content: "Usage: `/mode <strict|hybrid|proactive>`" },
        ]);
      }
      return;
    }

    if (cmd === "/web-search" || cmd === "/search") {
      const enabled = parts[1] ? ["on", "true", "1"].includes(parts[1].toLowerCase()) : !state.web_search_enabled;
      const newState = await client.setWebSearch(enabled);
      setMessages((prev) => [
        ...prev,
        { id: `sys_${Date.now()}`, role: "system", content: `Web Search is now ${newState.web_search_enabled ? "ON" : "OFF"}.` },
      ]);
      return;
    }

    if (cmd === "/sync") {
      const force = parts.includes("--force") || parts.includes("-f");
      await client.startSync(force);
      setMessages((prev) => [
        ...prev,
        { id: `sys_${Date.now()}`, role: "system", content: `⚡ Background synchronization started for workspace '${state.workspace}'.` },
      ]);
      return;
    }

    if (cmd === "/sources") {
      const sources = await client.listSources();
      const count = (sources?.folders?.length || 0) + (sources?.web_urls?.length || 0);
      setMessages((prev) => [
        ...prev,
        {
          id: `sys_${Date.now()}`,
          role: "system",
          content: `📂 Indexed Sources in '${state.workspace}': ${count} source(s) configured.`,
        },
      ]);
      return;
    }

    setMessages((prev) => [
      ...prev,
      { id: `sys_${Date.now()}`, role: "system", content: `Unknown command '${cmd}'. Type \`/help\` to see available commands.` },
    ]);
  };

  return (
    <box flexDirection="column" width="100%" height="100%" backgroundColor="#1a1b26">
      <box
        flexDirection="row"
        backgroundColor="#16161e"
        borderStyle="single"
        borderColor="#3b4261"
        paddingLeft={1}
        paddingRight={1}
        height={3}
        alignItems="center"
      >
        <text fg="#7aa2f7">
          <b>🤖 AnyContext OpenTUI v{state.version}</b>
        </text>
        <text fg="#565f89"> - Universal Multi-Context RAG Assistant & Engine</text>
      </box>

      <ChatView messages={messages} />

      <SlashCommandPalette
        isOpen={paletteOpen}
        filterText={inputValue}
        commands={client.commands}
        selectedIndex={paletteIndex}
      />

      <PromptInput
        value={inputValue}
        onChange={handleInputChange}
        onSubmit={handleSubmit}
        disabled={isGenerating}
      />

      <StatusBar state={state} />
    </box>
  );
};

