use crate::app::{App, ContentMode, Dialog, FocusPanel};
use crate::mail::{hex_lines, highlight_raw_source, image_to_lines_fitted};
use crate::ui::keybar::{format_keybar, keybar_hints};
use crate::ui::menu::{MenuBarItem, MenuState, MENU_BAR};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Scrollbar,
    ScrollbarOrientation, Wrap,
};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_menu_bar(f, chunks[0], app);
    draw_panels(f, chunks[1], app);
    draw_keybar(f, chunks[2], app);

    if app.menu.open_bar.is_some() {
        draw_dropdown_menu(f, app);
    }
    draw_dialog(f, app);
}

fn draw_menu_bar(f: &mut Frame, area: Rect, app: &App) {
    let mut spans = Vec::new();
    let menu_open = app.menu.open_bar.is_some();
    for (i, (label, item)) in MENU_BAR.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", app.theme.menu_style()));
        }
        let style = if menu_open && app.menu.open_bar == Some(*item) {
            // Selected main menu item: white on black
            app.theme.menu_selected_style()
        } else if menu_open {
            // Menu is open but this is not the selected item: white on cyan
            app.theme.menu_active_style()
        } else {
            // Menu is not open: black on cyan
            app.theme.menu_style()
        };
        spans.push(Span::styled(format!(" {label} "), style));
    }
    let remaining = area.width as usize;
    let used: usize = spans.iter().map(|s| s.width()).sum();
    if remaining > used {
        spans.push(Span::styled(
            " ".repeat(remaining - used),
            app.theme.menu_style(),
        ));
    }
    let p = Paragraph::new(Line::from(spans));
    f.render_widget(p, area);
}

fn panel_block<'a>(title: &'a str, focused: bool, app: &App) -> Block<'a> {
    if focused {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(app.theme.panel_focus_border_style())
            .title(Span::styled(
                format!(" ► {title} "),
                app.theme.panel_focus_title_style(),
            ))
    } else {
        Block::default()
            .borders(Borders::ALL)
            .border_style(app.theme.panel_border_style())
            .title(Span::styled(
                format!(" {title} "),
                app.theme.panel_title_style(),
            ))
    }
}

fn draw_panels(f: &mut Frame, area: Rect, app: &mut App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(28), Constraint::Percentage(72)])
        .split(area);

    draw_folders(f, cols[0], app);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(cols[1]);
    draw_messages(f, right[0], app);
    draw_content(f, right[1], app);
}

fn draw_folders(f: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.focus == FocusPanel::Folders;
    app.clamp_folder_cursor();
    let block = panel_block("Folders", focused, app);
    let inner = block.inner(area);
    f.render_widget(block, area);

    app.folder_panel_height = inner.height;

    let mut display_folders = app.display_folders();
    if !app.current_folder_path.is_empty() {
        display_folders.insert(0, "..".to_string());
    }

    let items: Vec<ListItem> = display_folders
        .iter()
        .map(|name| {
            let display_name = if name == ".." {
                "..".to_string()
            } else if app.has_subfolders(name) {
                format!("[{}]", name)
            } else {
                name.to_string()
            };
            ListItem::new(display_name)
        })
        .collect();
    let list = List::new(items)
        .highlight_style(app.theme.selection_style())
        .highlight_symbol("▸ ");
    f.render_stateful_widget(list, inner, &mut app.folder_list_state);

    let folder_count = display_folders.len();
    if folder_count > inner.height as usize {
        let mut sb_state = ratatui::widgets::ScrollbarState::default()
            .content_length(folder_count)
            .position(app.folder_cursor);
        let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        f.render_stateful_widget(sb, inner, &mut sb_state);
    }
}

fn draw_messages(f: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.focus == FocusPanel::Messages;
    app.clamp_message_cursor();
    let filtered = app.filtered_messages();
    let title = if app.message_filter.is_empty() {
        "Messages".to_string()
    } else {
        format!("Messages /{}", app.message_filter)
    };
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|m| {
            ListItem::new(format!("{:5} {:6} {}", m.uid, m.size, m.summary))
        })
        .collect();
    let list = List::new(items)
        .block(panel_block(&title, focused, app))
        .highlight_style(app.theme.selection_style())
        .highlight_symbol("▸ ");
    f.render_stateful_widget(list, area, &mut app.message_list_state);
}

