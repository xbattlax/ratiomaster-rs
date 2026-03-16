use eframe::egui;

use crate::tabs::{TorrentTab, PROXY_TYPES};

pub fn show(ui: &mut egui::Ui, tab: &mut TorrentTab) {
    egui::CollapsingHeader::new("Proxy")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("proxy_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Type:");
                    let proxy_label = PROXY_TYPES.get(tab.proxy_type).copied().unwrap_or("None");
                    egui::ComboBox::from_id_salt("proxy_combo")
                        .selected_text(proxy_label)
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            for (i, name) in PROXY_TYPES.iter().enumerate() {
                                ui.selectable_value(&mut tab.proxy_type, i, *name);
                            }
                        });
                    ui.end_row();

                    if tab.proxy_type != 0 {
                        ui.label("Host:");
                        ui.add(
                            egui::TextEdit::singleline(&mut tab.proxy_host)
                                .desired_width(180.0)
                                .hint_text("127.0.0.1"),
                        );
                        ui.end_row();

                        ui.label("Port:");
                        ui.add(
                            egui::TextEdit::singleline(&mut tab.proxy_port)
                                .desired_width(80.0)
                                .hint_text("1080"),
                        );
                        ui.end_row();

                        ui.label("User:");
                        ui.add(
                            egui::TextEdit::singleline(&mut tab.proxy_user).desired_width(180.0),
                        );
                        ui.end_row();

                        ui.label("Pass:");
                        ui.add(
                            egui::TextEdit::singleline(&mut tab.proxy_pass)
                                .desired_width(180.0)
                                .password(true),
                        );
                        ui.end_row();
                    }
                });
        });
}
