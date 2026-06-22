use anyhow::{Context, Result};
use mail_parser::{Message, MessagePart, PartType};
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
}

impl MimeTree {
    pub fn from_raw(raw: &str) -> Result<Self> {
        let msg = Message::parse(raw.as_bytes())
            .context("Failed to parse email")?;
        
        let root_lines: Vec<String> = raw.lines().map(String::from).collect();
        let mut nodes = Vec::new();
        let mut next_id = 0;
        
        build_nodes_from_parser(&msg, raw, &mut nodes, &mut next_id);
        
        Ok(Self { nodes, root_lines })
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
    raw: &str,
    nodes: &mut Vec<MimeNode>,
    next_id: &mut usize,
) {
    let id = *next_id;
    *next_id += 1;

    let content_type = msg.content_type().unwrap_or("application/octet-stream").to_string();
    let filename = msg.attachment_name().map(|s| s.to_string());
    let encoding = msg.transfer_encoding().map(|e| format!("{:?}", e));
    
    // Get raw and decoded bodies
    let (raw_body, decoded_body) = extract_bodies(msg, raw);
    
    let is_binary = !msg.is_text();
    
    // Build raw header string
    let raw_header = format!("{:?}", msg.headers());

    let boundary = None; // mail-parser handles boundaries internally

    let mut children = Vec::new();
    
    // Handle multipart messages
    if let Some(parts) = msg.parts() {
        for part in parts {
            build_nodes_from_parser(&part, raw, &mut children, next_id);
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

fn extract_bodies(msg: &Message, raw: &str) -> (Vec<u8>, Vec<u8>) {
    // Get the raw body from the original message
    let raw_body = if let Some(part_data) = get_raw_part_data(msg, raw) {
        part_data
    } else {
        // Fallback: use the raw message bytes
        msg.raw_message().to_vec()
    };
    
    // Get decoded body
    let decoded_body = if let Some(body) = msg.body_text(1024 * 1024) {
        body.as_bytes().to_vec()
    } else if let Some(contents) = msg.contents() {
        contents.into()
    } else {
        raw_body.clone()
    };
    
    (raw_body, decoded_body)
}

fn get_raw_part_data(msg: &Message, raw: &str) -> Option<Vec<u8>> {
    // Extract the raw data for this specific part from the source
    // by finding its position in the raw message
    let raw_bytes = raw.as_bytes();
    let raw_msg = msg.raw_message();
    
    if let Some(pos) = find_subsequence(raw_bytes, raw_msg) {
        Some(raw_bytes[pos..pos + raw_msg.len()].to_vec())
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
