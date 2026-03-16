use eframe::egui;

use crate::tabs::TorrentTab;

pub fn show(ui: &mut egui::Ui, tab: &mut TorrentTab) -> LogAction {
    let mut action = LogAction::None;

    ui.horizontal(|ui| {
        ui.checkbox(&mut tab.log_enabled, "Enable log");
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::singleline(&mut tab.log_filter)
                .desired_width(150.0)
                .hint_text("Filter..."),
        );
        ui.checkbox(&mut tab.log_auto_scroll, "Auto-scroll");
        if ui.button("Clear").clicked() {
            action = LogAction::Clear;
        }
        if ui.button("Save").clicked() {
            action = LogAction::Save;
        }
    });

    ui.separator();

    // Black background log area
    let frame = egui::Frame::new()
        .fill(egui::Color32::from_rgb(15, 15, 15))
        .inner_margin(4.0)
        .corner_radius(egui::CornerRadius::same(2));

    frame.show(ui, |ui| {
        let filter_lower = tab.log_filter.to_lowercase();
        let entries: Vec<_> = if tab.log_filter.is_empty() {
            tab.log_entries.iter().collect()
        } else {
            tab.log_entries
                .iter()
                .filter(|e| e.message.to_lowercase().contains(&filter_lower))
                .collect()
        };

        let text_style = egui::TextStyle::Monospace;
        let row_height = ui.text_style_height(&text_style);
        let total_rows = entries.len();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .max_height(200.0)
            .stick_to_bottom(tab.log_auto_scroll)
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                for idx in row_range {
                    if let Some(entry) = entries.get(idx) {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&entry.timestamp)
                                    .monospace()
                                    .color(egui::Color32::from_rgb(120, 120, 120)),
                            );
                            ui.label(
                                egui::RichText::new(&entry.message)
                                    .monospace()
                                    .color(egui::Color32::from_rgb(200, 200, 200)),
                            );
                        });
                    }
                }
            });
    });

    action
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogAction {
    None,
    Clear,
    Save,
}
