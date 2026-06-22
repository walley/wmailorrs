use anyhow::{Context, Result};
use mail_parser::{Message, MessageParser, MimeHeaders, HeaderValue};
use std::collections::HashSet;
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
    pub boundary: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MimeTree {
    pub nodes: Vec<MimeNode>,
    pub root_lines: Vec<String>,
    pub raw_message: Vec<u8>,
}

impl MimeTree {
    pub fn from_raw(raw: &str) -> Result<Self> {
        let raw_bytes = raw.as_bytes();
        let msg = MessageParser::default()
            .parse(raw_bytes)
            .context("Failed to parse email")?;
        
        let root_lines: Vec<String> = raw.lines().map(String::from).collect();
        let mut nodes = Vec::new();
        let mut next_id = 0;
        
        build_nodes_from_parser(&msg, raw_bytes, &mut nodes, &mut next_id);
        
        Ok(Self { 
            nodes, 
            root_lines,
            raw_message: raw_bytes.to_vec(),
        })
    }

    pub fn node(&self, id: usize) -> Option<&MimeNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn flatten_visible(
        &self,
        folded: &HashSet<usize>,
        show_decoded: &HashSet<usize>,
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

fn build_nodes_from_parser(
    msg: &Message,
    raw: &[u8],
    nodes: &mut Vec<MimeNode>,
    next_id: &mut usize,
) {
    let id = *next_id;
    *next_id += 1;

    // Get content type
    let content_type = if let Some(ct) = msg.content_type() {
        format!(
            "{}/{}",
            ct.maintype,
            ct.subtype
        )
    } else {
        "application/octet-stream".to_string()
    };

    // Get filename from Content-Disposition header
    let filename = msg.attachment_name().map(|s| s.to_string());
    
    // Get transfer encoding
    let encoding = msg.body_structures()
        .and_then(|structures| structures.first())
        .and_then(|s| s.encoding.as_ref())
        .map(|e| format!("{:?}", e));
    
    // Get raw and decoded bodies
    let (raw_body, decoded_body) = extract_bodies(msg, raw);
    
    // Check if binary (no text subtype)
    let is_binary = if let Some(ct) = msg.content_type() {
        ct.maintype != "text"
    } else {
        false
    };
    
    // Build raw header string from all headers
    let raw_header = format_headers(msg);

    let boundary = None; // mail-parser handles boundaries internally

    let mut children = Vec::new();
    
    // Handle multipart messages
    if let Some(parts) = msg.body_parts() {
        for part in parts {
            if let Some(part_msg) = part.as_message() {
                build_nodes_from_parser(&part_msg, raw, &mut children, next_id);
            }
        }
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
        boundary,
    });
}

fn format_headers(msg: &Message) -> String {
    let mut headers = String::new();
    for header in msg.headers() {
        let value_str = header_value_to_string(&header.value);
        headers.push_str(&format!("{}: {}\n", header.name(), value_str));
    }
    headers
}

fn header_value_to_string(value: &HeaderValue) -> String {
    match value {
        HeaderValue::Text(s) => s.to_string(),
        HeaderValue::TextList(list) => list.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", "),
        HeaderValue::Date(date) => format!("{:?}", date),
        HeaderValue::Address(addrs) => {
            addrs.iter()
                .map(|addr| {
                    if let Some(name) = &addr.name {
                        format!("{} <{}>", name, addr.address)
                    } else {
                        addr.address.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
        HeaderValue::ContentType(ct) => format!("{}/{}", ct.maintype, ct.subtype),
        HeaderValue::ContentDisposition(cd) => format!("{:?}", cd),
        other => format!("{:?}", other),
    }
}

fn extract_bodies(msg: &Message, raw: &[u8]) -> (Vec<u8>, Vec<u8>) {
    // Get decoded body first
    let decoded_body = if let Some(text) = msg.body_text(0) {
        text.as_bytes().to_vec()
    } else if let Some(html) = msg.body_html(0) {
        html.as_bytes().to_vec()
    } else {
        Vec::new()
    };
    
    // Get raw body by extracting from raw message
    let raw_body = if let Some(pos) = find_body_start(raw) {
        raw[pos..].to_vec()
    } else {
        raw.to_vec()
    };
    
    (raw_body, decoded_body)
}

fn find_body_start(raw: &[u8]) -> Option<usize> {
    // Find the double CRLF or double LF that separates headers from body
    if let Some(pos) = find_subsequence(raw, b"\r\n\r\n") {
        Some(pos + 4)
    } else if let Some(pos) = find_subsequence(raw, b"\n\n") {
        Some(pos + 2)
    } else {
        None
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len())
        .position(|window| window == needle)
}

fn emit_node(
    node: &MimeNode,
    out: &mut Vec<VisibleMimeLine>,
    folded: &HashSet<usize>,
    show_decoded: &HashSet<usize>,
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
    } else if show_decoded.contains(&node.id) {
        let text = String::from_utf8_lossy(&node.decoded_body);
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
        let text = String::from_utf8_lossy(&node.raw_body);
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

    // Show children
    for child in &node.children {
        emit_node(child, out, folded, show_decoded, indent + 1);
    }
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
