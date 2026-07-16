use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemePreset {
    Default,
    Midnight,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub header_bg: ColorDef,
    pub header_received: ColorDef,
    pub header_x: ColorDef,
    pub header_from: ColorDef,
    pub header_to: ColorDef,
    pub header_delivery: ColorDef,
    pub header_date: ColorDef,
    pub header_normal: ColorDef,
    pub body_normal: ColorDef,
    pub mime_boundary: ColorDef,
    pub mime_folded: ColorDef,
    pub hex_address: ColorDef,
    pub hex_bytes: ColorDef,
    pub hex_ascii: ColorDef,
    pub panel_border: ColorDef,
    pub panel_focus_border: ColorDef,
    pub panel_title: ColorDef,
    pub panel_focus_title: ColorDef,
    pub selection: ColorDef,
    pub keybar_bg: ColorDef,
    pub keybar_fg: ColorDef,
    pub menu_bg: ColorDef,
    pub menu_fg: ColorDef,
    pub menu_active_fg: ColorDef,
    pub menu_selected_bg: ColorDef,
    pub menu_selected_fg: ColorDef,
    pub menu_shortcut_fg: ColorDef,
    pub menu_border: ColorDef,
    pub status_ok: ColorDef,
    pub status_err: ColorDef,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColorDef {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ColorDef {
    pub fn to_color(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::midnight()
    }
}

impl Theme {
    pub fn midnight() -> Self {
        Self {
            header_bg: ColorDef { r: 0, g: 0, b: 170 },
            header_received: ColorDef { r: 170, g: 170, b: 255 },
            header_x: ColorDef { r: 255, g: 170, b: 255 },
            header_from: ColorDef { r: 0, g: 255, b: 0 },
            header_to: ColorDef { r: 255, g: 170, b: 0 },
            header_delivery: ColorDef { r: 255, g: 170, b: 170 },
            header_date: ColorDef { r: 255, g: 255, b: 0 },
            header_normal: ColorDef { r: 170, g: 170, b: 170 },
            body_normal: ColorDef { r: 170, g: 170, b: 170 },
            mime_boundary: ColorDef { r: 0, g: 170, b: 170 },
            mime_folded: ColorDef { r: 0, g: 128, b: 128 },
            hex_address: ColorDef { r: 170, g: 170, b: 255 },
            hex_bytes: ColorDef { r: 0, g: 255, b: 0 },
            hex_ascii: ColorDef { r: 170, g: 170, b: 0 },
            panel_border: ColorDef { r: 170, g: 170, b: 170 },
            panel_focus_border: ColorDef { r: 255, g: 255, b: 0 },
            panel_title: ColorDef { r: 170, g: 170, b: 170 },
            panel_focus_title: ColorDef { r: 255, g: 255, b: 255 },
            selection: ColorDef { r: 0, g: 0, b: 0 },  // black text on cyan bg
            keybar_bg: ColorDef { r: 170, g: 170, b: 170 },
            keybar_fg: ColorDef { r: 0, g: 0, b: 0 },
            menu_bg: ColorDef { r: 0, g: 170, b: 170 },  // cyan background
            menu_fg: ColorDef { r: 0, g: 0, b: 0 },  // black text (inactive)
            menu_active_fg: ColorDef { r: 255, g: 255, b: 255 },  // white text (active)
            menu_selected_bg: ColorDef { r: 0, g: 0, b: 0 },  // black background (selected main menu item)
            menu_selected_fg: ColorDef { r: 255, g: 255, b: 255 },  // white text (selected main menu item)
            menu_shortcut_fg: ColorDef { r: 255, g: 255, b: 0 },  // yellow shortcut
            menu_border: ColorDef { r: 255, g: 255, b: 255 },  // white border
            status_ok: ColorDef { r: 0, g: 255, b: 0 },
            status_err: ColorDef { r: 255, g: 0, b: 0 },
        }
    }

    pub fn light() -> Self {
        Self {
            header_bg: ColorDef { r: 255, g: 255, b: 255 },
            header_received: ColorDef { r: 0, g: 0, b: 170 },
            header_x: ColorDef { r: 170, g: 0, b: 170 },
            header_from: ColorDef { r: 0, g: 128, b: 0 },
            header_to: ColorDef { r: 170, g: 85, b: 0 },
            header_delivery: ColorDef { r: 170, g: 0, b: 0 },
            header_date: ColorDef { r: 128, g: 128, b: 0 },
            header_normal: ColorDef { r: 0, g: 0, b: 0 },
            body_normal: ColorDef { r: 0, g: 0, b: 0 },
            mime_boundary: ColorDef { r: 0, g: 128, b: 128 },
            mime_folded: ColorDef { r: 128, g: 128, b: 128 },
            hex_address: ColorDef { r: 0, g: 0, b: 170 },
            hex_bytes: ColorDef { r: 0, g: 128, b: 0 },
            hex_ascii: ColorDef { r: 128, g: 128, b: 0 },
            panel_border: ColorDef { r: 128, g: 128, b: 128 },
            panel_focus_border: ColorDef { r: 170, g: 0, b: 0 },
            panel_title: ColorDef { r: 0, g: 0, b: 128 },
            panel_focus_title: ColorDef { r: 170, g: 0, b: 0 },
            selection: ColorDef { r: 0, g: 0, b: 0 },  // black text
            keybar_bg: ColorDef { r: 220, g: 220, b: 220 },
            keybar_fg: ColorDef { r: 0, g: 0, b: 0 },
            menu_bg: ColorDef { r: 220, g: 220, b: 220 },  // light gray background
            menu_fg: ColorDef { r: 0, g: 0, b: 0 },  // black text
            menu_active_fg: ColorDef { r: 255, g: 255, b: 255 },
            menu_selected_bg: ColorDef { r: 0, g: 0, b: 170 },  // blue background for selected
            menu_selected_fg: ColorDef { r: 255, g: 255, b: 255 },  // white text for selected
            menu_shortcut_fg: ColorDef { r: 0, g: 0, b: 170 },  // blue shortcut
            menu_border: ColorDef { r: 0, g: 0, b: 170 },  // blue border
            status_ok: ColorDef { r: 0, g: 128, b: 0 },
            status_err: ColorDef { r: 170, g: 0, b: 0 },
        }
    }

    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::Default => Theme::midnight(),
            ThemePreset::Midnight => Theme::midnight(),
            ThemePreset::Light => Theme::light(),
        }
    }
}

