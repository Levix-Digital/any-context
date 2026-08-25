export interface Theme {
  background: string;
  surface: string;
  surfaceHighlight: string;
  foreground: string;
  foregroundMuted: string;
  accent: string;
  accentSecondary: string;
  accentSuccess: string;
  accentWarning: string;
  accentError: string;
  ruleColor: string;
  inputBackground: string;
  inputPlaceholder: string;
}

export const anyContextTheme: Theme = {
  background: "#1a1b26",
  surface: "#1f2335",
  surfaceHighlight: "#24283b",
  foreground: "#c0caf5",
  foregroundMuted: "#565f89",
  accent: "#7aa2f7",          // Neon Blue / Cyan (User & Prompts)
  accentSecondary: "#bb9af7", // Violet (AI Model)
  accentSuccess: "#73daca",   // Emerald (Web Search & Ready)
  accentWarning: "#e0af68",   // Warm Gold (Sync & System Notices)
  accentError: "#f7768e",     // Coral Red (Errors)
  ruleColor: "#3b4261",       // Subtle Divider Borders
  inputBackground: "#16161e", // Deep Dark Input Container
  inputPlaceholder: "#414868",
};
