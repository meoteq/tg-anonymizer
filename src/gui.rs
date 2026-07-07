use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use crate::anonymizer::{run_processing, ProgressMessage};
use crate::theme::{Md3Colors, Md3Shape, Md3Spacing, Md3Type};

// ─── Tab State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabState {
    Clean,
    Logs,
    Settings,
}

// ─── Application State ─────────────────────────────────────────────────────

pub struct AppState {
    pub selected_files: Vec<PathBuf>,
    pub logs: Vec<String>,
    pub status_text: String,
    pub progress: f32,
    pub is_processing: bool,

    // Channels for communicating with background thread
    pub rx: Receiver<ProgressMessage>,

    // Anonymization output result path
    pub last_output_path: Option<PathBuf>,
    pub active_tab: TabState,

    // Anonymization Settings flags
    pub hide_names: bool,
    pub hide_phones: bool,
    pub hide_emails: bool,
    pub hide_links: bool,
    pub hide_cards: bool,
    pub hide_addresses: bool,
    pub hide_tokens: bool,
}

impl AppState {
    pub fn new() -> Self {
        let (_tx, rx) = channel();
        Self {
            selected_files: Vec::new(),
            logs: Vec::new(),
            status_text: "Ready".to_string(),
            progress: 0.0,
            is_processing: false,
            rx,
            last_output_path: None,
            active_tab: TabState::Clean,

            // Enabled by default (conservative: protect all sensitive data)
            hide_names: true,
            hide_phones: true,
            hide_emails: true,
            hide_links: true,
            hide_cards: true,
            hide_addresses: true,
            hide_tokens: true,
        }
    }

    pub fn add_path(&mut self, path: PathBuf) {
        if path.is_file() {
            if path.extension().map_or(false, |ext| ext == "html" || ext == "htm") {
                if !self.selected_files.contains(&path) {
                    self.selected_files.push(path);
                }
            }
        } else if path.is_dir() {
            self.add_directory(path);
        }
    }

