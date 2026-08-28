import React, { useState, useEffect, useRef } from "react";
import { useKeyboard } from "@opentui/react";
import { BridgeClient } from "./bridge-client";
import type {
  AnyContextState,
  MenuTreeSchema,
  MenuItemSchema,
  OptionsGroupSchema,
  OptionItemSchema,
} from "./bridge-client";
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

  // Unified Interactive Modal state
  const [modalOpen, setModalOpen] = useState(false);
  const [modalMode, setModalMode] = useState<"options" | "menu">("menu");
  const [modalOptionsGroup, setModalOptionsGroup] = useState<OptionsGroupSchema | null>(null);
  const [modalMenuTree, setModalMenuTree] = useState<MenuTreeSchema | null>(null);
  const [modalIndex, setModalIndex] = useState(0);
  const [menuHistory, setMenuHistory] = useState<string[]>([]);

  const [isGenerating, setIsGenerating] = useState(false);
  const [inputHistory, setInputHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState<number>(-1);
  const [draftInput, setDraftInput] = useState<string>("");
  const scrollBoxRef = useRef<any>(null);

  useEffect(() => {
    client.onStateChange = (newState) => {
      setState(newState);
      if (newState.needs_onboarding && !modalOpen && !hasTriggeredOnboardingRef.current) {
        openOnboardingModal(newState.onboarding_state?.options_group);
      }
    };

    client.start().then(() => {
      if (client.state.needs_onboarding && !modalOpen && !hasTriggeredOnboardingRef.current) {
        openOnboardingModal(client.state.onboarding_state?.options_group);
      }
    }).catch((err) => {
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
      client.refreshState().then((newState) => {
        if (newState.needs_onboarding && !modalOpen && !hasTriggeredOnboardingRef.current) {
          openOnboardingModal(newState.onboarding_state?.options_group);
        }
      });
    }, 2000);

    return () => {
      clearInterval(interval);
      client.stop();
    };
  }, [client]);

  const hasTriggeredOnboardingRef = useRef(false);
  useEffect(() => {
    if (state.needs_onboarding && !hasTriggeredOnboardingRef.current && !modalOpen) {
      openOnboardingModal(state.onboarding_state?.options_group);
    }
  }, [state.needs_onboarding, modalOpen, state.onboarding_state]);

  const openOnboardingModal = async (overrideOpts?: OptionsGroupSchema) => {
    try {
      let opts = overrideOpts || (state.onboarding_state && state.onboarding_state.options_group);
      if (!opts || !opts.items || opts.items.length === 0) {
        opts = await client.getOptions("onboarding");
      }
      if (opts && opts.items && opts.items.length > 0) {
        setPaletteOpen(false);
        setModalMode("options");
        setModalOptionsGroup(opts);
        const activeIdx = opts.items.findIndex((item: any) => item.is_active);
        setModalIndex(activeIdx >= 0 ? activeIdx : 0);
        setModalOpen(true);
        hasTriggeredOnboardingRef.current = true;
      }
    } catch (err: any) {
      hasTriggeredOnboardingRef.current = false;
      setMessages((prev) => [
        ...prev,
        { id: `err_${Date.now()}`, role: "system", content: `❌ Could not load onboarding setup: ${err.message}` },
      ]);
    }
  };

  const openModeModal = async () => {
    try {
      const opts = await client.getOptions("grounding_mode");
      if (opts && opts.items) {
        setPaletteOpen(false);
        setModalMode("options");
        setModalOptionsGroup(opts);
        const activeIdx = opts.items.findIndex((item) => item.is_active);
        setModalIndex(activeIdx >= 0 ? activeIdx : 0);
        setModalOpen(true);
      }
    } catch (err: any) {
      setMessages((prev) => [
        ...prev,
        { id: `err_${Date.now()}`, role: "system", content: `❌ Could not load grounding modes: ${err.message}` },
      ]);
    }
  };

  const openSwitchModal = async () => {
    try {
      const opts = await client.getOptions("workspace");
      if (opts && opts.items) {
        setPaletteOpen(false);
        setModalMode("options");
        setModalOptionsGroup(opts);
        const activeIdx = opts.items.findIndex((item) => item.is_active);
        setModalIndex(activeIdx >= 0 ? activeIdx : 0);
        setModalOpen(true);
      }
    } catch (err: any) {
      setMessages((prev) => [
        ...prev,
        { id: `err_${Date.now()}`, role: "system", content: `❌ Could not load workspaces: ${err.message}` },
      ]);
    }
  };

  const openModelModal = async () => {
    try {
      const opts = await client.getOptions("inference_model");
      if (opts && opts.items) {
        setPaletteOpen(false);
        setModalMode("options");
        setModalOptionsGroup(opts);
        const activeIdx = opts.items.findIndex((item) => item.is_active);
        setModalIndex(activeIdx >= 0 ? activeIdx : 0);
        setModalOpen(true);
      }
    } catch (err: any) {
      setMessages((prev) => [
        ...prev,
        { id: `err_${Date.now()}`, role: "system", content: `❌ Could not load AI models: ${err.message}` },
      ]);
    }
  };

  const openConfigModal = async (menuId: string = "main", pushHistory: boolean = false) => {
    try {
      const tree = await client.getMenuTree(menuId);
      if (tree && tree.items) {
        setPaletteOpen(false);
        setModalMode("menu");
        setModalMenuTree(tree);
        setModalIndex(0);
        if (pushHistory && modalMenuTree) {
          setMenuHistory((prev) => [...prev, modalMenuTree.menu_id]);
        } else if (!pushHistory) {
          setMenuHistory([]);
        }
        setModalOpen(true);
      }
    } catch (err: any) {
      setMessages((prev) => [
        ...prev,
        { id: `err_${Date.now()}`, role: "system", content: `❌ Could not load configuration menu: ${err.message}` },
      ]);
    }
  };

  const openUpdateModal = async (targetVersion?: string) => {
    try {
      const opts = await client.getOptions("update", targetVersion ? { target_version: targetVersion } : undefined);
      if (opts && opts.items && opts.items.length > 0) {
        setPaletteOpen(false);
        setModalMode("options");
        setModalOptionsGroup(opts);
        const activeIdx = opts.items.findIndex((item) => item.is_active);
        setModalIndex(activeIdx >= 0 ? activeIdx : 0);
        setModalOpen(true);
      }
    } catch (err: any) {
      setMessages((prev) => [
        ...prev,
        { id: `err_${Date.now()}`, role: "system", content: `❌ Could not load update options: ${err.message}` },
      ]);
    }
  };

  const openDeleteWorkspaceModal = async () => {
    try {
      const opts = await client.getOptions("delete_workspace");
      if (opts && opts.items && opts.items.length > 0) {
        setPaletteOpen(false);
        setModalMode("options");
        setModalOptionsGroup(opts);
        setModalIndex(0);
        setModalOpen(true);
      } else {
        setMessages((prev) => [
          ...prev,
          { id: `sys_${Date.now()}`, role: "system", content: "ℹ️ No custom workspaces available to delete ('Default', 'Global', and 'Shared Sources' are protected)." },
        ]);
      }
    } catch (err: any) {
      setMessages((prev) => [
        ...prev,
        { id: `err_${Date.now()}`, role: "system", content: `❌ Could not load workspaces to delete: ${err.message}` },
      ]);
    }
  };

  const openConfirmDeleteWorkspaceModal = async (targetWorkspace: string) => {
    try {
      const opts = await client.getOptions("confirm_delete_workspace", { target_workspace: targetWorkspace });
      if (opts && opts.items && opts.items.length > 0) {
        setPaletteOpen(false);
        setModalMode("options");
        setModalOptionsGroup(opts);
        setModalIndex(1); // Default highlight on Cancel for safety
        setModalOpen(true);
      }
    } catch (err: any) {
      setMessages((prev) => [
        ...prev,
        { id: `err_${Date.now()}`, role: "system", content: `❌ Could not load confirmation prompt: ${err.message}` },
      ]);
    }
  };

  const handleInputChange = (val: string) => {
    setInputValue(val);
    if (val.startsWith("/")) {
      if (modalOpen) {
        setModalOpen(false);
      }
      setPaletteOpen(true);
      setPaletteIndex(0);
    } else {
      setPaletteOpen(false);
    }
  };

  useKeyboard((event) => {
    // 1. ESCAPE key handling
    if (event.name === "escape") {
      if (modalOpen) {
        if (modalMode === "menu" && menuHistory.length > 0) {
          const prevMenu = menuHistory[menuHistory.length - 1];
          setMenuHistory((prev) => prev.slice(0, -1));
          openConfigModal(prevMenu, false);
          return;
        }
        setModalOpen(false);
        return;
      }
      if (paletteOpen) {
        setPaletteOpen(false);
        return;
      }
      return;
    }

    // 2. MODAL navigation and action execution
    if (modalOpen) {
      const itemCount =
        modalMode === "options"
          ? modalOptionsGroup?.items.length || 0
          : modalMenuTree?.items.length || 0;

      if (event.name === "up" && itemCount > 0) {
        setModalIndex((prev) => (prev > 0 ? prev - 1 : itemCount - 1));
      } else if (event.name === "down" && itemCount > 0) {
        setModalIndex((prev) => (prev < itemCount - 1 ? prev + 1 : 0));
      } else if (
        (event.name === "return" || event.name === "enter" || event.name === "tab") &&
        itemCount > 0
      ) {
        if (modalMode === "options" && modalOptionsGroup) {
          const selectedOption: OptionItemSchema = modalOptionsGroup.items[modalIndex];
          if (selectedOption) {
            if (modalOptionsGroup.type === "onboarding") {
              setModalOpen(false);
              if (selectedOption.id === "openai") {
                setInputValue("/key openai ");
                setMessages((prev) => [
                  ...prev,
                  {
                    id: `sys_${Date.now()}`,
                    role: "system",
                    content: "🔑 **OpenAI Setup**: Please paste your OpenAI API Key after `/key openai ` below and press [Enter] to complete setup.",
                  },
                ]);
                return;
              } else if (selectedOption.id === "custom") {
                openConfigModal("models", false);
                return;
              } else if (selectedOption.id === "local_offline") {
                client
                  .completeOnboarding("local_offline")
                  .then((res) => {
                    setMessages((prev) => [
                      ...prev,
                      {
                        id: `sys_${Date.now()}`,
                        role: "system",
                        content: res.message || "✅ Local Offline Server (LM Studio / Ollama) configured successfully!",
                      },
                    ]);
                    client.refreshState();
                  })
                  .catch((err) => {
                    setMessages((prev) => [
                      ...prev,
                      { id: `err_${Date.now()}`, role: "system", content: `❌ Error setting local offline mode: ${err.message}` },
                    ]);
                  });
                return;
              }
            }

            setModalOpen(false);
            client
              .setOption(modalOptionsGroup.type, selectedOption.id)
              .then((res) => {
                if (res.action === "open_confirm_delete_workspace_modal" && res.state_updates && (res.state_updates as any).target_workspace) {
                  openConfirmDeleteWorkspaceModal((res.state_updates as any).target_workspace);
                  return;
                }
                setMessages((prev) => [
                  ...prev,
                  {
                    id: `sys_${Date.now()}`,
                    role: "system",
                    content: res.message || `Set ${modalOptionsGroup.type} to ${selectedOption.title}`,
                  },
                ]);
                client.refreshState();
                const isRestart = Boolean((res.state_updates && (res.state_updates as any).action === "restart") || (res as any).action === "restart");
                if (isRestart) {
                  setTimeout(() => {
                    client.stop();
                    if (onExit) onExit();
                    else process.exit(0);
                  }, 1200);
                }
              })
              .catch((err) => {
                setMessages((prev) => [
                  ...prev,
                  { id: `err_${Date.now()}`, role: "system", content: `❌ Error setting option: ${err.message}` },
                ]);
              });
          }
        } else if (modalMode === "menu" && modalMenuTree) {
          const selectedItem: MenuItemSchema = modalMenuTree.items[modalIndex];
          if (selectedItem) {
            if (selectedItem.type === "submenu") {
              openConfigModal(selectedItem.id, true);
            } else if (selectedItem.command_shortcut) {
              setModalOpen(false);
              handleSlashCommand(selectedItem.command_shortcut);
            } else {
              setModalOpen(false);
              client
                .executeMenuAction(selectedItem.id, {})
                .then((res) => {
                  if (res.action === "open_delete_workspace_modal" || res.action === "delete_workspace") {
                    openDeleteWorkspaceModal();
                    return;
                  }
                  setMessages((prev) => [
                    ...prev,
                    {
                      id: `sys_${Date.now()}`,
                      role: "system",
                      content: res.message || `Executed action: ${selectedItem.title}`,
                    },
                  ]);
                  client.refreshState();
                  const isRestart = Boolean((res.state_updates && (res.state_updates as any).action === "restart") || (res as any).action === "restart");
                  if (isRestart) {
                    setTimeout(() => {
                      client.stop();
                      if (onExit) onExit();
                      else process.exit(0);
                    }, 1200);
                  }
                })
                .catch((err) => {
                  setMessages((prev) => [
                    ...prev,
                    { id: `err_${Date.now()}`, role: "system", content: `❌ Error executing action: ${err.message}` },
                  ]);
                });
            }
          }
        }
      }
      return;
    }

    // 3. PALETTE navigation
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
      return;
    }

    // 4. USER INPUT PROMPT HISTORY NAVIGATION (Up / Down)
    if (!modalOpen && !paletteOpen) {
      if (event.name === "up" && !event.ctrl && !event.shift) {
        if (inputHistory.length > 0) {
          if (historyIndex === -1) {
            setDraftInput(inputValue);
            const nextIdx = inputHistory.length - 1;
            setHistoryIndex(nextIdx);
            setInputValue(inputHistory[nextIdx]);
          } else if (historyIndex > 0) {
            const nextIdx = historyIndex - 1;
            setHistoryIndex(nextIdx);
            setInputValue(inputHistory[nextIdx]);
          }
          return;
        }
      } else if (event.name === "down" && !event.ctrl && !event.shift) {
        if (historyIndex !== -1) {
          if (historyIndex < inputHistory.length - 1) {
            const nextIdx = historyIndex + 1;
            setHistoryIndex(nextIdx);
            setInputValue(inputHistory[nextIdx]);
          } else {
            setHistoryIndex(-1);
            setInputValue(draftInput);
          }
          return;
        }
      }

      // 5. TERMINAL CHAT SCROLLING (PageUp, PageDown, Shift+Up/Down, Ctrl+Up/Down, Home, End)
      if (event.name === "pageup") {
        if (scrollBoxRef.current) {
          if (typeof scrollBoxRef.current.scrollBy === "function") {
            scrollBoxRef.current.scrollBy({ y: -8 });
          } else if (scrollBoxRef.current.scrollTop !== undefined) {
            scrollBoxRef.current.scrollTop = Math.max(0, scrollBoxRef.current.scrollTop - 8);
          }
        }
        return;
      }
      if (event.name === "pagedown") {
        if (scrollBoxRef.current) {
          if (typeof scrollBoxRef.current.scrollBy === "function") {
            scrollBoxRef.current.scrollBy({ y: 8 });
          } else if (scrollBoxRef.current.scrollTop !== undefined) {
            scrollBoxRef.current.scrollTop = scrollBoxRef.current.scrollTop + 8;
          }
        }
        return;
      }
      if (event.name === "up" && (event.shift || event.ctrl)) {
        if (scrollBoxRef.current) {
          if (typeof scrollBoxRef.current.scrollBy === "function") {
            scrollBoxRef.current.scrollBy({ y: -3 });
          } else if (scrollBoxRef.current.scrollTop !== undefined) {
            scrollBoxRef.current.scrollTop = Math.max(0, scrollBoxRef.current.scrollTop - 3);
          }
        }
        return;
      }
      if (event.name === "down" && (event.shift || event.ctrl)) {
        if (scrollBoxRef.current) {
          if (typeof scrollBoxRef.current.scrollBy === "function") {
            scrollBoxRef.current.scrollBy({ y: 3 });
          } else if (scrollBoxRef.current.scrollTop !== undefined) {
            scrollBoxRef.current.scrollTop = scrollBoxRef.current.scrollTop + 3;
          }
        }
        return;
      }
      if (event.name === "home" && (event.shift || event.ctrl || !inputValue)) {
        if (scrollBoxRef.current) {
          if (typeof scrollBoxRef.current.scrollTo === "function") {
            scrollBoxRef.current.scrollTo(0);
          } else if (scrollBoxRef.current.scrollTop !== undefined) {
            scrollBoxRef.current.scrollTop = 0;
          }
        }
        return;
      }
      if (event.name === "end" && (event.shift || event.ctrl || !inputValue)) {
        if (scrollBoxRef.current) {
          const maxH = scrollBoxRef.current.scrollHeight || 999999;
          if (typeof scrollBoxRef.current.scrollTo === "function") {
            scrollBoxRef.current.scrollTo(maxH);
          } else if (scrollBoxRef.current.scrollTop !== undefined) {
            scrollBoxRef.current.scrollTop = maxH;
          }
        }
        return;
      }
    }
  });

  const handleSubmit = async (text?: string) => {
    const raw = (text !== undefined ? text : inputValue).trim();
    if (!raw) return;

    // Record into input history (avoiding immediate duplicates)
    setInputHistory((prev) => (prev.length === 0 || prev[prev.length - 1] !== raw ? [...prev, raw] : prev));
    setHistoryIndex(-1);
    setDraftInput("");

    if (paletteOpen) {
      const filtered = filterSlashCommands(client.commands, raw);
      const displayCount = Math.min(filtered.length, MAX_PALETTE_ITEMS);

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

    await client.streamChat(raw, {
      onToken: (token) => {
        setMessages((prev) =>
          prev.map((m) => (m.id === aiMsgId ? { ...m, content: m.content + token } : m))
        );
      },
      onTicker: (ticker) => {
        setMessages((prev) =>
          prev.map((m) => (m.id === aiMsgId ? { ...m, ticker } : m))
        );
      },
      onDone: () => {
        setIsGenerating(false);
        setMessages((prev) =>
          prev.map((m) => (m.id === aiMsgId ? { ...m, ticker: undefined } : m))
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

    if (cmd === "/mode" && parts.length === 1) {
      await openModeModal();
      return;
    }

    if (cmd === "/switch" && parts.length === 1) {
      await openSwitchModal();
      return;
    }

    if ((cmd === "/model" || cmd === "/models" || cmd === "/m") && parts.length === 1) {
      await openModelModal();
      return;
    }

    if ((cmd === "/update" || cmd.startsWith("/update@")) && parts.length === 1) {
      await openUpdateModal(cmd.includes("@") ? cmd.split("@")[1] : undefined);
      return;
    }

    if (cmd === "/menu" || cmd === "/config" || cmd === "/settings") {
      await openConfigModal("main");
      return;
    }

    if (cmd === "/onboarding" || cmd === "/setup") {
      await openOnboardingModal();
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
        if (res.action === "open_mode_modal") {
          await openModeModal();
          return;
        }
        if (res.action === "open_switch_modal") {
          await openSwitchModal();
          return;
        }
        if (res.action === "open_delete_workspace_modal" || res.action === "delete_workspace") {
          await openDeleteWorkspaceModal();
          return;
        }
        if (res.action === "open_model_modal") {
          await openModelModal();
          return;
        }
        if (res.action === "open_update_modal") {
          await openUpdateModal();
          return;
        }
        if (res.action === "open_config_modal" || res.action === "menu") {
          await openConfigModal("main");
          return;
        }
        const isRestart = res.action === "restart" || Boolean(res.state_updates && (res.state_updates as any).action === "restart");
        if (isRestart) {
          setMessages((prev) => [
            ...prev,
            {
              id: `sys_${Date.now()}`,
              role: "system",
              content: res.message || "🚀 Restarting AnyContext...",
            },
          ]);
          setTimeout(() => {
            client.stop();
            if (onExit) onExit();
            else process.exit(0);
          }, 1200);
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
      modalOpen={modalOpen}
      modalMode={modalMode}
      modalOptionsGroup={modalOptionsGroup}
      modalMenuTree={modalMenuTree}
      modalIndex={modalIndex}
      commands={client.commands}
      isGenerating={isGenerating}
      onInputChange={handleInputChange}
      onSubmit={(text?: string) => handleSubmit(text)}
      scrollBoxRef={scrollBoxRef}
    />
  );
};
