use anyhow::{Context, Result};
use base64::Engine;
use mailparse::{parse_mail, ParsedMail};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct MimeNode {
    pub id: usize,
    pub content_type: String,
    pub filename: Option<String>,
    pub encoding: Option<String>,
    pub raw_header: String,
    pub raw_body: Vec<u8>,
    pub decoded_body: Vec<u8>,
    pub is_binary: bool,
    pub children: Vec<MimeNode>,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Default)]
pub struct MimeTree {
    pub nodes: Vec<MimeNode>,
    pub root_lines: Vec<String>,
}

impl MimeTree {
    pub fn from_raw(raw: &str) -> Result<Self> {
        let mail = parse_mail(raw.as_bytes()).context("parse mail")?;
        let root_lines: Vec<String> = raw.lines().map(String::from).collect();
        let mut nodes = Vec::new();
        let mut next_id = 0;
        build_nodes(&mail, raw, &mut nodes, &mut next_id, 0, root_lines.len());
        Ok(Self { nodes, root_lines })
    }

    pub fn node(&self, id: usize) -> Option<&MimeNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn flatten_visible(
        &self,
        folded: &std::collections::HashSet<usize>,
        show_decoded: bool,
    ) -> Vec<VisibleMimeLine> {
        let mut out = Vec::new();
        for node in &self.nodes {
            emit_node(node, &mut out, folded, show_decoded, 0);
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct VisibleMimeLine {
    pub node_id: Option<usize>,
    pub indent: usize,
    pub text: String,
    pub kind: VisibleLineKind,
    pub foldable: bool,
    pub folded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibleLineKind {
    Summary,
    HeaderBlock,
    BodyRaw,
    BodyDecoded,
    BinaryHint,
    ChildBoundary,
}

fn build_nodes(
    mail: &ParsedMail<'_>,
    raw: &str,
    nodes: &mut Vec<MimeNode>,
    next_id: &mut usize,
    start: usize,
    end: usize,
) {
    let id = *next_id;
    *next_id += 1;

    let content_type = mail.ctype.mimetype.clone();
    let filename = mail
        .get_content_disposition()
        .params
        .get("filename")
        .cloned();
    let encoding = mail
        .headers
        .iter()
        .find(|h| h.get_key().eq_ignore_ascii_case("Content-Transfer-Encoding"))
        .map(|h| h.get_value());
    let raw_body = mail.get_body_raw().unwrap_or_default();
    let decoded_body = decode_part(mail);
    let is_binary = is_probably_binary(&content_type, &decoded_body);

    let raw_header = mail
        .headers
        .iter()
        .map(|h| format!("{}: {}", h.get_key(), h.get_value()))
        .collect::<Vec<_>>()
        .join("\n");

    let mut children = Vec::new();
    for sub in &mail.subparts {
        build_nodes(sub, raw, &mut children, next_id, start, end);
    }

    nodes.push(MimeNode {
        id,
        content_type,
        filename,
        encoding,
        raw_header,
        raw_body,
        decoded_body,
        is_binary,
        children,
        start_line: start,
        end_line: end,
    });
}

fn emit_node(
    node: &MimeNode,
    out: &mut Vec<VisibleMimeLine>,
    folded: &std::collections::HashSet<usize>,
    show_decoded: bool,
    indent: usize,
) {
    let is_folded = folded.contains(&node.id);
    let label = node
        .filename
        .clone()
        .unwrap_or_else(|| node.content_type.clone());
    let enc = node.encoding.as_deref().unwrap_or("none");
    out.push(VisibleMimeLine {
        node_id: Some(node.id),
        indent,
        text: format!("[part {}] {label} ({enc})", node.id),
        kind: VisibleLineKind::Summary,
        foldable: !node.children.is_empty() || !node.raw_body.is_empty(),
        folded: is_folded,
    });

    if is_folded {
        return;
    }

    for line in node.raw_header.lines() {
        out.push(VisibleMimeLine {
            node_id: Some(node.id),
            indent: indent + 1,
            text: line.to_string(),
            kind: VisibleLineKind::HeaderBlock,
            foldable: false,
            folded: false,
        });
    }

    if node.is_binary {
        out.push(VisibleMimeLine {
            node_id: Some(node.id),
            indent: indent + 1,
            text: format!(
                "<binary {} bytes — press x for hex, d to download>",
                node.raw_body.len()
            ),
            kind: VisibleLineKind::BinaryHint,
            foldable: false,
            folded: false,
        });
    } else if show_decoded {
        let text = String::from_utf8_lossy(&node.decoded_body).to_string();
        for line in text.lines() {
            out.push(VisibleMimeLine {
                node_id: Some(node.id),
                indent: indent + 1,
                text: line.to_string(),
                kind: VisibleLineKind::BodyDecoded,
                foldable: false,
                folded: false,
            });
        }
    } else {
        let text = String::from_utf8_lossy(&node.raw_body).to_string();
        for line in text.lines() {
            out.push(VisibleMimeLine {
                node_id: Some(node.id),
                indent: indent + 1,
                text: line.to_string(),
                kind: VisibleLineKind::BodyRaw,
                foldable: false,
                folded: false,
            });
        }
    }

    for child in &node.children {
        emit_node(child, out, folded, show_decoded, indent + 1);
    }
}

fn decode_part(mail: &ParsedMail<'_>) -> Vec<u8> {
    mail.get_body()
        .map(|s| s.into_bytes())
        .unwrap_or_else(|_| mail.get_body_raw().unwrap_or_default())
}

fn is_probably_binary(content_type: &str, body: &[u8]) -> bool {
    let ct = content_type.to_ascii_lowercase();
    if ct.starts_with("text/") || ct.contains("message/rfc822") {
        return false;
    }
    if body.is_empty() {
        return false;
    }
    let sample = body.len().min(512);
    let non_text = body[..sample]
        .iter()
        .filter(|&&b| b != b'\n' && b != b'\r' && b != b'\t' && !(32..=126).contains(&b))
        .count();
    non_text * 4 > sample
}

pub fn save_part(node: &MimeNode, path: PathBuf, decoded: bool) -> Result<()> {
    let data = if decoded {
        &node.decoded_body
    } else {
        &node.raw_body
    };
    std::fs::write(&path, data).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn try_base64_decode(data: &[u8]) -> Vec<u8> {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(v) = base64::engine::general_purpose::STANDARD.decode(s.trim()) {
            return v;
        }
    }
    data.to_vec()
}
