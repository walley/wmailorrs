mod app;
mod config;
mod imap;
mod mail;
mod ui;

use anyhow::Result;
use app::{App, ContentMode, Dialog, FocusPanel};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Stdout};
use std::time::Duration;
use ui::menu::{MenuAction, MenuBarItem, MenuState};

fn main() -> Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = App::new();
    let result = run_loop(&mut terminal, &mut app);
    restore_terminal()?;
    app.on_quit();
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(stdout(), crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    Ok(Terminal::new(backend)?)
}

fn restore_terminal() -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    loop {
        app.drain_imap();
        terminal.draw(|f| ui::draw(f, app))?;
        if app.should_quit {
            break;
        }
        if event::poll(Duration::from_millis(120))? {
            if let Event::Key(key) = event::read()? {
                handle_key(app, key);
            }
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    if key.code == KeyCode::F(10)
        || (key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL))
    {
        app.should_quit = true;
        return;
    }

    if app.menu.open_bar.is_some() {
        handle_menu_key(app, key);
        return;
    }

    if app.dialog != Dialog::None {
        handle_dialog_key(app, key);
        return;
    }

    match key.code {
        KeyCode::F(1) => app.dialog = Dialog::Help,
        KeyCode::F(2) => app.open_user_menu(),
        KeyCode::F(3) => app.execute_menu_action(MenuAction::Connect),
        KeyCode::F(4) => app.execute_menu_action(MenuAction::Disconnect),
        KeyCode::F(5) => app.execute_menu_action(MenuAction::SaveConnection),
        KeyCode::F(6) => app.execute_menu_action(MenuAction::LoadConnection),
        KeyCode::F(7) => app.execute_menu_action(MenuAction::RefreshFolders),
        KeyCode::F(8) => app.execute_menu_action(MenuAction::RefreshMessages),
        KeyCode::F(9) => app.menu.open(MenuBarItem::Server),
        KeyCode::Tab => app.cycle_focus(),
        KeyCode::BackTab => app.cycle_focus_back(),
        KeyCode::Up | KeyCode::Char('k') => {
            if app.is_image_expanded() {
                app.image_pan(0, -1);
            } else {
                app.move_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.is_image_expanded() {
                app.image_pan(0, 1);
            } else {
                app.move_down();
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if app.is_image_expanded() {
                app.image_pan(-1, 0);
            } else if app.focus == FocusPanel::Content {
                app.scroll_content_up(1);
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if app.is_image_expanded() {
                app.image_pan(1, 0);
            } else if app.focus == FocusPanel::Content {
                app.scroll_content_down(1);
            }
        }
        KeyCode::Home => app.home(),
        KeyCode::End => app.end(),
        KeyCode::PageUp => app.page_up(),
        KeyCode::PageDown => app.page_down(),
        KeyCode::Enter => {
            if app.focus == FocusPanel::Content && app.content_mode == ContentMode::MimeTree {
                app.mime_toggle_expand();
            } else {
                app.activate();
            }
        }
        KeyCode::Char('+') if app.focus == FocusPanel::Folders => app.open_folder(),
        KeyCode::Char('+') if app.content_mode == ContentMode::MimeTree
            && app.mime_expanded.iter().next()
                .and_then(|id| app.mime_tree.as_ref()?.node(*id))
                .map(|n| n.content_type.starts_with("image/"))
                .unwrap_or(false) => app.image_zoom_in(),
        KeyCode::Char('-') if app.content_mode == ContentMode::MimeTree
            && app.mime_expanded.iter().next()
                .and_then(|id| app.mime_tree.as_ref()?.node(*id))
                .map(|n| n.content_type.starts_with("image/"))
                .unwrap_or(false) => app.image_zoom_out(),
        KeyCode::Char(' ') if app.focus == FocusPanel::Content => app.page_down(),
        KeyCode::Char(' ') if app.focus == FocusPanel::Folders => app.open_folder(),
        KeyCode::Char('o') if app.content_mode == ContentMode::MimeTree || app.content_mode == ContentMode::Source => app.toggle_decoded(),
        KeyCode::Char('x') if app.content_mode == ContentMode::MimeTree => {
            let _ = app.show_hex_for_focused();
        }
        KeyCode::Char('d') if app.content_mode == ContentMode::MimeTree => {
            if let Ok(p) = app.download_focused_part() {
                app.status = format!("Saved {p}");
            }
        }
        KeyCode::Char('s') => {
            if let Ok(p) = app.save_current_message() {
                app.status = format!("Saved {p}");
            }
        }
        KeyCode::Char('1') => app.set_content_mode(ContentMode::Source),
        KeyCode::Char('2') => app.set_content_mode(ContentMode::MimeTree),
        KeyCode::Esc => {
            if app.content_mode == ContentMode::Hex {
                app.content_mode = ContentMode::MimeTree;
                app.content_scroll = 0;
                app.sync_mime_focus();
            }
        }
        KeyCode::Char('/') if app.focus == FocusPanel::Messages => {
            app.message_filter.clear();
        }
        KeyCode::Char(c) if app.focus == FocusPanel::Messages => {
            app.message_filter.push(c);
            app.message_cursor = 0;
            app.clamp_message_cursor();
        }
        KeyCode::Backspace if app.focus == FocusPanel::Messages => {
            app.message_filter.pop();
            app.clamp_message_cursor();
        }
        _ => {}
    }

    // Alt activates menu bar letters (Turbo Vision style)
    if key.modifiers.contains(KeyModifiers::ALT) {
        match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') => app.menu.open(MenuBarItem::Server),
            KeyCode::Char('m') | KeyCode::Char('M') => app.menu.open(MenuBarItem::Message),
            KeyCode::Char('v') | KeyCode::Char('V') => app.menu.open(MenuBarItem::View),
            KeyCode::Char('c') | KeyCode::Char('C') => app.menu.open(MenuBarItem::Colors),
            _ => {}
        }
    }
}

fn handle_menu_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }
    let bar = app.menu.open_bar.unwrap();
    let items = MenuState::items_for(bar);
    match key.code {
        KeyCode::Esc | KeyCode::F(2) => app.menu.close(),
        KeyCode::Up => app.menu.move_up(items.len()),
        KeyCode::Down => app.menu.move_down(items.len()),
        KeyCode::Left => {
            app.menu.move_bar_left();
        }
        KeyCode::Right => {
            app.menu.move_bar_right();
        }
        KeyCode::Enter => {
            if let Some(item) = items.get(app.menu.cursor) {
                let action = item.action;
                app.menu.close();
                app.execute_menu_action(action);
            }
        }
        _ => {}
    }
}