fn draw_content(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == FocusPanel::Content;
    let title = match app.content_mode {
        ContentMode::Source => "Source (RFC822)",
        ContentMode::MimeTree => {
            if let Some(id) = app.mime_focused_node {
                if app.mime_show_decoded.contains(&id) {
                    "MIME tree (decoded)"
                } else {
                    "MIME tree (original)"
                }
            } else {
                "MIME tree (original)"
            }
        }
        ContentMode::Hex => "Hex view",
    };

    let conn = app.connection_name.as_deref().unwrap_or("offline");
    let status_text = format!("{conn}: {}", app.status);

    let mut block = panel_block(title, focused, app);
    block = block.title_bottom(Span::styled(
        format!(" {status_text} "),
        Style::default().fg(app.theme.status_ok.to_color()),
    ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let line_count = lines_len_hint(app);
    let lines: Vec<Line> = match app.content_mode {
        ContentMode::Source => app
            .current_raw
            .as_ref()
            .map(|r| highlight_raw_source(r, &app.theme))
            .unwrap_or_else(|| vec![Line::from("(no message)")]),
        ContentMode::MimeTree => {
            let mut out = Vec::new();
            let expanded_is_image = app
                .mime_expanded
                .iter()
                .next()
                .and_then(|id| app.mime_tree.as_ref()?.node(*id))
                .map(|n| n.content_type.starts_with("image/"))
                .unwrap_or(false);

            if expanded_is_image {
                if let Some(id) = app.mime_expanded.iter().next() {
                    if let Some(tree) = &app.mime_tree {
                        if let Some(node) = tree.node(*id) {
                            let data = if !node.decoded_body.is_empty() {
                                &node.decoded_body
                            } else {
                                &node.raw_body
                            };
                            if !data.is_empty() {
                                let label = format!(
                                    "[part {}] {}",
                                    node.id,
                                    node.filename
                                        .as_deref()
                                        .unwrap_or(&node.content_type)
                                );
                                out.push(Line::from(Span::styled(
                                    label,
                                    app.theme.mime_boundary_style(),
                                )));
                                let img_w = inner.width.saturating_sub(2) as u32;
                                let img_h = inner.height.saturating_sub(2) as u32;
                                let img_lines = image_to_lines_fitted(data, img_w, img_h);
                                out.extend(img_lines);
                            }
                        }
                    }
                }
            } else {
                for (text, kind, node_id) in app.mime_lines_for_display().into_iter() {
                    let mut style = match kind {
                        crate::mail::VisibleLineKind::Summary => {
                            app.theme.mime_boundary_style()
                        }
                        crate::mail::VisibleLineKind::HeaderBlock => {
                            app.theme.header_line_style(&text)
                        }
                        crate::mail::VisibleLineKind::BinaryHint => {
                            app.theme.mime_folded_style()
                        }
                        _ => app.theme.body_style(),
                    };
                    if kind == crate::mail::VisibleLineKind::Summary
                        && node_id == app.mime_focused_node
                    {
                        style = app.theme.selection_style();
                    }
                    out.push(Line::from(Span::styled(text, style)));
                }
            }
            if out.is_empty() {
                out.push(Line::from("(fetch a message first)"));
            }
            out
        }
        ContentMode::Hex => {
            let data = app.hex_data.as_deref().unwrap_or(&[]);
            hex_lines(
                data,
                app.theme.hex_address_style(),
                app.theme.hex_bytes_style(),
                app.theme.hex_ascii_style(),
            )
        }
    };

    let scroll = app.content_scroll as usize;
    let visible: Vec<Line> = lines.into_iter().skip(scroll).collect();
    let p = Paragraph::new(visible).wrap(Wrap { trim: false });
    f.render_widget(p, inner);

    if line_count > inner.height as usize {
        let mut sb_state = ratatui::widgets::ScrollbarState::default()
            .content_length(line_count)
            .position(scroll);
        let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        f.render_stateful_widget(
            sb,
            inner,
            &mut sb_state,
        );
    }
}

fn lines_len_hint(app: &App) -> usize {
    match app.content_mode {
        ContentMode::Source => app.current_raw.as_ref().map(|r| r.lines().count()).unwrap_or(0),
        ContentMode::MimeTree => app.mime_lines_for_display().len(),
        ContentMode::Hex => app
            .hex_data
            .as_ref()
            .map(|d| (d.len() + 15) / 16)
            .unwrap_or(0),
    }
}

fn draw_keybar(f: &mut Frame, area: Rect, app: &App) {
    let hints = keybar_hints(app.focus, app.connected, app.content_mode);
    let text = format_keybar(&hints, area.width as usize);
    let p = Paragraph::new(text).style(app.theme.keybar_style());
    f.render_widget(p, area);
}

fn draw_dropdown_menu(f: &mut Frame, app: &App) {
    let bar = app.menu.open_bar.unwrap();
    let items = MenuState::items_for(bar);
    let w = 36u16;
    let h = (items.len() as u16 + 2).min(12);
    let x = match bar {
        MenuBarItem::Server | MenuBarItem::UserFolders => 1,
        MenuBarItem::Message | MenuBarItem::UserMessages => 10,
        MenuBarItem::View | MenuBarItem::UserContent => 22,
        MenuBarItem::Colors => 30,
        MenuBarItem::Main => 1,
    };
    let area = Rect {
        x,
        y: 1,
        width: w,
        height: h,
    };
    f.render_widget(Clear, area);
    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let is_selected = i == app.menu.cursor;
            let label_style = if is_selected {
                app.theme.menu_selected_style()
            } else {
                app.theme.menu_style()
            };
            let short = it.shortcut.as_deref().unwrap_or("");
            let short_style = if is_selected {
                app.theme.menu_shortcut_style()
            } else {
                app.theme.menu_style()
            };
            Line::from(vec![
                Span::styled(format!(" {:<24}", it.label), label_style),
                Span::styled(format!("{short:>6} ", short = short), short_style),
            ])
        })
        .collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(app.theme.menu_border_style())
        .style(app.theme.menu_style());
    let p = Paragraph::new(lines).block(block);
    f.render_widget(p, area);
}

