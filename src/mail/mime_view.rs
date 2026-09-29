use anyhow::{Context, Result};
use mail_parser::{
    Address, Addr, HeaderValue, Message, MessageParser, MessagePart, MimeHeaders, PartType,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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
        for node in &self.nodes {
            if let Some(found) = find_node(node, id) {
                return Some(found);
            }
        }
        None
    }

    pub fn flatten_visible(
        &self,
        folded: &HashSet<usize>,
        show_decoded: &HashSet<usize>,
        expanded_node: Option<usize>,
    ) -> Vec<VisibleMimeLine> {
        let mut out = Vec::new();
        let child_ids: HashSet<usize> = self
            .nodes
            .iter()
            .flat_map(|n| n.children.iter().map(|c| c.id))
            .collect();
        for node in &self.nodes {
            if !child_ids.contains(&node.id) {
                emit_node(node, &mut out, folded, show_decoded, 0, expanded_node);
            }
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

fn find_node(node: &MimeNode, id: usize) -> Option<&MimeNode> {
    if node.id == id {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_node(child, id) {
            return Some(found);
        }
    }
    None
}

fn emit_node(
    node: &MimeNode,
    out: &mut Vec<VisibleMimeLine>,
    folded: &HashSet<usize>,
    show_decoded: &HashSet<usize>,
    indent: usize,
    expanded_node: Option<usize>,
) {
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
        folded: false,
    });

    if expanded_node == Some(node.id) {
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
    }

    for child in &node.children {
        emit_node(child, out, folded, show_decoded, indent + 1, expanded_node);
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

pub fn mime_extension(content_type: &str) -> Option<&'static str> {
    let ct = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    match ct.as_str() {
        "text/plain" => Some(".txt"),
        "text/html" => Some(".html"),
        "text/css" => Some(".css"),
        "text/csv" => Some(".csv"),
        "text/markdown" => Some(".md"),
        "text/xml" => Some(".xml"),
        "text/calendar" => Some(".ics"),
        "text/vcard" => Some(".vcf"),
        "text/x-shellscript" => Some(".sh"),
        "application/json" => Some(".json"),
        "application/javascript" => Some(".js"),
        "application/xml" => Some(".xml"),
        "application/pdf" => Some(".pdf"),
        "application/zip" => Some(".zip"),
        "application/gzip" => Some(".gz"),
        "application/x-tar" => Some(".tar"),
        "application/x-7z-compressed" => Some(".7z"),
        "application/x-bzip2" => Some(".bz2"),
        "application/msword" => Some(".doc"),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => Some(".docx"),
        "application/vnd.ms-excel" => Some(".xls"),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => Some(".xlsx"),
        "application/vnd.ms-powerpoint" => Some(".ppt"),
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => Some(".pptx"),
        "application/rtf" => Some(".rtf"),
        "application/x-tex" => Some(".tex"),
        "application/postscript" => Some(".ps"),
        "application/vnd.rar" => Some(".rar"),
        "application/wasm" => Some(".wasm"),
        "application/pgp-signature" => Some(".asc"),
        "application/x-pem-file" => Some(".pem"),
        "application/pkix-cert" => Some(".cer"),
        "application/x-msdownload" => Some(".exe"),
        "application/epub+zip" => Some(".epub"),
        "application/vnd.amazon.ebook" => Some(".azw"),
        "application/x-mobipocket-ebook" => Some(".mobi"),
        "application/vnd.oasis.opendocument.text" => Some(".odt"),
        "application/vnd.oasis.opendocument.spreadsheet" => Some(".ods"),
        "application/vnd.oasis.opendocument.presentation" => Some(".odp"),
        "image/jpeg" | "image/pjpeg" => Some(".jpg"),
        "image/png" => Some(".png"),
        "image/gif" => Some(".gif"),
        "image/webp" => Some(".webp"),
        "image/svg+xml" => Some(".svg"),
        "image/bmp" => Some(".bmp"),
        "image/tiff" => Some(".tiff"),
        "image/x-icon" => Some(".ico"),
        "image/avif" => Some(".avif"),
        "image/x-portable-pixmap" => Some(".ppm"),
        "image/x-portable-graymap" => Some(".pgm"),
        "image/x-portable-bitmap" => Some(".pbm"),
        "audio/mpeg" => Some(".mp3"),
        "audio/ogg" => Some(".ogg"),
        "audio/wav" | "audio/x-wav" => Some(".wav"),
        "audio/flac" => Some(".flac"),
        "audio/aac" => Some(".aac"),
        "audio/mp4" | "audio/x-m4a" => Some(".m4a"),
        "audio/x-ms-wma" => Some(".wma"),
        "audio/midi" => Some(".mid"),
        "video/mp4" => Some(".mp4"),
        "video/mpeg" => Some(".mpg"),
        "video/ogg" => Some(".ogv"),
        "video/webm" => Some(".webm"),
        "video/x-msvideo" => Some(".avi"),
        "video/x-matroska" => Some(".mkv"),
        "video/quicktime" => Some(".mov"),
        "video/x-ms-wmv" => Some(".wmv"),
        "video/3gpp" => Some(".3gp"),
        "video/x-flv" => Some(".flv"),
        "message/rfc822" => Some(".eml"),
        "font/ttf" => Some(".ttf"),
        "font/otf" => Some(".otf"),
        "font/woff" => Some(".woff"),
        "font/woff2" => Some(".woff2"),
        _ => None,
    }
}

pub fn part_download_name(node: &MimeNode) -> String {
    let mut fname = node
        .filename
        .clone()
        .unwrap_or_else(|| format!("part-{}", node.id));
    if let Some(ext) = mime_extension(&node.content_type) {
        if !fname.to_lowercase().ends_with(ext) {
            fname.push_str(ext);
        }
    } else if !fname.rsplit('/').next().unwrap_or("").contains('.') {
        fname.push_str(".bin");
    }
    fname
}

pub fn part_download_paths(node: &MimeNode, dir: &Path) -> (PathBuf, PathBuf) {
    let base = part_download_name(node);
    (dir.join(&base), dir.join(format!("{base}.encoded")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_plain_message() {
        let raw = "From: a@b.com\r\nTo: c@d.com\r\nSubject: t\r\nContent-Type: text/plain\r\n\r\nhello\r\n";
        let tree = MimeTree::from_raw(raw).unwrap();
        assert!(!tree.nodes.is_empty());
        let lines = tree.flatten_visible(&HashSet::new(), &HashSet::new(), None);
        assert!(!lines.is_empty());
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

    fn node(content_type: &str, filename: Option<&str>, id: usize) -> MimeNode {
        MimeNode {
            id,
            content_type: content_type.to_string(),
            filename: filename.map(str::to_string),
            encoding: None,
            raw_header: String::new(),
            raw_body: vec![],
            decoded_body: vec![],
            is_binary: false,
            children: Vec::new(),
            boundary: None,
        }
    }

    #[test]
    fn mime_extension_known_types() {
        assert_eq!(mime_extension("text/plain; charset=utf-8"), Some(".txt"));
        assert_eq!(mime_extension("text/html"), Some(".html"));
        assert_eq!(mime_extension("IMAGE/JPEG"), Some(".jpg"));
        assert_eq!(mime_extension("application/pdf"), Some(".pdf"));
        assert_eq!(mime_extension("application/octet-stream"), None);
        assert_eq!(mime_extension("application/vnd.ms-excel"), Some(".xls"));
        assert_eq!(mime_extension("message/rfc822"), Some(".eml"));
        assert_eq!(mime_extension("video/mp4"), Some(".mp4"));
        assert_eq!(mime_extension("application/x-unknown"), None);
    }

    #[test]
    fn part_download_name_appends_mime_extension() {
        let n = node("text/plain", None, 3);
        assert_eq!(part_download_name(&n), "part-3.txt");

        let n = node("image/jpeg", None, 1);
        assert_eq!(part_download_name(&n), "part-1.jpg");
    }

    #[test]
    fn part_download_name_preserves_filenames() {
        let n = node("image/png", Some("photo.png"), 0);
        assert_eq!(part_download_name(&n), "photo.png");

        let n = node("image/jpeg", Some("photo.jpg"), 0);
        assert_eq!(part_download_name(&n), "photo.jpg");

        let n = node("text/plain", Some("notes"), 0);
        assert_eq!(part_download_name(&n), "notes.txt");

        let n = node("application/octet-stream", Some("archive.zip"), 0);
        assert_eq!(part_download_name(&n), "archive.zip");
    }

    #[test]
    fn part_download_name_unknown_mime_falls_back_to_bin() {
        let n = node("application/x-unknown", None, 5);
        assert_eq!(part_download_name(&n), "part-5.bin");

        let n = node("application/x-unknown", Some("script.py"), 5);
        assert_eq!(part_download_name(&n), "script.py");
    }

    #[test]
    fn part_download_paths_split_decoded_and_encoded() {
        let n = node("image/png", Some("photo.png"), 0);
        let dir = Path::new("/tmp/dl");
        let (decoded, encoded) = part_download_paths(&n, dir);
        assert_eq!(decoded, dir.join("photo.png"));
        assert_eq!(encoded, dir.join("photo.png.encoded"));

        let n = node("text/plain", None, 2);
        let (decoded, encoded) = part_download_paths(&n, dir);
        assert_eq!(decoded, dir.join("part-2.txt"));
        assert_eq!(encoded, dir.join("part-2.txt.encoded"));
    }
}
