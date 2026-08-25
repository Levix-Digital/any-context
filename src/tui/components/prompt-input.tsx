import React from "react";

interface PromptInputProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
}

export const PromptInput = ({
  value,
  onChange,
  onSubmit,
  placeholder = "Ask a question or type / for commands...",
  disabled = false,
}: PromptInputProps): any => {
  return (
    <box
      flexDirection="row"
      backgroundColor="#16161e"
      borderStyle="rounded"
      borderColor="#3b4261"
      paddingLeft={1}
      paddingRight={1}
      height={3}
      alignItems="center"
      marginBottom={0}
    >
      <text fg="#7dcfff">
        <b>👤 You: </b>
      </text>
      <input
        flexGrow={1}
        value={value}
        onChange={(val: any) => onChange(typeof val === "string" ? val : value)}
        onSubmit={(val: any) => onSubmit(typeof val === "string" ? val : value)}
        placeholder={placeholder}
      />
    </box>
  );
};

