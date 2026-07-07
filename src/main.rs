#![windows_subsystem = "windows"]

mod anonymizer;
mod gui;
mod theme;

use eframe::egui;

const APP_NAME: &str = "Telegram HTML → Markdown Cleaner";
const APP_ID: &str = "tg-anonymizer";
const WINDOW_WIDTH: f32 = 600.0;
const WINDOW_HEIGHT: f32 = 520.0;
const WINDOW_MIN_WIDTH: f32 = 500.0;
const WINDOW_MIN_HEIGHT: f32 = 450.0;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_min_inner_size([WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT])
            .with_drag_and_drop(true),
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
