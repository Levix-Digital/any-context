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
  onSubmit: () => void;
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
    <box flexDirection="column" width="100%" height="100%" backgroundColor={anyContextTheme.background}>
      {/* Top Header Bar */}
      <box
        flexDirection="row"
        backgroundColor={anyContextTheme.inputBackground}
        border={["bottom"]}
        borderStyle="single"
        borderColor={anyContextTheme.ruleColor}
        paddingLeft={1}
        paddingRight={1}
        height={3}
        alignItems="center"
      >
        <text fg={anyContextTheme.accent}>
          <b>🤖 AnyContext OpenTUI v{state.version}</b>
        </text>
        <text fg={anyContextTheme.foregroundMuted}> - Universal Multi-Context RAG Assistant & Engine</text>
      </box>

      {/* Main Chat Message Scroll View */}
      <ChatMessageList messages={messages} />

      {/* Floating Autocomplete / Slash Command Dropdown */}
      <AutocompleteDropdown
        isOpen={paletteOpen}
        filterText={inputValue}
        commands={commands}
        selectedIndex={paletteIndex}
      />

      {/* Input Bar (❯ textarea) */}
      <InputBar
        value={inputValue}
        onChange={onInputChange}
        onSubmit={onSubmit}
        disabled={isGenerating}
      />

      {/* Bottom Status Bar */}
      <StatusBar state={state} />
    </box>
  );
};
