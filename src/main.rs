#![windows_subsystem = "windows"]

mod anonymizer;
mod gui;
mod theme;

use eframe::egui;

const APP_NAME: &str = "Telegram HTML → Markdown Cleaner";
const APP_ID: &str = "tg-anonymizer";
const WINDOW_WIDTH: f32 = 640.0;
const WINDOW_HEIGHT: f32 = 560.0;
const WINDOW_MIN_WIDTH: f32 = 520.0;
const WINDOW_MIN_HEIGHT: f32 = 480.0;

fn load_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../resources/tg-anonymizer.png");
    let image = image::load_from_memory(icon_bytes).ok()?;
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn main() {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(APP_NAME)
        .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
        .with_min_inner_size([WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT])
        .with_drag_and_drop(true);

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let _ = eframe::run_native(
        APP_ID,
        options,
        Box::new(|cc| {
            theme::setup_custom_style(&cc.egui_ctx);
            Ok(Box::new(gui::AppState::new()))
        }),
    );
}