impl Theme {
    pub fn header_bg_style(&self) -> Style {
        Style::default()
            .bg(self.header_bg.to_color())
            .add_modifier(Modifier::BOLD)
    }

    pub fn header_line_style(&self, line: &str) -> Style {
        let key = line.split(':').next().unwrap_or("").trim();
        self.header_line_style_for_key(key)
    }

    /// Get the style for a header based on its key name
    pub fn header_line_style_for_key(&self, key: &str) -> Style {
        let fg = match key.to_ascii_lowercase().as_str() {
            "received" => self.header_received.to_color(),
            k if k.starts_with('x') => self.header_x.to_color(),
            "from" => self.header_from.to_color(),
            "to" | "cc" | "bcc" | "reply-to" => self.header_to.to_color(),
            "x-original-to" | "delivered-to" | "envelope-to" => self.header_delivery.to_color(),
            "date" => self.header_date.to_color(),
            _ => self.header_normal.to_color(),
        };
        self.header_bg_style().fg(fg)
    }

    pub fn body_style(&self) -> Style {
        Style::default().fg(self.body_normal.to_color())
    }

    pub fn mime_boundary_style(&self) -> Style {
        Style::default().fg(self.mime_boundary.to_color())
    }

    pub fn mime_folded_style(&self) -> Style {
        Style::default().fg(self.mime_folded.to_color())
    }

    pub fn selection_style(&self) -> Style {
        Style::default().bg(self.selection.to_color())
    }

    pub fn panel_border_style(&self) -> Style {
        Style::default().fg(self.panel_border.to_color())
    }

    pub fn panel_focus_border_style(&self) -> Style {
        Style::default()
            .fg(self.panel_focus_border.to_color())
            .add_modifier(Modifier::BOLD)
    }

    pub fn panel_title_style(&self) -> Style {
        Style::default()
            .fg(self.panel_title.to_color())
            .add_modifier(Modifier::BOLD)
    }

    pub fn panel_focus_title_style(&self) -> Style {
        Style::default()
            .fg(self.panel_focus_title.to_color())
            .bg(self.selection.to_color())
            .add_modifier(Modifier::BOLD)
    }

    pub fn keybar_style(&self) -> Style {
        Style::default()
            .bg(self.keybar_bg.to_color())
            .fg(self.keybar_fg.to_color())
    }

    pub fn menu_style(&self) -> Style {
        Style::default()
            .bg(self.menu_bg.to_color())
            .fg(self.menu_fg.to_color())
    }

    pub fn menu_active_style(&self) -> Style {
        Style::default()
            .bg(self.menu_bg.to_color())
            .fg(self.menu_active_fg.to_color())
    }

    pub fn menu_selected_style(&self) -> Style {
        Style::default()
            .bg(self.menu_selected_bg.to_color())
            .fg(self.menu_selected_fg.to_color())
    }

    pub fn menu_shortcut_style(&self) -> Style {
        Style::default()
            .bg(self.menu_selected_bg.to_color())
            .fg(self.menu_shortcut_fg.to_color())
    }

    pub fn menu_border_style(&self) -> Style {
        Style::default().fg(self.menu_border.to_color())
    }

    pub fn hex_address_style(&self) -> Style {
        Style::default().fg(self.hex_address.to_color())
    }

    pub fn hex_bytes_style(&self) -> Style {
        Style::default().fg(self.hex_bytes.to_color())
    }

    pub fn hex_ascii_style(&self) -> Style {
        Style::default().fg(self.hex_ascii.to_color())
    }
}
