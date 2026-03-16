use eframe::egui;

use crate::dialogs::{AboutDialog, SettingsDialog};
use crate::engine_bridge;
use crate::panels::controls::ControlAction;
use crate::panels::log::LogAction;
use crate::panels::{
    client, controls, custom, log, options, proxy, random_next, speed, stats, torrent,
};
use crate::tabs::{format_bytes, format_duration, TabStatus, TorrentTab};
use crate::theme::{self, Theme};

/// Main application state.
pub struct RatioMasterApp {
    tabs: Vec<TorrentTab>,
    active_tab: usize,
    theme: Theme,
    runtime: tokio::runtime::Runtime,
    about: AboutDialog,
    settings: SettingsDialog,
    pending_file: Option<std::path::PathBuf>,
    file_dialog_open: bool,
    file_channel: (
        std::sync::mpsc::Sender<Option<std::path::PathBuf>>,
        std::sync::mpsc::Receiver<Option<std::path::PathBuf>>,
    ),
}

impl RatioMasterApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Follow system theme (macOS dark/light mode)
        cc.egui_ctx.set_theme(egui::ThemePreference::System);
        let theme = if cc.egui_ctx.style().visuals.dark_mode {
            Theme::Dark
        } else {
            Theme::Light
        };

        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 4.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        cc.egui_ctx.set_style(style);

        let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let tabs = vec![TorrentTab::new("Tab 1".into())];

        Self {
            tabs,
            active_tab: 0,
            theme,
            runtime,
            about: AboutDialog::new(),
            settings: SettingsDialog::new(theme),
            pending_file: None,
            file_dialog_open: false,
            file_channel: std::sync::mpsc::channel(),
        }
    }

    fn add_tab(&mut self) {
        let n = self.tabs.len() + 1;
        let mut tab = TorrentTab::new(format!("Tab {n}"));
        // Apply settings defaults to new tabs
        tab.upload_speed = self.settings.default_upload.clone();
        tab.download_speed = self.settings.default_download.clone();
        tab.port = self.settings.default_port.clone();
        tab.interval = self.settings.default_interval.clone();
        tab.tcp_listener = self.settings.tcp_listener;
        tab.scrape = self.settings.scrape;
        tab.ignore_failure = self.settings.ignore_failure;
        // Apply default client if found
        if let Some(idx) = tab
            .client_names
            .iter()
            .position(|n| n == &self.settings.default_client)
        {
            tab.selected_client = idx;
        }
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
    }

    fn close_tab(&mut self, idx: usize) {
        if self.tabs.len() <= 1 {
            return;
        }
        if let Some(tab) = self.tabs.get_mut(idx) {
            engine_bridge::stop_engine(tab);
        }
        self.tabs.remove(idx);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }

    fn start_all(&mut self) {
        for tab in &mut self.tabs {
            if tab.status != TabStatus::Running && tab.torrent_data.is_some() {
                match engine_bridge::start_engine(tab, &self.runtime) {
                    Ok(()) => tab.add_log("Engine started".into()),
                    Err(e) => tab.add_log(format!("Start failed: {e}")),
                }
            }
        }
    }

    fn stop_all(&mut self) {
        for tab in &mut self.tabs {
            if tab.status == TabStatus::Running {
                engine_bridge::stop_engine(tab);
                tab.add_log("Engine stopped".into());
            }
        }
    }

    fn update_all(&mut self) {
        for tab in &self.tabs {
            engine_bridge::force_announce(tab, &self.runtime);
        }
        for tab in &mut self.tabs {
            if tab.status == TabStatus::Running {
                tab.add_log("Force announce requested".into());
            }
        }
    }

    fn clear_all_logs(&mut self) {
        for tab in &mut self.tabs {
            tab.log_entries.clear();
        }
    }

    fn open_file_dialog(&mut self, ctx: &egui::Context) {
        if self.file_dialog_open {
            return;
        }
        self.file_dialog_open = true;
        let tx = self.file_channel.0.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let file = rfd::FileDialog::new()
                .add_filter("Torrent Files", &["torrent"])
                .add_filter("All Files", &["*"])
                .pick_file();
            let _ = tx.send(file);
            ctx.request_repaint();
        });
    }

    fn check_file_dialog(&mut self) {
        if let Ok(result) = self.file_channel.1.try_recv() {
            self.file_dialog_open = false;
            if let Some(path) = result {
                self.pending_file = Some(path);
            }
        }
    }

    fn handle_pending_file(&mut self) {
        if let Some(path) = self.pending_file.take() {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                match engine_bridge::load_torrent(tab, path) {
                    Ok(()) => {
                        tab.add_log(format!(
                            "Loaded: {} ({})",
                            tab.torrent_name,
                            format_bytes(tab.total_size)
                        ));
                    }
                    Err(e) => {
                        tab.add_log(format!("Error: {e}"));
                    }
                }
            }
        }
    }

    fn save_log(tab: &TorrentTab) {
        let content: String = tab
            .log_entries
            .iter()
            .map(|e| format!("[{}] {}", e.timestamp, e.message))
            .collect::<Vec<_>>()
            .join("\n");

        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Log Files", &["log", "txt"])
                .set_file_name("ratiomaster.log")
                .save_file()
            {
                let _ = std::fs::write(path, content);
            }
        });
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::N) {
                self.add_tab();
            }
            if i.modifiers.command && i.key_pressed(egui::Key::W) && self.tabs.len() > 1 {
                let idx = self.active_tab;
                if let Some(tab) = self.tabs.get_mut(idx) {
                    engine_bridge::stop_engine(tab);
                }
                self.tabs.remove(idx);
                if self.active_tab >= self.tabs.len() {
                    self.active_tab = self.tabs.len() - 1;
                }
            }
        });

        let open_requested = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O));
        if open_requested {
            self.open_file_dialog(ctx);
        }

        let toggle_theme = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::T));
        if toggle_theme {
            let is_dark = ctx.style().visuals.dark_mode;
            let new_theme = if is_dark { Theme::Light } else { Theme::Dark };
            self.theme = new_theme;
            self.settings.theme = new_theme;
            ctx.set_theme(egui::ThemePreference::from(if is_dark {
                egui::Theme::Light
            } else {
                egui::Theme::Dark
            }));
            theme::apply_theme(ctx, new_theme);
        }
    }
}

