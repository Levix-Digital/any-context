use any_context_core_rs::theme::{AuroraTheme, RgbColor};
use ratatui::style::Color;

/// Converts a core `RgbColor` to a `ratatui::style::Color::Rgb`.
#[inline]
pub const fn to_ratatui_color(c: RgbColor) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

/// UI-layer adapter providing Ratatui and ANSI TrueColor mappings from the
/// single source of truth `AuroraTheme` defined in `any-context-core-rs`.
#[derive(Debug, Clone)]
pub struct UiTheme {
    /// Underlying core design tokens
    pub core: AuroraTheme,
    /// Brand Emerald (#00E5A3)
    pub primary: Color,
    /// Glacial Cyan (#00F0FF)
    pub accent: Color,
    /// Cosmic Violet (#A855F7)
    pub reasoning: Color,
    /// Polar Pink (#EC4899)
    pub magenta: Color,
    /// Solar Amber (#F59E0B)
    pub warning: Color,
    /// Polar Crimson (#EF4444)
    pub error: Color,
    /// Arctic Sky Blue (#38BDF8)
    pub info: Color,
    /// Polar Night (#0B0F19)
    pub bg_dark: Color,
    /// Deep Midnight Surface (#161F30)
    pub bg_surface: Color,
    /// Focused Border (#00F0FF)
    pub border_focus: Color,
    /// Subdued Slate Border (#334155)
    pub border_unfocused: Color,
    /// Polar Ice White (#F1F5F9)
    pub text_bright: Color,
    /// Glacial Silver (#CBD5E1)
    pub text_body: Color,
    /// Muted Slate Frost (#64748B)
    pub text_muted: Color,
}

impl From<AuroraTheme> for UiTheme {
    fn from(core: AuroraTheme) -> Self {
        Self {
            primary: to_ratatui_color(core.primary),
            accent: to_ratatui_color(core.accent),
            reasoning: to_ratatui_color(core.reasoning),
            magenta: to_ratatui_color(core.magenta),
            warning: to_ratatui_color(core.warning),
            error: to_ratatui_color(core.error),
            info: to_ratatui_color(core.info),
            bg_dark: to_ratatui_color(core.bg_dark),
            bg_surface: to_ratatui_color(core.bg_surface),
            border_focus: to_ratatui_color(core.border_focus),
            border_unfocused: to_ratatui_color(core.border_unfocused),
            text_bright: to_ratatui_color(core.text_bright),
            text_body: to_ratatui_color(core.text_body),
            text_muted: to_ratatui_color(core.text_muted),
            core,
        }
    }
}

impl Default for UiTheme {
    fn default() -> Self {
        Self::from(AuroraTheme::default())
    }
}

impl UiTheme {
    /// Formats a text snippet with ANSI TrueColor using the primary (Emerald) color.
    pub fn ansi_primary(&self, text: &str) -> String {
        format!("{}{}\x1b[0m", self.core.primary.ansi_fg(), text)
    }

    /// Formats a text snippet with ANSI TrueColor using the accent (Cyan) color.
    pub fn ansi_accent(&self, text: &str) -> String {
        format!("{}{}\x1b[0m", self.core.accent.ansi_fg(), text)
    }

    /// Formats a text snippet with ANSI TrueColor using the reasoning (Violet) color.
    pub fn ansi_reasoning(&self, text: &str) -> String {
        format!("{}{}\x1b[0m", self.core.reasoning.ansi_fg(), text)
    }

    /// Formats a text snippet with ANSI TrueColor using the warning (Amber) color.
    pub fn ansi_warning(&self, text: &str) -> String {
        format!("{}{}\x1b[0m", self.core.warning.ansi_fg(), text)
    }

    /// Formats a text snippet with ANSI TrueColor using the error (Crimson) color.
    pub fn ansi_error(&self, text: &str) -> String {
        format!("{}{}\x1b[0m", self.core.error.ansi_fg(), text)
    }

    /// Formats a text snippet with ANSI TrueColor using muted slate text.
    pub fn ansi_muted(&self, text: &str) -> String {
        format!("{}{}\x1b[0m", self.core.text_muted.ansi_fg(), text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_theme_ratatui_mapping() {
        let theme = UiTheme::default();
        assert_eq!(theme.primary, Color::Rgb(0x00, 0xE5, 0xA3));
        assert_eq!(theme.accent, Color::Rgb(0x00, 0xF0, 0xFF));
        assert_eq!(theme.reasoning, Color::Rgb(0xA8, 0x55, 0xF7));
        assert_eq!(theme.warning, Color::Rgb(0xF5, 0x9E, 0x0B));
        assert_eq!(theme.error, Color::Rgb(0xEF, 0x44, 0x44));
        assert_eq!(theme.border_unfocused, Color::Rgb(0x33, 0x41, 0x55));
    }

    #[test]
    fn test_ansi_formatting() {
        let theme = UiTheme::default();
        let formatted = theme.ansi_primary("AnyContext");
        assert!(formatted.starts_with("\x1b[38;2;0;229;163m"));
        assert!(formatted.ends_with("\x1b[0m"));
    }
}
