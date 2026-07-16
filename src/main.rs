mod app;
mod config;
mod imap;
mod mail;
mod theme;
mod ui;

use anyhow::Result;
use app::{App, ContentMode, Dialog, FocusPanel};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Stdout};
use std::time::Duration;
use ui::menu::{MenuBarItem, MenuState};

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
        KeyCode::F(3) => app.dialog = Dialog::Connect,
        KeyCode::F(4) if app.connected => app.disconnect(),
        KeyCode::F(9) => app.menu.open(MenuBarItem::Server),
        KeyCode::F(10) | KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Tab => app.cycle_focus(),
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::PageUp => match app.focus {
            FocusPanel::Folders | FocusPanel::Messages => app.page_up(),
            FocusPanel::Content => {
                app.content_scroll = app.content_scroll.saturating_sub(10);
                app.sync_mime_focus();
            }
        },
        KeyCode::PageDown => match app.focus {
            FocusPanel::Folders | FocusPanel::Messages => app.page_down(),
            FocusPanel::Content => {
                let max = app.content_line_count().saturating_sub(1) as u16;
                app.content_scroll = (app.content_scroll + 10).min(max);
                app.sync_mime_focus();
            }
        },
        KeyCode::Enter => app.activate(),
        KeyCode::Char('+') if app.focus == FocusPanel::Folders => app.open_folder(),
        KeyCode::Char(' ') if app.focus == FocusPanel::Content => app.toggle_mime_fold(),
        KeyCode::Char(' ') if app.focus == FocusPanel::Folders => app.open_folder(),
        KeyCode::Char('o') => app.toggle_decoded(),
        KeyCode::Char('x') => {
            let _ = app.show_hex_for_focused();
        }
        KeyCode::Char('d') => {
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
        Dialog::None | Dialog::Status => {}
    }
}
