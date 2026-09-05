import React, { useState, useEffect, useRef } from "react";
import { useKeyboard } from "@opentui/react";
import { BridgeClient } from "./bridge-client";
import type {
  AnyContextState,
  BootTelemetryStep,
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
  SlashCommandMeta,
  DEFAULT_SLASH_COMMANDS,
} from "./commands";
import { tuiLog } from "./logger";

interface AppProps {
  initialWorkspace?: string;
  onExit?: () => void;
}

export const App = ({ initialWorkspace = "Default", onExit }: AppProps): any => {
  const [client] = useState(() => new BridgeClient(initialWorkspace));
  const [state, setState] = useState<AnyContextState>(client.state);
  const [bootTelemetrySteps, setBootTelemetrySteps] = useState<BootTelemetryStep[]>(client.bootTelemetrySteps);
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
  const [isBackendReady, setIsBackendReady] = useState(false);
  const [inputHistory, setInputHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState<number>(-1);
  const [draftInput, setDraftInput] = useState<string>("");
  const [commands, setCommands] = useState<SlashCommandMeta[]>(
    client.commands.length > 0 ? client.commands : DEFAULT_SLASH_COMMANDS
  );
  const scrollBoxRef = useRef<any>(null);

  useEffect(() => {
    tuiLog.info("APP:MOUNT", "App component mounted", { initialWorkspace });
    client.onBootTelemetry = (_step, allSteps) => {
      setBootTelemetrySteps([...allSteps]);
    };

    client.onCommandsLoaded = (loadedCommands) => {
      tuiLog.info("APP:COMMANDS_LOADED", "Slash commands dynamically loaded from Core", {
        count: loadedCommands.length,
      });
      setCommands(loadedCommands);
    };

    client.onStateChange = (newState) => {
      tuiLog.info("APP:STATE_CHANGE", "client.onStateChange triggered", {
        workspace: newState.workspace,
        model: newState.model,
        needs_onboarding: newState.needs_onboarding,
      });
      setIsBackendReady(true);
      setState(newState);
      if (newState.needs_onboarding && !modalOpen && !hasTriggeredOnboardingRef.current) {
        openOnboardingModal(newState.onboarding_state?.options_group);
      }
    };

    client.start().then(() => {
      tuiLog.info("APP:START_OK", "client.start() completed successfully", {
        needs_onboarding: client.state.needs_onboarding,
      });
      setIsBackendReady(true);
      if (client.state.needs_onboarding && !modalOpen && !hasTriggeredOnboardingRef.current) {
        openOnboardingModal(client.state.onboarding_state?.options_group);
      }
    }).catch((err) => {
      tuiLog.error("APP:START_FAIL", `Failed to start BridgeClient: ${err.message}`, { error: err.stack });
      setMessages((prev) => [
        ...prev,
        {
          id: `err_${Date.now()}`,
          role: "system",
          content: `Failed to connect to AnyContext Python backend: ${err.message}`,
        },
      ]);
    });

    client.onNotification = (message: string, level: string) => {
      setMessages((prev) => [
        ...prev,
        {
          id: `notif_${Date.now()}_${Math.random()}`,
          role: "system",
          content: message,
        },
      ]);
    };

    const interval = setInterval(() => {
      client.refreshState().then((newState) => {
        if (newState.needs_onboarding && !modalOpen && !hasTriggeredOnboardingRef.current) {
          openOnboardingModal(newState.onboarding_state?.options_group);
        }
      });
    }, 1000);


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
    tuiLog.info("APP:OPEN_ONBOARDING", "Invoking openOnboardingModal", { overrideOpts: Boolean(overrideOpts) });
    try {
      let opts = overrideOpts || (state.onboarding_state && state.onboarding_state.options_group);
      if (!opts || !opts.items || opts.items.length === 0) {
        opts = await client.getOptions("onboarding");
      }
      if (opts && opts.items && opts.items.length > 0) {
        tuiLog.info("APP:MODAL_ACTIVE", "Opening onboarding options modal", { count: opts.items.length, active_id: opts.active_id });
        setPaletteOpen(false);
        setModalMode("options");
        setModalOptionsGroup(opts);
        const activeIdx = opts.items.findIndex((item: any) => item.is_active);
        setModalIndex(activeIdx >= 0 ? activeIdx : 0);
        setModalOpen(true);
        hasTriggeredOnboardingRef.current = true;
      } else {
        tuiLog.warn("APP:MODAL_EMPTY", "No onboarding options received from backend");
      }
    } catch (err: any) {
      tuiLog.error("APP:MODAL_ERROR", `Failed to load onboarding setup: ${err.message}`, { error: err.stack });
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

  const openDeleteWorkspaceModal = async (fromMenu: boolean = false) => {
    try {
      const opts = await client.getOptions("delete_workspace");
      if (opts && opts.items && opts.items.length > 0) {
        setPaletteOpen(false);
        setModalMode("options");
        setModalOptionsGroup(opts);
        setModalIndex(0);
        if (fromMenu && modalMenuTree) {
          setMenuHistory((prev) => [...prev, modalMenuTree.menu_id]);
        }
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

  const openDeleteSourceModal = async (fromMenu: boolean = false) => {
    try {
      const opts = await client.getOptions("delete_source");
      if (opts && opts.items && opts.items.length > 0) {
        setPaletteOpen(false);
        setModalMode("options");
        setModalOptionsGroup(opts);
        setModalIndex(0);
        if (fromMenu && modalMenuTree) {
          setMenuHistory((prev) => [...prev, modalMenuTree.menu_id]);
        }
        setModalOpen(true);
      }
    } catch (err: any) {
      setMessages((prev) => [
        ...prev,
        { id: `err_${Date.now()}`, role: "system", content: `❌ Could not load workspace sources: ${err.message}` },
      ]);
    }
  };

  const openConfirmDeleteSourceModal = async (sourceInfo: any) => {
    try {
      const opts = await client.getOptions("confirm_delete_source", { source_info: sourceInfo });
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
        if (menuHistory.length > 0) {
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
            setModalOpen(false);

            // Declarative client-side action from Option metadata (Single Source of Truth)
            const optMeta = selectedOption.metadata || {};
            if (optMeta.action === "prefill_input" && optMeta.prefill) {
              setInputValue(optMeta.prefill);
              if (optMeta.message || optMeta.instruction) {
                setMessages((prev) => [
                  ...prev,
                  {
                    id: `sys_${Date.now()}`,
                    role: "system",
                    content: optMeta.message || optMeta.instruction,
                  },
                ]);
              }
              return;
            }

            // Generic backend option dispatch
            client
              .setOption(modalOptionsGroup.type, selectedOption.id, undefined, false, selectedOption.metadata)
              .then((res) => {
                if (res.action === "open_config_modal") {
                  const targetMenu = (res.state_updates as any)?.target_menu || "main";
                  openConfigModal(targetMenu, false);
                  client.refreshState();
                  return;
                }
                if (res.action === "prefill_input" && res.state_updates && (res.state_updates as any).prefill) {
                  setInputValue((res.state_updates as any).prefill);
                  if (res.message) {
                    setMessages((prev) => [
                      ...prev,
                      { id: `sys_${Date.now()}`, role: "system", content: res.message },
                    ]);
                  }
                  return;
                }
                if (res.action === "open_confirm_delete_workspace_modal" && res.state_updates && (res.state_updates as any).target_workspace) {
                  openConfirmDeleteWorkspaceModal((res.state_updates as any).target_workspace);
                  return;
                }
                if (res.action === "open_confirm_delete_source_modal" && res.state_updates && (res.state_updates as any).source_info) {
                  openConfirmDeleteSourceModal((res.state_updates as any).source_info);
                  return;
                }
                if (res.action === "open_delete_source_modal") {
                  openDeleteSourceModal(false);
                  return;
                }
                if (res.action === "open_delete_workspace_modal") {
                  openDeleteWorkspaceModal(false);
                  return;
                }
                const isExitUpdate =
                  res.action === "exit_update" ||
                  (res.state_updates as any)?.action === "exit_update";
                if (isExitUpdate || res.action === "exit") {
                  setMessages((prev) => [
                    ...prev,
                    {
                      id: `sys_${Date.now()}`,
                      role: "system",
                      content:
                        res.message ||
                        "🎉 AnyContext updated! Closing session and returning to terminal...",
                    },
                  ]);
                  setTimeout(() => {
                    client.stop();
                    if (onExit) {
                      onExit();
                    } else {
                      process.exit(0);
                    }
                  }, 800);
                  return;
                }

                if (modalOptionsGroup.type === "workspace") {
                  if (res.chat_history) {
                    setMessages(res.chat_history);
                  } else {
                    client.getChatHistory().then((hist) => setMessages(hist));
                  }
                } else {
                  setMessages((prev) => [
                    ...prev,
                    {
                      id: `sys_${Date.now()}`,
                      role: "system",
                      content: res.message || `Set ${modalOptionsGroup.type} to ${selectedOption.title}`,
                    },
                  ]);
                }
                client.refreshState();
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
            } else if (selectedItem.id === "ws_sources_delete" || selectedItem.id === "ws_delete_source") {
              openDeleteSourceModal(true);
            } else if (selectedItem.id === "ws_delete") {
              openDeleteWorkspaceModal(true);
            } else if (selectedItem.id === "ws_switch") {
              openSwitchModal();
            } else {
              setModalOpen(false);
              client
                .executeMenuAction(selectedItem.id, {})
                .then((res) => {
                  if (res.action === "open_delete_source_modal") {
                    openDeleteSourceModal(true);
                    return;
                  }
                  if (res.action === "open_delete_workspace_modal" || res.action === "delete_workspace") {
                    openDeleteWorkspaceModal(true);
                    return;
                  }
                  if (res.action === "open_switch_modal") {
                    openSwitchModal();
                    return;
                  }
                  if (res.action === "prefill_input" && res.state_updates && (res.state_updates as any).prefill) {
                    setInputValue((res.state_updates as any).prefill);
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
      const filtered = filterSlashCommands(commands, inputValue);
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
      const filtered = filterSlashCommands(commands, raw);
      const displayCount = Math.min(filtered.length, MAX_PALETTE_ITEMS);

      if (
        raw === "/" ||
        (!raw.includes(" ") &&
          !commands.some(
            (c) =>
              c.command.toLowerCase() === raw.toLowerCase() ||
              (c.aliases && c.aliases.some((a: string) => a.toLowerCase() === raw.toLowerCase()))
          ))
      ) {
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
    tuiLog.info("APP:SLASH_COMMAND", `Executing slash command '${cmdText}'`);
    const parts = cmdText.trim().split(" ");
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
      try {
        await client.executeCommand(cmdText);
      } catch (_) {}
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
        if (res.state_updates && (res.state_updates as any).workspace) {
          if (res.chat_history) {
            setMessages(res.chat_history);
          } else {
            const hist = await client.getChatHistory((res.state_updates as any).workspace);
            setMessages(hist);
          }
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
        if (res.action === "open_delete_source_modal" || res.action === "delete_source") {
          await openDeleteSourceModal();
          return;
        }
        if (res.action === "open_model_modal") {
          await openModelModal();
          return;
        }
        if (res.action === "open_update_modal") {
          const targetVer = (res.state_updates as any)?.target_version;
          await openUpdateModal(targetVer);
          return;
        }
        if (res.action === "open_config_modal" || res.action === "menu") {
          await openConfigModal("main");
          return;
        }
        if (res.action === "open_onboarding_modal") {
          hasTriggeredOnboardingRef.current = false;
          await openOnboardingModal();
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
      commands={commands}
      isGenerating={isGenerating}
      isBackendReady={isBackendReady}
      bootTelemetrySteps={bootTelemetrySteps}
      onInputChange={handleInputChange}
      onSubmit={(text?: string) => handleSubmit(text)}
      scrollBoxRef={scrollBoxRef}
    />
  );
};
