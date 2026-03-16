mod app;
mod browser;
mod dropdown;
mod input;
mod theme;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, AppMode};

#[tokio::main]
async fn main() -> io::Result<()> {
    let initial_torrent = std::env::args().nth(1).map(PathBuf::from);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    if let Some(path) = initial_torrent {
        app.load_torrent(path);
    }

    app.add_log("RatioMaster-Rust TUI started. Press 'o' to open a torrent, '?' for help.".into());

    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("error: {e}");
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        app.poll_engine_states();
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(app, key);
            }
        }

        if app.should_quit {
            for tab in &app.tabs {
                if let Some(ref handles) = tab.handles {
                    let _ = handles.shutdown_tx.send(true);
                }
            }
            break;
        }
    }

    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) {
    // Handle minimized mode
    if app.minimized {
        app.minimized = false;
        return;
    }

    match app.mode {
        AppMode::Normal | AppMode::Editing => handle_normal_key(app, key),
        AppMode::FileBrowser => handle_browser_key(app, key),
        AppMode::LogFilter => handle_filter_key(app, key),
        AppMode::DropdownOpen => handle_dropdown_key(app, key),
        AppMode::HelpPopup => handle_help_key(app, key),
        AppMode::QuitConfirm => handle_quit_confirm_key(app, key),
        AppMode::TabRename => handle_tab_rename_key(app, key),
    }
}

fn handle_normal_key(app: &mut App, key: KeyEvent) {
    // If a text field is focused and we're in editing mode, route chars there
    if app.mode == AppMode::Editing && app.is_text_field_focused() {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                app.mode = AppMode::Normal;
            }
            (_, KeyCode::Tab) => {
                app.mode = AppMode::Normal;
                app.focus_next();
                // If new focus is a text field, go back to editing
                if app.is_text_field_focused() {
                    app.mode = AppMode::Editing;
                }
            }
            (_, KeyCode::BackTab) => {
                app.mode = AppMode::Normal;
                app.focus_prev();
                if app.is_text_field_focused() {
                    app.mode = AppMode::Editing;
                }
            }
            (_, KeyCode::Enter) => {
                app.mode = AppMode::Normal;
            }
            (_, KeyCode::Char(c)) => {
                if let Some(input) = app.focused_text_input() {
                    input.insert(c);
                }
            }
            (_, KeyCode::Backspace) => {
                if let Some(input) = app.focused_text_input() {
                    input.backspace();
                }
            }
            (_, KeyCode::Delete) => {
                if let Some(input) = app.focused_text_input() {
                    input.delete();
                }
            }
            (_, KeyCode::Left) => {
                if let Some(input) = app.focused_text_input() {
                    input.move_left();
                }
            }
            (_, KeyCode::Right) => {
                if let Some(input) = app.focused_text_input() {
                    input.move_right();
                }
            }
            (_, KeyCode::Home) => {
                if let Some(input) = app.focused_text_input() {
                    input.home();
                }
            }
            (_, KeyCode::End) => {
                if let Some(input) = app.focused_text_input() {
                    input.end();
                }
            }
            _ => {}
        }
        return;
    }

    // Normal mode with optional field focus
    match (key.modifiers, key.code) {
        // -- Global keys --

        // Help popup
        (_, KeyCode::Char('?')) => {
            app.mode = AppMode::HelpPopup;
        }

        // Quit
        (_, KeyCode::Char('q')) if app.focused_field.is_none() => {
            if app.any_engine_running() {
                app.mode = AppMode::QuitConfirm;
            } else {
                app.should_quit = true;
            }
        }

        // Escape: unfocus field or do nothing
        (_, KeyCode::Esc) => {
            app.unfocus();
        }

        // Tab: cycle focus forward
        (_, KeyCode::Tab) if key.modifiers == KeyModifiers::NONE => {
            app.focus_next();
            if app.is_text_field_focused() {
                app.mode = AppMode::Editing;
            }
        }

        // Shift+Tab: cycle focus backward
        (_, KeyCode::BackTab) => {
            app.focus_prev();
            if app.is_text_field_focused() {
                app.mode = AppMode::Editing;
            }
        }

        // Space: toggle checkbox when focused
        (_, KeyCode::Char(' ')) if app.is_checkbox_focused() => {
            app.toggle_focused_checkbox();
        }

        // Enter on dropdown field: open it
        (_, KeyCode::Enter) if app.is_dropdown_focused() => {
            app.open_focused_dropdown();
        }

        // Enter on text field: start editing
        (_, KeyCode::Enter) if app.is_text_field_focused() => {
            app.mode = AppMode::Editing;
        }

        // Enter with no focus: start/stop engine
        (_, KeyCode::Enter) if app.focused_field.is_none() => {
            if let Some(tab) = app.tabs.get(app.active_tab) {
                if matches!(tab.status, app::TabStatus::Running) {
                    app.stop_engine();
                } else {
                    app.start_engine();
                }
            }
        }

        // -- Ctrl combos (always available) --

        // Ctrl+N: new tab
        (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
            app.add_empty_tab();
        }

        // Ctrl+W: close tab
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
            app.close_tab();
        }

        // Ctrl+Left/Right: switch tabs
        (KeyModifiers::CONTROL, KeyCode::Left) => {
            app.prev_tab();
        }
        (KeyModifiers::CONTROL, KeyCode::Right) => {
            app.next_tab();
        }

        // Alt+Left/Right: reorder tabs
        (KeyModifiers::ALT, KeyCode::Left) => {
            app.move_tab_left();
        }
        (KeyModifiers::ALT, KeyCode::Right) => {
            app.move_tab_right();
        }

        // Ctrl+L: clear log
        (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
            app.clear_log();
        }

        // Ctrl+S: save log
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
            app.save_log();
            app.add_log("Log saved".into());
        }

        // Ctrl+Z: minimize
        (KeyModifiers::CONTROL, KeyCode::Char('z')) => {
            app.minimized = true;
        }

        // -- Keys only active when no field is focused --

        // Force announce
        (_, KeyCode::Char('u')) if app.focused_field.is_none() => {
            app.force_announce();
            app.add_log("Manual announce triggered".into());
        }

        // Open file browser
        (_, KeyCode::Char('o')) if app.focused_field.is_none() => {
            app.file_browser.refresh();
            app.mode = AppMode::FileBrowser;
        }

        // Log filter
        (_, KeyCode::Char('/')) if app.focused_field.is_none() => {
            app.log_filter.clear();
            app.mode = AppMode::LogFilter;
        }

        // Toggle scrape
        (_, KeyCode::Char('s')) if app.focused_field.is_none() => {
            let msg = if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                tab.scrape = !tab.scrape;
                Some(format!(
                    "Scrape {}",
                    if tab.scrape { "enabled" } else { "disabled" }
                ))
            } else {
                None
            };
            if let Some(msg) = msg {
                app.add_log(msg);
            }
        }

        // F2: rename tab
        (_, KeyCode::F(2)) => {
            if let Some(tab) = app.tabs.get(app.active_tab) {
                app.tab_rename_input = input::TextInput::new(tab.name.clone());
                app.mode = AppMode::TabRename;
            }
        }

        // F5: redraw (just return, loop redraws)
        (_, KeyCode::F(5)) => {}

        // Log scroll
        (_, KeyCode::PageUp) if app.focused_field.is_none() => {
            app.log_scroll = app.log_scroll.saturating_sub(10);
        }
        (_, KeyCode::PageDown) if app.focused_field.is_none() => {
            let max = app.filtered_log_len();
            app.log_scroll = (app.log_scroll + 10).min(max.saturating_sub(1));
        }
        (_, KeyCode::Up) if app.focused_field.is_none() => {
            app.log_scroll = app.log_scroll.saturating_sub(1);
        }
        (_, KeyCode::Down) if app.focused_field.is_none() => {
            let max = app.filtered_log_len();
            app.log_scroll = (app.log_scroll + 1).min(max.saturating_sub(1));
        }

        _ => {}
    }
}

