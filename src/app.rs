use crate::config::{self, ConnectionProfile};
use crate::imap::{FolderEntry, ImapCommand, ImapEvent, ImapWorker, MessageEntry};
use crate::mail::{save_part, MimeTree, VisibleLineKind};
use crate::theme::Theme;
use crate::ui::menu::{MenuAction, MenuState};
use anyhow::{Context, Result};
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPanel {
    Folders,
    Messages,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentMode {
    Source,
    MimeTree,
    Hex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    None,
    Connect,
    LoadConnection,
    Status,
    Help,
}

#[derive(Debug, Clone)]
pub struct ConnectForm {
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub tls: bool,
    pub field: usize,
}

pub struct App {
    pub theme: Theme,
    pub focus: FocusPanel,
    pub content_mode: ContentMode,
    pub menu: MenuState,
    pub dialog: Dialog,

    pub connected: bool,
    pub connection_name: Option<String>,
    pub status: String,

    pub folders: Vec<FolderEntry>,
    pub folder_cursor: usize,
    pub folder_list_state: ListState,
    pub selected_folder: Option<String>,

    pub messages: Vec<MessageEntry>,
    pub message_cursor: usize,
    pub message_list_state: ListState,
    pub message_filter: String,

    pub current_raw: Option<String>,
    pub current_uid: Option<u32>,
    pub mime_tree: Option<MimeTree>,
    pub mime_folded: HashSet<usize>,
    pub mime_show_decoded: bool,
    pub mime_focused_node: Option<usize>,
    pub content_scroll: u16,
    pub hex_data: Option<Vec<u8>>,

    pub connect_form: ConnectForm,
    pub saved_connections: Vec<String>,

    imap: ImapWorker,
    imap_events: Receiver<ImapEvent>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let (imap, imap_events) = ImapWorker::spawn();

        Self {
            theme: config::load_theme(),
            focus: FocusPanel::Folders,
            content_mode: ContentMode::Source,
            menu: MenuState::default(),
            dialog: Dialog::None,
            connected: false,
            connection_name: None,
            status: "Not connected".into(),
            folders: Vec::new(),
            folder_cursor: 0,
            folder_list_state: ListState::default(),
            selected_folder: None,
            messages: Vec::new(),
            message_cursor: 0,
            message_list_state: ListState::default(),
            message_filter: String::new(),
            current_raw: None,
            current_uid: None,
            mime_tree: None,
            mime_folded: HashSet::new(),
            mime_show_decoded: true,
            mime_focused_node: None,
            content_scroll: 0,
            hex_data: None,
            connect_form: ConnectForm::default(),
            saved_connections: config::list_connections().unwrap_or_default(),
            imap,
            imap_events: imap_events,
            should_quit: false,
        }
    }

    pub fn drain_imap(&mut self) {
        while let Ok(evt) = self.imap_events.try_recv() {
            self.handle_imap_event(evt);
        }
    }

    fn handle_imap_event(&mut self, evt: ImapEvent) {
        match evt {
            ImapEvent::Connected(name) => {
                self.connected = true;
                self.connection_name = Some(name);
                self.imap.send(ImapCommand::ListFolders);
            }
            ImapEvent::Disconnected => {
                self.connected = false;
                self.connection_name = None;
                self.folders.clear();
                self.messages.clear();
                self.current_raw = None;
                self.status = "Disconnected".into();
            }
            ImapEvent::Folders(f) => {
                self.folders = f;
                self.folder_cursor = 0;
                self.clamp_folder_cursor();
                self.status = format!("{} folders", self.folders.len());
            }
            ImapEvent::FolderSelected(name) => {
                self.selected_folder = Some(name);
                self.imap.send(ImapCommand::ListMessages {
                    offset: 0,
                    limit: 200,
                });
            }
            ImapEvent::Messages(m) => {
                self.messages = m;
                self.message_cursor = 0;
                self.clamp_message_cursor();
                self.status = format!("{} messages", self.messages.len());
            }
            ImapEvent::MessageFetched(msg) => {
                self.current_uid = Some(msg.uid);
                self.current_raw = Some(msg.raw.clone());
                self.mime_tree = MimeTree::from_raw(&msg.raw).ok();
                self.mime_folded.clear();
                self.mime_focused_node = None;
                self.content_scroll = 0;
                self.content_mode = ContentMode::Source;
                self.sync_mime_focus();
                self.status = format!("Fetched UID {}", msg.uid);
            }
            ImapEvent::Error(e) => self.status = format!("Error: {e}"),
            ImapEvent::Status(s) => self.status = s,
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusPanel::Folders => FocusPanel::Messages,
            FocusPanel::Messages => FocusPanel::Content,
            FocusPanel::Content => FocusPanel::Folders,
        };
    }

    pub fn clamp_folder_cursor(&mut self) {
        if self.folders.is_empty() {
            self.folder_cursor = 0;
            self.folder_list_state.select(None);
        } else if self.folder_cursor >= self.folders.len() {
            self.folder_cursor = self.folders.len() - 1;
            self.folder_list_state.select(Some(self.folder_cursor));
        } else {
            self.folder_list_state.select(Some(self.folder_cursor));
        }
    }

    pub fn clamp_message_cursor(&mut self) {
        let n = self.filtered_messages().len();
        if n == 0 {
            self.message_cursor = 0;
            self.message_list_state.select(None);
        } else if self.message_cursor >= n {
            self.message_cursor = n - 1;
            self.message_list_state.select(Some(self.message_cursor));
        } else {
            self.message_list_state.select(Some(self.message_cursor));
        }
    }

    pub fn move_up(&mut self) {
        match self.focus {
            FocusPanel::Folders => {
                if self.folder_cursor > 0 {
                    self.folder_cursor -= 1;
                    self.folder_list_state.select(Some(self.folder_cursor));
                }
            }
            FocusPanel::Messages => {
                if self.message_cursor > 0 {
                    self.message_cursor -= 1;
                    self.message_list_state.select(Some(self.message_cursor));
                }
            }
            FocusPanel::Content if self.content_scroll > 0 => {
                self.content_scroll -= 1;
                self.sync_mime_focus();
            }
            _ => {}
        }
    }

    pub fn move_down(&mut self) {
        match self.focus {
            FocusPanel::Folders => {
                if self.folder_cursor + 1 < self.folders.len() {
                    self.folder_cursor += 1;
                    self.folder_list_state.select(Some(self.folder_cursor));
                }
            }
            FocusPanel::Messages => {
                let n = self.filtered_messages().len();
                if self.message_cursor + 1 < n {
                    self.message_cursor += 1;
                    self.message_list_state.select(Some(self.message_cursor));
                }
            }
            FocusPanel::Content => {
                let max = self.content_line_count().saturating_sub(1);
                if (self.content_scroll as usize) < max {
                    self.content_scroll += 1;
                    self.sync_mime_focus();
                }
            }
        }
    }

    pub fn content_line_count(&self) -> usize {
        match self.content_mode {
            ContentMode::Source => self
                .current_raw
                .as_ref()
                .map(|r| r.lines().count())
                .unwrap_or(0),
            ContentMode::MimeTree => self.mime_visible_lines().len(),
            ContentMode::Hex => self
                .hex_data
                .as_ref()
                .map(|d| d.len().div_ceil(16))
                .unwrap_or(0),
        }
    }

    pub fn mime_visible_lines(&self) -> Vec<crate::mail::VisibleMimeLine> {
        let Some(tree) = &self.mime_tree else {
            return Vec::new();
        };
        tree.flatten_visible(&self.mime_folded, self.mime_show_decoded)
    }

    pub fn sync_mime_focus(&mut self) {
        if self.content_mode != ContentMode::MimeTree {
            return;
        }
        let lines = self.mime_visible_lines();
        if lines.is_empty() {
            self.mime_focused_node = None;
            return;
        }
        let idx = (self.content_scroll as usize).min(lines.len().saturating_sub(1));
        self.mime_focused_node = lines[..=idx]
            .iter()
            .rev()
            .find_map(|l| l.node_id);
    }

    pub fn filtered_messages(&self) -> Vec<&MessageEntry> {
        if self.message_filter.is_empty() {
            return self.messages.iter().collect();
        }
        let f = self.message_filter.to_ascii_lowercase();
        self.messages
            .iter()
            .filter(|m| m.summary.to_ascii_lowercase().contains(&f))
            .collect()
    }

    pub fn activate(&mut self) {
        match self.focus {
            FocusPanel::Folders => self.open_folder(),
            FocusPanel::Messages => self.fetch_selected_message(),
            FocusPanel::Content if self.content_mode == ContentMode::MimeTree => {
                self.toggle_mime_fold();
            }
            FocusPanel::Content => {}
        }
    }

    fn open_folder(&mut self) {
        if let Some(folder) = self.folders.get(self.folder_cursor) {
            let name = folder.name.clone();
            self.imap.send(ImapCommand::SelectFolder(name));
        }
    }

    fn fetch_selected_message(&mut self) {
        let filtered = self.filtered_messages();
        if let Some(msg) = filtered.get(self.message_cursor) {
            self.imap.send(ImapCommand::FetchMessage(msg.uid));
        }
    }

    pub fn toggle_mime_fold(&mut self) {
        if let Some(id) = self.mime_focused_node {
            if self.mime_folded.contains(&id) {
                self.mime_folded.remove(&id);
            } else {
                self.mime_folded.insert(id);
            }
        }
    }

    pub fn toggle_decoded(&mut self) {
        self.mime_show_decoded = !self.mime_show_decoded;
    }

    pub fn show_hex_for_focused(&mut self) -> bool {
        let Some(id) = self.mime_focused_node else {
            return false;
        };
        let Some(tree) = self.mime_tree.as_ref() else {
            return false;
        };
        let Some(node) = tree.node(id) else {
            return false;
        };
        self.hex_data = Some(node.raw_body.clone());
        self.content_mode = ContentMode::Hex;
        true
    }

    pub fn download_focused_part(&mut self) -> Result<String> {
        let id = self.mime_focused_node.context("no part focused")?;
        let tree = self.mime_tree.as_ref().context("no mime tree")?;
        let node = tree.node(id).context("unknown part")?;
        let fname = node
            .filename
            .clone()
            .unwrap_or_else(|| format!("part-{}.bin", node.id));
        let path = config::download_dir()?.join(fname);
        save_part(node, path.clone(), self.mime_show_decoded)?;
        Ok(path.display().to_string())
    }

    pub fn save_current_message(&mut self) -> Result<String> {
        let raw = self.current_raw.as_ref().context("no message")?;
        let uid = self.current_uid.unwrap_or(0);
        let path = config::download_dir()?.join(format!("message-{uid}.eml"));
        std::fs::write(&path, raw)?;
        Ok(path.display().to_string())
    }

    pub fn execute_menu_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::Connect => self.dialog = Dialog::Connect,
            MenuAction::Disconnect => self.imap.send(ImapCommand::Disconnect),
            MenuAction::SaveConnection => {
                if let Ok(()) = self.save_connect_form() {
                    self.status = "Connection saved".into();
                    self.saved_connections = config::list_connections().unwrap_or_default();
                }
            }
            MenuAction::LoadConnection => {
                self.dialog = Dialog::LoadConnection;
                self.connect_form.field = 0;
                self.saved_connections = config::list_connections().unwrap_or_default();
            }
            MenuAction::SaveMessage => {
                if let Ok(p) = self.save_current_message() {
                    self.status = format!("Saved {p}");
                }
            }
            MenuAction::SaveRawPart | MenuAction::DownloadPart => {
                if let Ok(p) = self.download_focused_part() {
                    self.status = format!("Saved {p}");
                }
            }
            MenuAction::ToggleMimeFold => self.toggle_mime_fold(),
            MenuAction::ToggleOriginalDecoded => self.toggle_decoded(),
            MenuAction::ShowHex => {
                let _ = self.show_hex_for_focused();
            }
            MenuAction::RefreshFolders => self.imap.send(ImapCommand::ListFolders),
            MenuAction::RefreshMessages => {
                self.imap.send(ImapCommand::ListMessages {
                    offset: 0,
                    limit: 200,
                });
            }
            MenuAction::ShowSourceView => self.set_content_mode(ContentMode::Source),
            MenuAction::ShowMimeTreeView => self.set_content_mode(ContentMode::MimeTree),
            MenuAction::EditColors => self.status = "Theme saved on quit (defaults in menu)".into(),
            MenuAction::ResetColors => {
                self.theme = Theme::default();
                let _ = config::save_theme(&self.theme);
                self.status = "Theme reset".into();
            }
            MenuAction::Quit => self.should_quit = true,
        }
    }

    pub fn do_connect(&mut self) {
        if let Ok(profile) = self.connect_form.to_profile() {
            self.imap.send(ImapCommand::Connect(profile));
            self.dialog = Dialog::None;
        }
    }

    pub fn save_connect_form(&mut self) -> Result<()> {
        let profile = self.connect_form.to_profile()?;
        config::save_connection(&profile)
    }

    pub fn load_connection_at(&mut self, idx: usize) {
        if let Some(name) = self.saved_connections.get(idx) {
            if let Ok(p) = config::load_connection(name) {
                self.connect_form = ConnectForm::from_profile(&p);
                self.imap.send(ImapCommand::Connect(p));
                self.dialog = Dialog::None;
            }
        }
    }

    pub fn mime_lines_for_display(&self) -> Vec<(String, VisibleLineKind, Option<usize>)> {
        self.mime_visible_lines()
            .into_iter()
            .map(|l| {
                (
                    format!("{}{}", "  ".repeat(l.indent), l.text),
                    l.kind,
                    l.node_id,
                )
            })
            .collect()
    }

    pub fn set_content_mode(&mut self, mode: ContentMode) {
        self.content_mode = mode;
        if mode == ContentMode::MimeTree {
            self.sync_mime_focus();
        }
    }

    pub fn disconnect(&mut self) {
        self.imap.send(ImapCommand::Disconnect);
    }

    pub fn on_quit(&self) {
        let _ = config::save_theme(&self.theme);
    }
}

impl ConnectForm {
    pub fn to_profile(&self) -> Result<ConnectionProfile> {
        Ok(ConnectionProfile {
            name: if self.name.is_empty() {
                format!("{}@{}", self.user, self.host)
            } else {
                self.name.clone()
            },
            host: self.host.clone(),
            port: self.port.parse().unwrap_or(993),
            user: self.user.clone(),
            password: self.password.clone(),
            tls: self.tls,
        })
    }

    pub fn from_profile(p: &ConnectionProfile) -> Self {
        Self {
            name: p.name.clone(),
            host: p.host.clone(),
            port: p.port.to_string(),
            user: p.user.clone(),
            password: p.password.clone(),
            tls: p.tls,
            field: 0,
        }
    }
}

impl Default for ConnectForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            host: "localhost".into(),
            port: "993".into(),
            user: String::new(),
            password: String::new(),
            tls: true,
            field: 0,
        }
    }
}