    pub fn add_directory(&mut self, dir: PathBuf) {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                self.add_path(entry.path());
            }
        }
    }

    fn pick_files_or_folders(&mut self) {
        if let Some(files) = rfd::FileDialog::new()
            .add_filter("Telegram HTML Exports", &["html", "htm"])
            .pick_files()
        {
            for file in files {
                self.add_path(file);
            }
        }
    }

    // ─── Tab Rendering ──────────────────────────────────────────────────

    fn render_clean_tab(&mut self, ui: &mut egui::Ui) {
        let c = Md3Colors::dark();
        let s = Md3Shape::default();
        let sp = Md3Spacing::default();
        let t = Md3Type::default();

        let card_height = 190.0;

        // Check if user is dragging files over the window
        let is_dragging = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());

        // Instant drop zone highlighting (no springs)
        let frame_stroke = if is_dragging {
            egui::Stroke::new(2.0, c.primary)
        } else {
            egui::Stroke::NONE
        };

        let frame_fill = if is_dragging {
            c.primary_container
        } else {
            c.surface_container_high
        };

        let frame = egui::Frame::none()
            .fill(frame_fill)
            .stroke(frame_stroke)
            .rounding(egui::Rounding::same(s.medium))
            .inner_margin(egui::Margin::same(sp.lg));

        if is_dragging {
            ui.ctx().request_repaint();
        }

        frame.show(ui, |ui| {
            ui.set_min_size(egui::vec2(ui.available_width(), card_height));

            if self.selected_files.is_empty() {
                // ── Empty state: icon + label ──
                let full_rect = ui.available_rect_before_wrap();
                let click_rect = egui::Rect::from_min_size(
                    full_rect.min,
                    egui::vec2(full_rect.width(), card_height),
                );

                ui.vertical_centered(|ui| {
                    let text_height = 72.0;
                    let space = ((card_height - text_height) / 2.0).max(0.0);
                    ui.add_space(space);

                    ui.label(
                        egui::RichText::new(egui_material_icons::icons::ICON_UPLOAD_FILE)
                            .size(36.0)
                            .color(c.on_surface_variant),
                    );
                    ui.add_space(sp.sm);
                    ui.label(
                        egui::RichText::new("Drag & Drop or Click to Add Files")
                            .size(t.title_medium)
                            .color(c.on_surface_variant),
                    );
                    ui.label(
                        egui::RichText::new("Supports .html and .htm files")
                            .size(t.body_small)
                            .color(c.outline),
                    );
                });

                let bg_response = ui.allocate_rect(click_rect, egui::Sense::click());
                if bg_response.hovered() && !self.is_processing {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if bg_response.clicked() && !self.is_processing {
                    self.pick_files_or_folders();
                }
            } else {
                // ── Files selected: header + list ──
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} {} file{} selected",
                            egui_material_icons::icons::ICON_DESCRIPTION,
                            self.selected_files.len(),
                            if self.selected_files.len() == 1 { "" } else { "s" }
                        ))
                        .size(t.title_small)
                        .color(c.on_surface),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let add_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new(format!("{} Add more", egui_material_icons::icons::ICON_ADD))
                                    .size(t.label_large)
                                    .color(c.on_secondary_container),
                            )
                            .fill(c.secondary_container)
                            .rounding(egui::Rounding::same(s.full.min(100.0)))
                            .min_size(egui::vec2(0.0, 32.0)),
                        );
                        if add_btn.hovered() && !self.is_processing {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if add_btn.clicked() && !self.is_processing {
                            self.pick_files_or_folders();
                        }
                    });
                });

                ui.add_space(sp.sm);

                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| {
                        let mut file_to_remove = None;
                        for (idx, path) in self.selected_files.iter().enumerate() {
                            let file_frame = egui::Frame::none()
                                .fill(c.surface_container)
                                .rounding(egui::Rounding::same(s.small))
                                .inner_margin(egui::Margin::symmetric(sp.md, sp.xs + 2.0));

                            file_frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(egui_material_icons::icons::ICON_INSERT_DRIVE_FILE)
                                            .size(t.body_medium)
                                            .color(c.primary),
                                    );
                                    ui.label(
                                        egui::RichText::new(
                                            path.file_name().unwrap_or_default().to_string_lossy(),
                                        )
                                        .size(t.body_medium)
                                        .color(c.on_surface),
                                    );

                                    if !self.is_processing {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let del_btn = ui.add(
                                                    egui::Button::new(
                                                        egui::RichText::new(egui_material_icons::icons::ICON_CLOSE)
                                                            .size(t.body_medium)
                                                            .color(c.on_surface_variant),
                                                    )
                                                    .frame(false),
                                                );
                                                if del_btn.hovered() {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                }
                                                if del_btn.clicked() {
                                                    file_to_remove = Some(idx);
                                                }
                                            },
                                        );
                                    }
                                });
                            });
                            ui.add_space(sp.xs);
                        }
                        if let Some(idx) = file_to_remove {
                            self.selected_files.remove(idx);
                        }
                    });
            }
        });

        ui.add_space(sp.lg);

        // ── Actions Panel ──
        ui.horizontal(|ui| {
            let is_start_enabled = !self.selected_files.is_empty() && !self.is_processing;

            let btn_fill = if is_start_enabled { c.primary } else { c.surface_container_high };
            let btn_text_color = if is_start_enabled { c.on_primary } else { c.outline };

            let start_btn = ui.add_enabled(
                is_start_enabled,
                egui::Button::new(
                    egui::RichText::new(format!(
                        "{} Start Cleaning",
                        egui_material_icons::icons::ICON_ROCKET_LAUNCH
                    ))
                    .size(Md3Type::default().label_large)
                    .color(btn_text_color),
                )
                .fill(btn_fill)
                .min_size(egui::vec2(160.0, 40.0))
                .rounding(egui::Rounding::same(s.full.min(100.0))),
            );

            if start_btn.hovered() && is_start_enabled {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            if start_btn.clicked() {
                self.start_processing();
            }

            if !self.selected_files.is_empty() && !self.is_processing {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let clear_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new(format!(
                                "{} Clear",
                                egui_material_icons::icons::ICON_DELETE_SWEEP
                            ))
                            .size(Md3Type::default().label_large)
                            .color(c.primary),
                        )
                        .frame(false),
                    );
                    if clear_btn.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if clear_btn.clicked() {
                        self.selected_files.clear();
                        self.last_output_path = None;
                        self.progress = 0.0;
                        self.status_text = "File list cleared.".to_string();
                    }
                });
            }
        });

        ui.add_space(sp.lg);

        // ── Progress ──
        if self.is_processing || self.progress > 0.0 {
            ui.label(
                egui::RichText::new(&self.status_text)
                    .size(Md3Type::default().body_medium)
                    .color(c.on_surface_variant),
            );
            ui.add_space(sp.xs);

            // Pass immediate self.progress (no springs)
            self.render_md3_progress_bar(ui, self.progress, &c, &s);

            ui.add_space(sp.sm);
        }

        // ── Completion Banner ──
        if self.last_output_path.is_some() && !self.is_processing {
            if let Some(output_path) = self.last_output_path.clone() {
                let banner_frame = egui::Frame::none()
                    .fill(c.surface_container_high)
                    .rounding(egui::Rounding::same(s.medium))
                    .inner_margin(egui::Margin::same(sp.lg));

                banner_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(egui_material_icons::icons::ICON_CHECK_CIRCLE)
                                .size(Md3Type::default().title_medium + 4.0)
                                .color(c.success),
                        );
                        ui.add_space(sp.sm);

                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("Processing Complete")
                                    .size(Md3Type::default().title_small)
                                    .color(c.on_surface)
                                    .family(egui::FontFamily::Name("bold".into())),
                            );
                            let file_name = output_path.file_name().unwrap_or_default().to_string_lossy();
                            ui.label(
                                egui::RichText::new(format!("Saved to: {}", file_name))
                                    .size(Md3Type::default().body_medium)
                                    .color(c.on_surface_variant),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let open_btn = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(format!(
                                        "{} Open Folder",
                                        egui_material_icons::icons::ICON_FOLDER_OPEN
                                    ))
                                    .size(Md3Type::default().label_large)
                                    .color(c.on_secondary_container),
                                )
                                .fill(c.secondary_container)
                                .rounding(egui::Rounding::same(Md3Shape::default().full.min(100.0)))
                                .min_size(egui::vec2(0.0, 36.0)),
                            );
                            if open_btn.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if open_btn.clicked() {
                                if let Some(parent) = output_path.parent() {
                                    #[cfg(target_os = "windows")]
                                    let _ = std::process::Command::new("explorer").arg(parent).spawn();
                                    #[cfg(target_os = "macos")]
                                    let _ = std::process::Command::new("open").arg(parent).spawn();
                                    #[cfg(target_os = "linux")]
                                    let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
                                }
                            }
                        });
                    });
                });
            }
        }
    }

    /// Custom MD3-styled progress bar with tonal colors.
    fn render_md3_progress_bar(
        &self,
        ui: &mut egui::Ui,
        progress: f32,
        c: &Md3Colors,
        s: &Md3Shape,
    ) {
        let desired_size = egui::vec2(ui.available_width(), 8.0);
        let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let rounding = egui::Rounding::same(s.full.min(100.0));

            // Track: surface-container-highest
            ui.painter().rect_filled(rect, rounding, c.surface_container_highest);

            // Active indicator: primary
            if progress > 0.0 {
                let active_rect = egui::Rect::from_min_size(
                    rect.min,
                    egui::vec2(rect.width() * progress, rect.height()),
                );
                ui.painter().rect_filled(active_rect, rounding, c.primary);
            }

            // Percentage label right-aligned below
            let pct_text = format!("{}%", (progress * 100.0) as i32);
            let text_pos = egui::pos2(rect.right() - 30.0, rect.bottom() + 4.0);
            ui.painter().text(
                text_pos,
                egui::Align2::RIGHT_TOP,
                pct_text,
                egui::FontId::proportional(Md3Type::default().label_small),
                c.on_surface_variant,
            );
            ui.add_space(16.0);
        }
    }

    fn render_logs_tab(&mut self, ui: &mut egui::Ui) {
        let c = Md3Colors::dark();
        let s = Md3Shape::default();
        let sp = Md3Spacing::default();
        let t = Md3Type::default();

        // Header: use ICON_DESCRIPTION which works
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{} Process Log",
                    egui_material_icons::icons::ICON_DESCRIPTION
                ))
                .size(t.title_medium)
                .color(c.on_surface)
                .family(egui::FontFamily::Name("bold".into())),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.logs.is_empty() {
                    let clear_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new(format!(
                                "{} Clear",
                                egui_material_icons::icons::ICON_DELETE_SWEEP
                            ))
                            .size(t.label_large)
                            .color(c.primary),
                        )
                        .frame(false),
                    );
                    if clear_btn.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if clear_btn.clicked() {
                        self.logs.clear();
                    }
                }
            });
        });
        ui.add_space(sp.sm);

        let available_height = ui.available_height() - sp.sm;

        let log_frame = egui::Frame::none()
            .fill(c.surface_container_lowest)
            .rounding(egui::Rounding::same(s.medium))
            .inner_margin(egui::Margin::same(sp.md));

        log_frame.show(ui, |ui| {
            ui.set_min_height(available_height);
            if self.logs.is_empty() {
                // Empty state centered vertically and horizontally
                let frame_height = ui.available_height();
                let content_height = 90.0;
                let top_space = ((frame_height - content_height) / 2.0).max(0.0);

                ui.vertical_centered(|ui| {
                    ui.add_space(top_space);
                    ui.label(
                        egui::RichText::new(egui_material_icons::icons::ICON_TERMINAL)
                            .size(36.0)
                            .color(c.outline_variant),
                    );
                    ui.add_space(sp.sm);
                    ui.label(
                        egui::RichText::new("No logs yet")
                            .size(t.body_large)
                            .color(c.outline),
                    );
                    ui.label(
                        egui::RichText::new("Start processing files to see progress here.")
                            .size(t.body_small)
                            .color(c.outline_variant),
                    );
                });
            } else {
                egui::ScrollArea::vertical()
                    .max_height(available_height)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        let mut text = String::new();
                        for log_line in &self.logs {
                            text.push_str(log_line);
                            text.push('\n');
                        }

                        ui.add(
                            egui::TextEdit::multiline(&mut text)
                                .font(egui::TextStyle::Monospace)
                                .text_color(c.tertiary)
                                .desired_width(f32::INFINITY)
                                .desired_rows(22)
                                .frame(false),
                        );
                    });
            }
        });
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        let c = Md3Colors::dark();
        let s = Md3Shape::default();
        let sp = Md3Spacing::default();
        let t = Md3Type::default();

        // Title: use ICON_SETTINGS which works
        ui.label(
            egui::RichText::new(format!(
                "{} Anonymization Settings",
                egui_material_icons::icons::ICON_SETTINGS
            ))
            .size(t.title_medium)
            .color(c.on_surface)
            .family(egui::FontFamily::Name("bold".into())),
        );

        ui.add_space(sp.xs);

        ui.label(
            egui::RichText::new("Choose which types of sensitive data to mask during cleaning.")
                .size(t.body_medium)
                .color(c.on_surface_variant),
        );

        ui.add_space(sp.md);

        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - sp.sm)
            .show(ui, |ui| {
                let frame = egui::Frame::none()
                    .fill(c.surface_container_high)
                    .rounding(egui::Rounding::same(s.medium))
                    .inner_margin(egui::Margin::same(sp.lg));

                frame.show(ui, |ui| {
                    ui.set_min_size(egui::vec2(ui.available_width(), 0.0));

                    ui.vertical(|ui| {
                        ui.add_space(sp.xs);

                        md3_settings_row(ui, egui_material_icons::icons::ICON_PERSON, "Participant Names", "[Participant N]", &mut self.hide_names, &c, &s, &t, &sp);
                        md3_divider(ui, &c);
                        md3_settings_row(ui, egui_material_icons::icons::ICON_PHONE, "Phone Numbers", "[PHONE]", &mut self.hide_phones, &c, &s, &t, &sp);
                        md3_divider(ui, &c);
                        md3_settings_row(ui, egui_material_icons::icons::ICON_EMAIL, "Email Addresses", "[EMAIL]", &mut self.hide_emails, &c, &s, &t, &sp);
                        md3_divider(ui, &c);
                        md3_settings_row(ui, egui_material_icons::icons::ICON_LINK, "Web Links / URLs", "[LINK]", &mut self.hide_links, &c, &s, &t, &sp);
                        md3_divider(ui, &c);
                        md3_settings_row(ui, egui_material_icons::icons::ICON_CREDIT_CARD, "Credit / Debit Cards", "[CARD]", &mut self.hide_cards, &c, &s, &t, &sp);
                        md3_divider(ui, &c);
                        md3_settings_row(ui, egui_material_icons::icons::ICON_LOCATION_ON, "Physical Addresses", "[ADDRESS]", &mut self.hide_addresses, &c, &s, &t, &sp);
                        md3_divider(ui, &c);
                        md3_settings_row(ui, egui_material_icons::icons::ICON_KEY, "API Keys / Tokens", "[TOKEN]", &mut self.hide_tokens, &c, &s, &t, &sp);
                    });
                });
            });
    }

    fn start_processing(&mut self) {
        let default_name = "cleaned_telegram_chat.md";
        let starting_dir = self.selected_files.first()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        if let Some(save_path) = rfd::FileDialog::new()
            .set_directory(starting_dir)
            .set_file_name(default_name)
            .add_filter("Markdown Files", &["md"])
            .save_file()
        {
            self.is_processing = true;
            self.progress = 0.0;
            self.logs.clear();

            let (new_tx, new_rx) = std::sync::mpsc::channel();
            self.rx = new_rx;

            let files = self.selected_files.clone();
            let tx = new_tx;

            let settings = crate::anonymizer::AnonymizeSettings {
                hide_names: self.hide_names,
                hide_phones: self.hide_phones,
                hide_emails: self.hide_emails,
                hide_links: self.hide_links,
                hide_cards: self.hide_cards,
                hide_addresses: self.hide_addresses,
                hide_tokens: self.hide_tokens,
            };

            thread::spawn(move || {
                run_processing(files, save_path, settings, tx);
            });
        }
    }
}

