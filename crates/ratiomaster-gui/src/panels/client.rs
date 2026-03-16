use eframe::egui;
use std::sync::LazyLock;

use crate::tabs::{build_client_families, ClientFamilyGroup, TorrentTab, STOP_TYPES};

static CLIENT_FAMILIES: LazyLock<Vec<ClientFamilyGroup>> = LazyLock::new(build_client_families);

pub fn show(ui: &mut egui::Ui, tab: &mut TorrentTab) {
    let running = tab.is_running();

    egui::CollapsingHeader::new("Client Settings")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("client_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // Client dropdown (grouped by family)
                    ui.label("Client:");
                    let current_name = tab
                        .client_names
                        .get(tab.selected_client)
                        .cloned()
                        .unwrap_or_default();
                    ui.add_enabled_ui(!running, |ui| {
                        egui::ComboBox::from_id_salt("client_combo")
                            .selected_text(&current_name)
                            .width(280.0)
                            .show_ui(ui, |ui| {
                                let families = &*CLIENT_FAMILIES;
                                for family in families {
                                    ui.label(
                                        egui::RichText::new(&family.label)
                                            .strong()
                                            .color(egui::Color32::from_rgb(120, 180, 255)),
                                    );
                                    for profile_name in &family.profiles {
                                        let idx = tab
                                            .client_names
                                            .iter()
                                            .position(|n| n == profile_name)
                                            .unwrap_or(0);
                                        ui.selectable_value(
                                            &mut tab.selected_client,
                                            idx,
                                            profile_name,
                                        );
                                    }
                                    ui.separator();
                                }
                            });
                    });
                    ui.end_row();

                    // Interval
                    ui.label("Interval (sec):");
                    ui.add_enabled(
                        !running,
                        egui::TextEdit::singleline(&mut tab.interval)
                            .desired_width(80.0)
                            .hint_text("1800"),
                    );
                    ui.end_row();

                    // File size (downloaded)
                    ui.label("File size (downloaded):");
                    ui.add(
                        egui::TextEdit::singleline(&mut tab.file_size_downloaded)
                            .desired_width(100.0)
                            .hint_text("bytes"),
                    );
                    ui.end_row();

                    // Stop after
                    ui.label("Stop after:");
                    ui.horizontal(|ui| {
                        let stop_label = STOP_TYPES.get(tab.stop_type).copied().unwrap_or("Never");
                        egui::ComboBox::from_id_salt("stop_combo")
                            .selected_text(stop_label)
                            .width(140.0)
                            .show_ui(ui, |ui| {
                                for (i, name) in STOP_TYPES.iter().enumerate() {
                                    ui.selectable_value(&mut tab.stop_type, i, *name);
                                }
                            });
                        if tab.stop_type != 0 {
                            let hint = match tab.stop_type {
                                1 => "MB uploaded",
                                2 => "MB downloaded",
                                3 => "seconds",
                                4 => "seeders",
                                5 => "leechers",
                                6 => "ratio (e.g. 2.0)",
                                _ => "value",
                            };
                            ui.add(
                                egui::TextEdit::singleline(&mut tab.stop_value)
                                    .desired_width(100.0)
                                    .hint_text(hint),
                            );
                        }
                    });
                    ui.end_row();
                });
        });
}