impl eframe::App for RatioMasterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for tab in &mut self.tabs {
            tab.poll_engine_state();
        }

        self.check_file_dialog();
        self.handle_pending_file();
        self.handle_keyboard_shortcuts(ctx);

        if self.tabs.iter().any(|t| t.status == TabStatus::Running) {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }

        // Dialogs
        self.about.show(ctx);
        if self.settings.show(ctx) {
            self.theme = self.settings.theme;
            theme::apply_theme(ctx, self.theme);
        }

        // ─── MENU BAR ───
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // New
                if ui.button("New").clicked() {
                    self.add_tab();
                }

                // Current
                ui.menu_button("Current", |ui| {
                    if ui.button("Remove").clicked() {
                        ui.close_menu();
                        let idx = self.active_tab;
                        self.close_tab(idx);
                    }
                    ui.separator();
                    if ui.button("Start").clicked() {
                        ui.close_menu();
                        let idx = self.active_tab;
                        if let Some(tab) = self.tabs.get_mut(idx) {
                            match engine_bridge::start_engine(tab, &self.runtime) {
                                Ok(()) => tab.add_log("Engine started".into()),
                                Err(e) => tab.add_log(format!("Start failed: {e}")),
                            }
                        }
                    }
                    if ui.button("Manual Update").clicked() {
                        ui.close_menu();
                        let idx = self.active_tab;
                        if let Some(tab) = self.tabs.get(idx) {
                            engine_bridge::force_announce(tab, &self.runtime);
                        }
                        if let Some(tab) = self.tabs.get_mut(idx) {
                            tab.add_log("Force announce requested".into());
                        }
                    }
                    if ui.button("Stop").clicked() {
                        ui.close_menu();
                        let idx = self.active_tab;
                        if let Some(tab) = self.tabs.get_mut(idx) {
                            engine_bridge::stop_engine(tab);
                            tab.add_log("Engine stopped".into());
                        }
                    }
                });

                // All RatioMasters
                ui.menu_button("All RatioMasters", |ui| {
                    if ui.button("Start all").clicked() {
                        ui.close_menu();
                        self.start_all();
                    }
                    if ui.button("Stop all").clicked() {
                        ui.close_menu();
                        self.stop_all();
                    }
                    if ui.button("Update all").clicked() {
                        ui.close_menu();
                        self.update_all();
                    }
                    ui.separator();
                    if ui.button("Clear all logs").clicked() {
                        ui.close_menu();
                        self.clear_all_logs();
                    }
                });

                // Settings
                ui.menu_button("Settings", |ui| {
                    if ui.button("Settings...").clicked() {
                        ui.close_menu();
                        self.settings.open = true;
                    }
                    ui.separator();
                    let is_dark = ctx.style().visuals.dark_mode;
                    let theme_label = if is_dark {
                        "Switch to Light Mode"
                    } else {
                        "Switch to Dark Mode"
                    };
                    if ui.button(theme_label).clicked() {
                        ui.close_menu();
                        let new_theme = if is_dark { Theme::Light } else { Theme::Dark };
                        self.theme = new_theme;
                        self.settings.theme = new_theme;
                        ctx.set_theme(egui::ThemePreference::from(if is_dark {
                            egui::Theme::Light
                        } else {
                            egui::Theme::Dark
                        }));
                        theme::apply_theme(ctx, new_theme);
                    }
                });

                // Help
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        ui.close_menu();
                        self.about.open = true;
                    }
                    ui.separator();
                    ui.hyperlink_to("GitHub", "https://github.com/xbattlax/ratiomaster-rs");
                });

                // Exit
                if ui.button("Exit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });

        // ─── TAB BAR ───
        egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut close_idx = None;
                for (i, tab) in self.tabs.iter().enumerate() {
                    let selected = i == self.active_tab;
                    let label = if tab.torrent_name.is_empty() {
                        &tab.name
                    } else {
                        &tab.torrent_name
                    };

                    let text = if selected {
                        egui::RichText::new(label).strong()
                    } else {
                        egui::RichText::new(label)
                    };

                    if ui.selectable_label(selected, text).clicked() {
                        self.active_tab = i;
                    }

                    if self.tabs.len() > 1 && ui.small_button("x").clicked() {
                        close_idx = Some(i);
                    }
                    ui.separator();
                }

                if ui.button("+").clicked() {
                    self.add_tab();
                }

                if let Some(idx) = close_idx {
                    self.close_tab(idx);
                }
            });
        });

        // ─── STATUS BAR ───
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    let next = tab
                        .next_announce_secs()
                        .map(|s| format!("{s}s"))
                        .unwrap_or_else(|| "-".into());
                    ui.label(format!("Update in: {next}"));
                    ui.separator();
                    ui.label(format!("Uploaded: {}", format_bytes(tab.uploaded)));
                    ui.separator();
                    ui.label(format!("Downloaded: {}", format_bytes(tab.downloaded)));
                    ui.separator();
                    let ratio = tab.ratio();
                    ui.colored_label(theme::ratio_color(ratio), format!("Ratio: {ratio:.3}"));
                    ui.separator();
                    ui.label(format!("Seeders: {}", tab.seeders));
                    ui.separator();
                    ui.label(format!("Leechers: {}", tab.leechers));
                    ui.separator();
                    ui.label(format!(
                        "Total time: {}",
                        tab.running_time()
                            .map(format_duration)
                            .unwrap_or_else(|| "00:00:00".into())
                    ));
                }
            });
        });

        // ─── MAIN CONTENT (two columns) ───
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.tabs.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("No tabs open. Click + or New to add one.");
                });
                return;
            }

            let tab_idx = self.active_tab;
            let mut open_file_requested = false;
            let mut control_action = ControlAction::None;
            let mut log_action = LogAction::None;

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if let Some(tab) = self.tabs.get_mut(tab_idx) {
                        // Two-column layout
                        ui.columns(2, |cols| {
                            // ─── LEFT COLUMN ───
                            let left = &mut cols[0];

                            // Panel 1 + 4: Torrent File + Info
                            torrent::show(left, tab, &mut open_file_requested);
                            left.add_space(4.0);

                            // Panel 6: Client Settings (includes stop condition)
                            client::show(left, tab);
                            left.add_space(4.0);

                            // Panel 5: Speed
                            speed::show(left, tab);
                            left.add_space(4.0);

                            // Panel 7: Custom Values
                            custom::show(left, tab);

                            // ─── RIGHT COLUMN ───
                            let right = &mut cols[1];

                            // Panel 3: Proxy
                            proxy::show(right, tab);
                            right.add_space(4.0);

                            // Panel 2: Options
                            options::show(right, tab);
                            right.add_space(4.0);

                            // Panel 9: Random Speeds on Next Update
                            random_next::show(right, tab);
                            right.add_space(4.0);

                            // Panel 8: Log
                            log_action = log::show(right, tab);
                        });

                        ui.add_space(4.0);

                        // Controls below both columns
                        control_action = controls::show(ui, tab);
                        ui.add_space(2.0);

                        // Stats
                        stats::show(ui, tab);
                    }
                });

            // Handle actions
            if open_file_requested {
                self.open_file_dialog(ctx);
            }

            match control_action {
                ControlAction::Start => {
                    if let Some(tab) = self.tabs.get_mut(tab_idx) {
                        match engine_bridge::start_engine(tab, &self.runtime) {
                            Ok(()) => tab.add_log("Engine started".into()),
                            Err(e) => tab.add_log(format!("Start failed: {e}")),
                        }
                    }
                }
                ControlAction::Stop => {
                    if let Some(tab) = self.tabs.get_mut(tab_idx) {
                        engine_bridge::stop_engine(tab);
                        tab.add_log("Engine stopped".into());
                    }
                }
                ControlAction::ForceAnnounce => {
                    if let Some(tab) = self.tabs.get(tab_idx) {
                        engine_bridge::force_announce(tab, &self.runtime);
                    }
                    if let Some(tab) = self.tabs.get_mut(tab_idx) {
                        tab.add_log("Force announce requested".into());
                    }
                }
                ControlAction::SetDefaults => {
                    if let Some(tab) = self.tabs.get_mut(tab_idx) {
                        tab.set_defaults();
                        tab.add_log("Reset to default values".into());
                    }
                }
                ControlAction::None => {}
            }

            match log_action {
                LogAction::Clear => {
                    if let Some(tab) = self.tabs.get_mut(tab_idx) {
                        tab.log_entries.clear();
                    }
                }
                LogAction::Save => {
                    if let Some(tab) = self.tabs.get(tab_idx) {
                        Self::save_log(tab);
                    }
                }
                LogAction::None => {}
            }
        });
    }
}