// ─── eframe::App Implementation ─────────────────────────────────────────────

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain incoming progress messages from the background thread
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                ProgressMessage::Log(log_line) => {
                    self.logs.push(log_line);
                }
                ProgressMessage::Progress(val) => {
                    self.progress = val;
                }
                ProgressMessage::Status(status) => {
                    self.status_text = status;
                }
                ProgressMessage::Finished(count, path) => {
                    self.is_processing = false;
                    self.progress = 1.0;
                    self.status_text = format!("Done! Processed {} files.", count);
                    self.last_output_path = Some(path.clone());
                    self.logs.push(format!("[Info] Output saved to: {}", path.display()));
                }
                ProgressMessage::Error(err) => {
                    self.is_processing = false;
                    self.status_text = format!("Error: {}", err);
                    self.logs.push(format!("[Error] {}", err));
                }
            }
        }

        // Repaint only during background processing
        if self.is_processing {
            ctx.request_repaint();
        }

        // Handle file / folder drag and drop
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path {
                        self.add_path(path.clone());
                    }
                }
            }
        });

        let c = Md3Colors::dark();
        let s = Md3Shape::default();
        let sp = Md3Spacing::default();
        let t = Md3Type::default();

        // ── Navigation Drawer Style Sidebar (160dp wide) ──
        egui::SidePanel::left("nav_rail")
            .resizable(false)
            .default_width(160.0)
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(c.surface_container)
                    .inner_margin(egui::Margin::symmetric(sp.md, sp.md)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(sp.sm);

                    // App brand mark: "TgHMC" (Title Large)
                    ui.label(
                        egui::RichText::new("TgHMC")
                            .font(egui::FontId::new(t.title_large, egui::FontFamily::Name("bold".into())))
                            .color(c.primary),
                    );

                    ui.add_space(sp.xl);

                    // Navigation tabs
                    let tabs = [
                        (TabState::Clean, egui_material_icons::icons::ICON_ROCKET_LAUNCH, "Clean"),
                        (TabState::Logs, egui_material_icons::icons::ICON_DESCRIPTION, "Logs"),
                        (TabState::Settings, egui_material_icons::icons::ICON_SETTINGS, "Settings"),
                    ];

                    for (tab, icon, label) in tabs {
                        let is_active = self.active_tab == tab;

                        let pill_bg = if is_active {
                            c.secondary_container
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        let icon_color = if is_active { c.on_secondary_container } else { c.on_surface_variant };
                        let label_color = if is_active { c.on_surface } else { c.on_surface_variant };

                        // Allocate custom rect for horizontal navigation item (LocalSend style)
                        let row_size = egui::vec2(ui.available_width(), 44.0);
                        let (rect, response) = ui.allocate_exact_size(row_size, egui::Sense::click());

                        if response.clicked() {
                            self.active_tab = tab;
                        }

                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        if ui.is_rect_visible(rect) {
                            // Subtle hover background highlight for the pill if not active
                            let pill_color = if response.hovered() && !is_active {
                                c.surface_container_highest
                            } else {
                                pill_bg
                            };

                            // Draw pill background (only around the icon, like LocalSend)
                            let pill_rect = egui::Rect::from_center_size(
                                egui::pos2(rect.left() + 28.0, rect.center().y),
                                egui::vec2(56.0, 32.0)
                            );
                            ui.painter().rect_filled(
                                pill_rect,
                                egui::Rounding::same(s.full.min(100.0)),
                                pill_color
                            );

                            // Draw icon inside the pill
                            ui.painter().text(
                                pill_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                icon,
                                egui::FontId::proportional(t.title_medium),
                                icon_color
                            );

                            // Draw label text to the right of the pill, completely separate
                            let text_pos = egui::pos2(rect.left() + 56.0 + sp.md, rect.center().y);
                            ui.painter().text(
                                text_pos,
                                egui::Align2::LEFT_CENTER,
                                label,
                                egui::FontId::proportional(t.label_large),
                                label_color
                            );
                        }

                        ui.add_space(sp.xs);
                    }
                });
            });

        // ── Central Content Panel ──
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(c.surface)
                    .inner_margin(egui::Margin::same(sp.xl)),
            )
            .show(ctx, |ui| {
                ui.add_space(sp.sm);

                match self.active_tab {
                    TabState::Clean => self.render_clean_tab(ui),
                    TabState::Logs => self.render_logs_tab(ui),
                    TabState::Settings => self.render_settings_tab(ui),
                }
            });
    }
}

