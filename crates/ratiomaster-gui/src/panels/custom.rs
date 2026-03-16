use eframe::egui;

use crate::tabs::TorrentTab;

pub fn show(ui: &mut egui::Ui, tab: &mut TorrentTab) {
    egui::CollapsingHeader::new("Custom Values")
        .default_open(true)
        .show(ui, |ui| {
            ui.checkbox(
                &mut tab.generate_new_values,
                "Generate new values on each start",
            );

            egui::Grid::new("custom_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Custom port:");
                    ui.add(
                        egui::TextEdit::singleline(&mut tab.custom_port)
                            .desired_width(100.0)
                            .hint_text("e.g. 6881"),
                    );
                    ui.end_row();

                    ui.label("Peers (numwant):");
                    ui.add(
                        egui::TextEdit::singleline(&mut tab.custom_numwant)
                            .desired_width(100.0)
                            .hint_text("e.g. 200"),
                    );
                    ui.end_row();

                    ui.label("Custom peer_id:");
                    ui.add(
                        egui::TextEdit::singleline(&mut tab.custom_peer_id)
                            .desired_width(200.0)
                            .hint_text("20 chars"),
                    );
                    ui.end_row();

                    ui.label("Custom key:");
                    ui.add(
                        egui::TextEdit::singleline(&mut tab.custom_key)
                            .desired_width(200.0)
                            .hint_text("session key"),
                    );
                    ui.end_row();
                });
        });
}
