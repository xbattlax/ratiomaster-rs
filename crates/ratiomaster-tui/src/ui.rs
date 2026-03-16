/// TUI rendering with ratatui — fully interactive layout.
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Tabs, Wrap,
};
use ratatui::Frame;

use crate::app::{format_bytes, format_duration, App, AppMode, FocusableField, TabStatus};
use crate::theme;

/// Draws the entire application UI.
pub fn draw(f: &mut Frame, app: &App) {
    if app.minimized {
        draw_minimized(f, app);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title + tabs
            Constraint::Length(5), // Torrent info
            Constraint::Length(4), // Client & Network
            Constraint::Length(5), // Speed settings
            Constraint::Length(3), // Stop condition
            Constraint::Length(4), // Controls + progress
            Constraint::Min(5),    // Log
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    draw_title_tabs(f, app, chunks[0]);
    draw_torrent_info(f, app, chunks[1]);
    draw_client_network(f, app, chunks[2]);
    draw_speed_settings(f, app, chunks[3]);
    draw_stop_condition(f, app, chunks[4]);
    draw_controls(f, app, chunks[5]);
    draw_log(f, app, chunks[6]);
    draw_status_bar(f, app, chunks[7]);

    // Overlays (drawn last, on top)
    if app.mode == AppMode::FileBrowser {
        draw_file_browser(f, app);
    }
    if app.mode == AppMode::DropdownOpen {
        draw_dropdown_overlay(f, app);
    }
    if app.mode == AppMode::HelpPopup {
        draw_help_popup(f);
    }
    if app.mode == AppMode::QuitConfirm {
        draw_quit_confirm(f);
    }
    if app.mode == AppMode::TabRename {
        draw_tab_rename(f, app);
    }
}

// -- Helper: render a text input field with optional cursor --

fn render_field(
    value: &str,
    cursor: Option<usize>,
    width: usize,
    focused: bool,
) -> Vec<Span<'static>> {
    let style = if focused {
        theme::field_focused()
    } else {
        theme::field_normal()
    };

    if focused {
        if let Some(cur) = cursor {
            let (before, after) = value.split_at(cur.min(value.len()));
            let cursor_char = after.chars().next().unwrap_or(' ');
            let rest = if after.len() > cursor_char.len_utf8() {
                &after[cursor_char.len_utf8()..]
            } else {
                ""
            };
            let pad = width
                .saturating_sub(value.len())
                .max(if after.is_empty() { 1 } else { 0 });
            vec![
                Span::styled("[", style),
                Span::styled(before.to_string(), style),
                Span::styled(
                    cursor_char.to_string(),
                    Style::default()
                        .fg(theme::BG)
                        .bg(theme::FOCUSED)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(rest.to_string(), style),
                Span::styled(" ".repeat(pad), style),
                Span::styled("]", style),
            ]
        } else {
            let pad = width.saturating_sub(value.len());
            vec![
                Span::styled("[", style),
                Span::styled(value.to_string(), style),
                Span::styled(" ".repeat(pad), style),
                Span::styled("]", style),
            ]
        }
    } else {
        let pad = width.saturating_sub(value.len());
        vec![
            Span::styled("[".to_string(), style),
            Span::styled(value.to_string(), style),
            Span::styled(" ".repeat(pad), style),
            Span::styled("]".to_string(), style),
        ]
    }
}

fn render_checkbox(label: &str, checked: bool, focused: bool) -> Vec<Span<'static>> {
    let mark = if checked { "x" } else { " " };
    let style = if focused {
        theme::field_focused()
    } else {
        theme::checkbox(checked)
    };
    vec![
        Span::styled(format!("[{mark}]"), style),
        Span::raw(" "),
        Span::styled(
            label.to_string(),
            if focused {
                theme::field_focused()
            } else {
                theme::label()
            },
        ),
    ]
}

