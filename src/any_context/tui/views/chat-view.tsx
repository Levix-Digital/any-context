import React from "react";
import { AnyContextState, SlashCommandMeta } from "../bridge-client";
import { ChatMessage, ChatMessageList } from "../components/chat-message-list";
import { AutocompleteDropdown } from "../components/autocomplete-dropdown";
import { InputBar } from "../components/input-bar";
import { StatusBar } from "../components/status-bar";
import { anyContextTheme } from "../themes";

interface ChatViewProps {
  state: AnyContextState;
  messages: ChatMessage[];
  inputValue: string;
  paletteOpen: boolean;
  paletteIndex: number;
  commands: SlashCommandMeta[];
  isGenerating: boolean;
  onInputChange: (val: string) => void;
  onSubmit: (text?: string) => void;
}

export const ChatView = ({
  state,
  messages,
  inputValue,
  paletteOpen,
  paletteIndex,
  commands,
  isGenerating,
  onInputChange,
  onSubmit,
}: ChatViewProps): any => {
  return (
    <box flexDirection="column" width="100%" height="100%">
      {/* Main Chat Message Scroll View with ASCII Banner and Natural Top-Down Flow */}
      <ChatMessageList messages={messages} state={state} />

      {/* Floating Autocomplete / Slash Command Dropdown */}
      <AutocompleteDropdown
        isOpen={paletteOpen}
        filterText={inputValue}
        commands={commands}
        selectedIndex={paletteIndex}
      />

      {/* Input Bar (👤 You: prompt) */}
      <InputBar
        value={inputValue}
        onChange={onInputChange}
        onSubmit={onSubmit}
        disabled={isGenerating}
      />

      {/* Bottom Status Bar (1-line unified dock) */}
      <StatusBar state={state} />
    </box>
  );
};
