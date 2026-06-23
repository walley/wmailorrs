use anyhow::{Context, Result};
use mail_parser::{
    Address, Addr, HeaderValue, Message, MessageParser, MessagePart, MimeHeaders, PartType,
};
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

        if let Some(root_part) = msg.parts.first() {
            build_nodes_from_part(root_part, &msg, raw_bytes, &mut nodes, &mut next_id);
        }

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

fn build_nodes_from_part(
    part: &MessagePart,
    msg: &Message,
    raw: &[u8],
    nodes: &mut Vec<MimeNode>,
    next_id: &mut usize,
) {
    let id = *next_id;
    *next_id += 1;

    let content_type = part
        .content_type()
        .map(|ct| {
            let subtype = ct.c_subtype.as_deref().unwrap_or("plain");
            format!("{}/{}", ct.c_type, subtype)
        })
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let filename = part.attachment_name().map(str::to_string);
    let encoding = part
        .content_transfer_encoding()
        .map(str::to_string);

    let h_start = part.raw_header_offset() as usize;
    let b_start = part.raw_body_offset() as usize;
    let b_end = part.raw_end_offset() as usize;

    let raw_header = if h_start < b_start && b_start <= raw.len() {
        String::from_utf8_lossy(&raw[h_start..b_start]).trim_end().to_string()
    } else {
        format_part_headers(part)
    };

    let raw_body = if b_start <= b_end && b_end <= raw.len() {
        raw[b_start..b_end].to_vec()
    } else {
        part.contents().to_vec()
    };

    let decoded_body = part.contents().to_vec();
    let is_binary = part.is_binary();

    let boundary = part.content_type().and_then(|ct| {
        ct.attributes.as_ref()?.iter().find_map(|attr| {
            if attr.name.eq_ignore_ascii_case("boundary") {
                Some(attr.value.to_string())
            } else {
                None
            }
        })
    });

    let mut children = Vec::new();
    match &part.body {
        PartType::Multipart(sub_ids) => {
            for &part_id in sub_ids {
                if let Some(sub) = msg.part(part_id) {
                    build_nodes_from_part(sub, msg, raw, &mut children, next_id);
                }
            }
        }
        PartType::Message(nested) => {
            if let Some(nested_root) = nested.parts.first() {
                build_nodes_from_part(nested_root, nested, raw, &mut children, next_id);
            }
        }
        _ => {}
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

fn format_part_headers(part: &MessagePart) -> String {
    part.headers()
        .iter()
        .map(|h| format!("{}: {}", h.name(), header_value_to_string(&h.value)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn header_value_to_string(value: &HeaderValue) -> String {
    match value {
        HeaderValue::Text(s) => s.to_string(),
        HeaderValue::TextList(list) => list
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        HeaderValue::DateTime(date) => format!("{date:?}"),
        HeaderValue::Address(addr) => format_address(addr),
        HeaderValue::ContentType(ct) => {
            let subtype = ct.c_subtype.as_deref().unwrap_or("plain");
            format!("{}/{}", ct.c_type, subtype)
        }
        HeaderValue::Received(received) => format!("{received:?}"),
        HeaderValue::Empty => String::new(),
    }
}

fn format_address(addr: &Address) -> String {
    match addr {
        Address::List(addrs) => addrs
            .iter()
            .map(format_addr)
            .collect::<Vec<_>>()
            .join(", "),
        Address::Group(groups) => groups
            .iter()
            .map(|g| g.name.as_ref().map(|n| n.to_string()).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn format_addr(addr: &Addr) -> String {
    if let Some(name) = &addr.name {
        if let Some(email) = &addr.address {
            format!("{name} <{email}>")
        } else {
            name.to_string()
        }
    } else if let Some(email) = &addr.address {
        email.to_string()
    } else {
        String::new()
    }
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
    } else if !node.raw_body.is_empty() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_plain_message() {
        let raw = "From: a@b.com\r\nTo: c@d.com\r\nSubject: t\r\nContent-Type: text/plain\r\n\r\nhello\r\n";
        let tree = MimeTree::from_raw(raw).unwrap();
        assert!(!tree.nodes.is_empty());
        let lines = tree.flatten_visible(&HashSet::new(), &HashSet::new());
        assert!(lines.len() > 1);
    }

    #[test]
    fn parse_multipart_message() {
        let raw = concat!(
            "From: a@b.com\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: multipart/alternative; boundary=\"b\"\r\n",
            "\r\n",
            "--b\r\n",
            "Content-Type: text/plain\r\n",
            "\r\n",
            "plain\r\n",
            "--b\r\n",
            "Content-Type: text/html\r\n",
            "\r\n",
            "<p>html</p>\r\n",
            "--b--\r\n",
        );
        let tree = MimeTree::from_raw(raw).unwrap();
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.nodes[0].children.len(), 2);
    }
}