fn render_dropdown_field(value: &str, width: usize, focused: bool) -> Vec<Span<'static>> {
    let style = if focused {
        theme::field_focused()
    } else {
        theme::field_normal()
    };
    let pad = width.saturating_sub(value.len() + 2); // 2 for "▾ "
    vec![
        Span::styled("[", style),
        Span::styled("▾ ".to_string(), style),
        Span::styled(value.to_string(), style),
        Span::styled(" ".repeat(pad), style),
        Span::styled("]", style),
    ]
}

fn is_focused(app: &App, field: FocusableField) -> bool {
    app.focused_field == Some(field)
}

// -- Section renderers --

fn draw_title_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let name = if tab.name.len() > 20 {
                format!("{}...", &tab.name[..17])
            } else {
                tab.name.clone()
            };
            let status_indicator = match tab.status {
                TabStatus::Running => " *",
                TabStatus::Error(_) => " !",
                _ => "",
            };
            let label = format!("{name}{status_indicator}");
            if i == app.active_tab {
                Line::from(Span::styled(label, theme::title()))
            } else {
                Line::from(label)
            }
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(Span::styled(
                    " RatioMaster-Rust v0.1.0 ── [?=Help] ",
                    theme::title(),
                )),
        )
        .select(app.active_tab)
        .style(Style::default().fg(theme::FG))
        .highlight_style(theme::title());

    f.render_widget(tabs, area);
}

