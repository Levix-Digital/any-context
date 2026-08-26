export interface Theme {
  background?: string;
  surface?: string;
  surfaceHighlight: string;
  foreground: string;
  foregroundMuted: string;
  accent: string;
  accentSecondary: string;
  accentSuccess: string;
  accentWarning: string;
  accentError: string;
  ruleColor: string;
  inputBackground?: string;
  inputPlaceholder: string;
}

export const anyContextTheme: Theme = {
  background: undefined,      // Transparent - inherits user's native terminal background
  surface: undefined,         // Transparent
  surfaceHighlight: "#24283b", // Highlight for active palette selection
  foreground: "#c0caf5",
  foregroundMuted: "#565f89",
  accent: "#7dcfff",          // Cyan (User Prompt & Highlights - ANSI 96m)
  accentSecondary: "#bb9af7", // Violet / Magenta (AI Model - ANSI 95m)
  accentSuccess: "#73daca",   // Emerald / Green (Web Search & Edition Badge - ANSI 92m)
  accentWarning: "#e0af68",   // Warm Gold / Yellow (Workspace & AI Header - ANSI 93m)
  accentError: "#f7768e",     // Coral Red (Exit & Errors - ANSI 91m)
  ruleColor: "#444b6a",       // Subtle Divider Border (ANSI 90m)
  inputBackground: undefined, // Transparent
  inputPlaceholder: "#565f89",
};
