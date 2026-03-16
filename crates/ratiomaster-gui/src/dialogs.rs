use eframe::egui;

use crate::theme;

/// State for the About dialog.
pub struct AboutDialog {
    pub open: bool,
}

impl AboutDialog {
    pub fn new() -> Self {
        Self { open: false }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        egui::Window::new("About RatioMaster-Rust")
            .open(&mut self.open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("RatioMaster-Rust");
                    ui.label("v0.1.0");
                    ui.add_space(8.0);
                    ui.label("A BitTorrent tracker communication tool");
                    ui.label("41 client profiles across 16 families");
                    ui.add_space(8.0);
                    ui.label("Built with egui/eframe");
                    ui.add_space(8.0);
                    ui.hyperlink_to(
                        "GitHub Repository",
                        "https://github.com/xbattlax/ratiomaster-rs",
                    );
                    ui.add_space(8.0);
                    ui.label("MIT License");
                });
            });
    }
}

/// State for the Settings dialog.
pub struct SettingsDialog {
    pub open: bool,
    pub default_client: String,
    pub default_upload: String,
    pub default_download: String,
    pub default_port: String,
    pub default_interval: String,
    pub tcp_listener: bool,
    pub scrape: bool,
    pub ignore_failure: bool,
    pub theme: theme::Theme,
}

impl SettingsDialog {
    pub fn new(theme: theme::Theme) -> Self {
        Self {
            open: false,
            default_client: "uTorrent 3.3.2".into(),
            default_upload: "100".into(),
            default_download: "0".into(),
            default_port: "6881".into(),
            default_interval: "1800".into(),
            tcp_listener: false,
            scrape: false,
            ignore_failure: false,
            theme,
        }
    }

    /// Returns true if the theme was changed.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut theme_changed = false;

        egui::Window::new("Settings")
            .open(&mut self.open)
            .resizable(false)
            .collapsible(false)
            .min_width(350.0)
            .show(ctx, |ui| {
                ui.heading("Default Settings");
                ui.add_space(4.0);

                egui::Grid::new("settings_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Default Client:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.default_client)
                                .desired_width(200.0),
                        );
                        ui.end_row();

                        ui.label("Upload (KB/s):");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.default_upload)
                                .desired_width(80.0),
                        );
                        ui.end_row();

                        ui.label("Download (KB/s):");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.default_download)
                                .desired_width(80.0),
                        );
                        ui.end_row();

                        ui.label("Port:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.default_port).desired_width(80.0),
                        );
                        ui.end_row();

                        ui.label("Interval (sec):");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.default_interval)
                                .desired_width(80.0),
                        );
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.tcp_listener, "TCP Listener");
                    ui.checkbox(&mut self.scrape, "Scrape");
                    ui.checkbox(&mut self.ignore_failure, "Ignore Failure");
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Theme:");
                    let label = match self.theme {
                        theme::Theme::Dark => "Dark",
                        theme::Theme::Light => "Light",
                    };
                    if ui.button(label).clicked() {
                        self.theme.toggle();
                        theme_changed = true;
                    }
                });
            });

        theme_changed
    }
}