fn handle_dropdown_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.dropdown_cancel(),
        KeyCode::Enter => app.dropdown_confirm(),
        KeyCode::Up => app.dropdown_up(),
        KeyCode::Down => app.dropdown_down(),
        _ => {}
    }
}

fn handle_browser_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Up => {
            app.file_browser.up();
        }
        KeyCode::Down => {
            app.file_browser.down();
        }
        KeyCode::Enter => {
            if let Some(path) = app.file_browser.select() {
                app.load_torrent(path);
                app.mode = AppMode::Normal;
            }
        }
        _ => {}
    }
}

fn handle_filter_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Char(c) => {
            app.log_filter.push(c);
            app.log_scroll = 0;
        }
        KeyCode::Backspace => {
            app.log_filter.pop();
            app.log_scroll = 0;
        }
        _ => {}
    }
}

fn handle_help_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_quit_confirm_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.should_quit = true;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_tab_rename_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Enter => {
            let new_name = app.tab_rename_input.value.clone();
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                if !new_name.is_empty() {
                    tab.name = new_name;
                }
            }
            app.mode = AppMode::Normal;
        }
        KeyCode::Char(c) => {
            app.tab_rename_input.insert(c);
        }
        KeyCode::Backspace => {
            app.tab_rename_input.backspace();
        }
        KeyCode::Delete => {
            app.tab_rename_input.delete();
        }
        KeyCode::Left => {
            app.tab_rename_input.move_left();
        }
        KeyCode::Right => {
            app.tab_rename_input.move_right();
        }
        KeyCode::Home => {
            app.tab_rename_input.home();
        }
        KeyCode::End => {
            app.tab_rename_input.end();
        }
        _ => {}
    }
}
