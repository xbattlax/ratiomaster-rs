use eframe::egui;

use crate::tabs::{format_bytes, format_duration, TorrentTab};
use crate::theme;

pub fn show(ui: &mut egui::Ui, tab: &TorrentTab) {
    egui::CollapsingHeader::new("Statistics")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("stats_grid")
                .num_columns(4)
                .spacing([16.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // Row 1: Uploaded / Downloaded
                    ui.label("Uploaded:");
                    ui.label(format_bytes(tab.uploaded));
                    ui.label("Downloaded:");
                    ui.label(format_bytes(tab.downloaded));
                    ui.end_row();

                    // Row 2: Seeders / Leechers
                    ui.label("Seeders:");
                    ui.label(tab.seeders.to_string());
                    ui.label("Leechers:");
                    ui.label(tab.leechers.to_string());
                    ui.end_row();

                    // Row 3: Ratio / Announces
                    ui.label("Ratio:");
                    let ratio = tab.ratio();
                    ui.colored_label(theme::ratio_color(ratio), format!("{ratio:.3}"));
                    ui.label("Announces:");
                    ui.label(tab.announce_count.to_string());
                    ui.end_row();

                    // Row 4: Running time / Next announce
                    ui.label("Running:");
                    ui.label(
                        tab.running_time()
                            .map(format_duration)
                            .unwrap_or_else(|| "-".into()),
                    );
                    ui.label("Next:");
                    ui.label(
                        tab.next_announce_secs()
                            .map(|s| format!("{s}s"))
                            .unwrap_or_else(|| "-".into()),
                    );
                    ui.end_row();
                });

            // Ratio progress bar
            if tab.total_size > 0 {
                ui.add_space(4.0);
                let ratio = tab.ratio();
                let progress = (ratio / 3.0).min(1.0) as f32;
                let bar = egui::ProgressBar::new(progress)
                    .text(format!("Ratio: {ratio:.3}"))
                    .fill(theme::ratio_color(ratio));
                ui.add(bar);
            }
        });
}
