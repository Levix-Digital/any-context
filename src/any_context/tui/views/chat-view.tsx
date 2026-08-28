import React from "react";
import type {
  AnyContextState,
  SlashCommandMeta,
  MenuTreeSchema,
  OptionsGroupSchema,
} from "../bridge-client";
import { HeaderBar } from "../components/header-bar";
import { ChatMessage, ChatMessageList } from "../components/chat-message-list";
import { AutocompleteDropdown } from "../components/autocomplete-dropdown";
import { InteractiveModal } from "../components/interactive-modal";
import { InputBar } from "../components/input-bar";
import { StatusBar } from "../components/status-bar";
import { anyContextTheme } from "../themes";

interface ChatViewProps {
  state: AnyContextState;
  messages: ChatMessage[];
  inputValue: string;
  paletteOpen: boolean;
  paletteIndex: number;
  modalOpen: boolean;
  modalMode: "options" | "menu";
  modalOptionsGroup: OptionsGroupSchema | null;
  modalMenuTree: MenuTreeSchema | null;
  modalIndex: number;
  commands: SlashCommandMeta[];
  isGenerating: boolean;
  onInputChange: (val: string) => void;
  onSubmit: (text?: string) => void;
  scrollBoxRef?: any;
}

export const ChatView = ({
  state,
  messages,
  inputValue,
  paletteOpen,
  paletteIndex,
  modalOpen,
  modalMode,
  modalOptionsGroup,
  modalMenuTree,
  modalIndex,
  commands,
  isGenerating,
  onInputChange,
  onSubmit,
  scrollBoxRef,
}: ChatViewProps): any => {
  return (
    <box flexDirection="column" width="100%" height="100%">
      {/* Dynamic Header: Full ASCII banner when 0 messages / post-clear, sleek 1-line top bar during chat */}
      <HeaderBar state={state} hasMessages={messages.length > 0} />

      {/* Main Chat Message View with 100% full-screen flex layout */}
      <ChatMessageList ref={scrollBoxRef} messages={messages} state={state} />

      {/* Unified Interactive Modal (Options list or Hierarchical Config Menu) */}
      <InteractiveModal
        isOpen={modalOpen}
        mode={modalMode}
        optionsGroup={modalOptionsGroup}
        menuTree={modalMenuTree}
        selectedIndex={modalIndex}
        state={state}
      />

      {/* Floating Autocomplete / Slash Command Dropdown */}
      <AutocompleteDropdown
        isOpen={paletteOpen && !modalOpen}
        filterText={inputValue}
        commands={commands}
        selectedIndex={paletteIndex}
      />

      {/* Input Bar (👤 You: prompt) */}
      <InputBar
        value={inputValue}
        onChange={onInputChange}
        onSubmit={onSubmit}
        disabled={isGenerating || modalOpen}
        placeholder={
          modalOpen
            ? "Modal active: Use [↑/↓] to navigate options, [Enter] to select, [Esc] to close"
            : undefined
        }
      />

      {/* Bottom Status Bar (1-line unified dock) */}
      <StatusBar state={state} />
    </box>
  );
};
