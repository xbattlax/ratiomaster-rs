/// Cohesive dark theme constants and style helpers for the TUI.
use ratatui::style::{Color, Modifier, Style};

// -- Base palette --
pub const BG: Color = Color::Reset;
pub const FG: Color = Color::White;
pub const LABEL: Color = Color::Cyan;
pub const VALUE: Color = Color::Green;
pub const FOCUSED: Color = Color::Yellow;
pub const WARNING: Color = Color::Yellow;
pub const DIR_COLOR: Color = Color::Blue;
pub const TIMESTAMP: Color = Color::DarkGray;
pub const TITLE: Color = Color::Yellow;
pub const HINT: Color = Color::DarkGray;
pub const SELECTED_BG: Color = Color::DarkGray;
pub const GAUGE_LOW: Color = Color::Red;
pub const GAUGE_MID: Color = Color::Yellow;
pub const GAUGE_HIGH: Color = Color::Green;
pub const CHECKBOX_ON: Color = Color::Green;
pub const CHECKBOX_OFF: Color = Color::Gray;
pub const STATUS_RUNNING: Color = Color::Green;
pub const STATUS_STOPPED: Color = Color::Yellow;
pub const STATUS_IDLE: Color = Color::Gray;
pub const STATUS_ERROR: Color = Color::Red;

// -- Composite styles --

pub fn label() -> Style {
    Style::default().fg(LABEL)
}

pub fn value() -> Style {
    Style::default().fg(VALUE)
}

pub fn title() -> Style {
    Style::default().fg(TITLE).add_modifier(Modifier::BOLD)
}

pub fn hint() -> Style {
    Style::default().fg(HINT)
}

pub fn selected() -> Style {
    Style::default()
        .fg(FOCUSED)
        .bg(SELECTED_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn field_normal() -> Style {
    Style::default().fg(VALUE)
}

pub fn field_focused() -> Style {
    Style::default().fg(FOCUSED).add_modifier(Modifier::BOLD)
}

pub fn checkbox(checked: bool) -> Style {
    if checked {
        Style::default().fg(CHECKBOX_ON)
    } else {
        Style::default().fg(CHECKBOX_OFF)
    }
}

/// Returns a color for the ratio gauge based on value.
pub fn ratio_color(ratio: f64) -> Color {
    if ratio < 1.0 {
        GAUGE_LOW
    } else if ratio < 2.0 {
        GAUGE_MID
    } else {
        GAUGE_HIGH
    }
}