fn draw_dialog(f: &mut Frame, app: &App) {
    match app.dialog {
        Dialog::None => {}
        Dialog::Connect => draw_connect_dialog(f, app),
        Dialog::LoadConnection => draw_load_dialog(f, app),
        Dialog::Help => draw_help_dialog(f),
        Dialog::Status => {}
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_connect_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, f.area());
    f.render_widget(Clear, area);
    let form = &app.connect_form;
    let password_display = if form.password.is_empty() {
        String::new()
    } else {
        "********".to_string()
    };
    let tls_display = if form.tls { "yes" } else { "no" }.to_string();
    let fields: [(&str, &str); 6] = [
        ("Name", &form.name),
        ("Host", &form.host),
        ("Port", &form.port),
        ("User", &form.user),
        ("Password", &password_display),
        ("TLS", &tls_display),
    ];
    let mut lines = vec![Line::from(" Connection profile ")];
    for (i, (label, val)) in fields.iter().enumerate() {
        let mark = if i == form.field { "▶" } else { " " };
        lines.push(Line::from(format!("{mark} {label}: {val}")));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(" Enter=connect  F5/Ctrl+S=save  Esc=cancel "));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(" Connect ");
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_load_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 40, f.area());
    f.render_widget(Clear, area);
    let items: Vec<Line> = app
        .saved_connections
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let mark = if i == app.connect_form.field { "▶" } else { " " };
            Line::from(format!("{mark} {}. {}", i + 1, n))
        })
        .collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(" Load connection ");
    f.render_widget(Paragraph::new(items).block(block), area);
}

fn draw_help_dialog(f: &mut Frame) {
    let area = centered_rect(70, 60, f.area());
    f.render_widget(Clear, area);
    let text = "\
wmailor — admin IMAP client (raw source only)\n\
\n\
Tab       cycle panels\n\
Enter     open folder / fetch message\n\
F2        menu   F3 connect   F10 quit\n\
Space     toggle MIME fold (MIME view)\n\
o         original/decoded   x hex   d download\n\
1/2       source / MIME tree views\n\
";
    f.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(" Help "),
        ),
        area,
    );
}
