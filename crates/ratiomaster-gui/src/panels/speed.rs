use eframe::egui;

use crate::tabs::TorrentTab;

pub fn show(ui: &mut egui::Ui, tab: &mut TorrentTab) {
    egui::CollapsingHeader::new("Speed")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("speed_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // Upload speed
                    ui.label("Upload (KB/s):");
                    ui.add(egui::TextEdit::singleline(&mut tab.upload_speed).desired_width(80.0));
                    ui.end_row();

                    // Download speed
                    ui.label("Download (KB/s):");
                    ui.add(egui::TextEdit::singleline(&mut tab.download_speed).desired_width(80.0));
                    ui.end_row();

                    // Random upload
                    ui.label("");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut tab.upload_random, "Random Upload");
                        if tab.upload_random {
                            ui.label("Min:");
                            ui.add(
                                egui::TextEdit::singleline(&mut tab.upload_random_min)
                                    .desired_width(50.0),
                            );
                            ui.label("Max:");
                            ui.add(
                                egui::TextEdit::singleline(&mut tab.upload_random_max)
                                    .desired_width(50.0),
                            );
                        }
                    });
                    ui.end_row();

                    // Random download
                    ui.label("");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut tab.download_random, "Random Download");
                        if tab.download_random {
                            ui.label("Min:");
                            ui.add(
                                egui::TextEdit::singleline(&mut tab.download_random_min)
                                    .desired_width(50.0),
                            );
                            ui.label("Max:");
                            ui.add(
                                egui::TextEdit::singleline(&mut tab.download_random_max)
                                    .desired_width(50.0),
                            );
                        }
                    });
                    ui.end_row();
                });
        });
}
