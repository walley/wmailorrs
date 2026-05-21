use crate::theme::Theme;
use mailparse::MailHeaderMap;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceRegion {
    Header,
    Body,
    MimeBoundary,
    MimeFoldMarker,
}

pub struct HighlightedLine {
    pub text: String,
    pub region: SourceRegion,
    pub header_key: Option<String>,
}

pub fn split_headers_body(raw: &str) -> (Vec<&str>, Vec<&str>) {
    let mut header_lines = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_body = false;
    for line in raw.lines() {
        if !in_body {
            if line.is_empty() {
                in_body = true;
                continue;
            }
            header_lines.push(line);
        } else {
            body_lines.push(line);
        }
    }
    (header_lines, body_lines)
}

pub fn highlight_raw_source(raw: &str, theme: &Theme) -> Vec<Line<'static>> {
    let (header_lines, body_lines) = split_headers_body(raw);
    let mut out = Vec::new();

    for line in &header_lines {
        let style = theme.header_line_style(line);
        out.push(Line::from(Span::styled(line.to_string(), style)));
    }

    if !header_lines.is_empty() && !body_lines.is_empty() {
        out.push(Line::from(Span::raw(String::new())));
    }

    for line in body_lines {
        let style = if line.starts_with("--") && line.ends_with("--") {
            theme.mime_boundary_style()
        } else {
            theme.body_style()
        };
        out.push(Line::from(Span::styled(line.to_string(), style)));
    }

    out
}

pub fn header_key_style(theme: &Theme, key: &str) -> Style {
    theme.header_line_style(&format!("{key}:"))
}

pub fn format_header_summary(headers: &mailparse::headers::Headers) -> String {
    let from = headers
        .get_first_value("From")
        .unwrap_or_else(|| "?".into());
    let subj = headers
        .get_first_value("Subject")
        .unwrap_or_else(|| "(no subject)".into());
    let date = headers.get_first_value("Date").unwrap_or_default();
    if date.is_empty() {
        format!("{from} — {subj}")
    } else {
        format!("{date} | {from} — {subj}")
    }
}
