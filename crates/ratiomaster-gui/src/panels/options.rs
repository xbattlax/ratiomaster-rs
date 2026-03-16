use eframe::egui;

use crate::tabs::TorrentTab;

pub fn show(ui: &mut egui::Ui, tab: &mut TorrentTab) {
    egui::CollapsingHeader::new("Options")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut tab.scrape, "Request scrape");
                ui.add_space(16.0);
                ui.checkbox(&mut tab.tcp_listener, "TCP listen");
                ui.add_space(16.0);
                ui.checkbox(&mut tab.ignore_failure, "Ignore failure reason");
            });
        });
}
