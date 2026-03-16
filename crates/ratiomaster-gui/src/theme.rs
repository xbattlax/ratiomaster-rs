use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn toggle(&mut self) {
        *self = match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        };
    }
}

/// Accent blue used for highlights and active elements.
pub const ACCENT: Color32 = Color32::from_rgb(66, 135, 245);
/// Success green for running status and good ratios.
pub const SUCCESS: Color32 = Color32::from_rgb(76, 175, 80);
/// Warning yellow for stopped status and mediocre ratios.
pub const WARNING: Color32 = Color32::from_rgb(255, 193, 7);
/// Error red for errors and poor ratios.
pub const ERROR: Color32 = Color32::from_rgb(244, 67, 54);
/// Muted gray for idle status.
pub const MUTED: Color32 = Color32::from_rgb(158, 158, 158);

pub fn apply_theme(ctx: &egui::Context, theme: Theme) {
    let mut visuals = match theme {
        Theme::Dark => Visuals::dark(),
        Theme::Light => Visuals::light(),
    };

    if theme == Theme::Dark {
        // Darker background for contrast
        visuals.panel_fill = Color32::from_rgb(30, 30, 36);
        visuals.window_fill = Color32::from_rgb(36, 36, 42);
        visuals.extreme_bg_color = Color32::from_rgb(22, 22, 28);

        // Accent-colored selection
        visuals.selection.bg_fill = ACCENT.gamma_multiply(0.4);
        visuals.selection.stroke = Stroke::new(1.0, ACCENT);

        // Widget styling
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(45, 45, 52);
        visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(45, 45, 52);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(4);

        visuals.widgets.hovered.bg_fill = Color32::from_rgb(55, 55, 65);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(55, 55, 65);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(4);

        visuals.widgets.active.bg_fill = ACCENT.gamma_multiply(0.6);
        visuals.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.6);
        visuals.widgets.active.corner_radius = CornerRadius::same(4);
    }

    ctx.set_visuals(visuals);
}

/// Returns a color for the given ratio value.
pub fn ratio_color(ratio: f64) -> Color32 {
    if ratio < 1.0 {
        ERROR
    } else if ratio < 2.0 {
        WARNING
    } else {
        SUCCESS
    }
}
