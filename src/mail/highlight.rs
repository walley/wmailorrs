use crate::ui::theme::Theme;
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

/// Replace tab characters with "<------>" like mcedit does
fn replace_tabs(text: &str) -> String {
    text.replace('\t', "<------>")
}

/// Extract header key from a header line
/// Returns the key (e.g., "From", "To", "Date") if this is a new header line,
/// or None if this is a continuation line (starts with whitespace)
fn get_header_key(line: &str) -> Option<String> {
    if line.is_empty() {
        return None;
    }
    
    // Continuation lines start with whitespace (space or tab)
    if line.starts_with(' ') || line.starts_with('\t') {
        return None;
    }
    
    // Extract the header key (part before the colon)
    line.split(':').next().map(|s| s.trim().to_string())
}

pub fn highlight_raw_source(raw: &str, theme: &Theme) -> Vec<Line<'static>> {
    let (header_lines, body_lines) = split_headers_body(raw);
    let mut out = Vec::new();
    let mut current_header_key: Option<String> = None;

    for line in &header_lines {
        let display_line = replace_tabs(line);
        
        // Check if this is a new header or a continuation
        if let Some(key) = get_header_key(line) {
            current_header_key = Some(key.clone());
        }
        
        // Use the current header key to determine styling
        let style = if let Some(ref key) = current_header_key {
            theme.header_line_style_for_key(key)
        } else {
            theme.header_line_style(&display_line)
        };
        
        out.push(Line::from(Span::styled(display_line, style)));
    }

    if !header_lines.is_empty() && !body_lines.is_empty() {
        out.push(Line::from(Span::raw(String::new())));
    }

    for line in body_lines {
        let display_line = replace_tabs(line);
        let style = if display_line.starts_with("--") && display_line.ends_with("--") {
            theme.mime_boundary_style()
        } else {
            theme.body_style()
        };
        out.push(Line::from(Span::styled(display_line, style)));
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

