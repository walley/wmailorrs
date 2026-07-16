#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuBarItem {
    Main,
    Server,
    Message,
    View,
    Colors,
    UserFolders,
    UserMessages,
    UserContent,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub action: MenuAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Noop,
    Connect,
    Disconnect,
    SaveConnection,
    LoadConnection,
    SaveMessage,
    SaveRawPart,
    DownloadPart,
    ToggleMimeFold,
    ToggleOriginalDecoded,
    ShowHex,
    RefreshFolders,
    RefreshMessages,
    ShowSourceView,
    ShowMimeTreeView,
    EditColors,
    ResetColors,
    SetThemeDefault,
    SetThemeMidnight,
    SetThemeLight,
    Quit,
}

pub struct MenuState {
    pub open_bar: Option<MenuBarItem>,
    pub cursor: usize,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            open_bar: None,
            cursor: 0,
        }
    }
}

impl MenuState {
    pub fn items_for(bar: MenuBarItem) -> Vec<MenuItem> {
        match bar {
            MenuBarItem::Main => vec![
                item("Server", "s", MenuAction::Noop),
                item("Message", "m", MenuAction::Noop),
                item("View", "v", MenuAction::Noop),
                item("Colors", "c", MenuAction::Noop),
            ],
            MenuBarItem::Server => vec![
                item("Connect…", "F3", MenuAction::Connect),
                item("Disconnect", "F4", MenuAction::Disconnect),
                item("Save connection", "F5", MenuAction::SaveConnection),
                item("Load connection", "F6", MenuAction::LoadConnection),
                item("Refresh folders", "F7", MenuAction::RefreshFolders),
            ],
            MenuBarItem::Message => vec![
                item("Save message (RFC822)", "s", MenuAction::SaveMessage),
                item("Save focused part", "S", MenuAction::SaveRawPart),
                item("Download part", "d", MenuAction::DownloadPart),
                item("Toggle MIME fold", "Space", MenuAction::ToggleMimeFold),
                item("Original / decoded", "o", MenuAction::ToggleOriginalDecoded),
                item("Hex view (binary)", "x", MenuAction::ShowHex),
                item("Refresh list", "F7", MenuAction::RefreshMessages),
            ],
            MenuBarItem::View => vec![
                item("Source view", "1", MenuAction::ShowSourceView),
                item("MIME tree view", "2", MenuAction::ShowMimeTreeView),
                item("Hex view", "x", MenuAction::ShowHex),
            ],
            MenuBarItem::Colors => vec![
                item("Default theme", "1", MenuAction::SetThemeDefault),
                item("Midnight theme", "2", MenuAction::SetThemeMidnight),
                item("Light theme", "3", MenuAction::SetThemeLight),
                item("Reset to defaults", "r", MenuAction::ResetColors),
            ],
            MenuBarItem::UserFolders => vec![
                item("--- Folders Panel ---", "", MenuAction::Noop),
            ],
            MenuBarItem::UserMessages => vec![
                item("--- Messages Panel ---", "", MenuAction::Noop),
            ],
            MenuBarItem::UserContent => vec![
                item("--- Content Panel ---", "", MenuAction::Noop),
            ],
        }
    }

    pub fn open(&mut self, bar: MenuBarItem) {
        self.open_bar = Some(bar);
        self.cursor = 0;
    }

    pub fn close(&mut self) {
        self.open_bar = None;
    }

    pub fn move_up(&mut self, len: usize) {
        if len > 0 {
            self.cursor = if self.cursor > 0 {
                self.cursor - 1
            } else {
                len - 1
            };
        }
    }

    pub fn move_down(&mut self, len: usize) {
        if len > 0 {
            self.cursor = if self.cursor + 1 < len {
                self.cursor + 1
            } else {
                0
            };
        }
    }

    pub fn move_bar_left(&mut self) {
        if let Some(bar) = self.open_bar {
            let current_index = match bar {
                MenuBarItem::Server => 0,
                MenuBarItem::Message => 1,
                MenuBarItem::View => 2,
                MenuBarItem::Colors => 3,
                _ => 0,
            };
            let new_index = if current_index > 0 {
                current_index - 1
            } else {
                3
            };
            let new_bar = match new_index {
                0 => MenuBarItem::Server,
                1 => MenuBarItem::Message,
                2 => MenuBarItem::View,
                _ => MenuBarItem::Colors,
            };
            self.open(new_bar);
        }
    }

    pub fn move_bar_right(&mut self) {
        if let Some(bar) = self.open_bar {
            let current_index = match bar {
                MenuBarItem::Server => 0,
                MenuBarItem::Message => 1,
                MenuBarItem::View => 2,
                MenuBarItem::Colors => 3,
                _ => 3,
            };
            let new_index = if current_index < 3 {
                current_index + 1
            } else {
                0
            };
            let new_bar = match new_index {
                0 => MenuBarItem::Server,
                1 => MenuBarItem::Message,
                2 => MenuBarItem::View,
                _ => MenuBarItem::Colors,
            };
            self.open(new_bar);
        }
    }
}

fn item(label: &str, shortcut: &str, action: MenuAction) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        shortcut: Some(shortcut.to_string()),
        action,
    }
}

pub const MENU_BAR: &[(&str, MenuBarItem)] = &[
    ("Server", MenuBarItem::Server),
    ("Message", MenuBarItem::Message),
    ("View", MenuBarItem::View),
    ("Colors", MenuBarItem::Colors),
];
