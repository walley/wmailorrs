use crate::app::{ContentMode, FocusPanel};

pub struct KeyHint {
    pub key: &'static str,
    pub label: &'static str,
}

pub fn keybar_hints(focus: FocusPanel, connected: bool, content_mode: ContentMode) -> Vec<KeyHint> {
    let mut hints = vec![
        KeyHint {
            key: "F1",
            label: "Help",
        },
        KeyHint {
            key: "F2",
            label: "Menu",
        },
        KeyHint {
            key: "F10",
            label: "Quit",
        },
        KeyHint {
            key: "Tab",
            label: "Focus",
        },
    ];

    match focus {
        FocusPanel::Folders => {
            hints.push(KeyHint {
                key: "Enter",
                label: "Open folder",
            });
            if connected {
                hints.push(KeyHint {
                    key: "F7",
                    label: "Refresh",
                });
            }
        }
        FocusPanel::Messages => {
            hints.push(KeyHint {
                key: "Enter",
                label: "Fetch",
            });
            hints.push(KeyHint {
                key: "/",
                label: "Filter",
            });
            hints.push(KeyHint {
                key: "s",
                label: "Save msg",
            });
        }
        FocusPanel::Content => match content_mode {
            ContentMode::Source => {
                hints.push(KeyHint {
                    key: "2",
                    label: "MIME tree",
                });
            }
            ContentMode::MimeTree => {
                hints.push(KeyHint {
                    key: "Space",
                    label: "Fold",
                });
                hints.push(KeyHint {
                    key: "o",
                    label: "Orig/dec",
                });
                hints.push(KeyHint {
                    key: "d",
                    label: "Download",
                });
                hints.push(KeyHint {
                    key: "x",
                    label: "Hex",
                });
                hints.push(KeyHint {
                    key: "1",
                    label: "Source",
                });
            }
            ContentMode::Hex => {
                hints.push(KeyHint {
                    key: "Esc",
                    label: "Back",
                });
            }
        },
    }

    if !connected {
        hints.insert(
            2,
            KeyHint {
                key: "F3",
                label: "Connect",
            },
        );
    }

    hints
}

pub fn format_keybar(hints: &[KeyHint], width: usize) -> String {
    let mut parts = Vec::new();
    let mut used = 0usize;
    for h in hints {
        let segment = format!("{} {}", h.key, h.label);
        if used + segment.len() + 2 > width.saturating_sub(1) {
            break;
        }
        used += segment.len() + 2;
        parts.push(segment);
    }
    parts.join("  ")
}
