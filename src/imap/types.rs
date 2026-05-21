#[derive(Debug, Clone)]
pub struct FolderEntry {
    pub name: String,
    pub delimiter: Option<char>,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MessageEntry {
    pub uid: u32,
    pub seq: u32,
    pub size: u32,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct FetchedMessage {
    pub uid: u32,
    pub raw: String,
}
