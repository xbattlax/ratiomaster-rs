use eframe::egui;

use crate::tabs::{format_bytes, TorrentTab};

pub fn show(ui: &mut egui::Ui, tab: &mut TorrentTab, open_file_requested: &mut bool) {
    let running = tab.is_running();

    // Panel 1 - Torrent File
    egui::CollapsingHeader::new("Torrent File")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let path_text = tab
                    .torrent_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "No file selected".into());
                ui.add(
                    egui::TextEdit::singleline(&mut { path_text })
                        .desired_width(ui.available_width() - 90.0)
                        .interactive(false),
                );
                ui.add_enabled_ui(!running, |ui| {
                    if ui.button("Browse...").clicked() {
                        *open_file_requested = true;
                    }
                });
            });
        });

    ui.add_space(2.0);

    // Panel 4 - Torrent Info
    egui::CollapsingHeader::new("Torrent Info")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("torrent_info_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Tracker:");
                    ui.add_enabled(
                        !running,
                        egui::TextEdit::singleline(&mut tab.tracker_url)
                            .desired_width(ui.available_width() - 8.0)
                            .hint_text("http://tracker/announce"),
                    );
                    ui.end_row();

                    ui.label("SHA Hash:");
                    ui.horizontal(|ui| {
                        let mut display = if tab.info_hash.is_empty() {
                            "-".into()
                        } else {
                            tab.info_hash.clone()
                        };
                        ui.add(
                            egui::TextEdit::singleline(&mut display)
                                .desired_width(300.0)
                                .interactive(false),
                        );
                        if !tab.info_hash.is_empty() && ui.small_button("Copy").clicked() {
                            ui.ctx().copy_text(tab.info_hash.clone());
                        }
                    });
                    ui.end_row();

                    ui.label("Torrent Size:");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut tab.torrent_size_override)
                                .desired_width(100.0)
                                .hint_text("override"),
                        );
                        if tab.total_size > 0 {
                            ui.label(format!("({})", format_bytes(tab.total_size)));
                        }
                    });
                    ui.end_row();
                });
        });
}