// ─── MD3 Helper: Settings Row (MD3 List Item with trailing switch) ──────────

fn md3_settings_row(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    mask_label: &str,
    on: &mut bool,
    c: &Md3Colors,
    s: &Md3Shape,
    t: &Md3Type,
    sp: &Md3Spacing,
) {
    ui.horizontal(|ui| {
        ui.add_space(sp.xs);

        ui.label(
            egui::RichText::new(icon)
                .size(t.title_small + 4.0)
                .color(c.on_surface_variant),
        );

        ui.add_space(sp.md);

        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(title)
                    .size(t.body_large)
                    .color(c.on_surface),
            );
            ui.label(
                egui::RichText::new(mask_label)
                    .size(t.body_small)
                    .color(c.on_surface_variant),
            );
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(sp.sm);
            md3_switch(ui, on, c, s);
        });
    });
    ui.add_space(sp.sm);
}

// ─── MD3 Switch Component ───────────────────────────────────────────────────

fn md3_switch(ui: &mut egui::Ui, on: &mut bool, c: &Md3Colors, s: &Md3Shape) {
    let desired_size = egui::vec2(52.0, 32.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, *on);
        let track_rounding = egui::Rounding::same(s.full.min(100.0));

        let track_color = lerp_color(c.surface_container_highest, c.primary, how_on);

        let track_stroke = if how_on < 0.5 {
            egui::Stroke::new(2.0, c.outline)
        } else {
            egui::Stroke::NONE
        };

        ui.painter().rect(rect, track_rounding, track_color, track_stroke);

        let thumb_radius = egui::lerp(8.0..=12.0, how_on);
        let thumb_color = if how_on > 0.5 { c.on_primary } else { c.outline };

        let thumb_x = egui::lerp(
            (rect.left() + 16.0)..=(rect.right() - 16.0),
            how_on,
        );
        let center = egui::pos2(thumb_x, rect.center().y);
        ui.painter().circle(center, thumb_radius, thumb_color, egui::Stroke::NONE);
    }
}

// ─── MD3 Divider ────────────────────────────────────────────────────────────

fn md3_divider(ui: &mut egui::Ui, c: &Md3Colors) {
    let available_width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(available_width, 1.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, egui::Rounding::ZERO, c.outline_variant);
    }
}

// ─── Color Interpolation Utility ────────────────────────────────────────────

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}
