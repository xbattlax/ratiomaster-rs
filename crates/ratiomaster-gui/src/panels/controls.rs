use eframe::egui;

use crate::tabs::{TabStatus, TorrentTab};
use crate::theme;

pub fn show(ui: &mut egui::Ui, tab: &TorrentTab) -> ControlAction {
    let mut action = ControlAction::None;

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        // START button (blue text, silver bg)
        let start_enabled = tab.status != TabStatus::Running && tab.torrent_data.is_some();
        ui.add_enabled_ui(start_enabled, |ui| {
            let btn = egui::Button::new(
                egui::RichText::new("  START  ")
                    .color(egui::Color32::from_rgb(50, 120, 255))
                    .strong(),
            )
            .fill(egui::Color32::from_rgb(60, 60, 70))
            .min_size(egui::vec2(90.0, 32.0));
            if ui.add(btn).clicked() {
                action = ControlAction::Start;
            }
        });

        // STOP button (red text)
        let stop_enabled = tab.status == TabStatus::Running;
        ui.add_enabled_ui(stop_enabled, |ui| {
            let btn = egui::Button::new(
                egui::RichText::new("  STOP  ")
                    .color(egui::Color32::from_rgb(255, 60, 60))
                    .strong(),
            )
            .fill(egui::Color32::from_rgb(60, 60, 70))
            .min_size(egui::vec2(90.0, 32.0));
            if ui.add(btn).clicked() {
                action = ControlAction::Stop;
            }
        });

        // Manual Update button
        ui.add_enabled_ui(stop_enabled, |ui| {
            let btn = egui::Button::new("Manual Update").min_size(egui::vec2(100.0, 32.0));
            if ui.add(btn).clicked() {
                action = ControlAction::ForceAnnounce;
            }
        });

        // Set default values button
        let btn = egui::Button::new("Set default values").min_size(egui::vec2(120.0, 32.0));
        if ui.add(btn).clicked() {
            action = ControlAction::SetDefaults;
        }

        ui.add_space(16.0);

        // Status indicator
        let (color, label) = match &tab.status {
            TabStatus::Idle => (theme::MUTED, "Idle"),
            TabStatus::Running => (theme::SUCCESS, "Running"),
            TabStatus::Stopped => (theme::WARNING, "Stopped"),
            TabStatus::Error(_) => (theme::ERROR, "Error"),
        };

        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 6.0, color);
        ui.label(label);
    });
    ui.add_space(4.0);

    action
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    None,
    Start,
    Stop,
    ForceAnnounce,
    SetDefaults,
}
