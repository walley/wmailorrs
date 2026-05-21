use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

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
        Self {
            header_bg: ColorDef {
                r: 30,
                g: 30,
                b: 60,
            },
            header_received: ColorDef {
                r: 180,
                g: 220,
                b: 255,
            },
            header_x: ColorDef {
                r: 200,
                g: 160,
                b: 255,
            },
            header_from: ColorDef {
                r: 120,
                g: 255,
                b: 160,
            },
            header_to: ColorDef {
                r: 255,
                g: 200,
                b: 120,
            },
            header_delivery: ColorDef {
                r: 255,
                g: 180,
                b: 200,
            },
            header_date: ColorDef {
                r: 255,
                g: 255,
                b: 140,
            },
            header_normal: ColorDef {
                r: 220,
                g: 220,
                b: 220,
            },
            body_normal: ColorDef {
                r: 200,
                g: 200,
                b: 200,
            },
            mime_boundary: ColorDef {
                r: 140,
                g: 200,
                b: 200,
            },
            mime_folded: ColorDef {
                r: 100,
                g: 140,
                b: 140,
            },
            hex_address: ColorDef {
                r: 120,
                g: 120,
                b: 180,
            },
            hex_bytes: ColorDef {
                r: 180,
                g: 220,
                b: 180,
            },
            hex_ascii: ColorDef {
                r: 200,
                g: 200,
                b: 160,
            },
            panel_border: ColorDef {
                r: 80,
                g: 80,
                b: 120,
            },
            panel_focus_border: ColorDef {
                r: 255,
                g: 220,
                b: 80,
            },
            panel_title: ColorDef {
                r: 180,
                g: 200,
                b: 255,
            },
            panel_focus_title: ColorDef {
                r: 255,
                g: 255,
                b: 255,
            },
            selection: ColorDef {
                r: 60,
                g: 80,
                b: 140,
            },
            keybar_bg: ColorDef {
                r: 20,
                g: 20,
                b: 30,
            },
            keybar_fg: ColorDef {
                r: 220,
                g: 220,
                b: 100,
            },
            menu_bg: ColorDef {
                r: 0,
                g: 0,
                b: 128,
            },
            menu_fg: ColorDef {
                r: 255,
                g: 255,
                b: 255,
            },
            status_ok: ColorDef {
                r: 120,
                g: 255,
                b: 120,
            },
            status_err: ColorDef {
                r: 255,
                g: 100,
                b: 100,
            },
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
