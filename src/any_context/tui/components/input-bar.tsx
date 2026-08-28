import React, { useRef, useEffect } from "react";
import { anyContextTheme } from "../themes";

interface InputBarProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: (text?: string) => void;
  placeholder?: string;
  disabled?: boolean;
}

export const InputBar = ({
  value,
  onChange,
  onSubmit,
  placeholder = "Ask a question or type / for commands...",
  disabled = false,
}: InputBarProps): any => {
  const textareaRef = useRef<any>(null);
  const lastEmittedValueRef = useRef<string>(value);
  const isSyncingRef = useRef<boolean>(false);

  // Sync external value updates (e.g. from slash command autocomplete or clear)
  useEffect(() => {
    if (value !== lastEmittedValueRef.current) {
      lastEmittedValueRef.current = value;
      if (textareaRef.current) {
        isSyncingRef.current = true;
        textareaRef.current.setText(value || "");
        if (value) {
          try {
            if (typeof textareaRef.current.setCursorByOffset === "function") {
              textareaRef.current.setCursorByOffset(value.length);
            } else if (typeof textareaRef.current.gotoLineEnd === "function") {
              textareaRef.current.gotoLineEnd();
            }
          } catch (_) {}
        }
        // Release sync flag in next microtask
        setTimeout(() => {
          isSyncingRef.current = false;
        }, 0);
      }
    }
  }, [value]);

  const handleContentChange = () => {
    if (isSyncingRef.current) {
      return;
    }
    if (textareaRef.current) {
      const text = textareaRef.current.plainText || "";
      if (text !== lastEmittedValueRef.current) {
        lastEmittedValueRef.current = text;
        onChange(text);
      }
    }
  };

  const handleSubmit = () => {
    if (textareaRef.current) {
      const text = textareaRef.current.plainText || "";
      lastEmittedValueRef.current = "";
      textareaRef.current.clear();
      onSubmit(text);
    } else {
      onSubmit(value);
    }
  };

  return (
    <box
      flexDirection="row"
      alignItems="flex-start"
      border={["top"]}
      borderStyle="single"
      borderColor={anyContextTheme.ruleColor}
      backgroundColor={anyContextTheme.inputBackground}
      paddingLeft={1}
      paddingRight={1}
      paddingTop={0}
      paddingBottom={0}
      flexShrink={0}
    >
      <box paddingTop={0} paddingRight={1}>
        <text fg={anyContextTheme.accent}>
          <b>👤 You:</b>
        </text>
      </box>
      <box flexGrow={1}>
        <textarea
          ref={textareaRef}
          flexGrow={1}
          minHeight={1}
          maxHeight={6}
          wrapMode="word"
          focused={!disabled}
          placeholder={placeholder}
          placeholderColor={anyContextTheme.inputPlaceholder}
          textColor={anyContextTheme.foreground}
          focusedTextColor={anyContextTheme.foreground}
          cursorColor={anyContextTheme.accent}
          onContentChange={handleContentChange}
          onSubmit={handleSubmit}
          keyBindings={[
            { name: "return", action: "submit" },
            { name: "return", shift: true, action: "newline" },
            { name: "return", ctrl: true, action: "newline" },
            { name: "return", meta: true, action: "newline" },
          ]}
        />
      </box>
    </box>
  );
};
