use eframe::egui;

use crate::tabs::TorrentTab;

pub fn show(ui: &mut egui::Ui, tab: &mut TorrentTab) {
    egui::CollapsingHeader::new("Random Speeds on Next Update")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("random_next_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut tab.next_upload_random, "Upload");
                        if tab.next_upload_random {
                            ui.label("Min:");
                            ui.add(
                                egui::TextEdit::singleline(&mut tab.next_upload_random_min)
                                    .desired_width(50.0),
                            );
                            ui.label("Max:");
                            ui.add(
                                egui::TextEdit::singleline(&mut tab.next_upload_random_max)
                                    .desired_width(50.0),
                            );
                        }
                    });
                    ui.end_row();

                    ui.label("");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut tab.next_download_random, "Download");
                        if tab.next_download_random {
                            ui.label("Min:");
                            ui.add(
                                egui::TextEdit::singleline(&mut tab.next_download_random_min)
                                    .desired_width(50.0),
                            );
                            ui.label("Max:");
                            ui.add(
                                egui::TextEdit::singleline(&mut tab.next_download_random_max)
                                    .desired_width(50.0),
                            );
                        }
                    });
                    ui.end_row();
                });
        });
}
