#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod dialogs;
mod engine_bridge;
mod panels;
mod tabs;
mod theme;

fn load_icon() -> Option<eframe::egui::IconData> {
    let png_bytes = include_bytes!("../../../assets/icon.png");
    let img = image::load_from_memory(png_bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Some(eframe::egui::IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    })
}

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("ratiomaster=info")
        .init();

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("RatioMaster")
        .with_inner_size([1100.0, 800.0])
        .with_min_inner_size([800.0, 600.0]);

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "RatioMaster",
        options,
        Box::new(|cc| Ok(Box::new(app::RatioMasterApp::new(cc)))),
    )
}
