use ratatui::style::Style;
use ratatui::text::{Line, Span};

const BYTES_PER_LINE: usize = 16;

pub fn hex_lines(data: &[u8], addr_style: Style, byte_style: Style, ascii_style: Style) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (chunk_idx, chunk) in data.chunks(BYTES_PER_LINE).enumerate() {
        let addr = chunk_idx * BYTES_PER_LINE;
        let mut spans = vec![Span::styled(format!("{addr:08X}  "), addr_style)];

        for (i, byte) in chunk.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(format!("{byte:02X}"), byte_style));
        }

        let padding = (BYTES_PER_LINE - chunk.len()) * 3;
        if padding > 0 {
            spans.push(Span::raw(" ".repeat(padding)));
        }

        spans.push(Span::raw("  "));
        let ascii: String = chunk
            .iter()
            .map(|b| {
                let c = *b as char;
                if c.is_ascii_graphic() || c == ' ' {
                    c
                } else {
                    '.'
                }
            })
            .collect();
        spans.push(Span::styled(ascii, ascii_style));

        lines.push(Line::from(spans));
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::raw("(empty)")));
    }
    lines
}