fn draw_torrent_info(f: &mut Frame, app: &App, area: Rect) {
    let tab = match app.tabs.get(app.active_tab) {
        Some(t) => t,
        None => return,
    };

    let path_display = tab
        .torrent_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(no torrent loaded — press 'o' to open)".into());

    let hash_short = if tab.info_hash.len() > 20 {
        format!("{}...", &tab.info_hash[..20])
    } else if tab.info_hash.is_empty() {
        "—".into()
    } else {
        tab.info_hash.clone()
    };

    let size_str = if tab.total_size > 0 {
        format_bytes(tab.total_size)
    } else {
        "—".into()
    };

    let piece_info = if tab.piece_count > 0 {
        format!(
            "  Pieces: {} x {}",
            tab.piece_count,
            format_bytes(tab.piece_length)
        )
    } else {
        String::new()
    };

    let text = vec![
        Line::from(vec![
            Span::styled("  File:    ", theme::label()),
            Span::raw(path_display),
        ]),
        Line::from(vec![
            Span::styled("  Tracker: ", theme::label()),
            Span::raw(if tab.tracker_url.is_empty() {
                "—".to_string()
            } else {
                tab.tracker_url.clone()
            }),
        ]),
        Line::from(vec![
            Span::styled("  Hash:    ", theme::label()),
            Span::raw(hash_short),
            Span::raw("  "),
            Span::styled("Size: ", theme::label()),
            Span::raw(size_str),
            Span::raw(piece_info),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(" Torrent ", theme::label()));
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_client_network(f: &mut Frame, app: &App, area: Rect) {
    let tab = match app.tabs.get(app.active_tab) {
        Some(t) => t,
        None => return,
    };

    let mut line1 = vec![Span::styled("  Client: ", theme::label())];
    line1.extend(render_dropdown_field(
        tab.client_dropdown.current(),
        22,
        is_focused(app, FocusableField::Client),
    ));
    line1.push(Span::raw("  "));
    line1.push(Span::styled("Port: ", theme::label()));
    line1.extend(render_field(
        &tab.port.value,
        if is_focused(app, FocusableField::Port) {
            Some(tab.port.cursor)
        } else {
            None
        },
        6,
        is_focused(app, FocusableField::Port),
    ));

    let mut line2 = vec![Span::styled("  Proxy:  ", theme::label())];
    line2.extend(render_dropdown_field(
        tab.proxy_dropdown.current(),
        22,
        is_focused(app, FocusableField::ProxyType),
    ));

    if tab.proxy_dropdown.current() != "None" {
        line2.push(Span::raw("  "));
        line2.push(Span::styled("Host: ", theme::label()));
        line2.extend(render_field(
            &tab.proxy_host.value,
            if is_focused(app, FocusableField::ProxyHost) {
                Some(tab.proxy_host.cursor)
            } else {
                None
            },
            15,
            is_focused(app, FocusableField::ProxyHost),
        ));
        line2.push(Span::raw(":"));
        line2.extend(render_field(
            &tab.proxy_port.value,
            if is_focused(app, FocusableField::ProxyPort) {
                Some(tab.proxy_port.cursor)
            } else {
                None
            },
            5,
            is_focused(app, FocusableField::ProxyPort),
        ));
    }

    let text = vec![Line::from(line1), Line::from(line2)];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(" Client & Network ", theme::label()));
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_speed_settings(f: &mut Frame, app: &App, area: Rect) {
    let tab = match app.tabs.get(app.active_tab) {
        Some(t) => t,
        None => return,
    };

    // Upload line
    let mut line1 = vec![Span::styled("  Upload:   ", theme::label())];
    line1.extend(render_field(
        &tab.upload_speed.value,
        if is_focused(app, FocusableField::UploadSpeed) {
            Some(tab.upload_speed.cursor)
        } else {
            None
        },
        5,
        is_focused(app, FocusableField::UploadSpeed),
    ));
    line1.push(Span::raw(" KB/s  "));
    line1.extend(render_checkbox(
        "Random:",
        tab.upload_random_enabled,
        is_focused(app, FocusableField::UploadRandomEnabled),
    ));
    line1.push(Span::raw(" "));
    if tab.upload_random_enabled {
        line1.extend(render_field(
            &tab.upload_random_min.value,
            if is_focused(app, FocusableField::UploadRandomMin) {
                Some(tab.upload_random_min.cursor)
            } else {
                None
            },
            5,
            is_focused(app, FocusableField::UploadRandomMin),
        ));
        line1.push(Span::raw(" - "));
        line1.extend(render_field(
            &tab.upload_random_max.value,
            if is_focused(app, FocusableField::UploadRandomMax) {
                Some(tab.upload_random_max.cursor)
            } else {
                None
            },
            5,
            is_focused(app, FocusableField::UploadRandomMax),
        ));
        line1.push(Span::raw(" KB/s"));
    }

    // Download line
    let mut line2 = vec![Span::styled("  Download: ", theme::label())];
    line2.extend(render_field(
        &tab.download_speed.value,
        if is_focused(app, FocusableField::DownloadSpeed) {
            Some(tab.download_speed.cursor)
        } else {
            None
        },
        5,
        is_focused(app, FocusableField::DownloadSpeed),
    ));
    line2.push(Span::raw(" KB/s  "));
    line2.extend(render_checkbox(
        "Random:",
        tab.download_random_enabled,
        is_focused(app, FocusableField::DownloadRandomEnabled),
    ));
    line2.push(Span::raw(" "));
    if tab.download_random_enabled {
        line2.extend(render_field(
            &tab.download_random_min.value,
            if is_focused(app, FocusableField::DownloadRandomMin) {
                Some(tab.download_random_min.cursor)
            } else {
                None
            },
            5,
            is_focused(app, FocusableField::DownloadRandomMin),
        ));
        line2.push(Span::raw(" - "));
        line2.extend(render_field(
            &tab.download_random_max.value,
            if is_focused(app, FocusableField::DownloadRandomMax) {
                Some(tab.download_random_max.cursor)
            } else {
                None
            },
            5,
            is_focused(app, FocusableField::DownloadRandomMax),
        ));
        line2.push(Span::raw(" KB/s"));
    }

    // Interval / checkboxes line
    let mut line3 = vec![Span::styled("  Interval: ", theme::label())];
    line3.extend(render_field(
        &tab.interval.value,
        if is_focused(app, FocusableField::Interval) {
            Some(tab.interval.cursor)
        } else {
            None
        },
        6,
        is_focused(app, FocusableField::Interval),
    ));
    line3.push(Span::raw(" sec   "));
    line3.extend(render_checkbox(
        "TCP Listener",
        tab.tcp_listener,
        is_focused(app, FocusableField::TcpListener),
    ));
    line3.push(Span::raw("  "));
    line3.extend(render_checkbox(
        "Scrape",
        tab.scrape,
        is_focused(app, FocusableField::Scrape),
    ));
    line3.push(Span::raw("  "));
    line3.extend(render_checkbox(
        "Ignore Failures",
        tab.ignore_failure,
        is_focused(app, FocusableField::IgnoreFailure),
    ));

    let text = vec![Line::from(line1), Line::from(line2), Line::from(line3)];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(" Speed ", theme::label()));
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_stop_condition(f: &mut Frame, app: &App, area: Rect) {
    let tab = match app.tabs.get(app.active_tab) {
        Some(t) => t,
        None => return,
    };

    let mut line = vec![Span::styled("  Stop: ", theme::label())];
    line.extend(render_dropdown_field(
        tab.stop_dropdown.current(),
        18,
        is_focused(app, FocusableField::StopType),
    ));

    if tab.stop_dropdown.current() != "Never" {
        line.push(Span::raw("  "));
        line.push(Span::styled("Value: ", theme::label()));
        line.extend(render_field(
            &tab.stop_value.value,
            if is_focused(app, FocusableField::StopValue) {
                Some(tab.stop_value.cursor)
            } else {
                None
            },
            10,
            is_focused(app, FocusableField::StopValue),
        ));
    }

    let text = vec![Line::from(line)];
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(" Stop Condition ", theme::label()));
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_controls(f: &mut Frame, app: &App, area: Rect) {
    let tab = match app.tabs.get(app.active_tab) {
        Some(t) => t,
        None => return,
    };

    let (status_str, status_color) = match &tab.status {
        TabStatus::Idle => ("Idle", theme::STATUS_IDLE),
        TabStatus::Running => ("Running", theme::STATUS_RUNNING),
        TabStatus::Stopped => ("Stopped", theme::STATUS_STOPPED),
        TabStatus::Error(e) => {
            let _ = e;
            ("Error", theme::STATUS_ERROR)
        }
    };

    let announce_info = if tab.announce_count > 0 {
        format!(" (Announce #{})", tab.announce_count)
    } else {
        String::new()
    };

    let elapsed = tab
        .started_at
        .map(|s| format_duration(s.elapsed()))
        .unwrap_or_else(|| "00:00:00".into());

    // Line 1: status + stats
    let line1 = Line::from(vec![
        Span::raw("  "),
        Span::styled("Status: ", theme::label()),
        Span::styled(
            format!("● {status_str}{announce_info}"),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("Up: ", theme::label()),
        Span::styled(format_bytes(tab.uploaded), theme::value()),
        Span::raw("  "),
        Span::styled("Down: ", theme::label()),
        Span::styled(format_bytes(tab.downloaded), theme::value()),
        Span::raw("  "),
        Span::styled("S: ", theme::label()),
        Span::raw(format!("{}", tab.seeders)),
        Span::raw("  "),
        Span::styled("L: ", theme::label()),
        Span::raw(format!("{}", tab.leechers)),
        Span::raw("  "),
        Span::styled("Time: ", theme::label()),
        Span::raw(elapsed),
    ]);

    // Line 2: ratio gauge
    let ratio = if tab.downloaded > 0 {
        tab.uploaded as f64 / tab.downloaded as f64
    } else if tab.uploaded > 0 {
        f64::INFINITY
    } else {
        0.0
    };

    let ratio_str = if ratio.is_infinite() {
        "∞".to_string()
    } else {
        format!("{ratio:.2}")
    };

    let gauge_ratio = if ratio.is_infinite() {
        1.0
    } else {
        (ratio / 3.0).min(1.0)
    };

    // We render the gauge as a separate widget, so build a sub-layout
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .margin(0)
        .split(Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        });

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(" Controls ", theme::label()));
    f.render_widget(block, area);

    f.render_widget(Paragraph::new(line1), inner[0]);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(theme::ratio_color(ratio)))
        .label(Span::styled(
            format!("  Ratio: {ratio_str}x"),
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .ratio(gauge_ratio);
    f.render_widget(gauge, inner[1]);
}

fn draw_log(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.mode == AppMode::LogFilter {
        format!(" Log (filter: {}_) ", app.log_filter)
    } else if !app.log_filter.is_empty() {
        format!(" Log (filter: {}) ", app.log_filter)
    } else {
        " Log [/=filter │ Ctrl+L=clear │ Ctrl+S=save] ".into()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(title, theme::label()));

    let logs = app.filtered_logs();
    let inner_height = area.height.saturating_sub(2) as usize;

    let start = if logs.len() > inner_height {
        let max_scroll = logs.len() - inner_height;
        app.log_scroll.min(max_scroll)
    } else {
        0
    };

    let items: Vec<ListItem> = logs
        .iter()
        .skip(start)
        .take(inner_height)
        .map(|entry| {
            let line = Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(theme::TIMESTAMP),
                ),
                Span::raw(&entry.message),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let tab = match app.tabs.get(app.active_tab) {
        Some(t) => t,
        None => return,
    };

    let upload_speed = tab.upload_speed.as_u64();
    let elapsed = tab
        .started_at
        .map(|s| format_duration(s.elapsed()))
        .unwrap_or_else(|| "—".into());

    let focus_hint = if app.focused_field.is_some() {
        "Esc=unfocus"
    } else {
        "Tab=focus fields"
    };

    let line = Line::from(vec![
        Span::styled(
            format!(
                " Tab:{}/{} │ Up:{}KB/s │ {} │ {} ",
                app.active_tab + 1,
                app.tabs.len(),
                upload_speed,
                elapsed,
                focus_hint,
            ),
            theme::hint(),
        ),
        Span::styled(" Enter=Start/Stop  o=Open  q=Quit  ?=Help ", theme::hint()),
    ]);

    f.render_widget(Paragraph::new(line), area);
}

// -- Overlays --

fn draw_file_browser(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 80, f.area());
    f.render_widget(Clear, area);

    let title = format!(
        " Open Torrent — {} ",
        app.file_browser.current_dir.display()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(title, theme::title()))
        .style(Style::default().bg(ratatui::style::Color::Black));

    let items: Vec<ListItem> = app
        .file_browser
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let style = if i == app.file_browser.selected {
                theme::selected()
            } else if entry.is_dir {
                Style::default().fg(theme::DIR_COLOR)
            } else {
                Style::default().fg(theme::FG)
            };

            let prefix = if entry.is_dir { "📁 " } else { "   " };
            ListItem::new(Line::from(Span::styled(
                format!("{prefix}{}", entry.name),
                style,
            )))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_dropdown_overlay(f: &mut Frame, app: &App) {
    let tab = match app.tabs.get(app.active_tab) {
        Some(t) => t,
        None => return,
    };

    let (dropdown, title) = match app.focused_field {
        Some(FocusableField::Client) => (&tab.client_dropdown, "Select Client"),
        Some(FocusableField::StopType) => (&tab.stop_dropdown, "Stop Condition"),
        Some(FocusableField::ProxyType) => (&tab.proxy_dropdown, "Proxy Type"),
        _ => return,
    };

    let visible = dropdown.visible_count();
    let height = (visible as u16) + 2; // +2 for borders
    let width = 35;

    let area = centered_rect_fixed(width, height, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(format!(" {title} "), theme::title()))
        .style(Style::default().bg(ratatui::style::Color::Black));

    let items: Vec<ListItem> = dropdown
        .items
        .iter()
        .enumerate()
        .skip(dropdown.scroll)
        .take(visible)
        .map(|(i, item)| {
            let style = if i == dropdown.selected {
                theme::selected()
            } else {
                Style::default().fg(theme::FG)
            };
            let prefix = if i == dropdown.selected { "▸ " } else { "  " };
            ListItem::new(Line::from(Span::styled(format!("{prefix}{item}"), style)))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_help_popup(f: &mut Frame) {
    let area = centered_rect(60, 80, f.area());
    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from(Span::styled(
            "  Keybindings",
            Style::default()
                .fg(theme::TITLE)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tab / Shift+Tab  ", theme::field_focused()),
            Span::raw("Cycle focus between fields"),
        ]),
        Line::from(vec![
            Span::styled("  Enter            ", theme::field_focused()),
            Span::raw("Start/stop engine or confirm dropdown"),
        ]),
        Line::from(vec![
            Span::styled("  Space            ", theme::field_focused()),
            Span::raw("Toggle checkbox"),
        ]),
        Line::from(vec![
            Span::styled("  Up/Down          ", theme::field_focused()),
            Span::raw("Navigate dropdown or scroll log"),
        ]),
        Line::from(vec![
            Span::styled("  Left/Right       ", theme::field_focused()),
            Span::raw("Move cursor in text field"),
        ]),
        Line::from(vec![
            Span::styled("  Esc              ", theme::field_focused()),
            Span::raw("Cancel / unfocus"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  o                ", theme::field_focused()),
            Span::raw("Open file browser"),
        ]),
        Line::from(vec![
            Span::styled("  u                ", theme::field_focused()),
            Span::raw("Force announce/update"),
        ]),
        Line::from(vec![
            Span::styled("  q                ", theme::field_focused()),
            Span::raw("Quit (confirms if engines running)"),
        ]),
        Line::from(vec![
            Span::styled("  ?                ", theme::field_focused()),
            Span::raw("Toggle this help"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Ctrl+N           ", theme::field_focused()),
            Span::raw("New tab"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+W           ", theme::field_focused()),
            Span::raw("Close tab"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Left/Right  ", theme::field_focused()),
            Span::raw("Switch tabs"),
        ]),
        Line::from(vec![
            Span::styled("  Alt+Left/Right   ", theme::field_focused()),
            Span::raw("Reorder tabs"),
        ]),
        Line::from(vec![
            Span::styled("  F2               ", theme::field_focused()),
            Span::raw("Rename tab"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  /                ", theme::field_focused()),
            Span::raw("Filter log"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+L           ", theme::field_focused()),
            Span::raw("Clear log"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+S           ", theme::field_focused()),
            Span::raw("Save log"),
        ]),
        Line::from(vec![
            Span::styled("  F5               ", theme::field_focused()),
            Span::raw("Refresh/redraw"),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Press Esc or ? to close", theme::hint())),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(" Help ", theme::title()))
        .style(Style::default().bg(ratatui::style::Color::Black));

    let paragraph = Paragraph::new(help_text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_quit_confirm(f: &mut Frame) {
    let area = centered_rect_fixed(40, 5, f.area());
    f.render_widget(Clear, area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Engines are still running!",
            Style::default()
                .fg(theme::WARNING)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("  Quit anyway? (y/n)", theme::label())),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(" Quit? ", theme::title()))
        .style(Style::default().bg(ratatui::style::Color::Black));

    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_tab_rename(f: &mut Frame, app: &App) {
    let area = centered_rect_fixed(40, 3, f.area());
    f.render_widget(Clear, area);

    let (before, after) = app.tab_rename_input.split_at_cursor();
    let cursor_char = after.chars().next().unwrap_or(' ');
    let rest = if after.len() > cursor_char.len_utf8() {
        &after[cursor_char.len_utf8()..]
    } else {
        ""
    };

    let line = Line::from(vec![
        Span::raw(" "),
        Span::raw(before.to_string()),
        Span::styled(
            cursor_char.to_string(),
            Style::default().fg(theme::BG).bg(theme::FOCUSED),
        ),
        Span::raw(rest.to_string()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(" Rename Tab ", theme::title()))
        .style(Style::default().bg(ratatui::style::Color::Black));

    let paragraph = Paragraph::new(vec![line]).block(block);
    f.render_widget(paragraph, area);
}

fn draw_minimized(f: &mut Frame, app: &App) {
    let running_count = app
        .tabs
        .iter()
        .filter(|t| matches!(t.status, TabStatus::Running))
        .count();

    let text = Paragraph::new(Line::from(vec![
        Span::styled(" RatioMaster-Rust ", theme::title()),
        Span::raw(format!(
            "| {running_count} engine(s) running | Press any key to restore"
        )),
    ]))
    .wrap(Wrap { trim: false });

    f.render_widget(text, f.area());
}

// -- Layout helpers --

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}