fn handle_dialog_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }
    match app.dialog {
        Dialog::Connect => match key.code {
            KeyCode::Esc => app.dialog = Dialog::None,
            KeyCode::Tab | KeyCode::Down => {
                app.connect_form.field = (app.connect_form.field + 1) % 6;
            }
            KeyCode::Up => {
                app.connect_form.field = app.connect_form.field.saturating_sub(1);
            }
            KeyCode::Char(' ') if app.connect_form.field == 5 => {
                app.connect_form.tls = !app.connect_form.tls;
            }
            KeyCode::Enter => app.do_connect(),
            KeyCode::F(5)
            | KeyCode::Char('s')
                if key.modifiers.intersects(KeyModifiers::CONTROL) =>
            {
                if let Err(e) = app.save_connect_form() {
                    app.status = format!("Save failed: {e}");
                } else {
                    app.status = "Connection saved".into();
                    app.saved_connections = config::list_connections().unwrap_or_default();
                }
            }
            KeyCode::Char(c) if app.connect_form.field != 5 => {
                let field = &mut app.connect_form;
                match field.field {
                    0 => field.name.push(c),
                    1 => field.host.push(c),
                    2 if c.is_ascii_digit() => field.port.push(c),
                    3 => field.user.push(c),
                    4 => field.password.push(c),
                    _ => {}
                }
            }
            KeyCode::Backspace => {
                let field = &mut app.connect_form;
                match field.field {
                    0 => {
                        field.name.pop();
                    }
                    1 => {
                        field.host.pop();
                    }
                    2 => {
                        field.port.pop();
                    }
                    3 => {
                        field.user.pop();
                    }
                    4 => {
                        field.password.pop();
                    }
                    _ => {}
                }
            }
            _ => {}
        },
        Dialog::LoadConnection => match key.code {
            KeyCode::Esc => app.dialog = Dialog::None,
            KeyCode::Up => {
                app.connect_form.field = app.connect_form.field.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = app.saved_connections.len().saturating_sub(1);
                app.connect_form.field = (app.connect_form.field + 1).min(max);
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let idx = (c as u8 - b'1') as usize;
                app.load_connection_at(idx);
            }
            KeyCode::Enter => app.load_connection_at(app.connect_form.field),
            _ => {}
        },
        Dialog::Help => {
            if matches!(key.code, KeyCode::Esc | KeyCode::F(1) | KeyCode::Enter) {
                app.dialog = Dialog::None;
            }
        }
        Dialog::MessageBox(_, _) => {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('o') | KeyCode::Char('O')) {
                app.dialog = Dialog::None;
            }
        }
        Dialog::None | Dialog::Status => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f10_quits_app() {
        let mut app = App::new();
        assert!(!app.should_quit);

        let key = KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE);
        handle_key(&mut app, key);
        assert!(app.should_quit);
    }

    #[test]
    fn test_ctrl_q_quits_app() {
        let mut app = App::new();
        assert!(!app.should_quit);

        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        handle_key(&mut app, key);
        assert!(app.should_quit);
    }

    #[test]
    fn test_f10_quits_when_dialog_or_menu_open() {
        let mut app = App::new();
        app.dialog = Dialog::Help;
        let key = KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE);
        handle_key(&mut app, key);
        assert!(app.should_quit);

        let mut app = App::new();
        app.menu.open(MenuBarItem::Server);
        let key = KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE);
        handle_key(&mut app, key);
        assert!(app.should_quit);
    }

    fn mime_tree_with_expanded_body(app: &mut App) {
        let raw = concat!(
            "From: a@b.com\r\n",
            "To: c@d.com\r\n",
            "Subject: t\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: multipart/mixed; boundary=\"b\"\r\n",
            "\r\n",
            "--b\r\n",
            "Content-Type: text/plain\r\n",
            "\r\n",
            "line one\r\n",
            "--b\r\n",
            "Content-Type: text/plain\r\n",
            "\r\n",
            "four\r\n",
            "five\r\n",
            "six\r\n",
            "--b--\r\n",
        );
        let tree = crate::mail::MimeTree::from_raw(raw).unwrap();
        let body_node = tree
            .nodes
            .iter()
            .flat_map(|n| n.children.iter().map(|c| c.id))
            .find(|id| {
                tree.node(*id)
                    .map(|n| n.decoded_body.iter().filter(|&&b| b == b'\n').count() > 1)
                    .unwrap_or(false)
            })
            .unwrap();
        app.focus = FocusPanel::Content;
        app.content_mode = ContentMode::MimeTree;
        app.mime_tree = Some(tree);
        app.mime_expanded.insert(body_node);
        app.mime_show_decoded.insert(body_node);
        app.mime_focused_node = Some(body_node);
        app.mime_cursor = 0;
        app.content_panel_height = 20;
        app.sync_mime_focus();
    }

    #[test]
    fn test_content_scrolls_in_mime_tree() {
        let mut app = App::new();
        mime_tree_with_expanded_body(&mut app);
        assert_eq!(app.content_scroll, 0);

        let pgdn = KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE);
        handle_key(&mut app, pgdn);
        let max_scroll = app.content_line_count().saturating_sub(1) as u16;
        assert!(app.content_scroll > 0);
        assert!(app.content_scroll <= max_scroll);
        let scrolled = app.content_scroll;

        let pgup = KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE);
        handle_key(&mut app, pgup);
        assert!(app.content_scroll < scrolled);
    }

    #[test]
    fn test_space_scrolls_content() {
        let mut app = App::new();
        mime_tree_with_expanded_body(&mut app);
        assert_eq!(app.content_scroll, 0);

        let space = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        handle_key(&mut app, space);
        let max_scroll = app.content_line_count().saturating_sub(1) as u16;
        assert!(app.content_scroll > 0);
        assert!(app.content_scroll <= max_scroll);
    }

    #[test]
    fn test_left_right_scroll_content_one_line() {
        let mut app = App::new();
        mime_tree_with_expanded_body(&mut app);
        assert_eq!(app.content_scroll, 0);

        let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        handle_key(&mut app, right);
        assert_eq!(app.content_scroll, 1);

        let left = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        handle_key(&mut app, left);
        assert_eq!(app.content_scroll, 0);
    }

    #[test]
    fn test_f3_opens_connect_dialog() {
        let mut app = App::new();
        let key = KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE);
        handle_key(&mut app, key);
        assert_eq!(app.dialog, Dialog::Connect);
    }

    #[test]
    fn test_f6_opens_load_connection_dialog() {
        let mut app = App::new();
        let key = KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE);
        handle_key(&mut app, key);
        assert_eq!(app.dialog, Dialog::LoadConnection);
    }

    #[test]
    fn test_f4_disconnect_works_when_disconnected() {
        let mut app = App::new();
        assert!(!app.connected);
        let key = KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE);
        handle_key(&mut app, key);
        assert!(!app.connected);
    }

    #[test]
    fn test_f7_f8_refresh_no_panic() {
        let mut app = App::new();
        let f7 = KeyEvent::new(KeyCode::F(7), KeyModifiers::NONE);
        let f8 = KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE);
        handle_key(&mut app, f7);
        handle_key(&mut app, f8);
        assert!(true);
    }

    #[test]
    fn test_tab_cycles_focus_in_mime_tree() {
        let mut app = App::new();
        mime_tree_with_expanded_body(&mut app);
        assert_eq!(app.focus, FocusPanel::Content);

        let tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        handle_key(&mut app, tab);
        assert_eq!(app.focus, FocusPanel::Folders);

        handle_key(&mut app, tab);
        assert_eq!(app.focus, FocusPanel::Messages);

        handle_key(&mut app, tab);
        assert_eq!(app.focus, FocusPanel::Content);

        let backtab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE);
        handle_key(&mut app, backtab);
        assert_eq!(app.focus, FocusPanel::Messages);
    }

    #[test]
    fn test_home_end_scroll_content() {
        let mut app = App::new();
        mime_tree_with_expanded_body(&mut app);
        assert_eq!(app.content_scroll, 0);

        let end = KeyEvent::new(KeyCode::End, KeyModifiers::NONE);
        handle_key(&mut app, end);
        let max_scroll = app.content_line_count().saturating_sub(1) as u16;
        assert_eq!(app.content_scroll, max_scroll);

        let home = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
        handle_key(&mut app, home);
        assert_eq!(app.content_scroll, 0);
    }
}

