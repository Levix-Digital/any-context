use serde::{Deserialize, Serialize};

/// Represents a 24-bit RGB color with conversion utilities for Hex, ANSI TrueColor, and styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    /// Creates a new RGB color instance.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Converts the RGB color to an uppercase Hex string format (e.g., `#00E5A3`).
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Generates standard 24-bit ANSI TrueColor foreground escape code (`\x1b[38;2;R;G;Bm`).
    pub fn ansi_fg(&self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b)
    }

    /// Generates standard 24-bit ANSI TrueColor background escape code (`\x1b[48;2;R;G;Bm`).
    pub fn ansi_bg(&self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.r, self.g, self.b)
    }

    /// Returns a tuple `(r, g, b)`.
    pub const fn to_rgb_tuple(&self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }
}

/// Official Levix Digital Aurora Boreal Design System Tokens.
/// Single source of truth for color definitions across CLI, TUI, Web, REST, and Desktop UIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuroraTheme {
    /// Brand Emerald / Aurora Green (#00E5A3) - Primary brand, active, ready, success
    pub primary: RgbColor,
    /// Glacial Cyan / Electric Teal (#00F0FF) - Highlight, hyperlinks, user prompts, focused borders
    pub accent: RgbColor,
    /// Cosmic Violet / Aurora Purple (#A855F7) - Deep reasoning, models, classification, AI router badges
    pub reasoning: RgbColor,
    /// Polar Pink / Magenta (#EC4899) - Proactive mode, special badges, creative tokens
    pub magenta: RgbColor,
    /// Solar Amber (#F59E0B) - Syncing in progress, thinking/warning, interactive highlights
    pub warning: RgbColor,
    /// Polar Crimson / Coral (#EF4444) - Errors, cancellation, critical failures
    pub error: RgbColor,
    /// Arctic Sky Blue (#38BDF8) - Sub-queries, tool inspections, informational badges
    pub info: RgbColor,
    /// Polar Night / Obsidian (#0B0F19) - Dark application base background
    pub bg_dark: RgbColor,
    /// Deep Midnight Surface (#161F30) - Panel backgrounds, selected items, popup surface
    pub bg_surface: RgbColor,
    /// Focused Border (#00F0FF) - Glacial Cyan border for focused panels and active inputs
    pub border_focus: RgbColor,
    /// Subdued Slate Border (#334155) - Neutral border for passive panels and dividers
    pub border_unfocused: RgbColor,
    /// Polar Ice White (#F1F5F9) - High-contrast bright text and headings
    pub text_bright: RgbColor,
    /// Glacial Silver (#CBD5E1) - Primary readable body text
    pub text_body: RgbColor,
    /// Muted Slate Frost (#64748B) - Timestamps, hints, ghost text, secondary labels
    pub text_muted: RgbColor,
}

impl Default for AuroraTheme {
    fn default() -> Self {
        Self {
            primary: RgbColor::new(0x00, 0xE5, 0xA3),          // #00E5A3 - Emerald Boreal
            accent: RgbColor::new(0x00, 0xF0, 0xFF),           // #00F0FF - Glacial Cyan
            reasoning: RgbColor::new(0xA8, 0x55, 0xF7),        // #A855F7 - Cosmic Violet
            magenta: RgbColor::new(0xEC, 0x48, 0x99),          // #EC4899 - Polar Pink
            warning: RgbColor::new(0xF5, 0x9E, 0x0B),          // #F59E0B - Solar Amber
            error: RgbColor::new(0xEF, 0x44, 0x44),            // #EF4444 - Polar Crimson
            info: RgbColor::new(0x38, 0xBD, 0xF8),             // #38BDF8 - Arctic Sky Blue
            bg_dark: RgbColor::new(0x0B, 0x0F, 0x19),          // #0B0F19 - Polar Night
            bg_surface: RgbColor::new(0x16, 0x1F, 0x30),       // #161F30 - Deep Midnight Surface
            border_focus: RgbColor::new(0x00, 0xF0, 0xFF),     // #00F0FF - Glacial Cyan
            border_unfocused: RgbColor::new(0x33, 0x41, 0x55), // #334155 - Subdued Slate
            text_bright: RgbColor::new(0xF1, 0xF5, 0xF9),      // #F1F5F9 - Polar Ice White
            text_body: RgbColor::new(0xCB, 0xD5, 0xE1),        // #CBD5E1 - Glacial Silver
            text_muted: RgbColor::new(0x64, 0x74, 0x8B),       // #64748B - Muted Slate Frost
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_color_conversions() {
        let emerald = RgbColor::new(0, 229, 163);
        assert_eq!(emerald.to_hex(), "#00E5A3");
        assert_eq!(emerald.ansi_fg(), "\x1b[38;2;0;229;163m");
        assert_eq!(emerald.ansi_bg(), "\x1b[48;2;0;229;163m");
        assert_eq!(emerald.to_rgb_tuple(), (0, 229, 163));
    }

    #[test]
    fn test_theme_json_serialization() {
        let theme = AuroraTheme::default();
        let json = serde_json::to_string_pretty(&theme).expect("Serialization failed");
        assert!(json.contains("\"#00E5A3\"") || json.contains("\"r\": 0"));

        let deserialized: AuroraTheme = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(theme, deserialized);
        assert_eq!(deserialized.primary.to_hex(), "#00E5A3");
        assert_eq!(deserialized.accent.to_hex(), "#00F0FF");
        assert_eq!(deserialized.reasoning.to_hex(), "#A855F7");
    }

    #[test]
    fn test_default_aurora_theme_tokens() {
        let theme = AuroraTheme::default();
        assert_eq!(theme.primary.to_hex(), "#00E5A3");
        assert_eq!(theme.accent.to_hex(), "#00F0FF");
        assert_eq!(theme.reasoning.to_hex(), "#A855F7");
        assert_eq!(theme.magenta.to_hex(), "#EC4899");
        assert_eq!(theme.warning.to_hex(), "#F59E0B");
        assert_eq!(theme.error.to_hex(), "#EF4444");
        assert_eq!(theme.info.to_hex(), "#38BDF8");
        assert_eq!(theme.bg_dark.to_hex(), "#0B0F19");
        assert_eq!(theme.bg_surface.to_hex(), "#161F30");
        assert_eq!(theme.border_focus.to_hex(), "#00F0FF");
        assert_eq!(theme.border_unfocused.to_hex(), "#334155");
        assert_eq!(theme.text_bright.to_hex(), "#F1F5F9");
        assert_eq!(theme.text_body.to_hex(), "#CBD5E1");
        assert_eq!(theme.text_muted.to_hex(), "#64748B");
    }
}
