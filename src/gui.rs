use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use crate::anonymizer::{run_processing, ProgressMessage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabState {
    Clean,
    Logs,
    Settings,
}

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
                self.add_path(entry.path()); // Recursive: add_path calls add_directory for subdirs
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

    fn render_clean_tab(&mut self, ui: &mut egui::Ui) {
        let card_height = 180.0;

        // Check if user is dragging files over the window
        let is_dragging = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());

        let primary = ui.visuals().widgets.active.bg_fill;
        let frame_fill = if is_dragging {
            // Tint the surface with a hint of primary color
            let base = ui.visuals().widgets.inactive.bg_fill;
            egui::Color32::from_rgba_unmultiplied(
                ((base.r() as u16 + primary.r() as u16 / 4) as u8).min(255),
                ((base.g() as u16 + primary.g() as u16 / 4) as u8).min(255),
                ((base.b() as u16 + primary.b() as u16 / 4) as u8).min(255),
                255,
            )
        } else {
            ui.visuals().widgets.inactive.bg_fill
        };

        let frame_stroke = if is_dragging {
            egui::Stroke::new(2.0, primary)
        } else {
            egui::Stroke::NONE
        };

        let frame = egui::Frame::group(ui.style())
            .fill(frame_fill)
            .stroke(frame_stroke)
            .rounding(egui::Rounding::same(20.0))
            .inner_margin(egui::Margin::same(16.0));

        if is_dragging {
            ui.ctx().request_repaint();
        }

        frame.show(ui, |ui| {
            ui.set_min_size(egui::vec2(ui.available_width(), card_height));
            
            if self.selected_files.is_empty() {
                // FIX: clip rect to card_height, not entire remaining window height
                let full_rect = ui.available_rect_before_wrap();
                let click_rect = egui::Rect::from_min_size(
                    full_rect.min,
                    egui::vec2(full_rect.width(), card_height),
                );

                // Draw text centered vertically within the card height
                ui.vertical_centered(|ui| {
                    let text_height = 56.0; // icon line + label line
                    let space = ((card_height - text_height) / 2.0).max(0.0);
                    ui.add_space(space);

                    ui.label(
                        egui::RichText::new(egui_material_icons::icons::ICON_FOLDER)
                            .size(28.0)
                            .color(ui.visuals().widgets.inactive.fg_stroke.color),
                    );
                    ui.label(
                        egui::RichText::new("Drag & Drop or Click to Add Files")
                            .size(16.0)
                            .color(ui.visuals().widgets.inactive.fg_stroke.color),
                    );
                });

                // Click/hover response on exactly the card rect
                let bg_response = ui.allocate_rect(click_rect, egui::Sense::click());

                if bg_response.hovered() && !self.is_processing {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                if bg_response.clicked() && !self.is_processing {
                    self.pick_files_or_folders();
                }
            } else {
                // Files selected: Header add files button + list of files
                ui.vertical_centered(|ui| {
                    let header_label = egui::RichText::new(format!("{} Click to add more files / Drag & Drop here", egui_material_icons::icons::ICON_ADD))
                        .size(13.0)
                        .color(ui.visuals().widgets.active.bg_fill); // Primary accent text
                    
                    let btn_response = ui.add(egui::Button::new(header_label).frame(false));
                    
                    if btn_response.hovered() && !self.is_processing {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    
                    if btn_response.clicked() && !self.is_processing {
                        self.pick_files_or_folders();
                    }
                });
                
                ui.add_space(10.0);

                // Scroll area inside the card showing selected files
                egui::ScrollArea::vertical()
                    .max_height(160.0)
                    .show(ui, |ui| {
                        let mut file_to_remove = None;
                        for (idx, path) in self.selected_files.iter().enumerate() {
                            ui.horizontal(|ui| {
                                if !self.is_processing {
                                    let del_btn = ui.small_button(egui_material_icons::icons::ICON_CLOSE);
                                    if del_btn.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
                                    if del_btn.clicked() {
                                        file_to_remove = Some(idx);
                                    }
                                }
                                ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                            });
                        }
                        if let Some(idx) = file_to_remove {
                            self.selected_files.remove(idx);
                        }
                    });
            }
        });
        ui.add_space(20.0);

        // Actions panel
        ui.horizontal(|ui| {
            let is_start_enabled = !self.selected_files.is_empty() && !self.is_processing;
            let primary = ui.visuals().widgets.active.bg_fill;
            let on_primary = ui.visuals().widgets.active.fg_stroke.color;
            let start_btn = ui.add_enabled(
                is_start_enabled,
                egui::Button::new(
                    egui::RichText::new(format!("{} Start Cleaning", egui_material_icons::icons::ICON_ROCKET_LAUNCH))
                        .color(if is_start_enabled { on_primary } else { ui.visuals().widgets.noninteractive.fg_stroke.color })
                )
                .fill(if is_start_enabled { primary } else { ui.visuals().widgets.noninteractive.bg_fill })
                .min_size(egui::vec2(130.0, 36.0))
                .rounding(egui::Rounding::same(18.0))
            );

            if start_btn.hovered() && is_start_enabled {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            if start_btn.clicked() {
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

                    // Reset the channel so stale messages from a previous run don't leak in
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

            if !self.selected_files.is_empty() && !self.is_processing {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let clear_btn = ui.button(format!("{} Clear List", egui_material_icons::icons::ICON_DELETE));
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

        ui.add_space(20.0);

        // Progress bar and status
        if self.is_processing || self.progress > 0.0 {
            ui.horizontal(|ui| {
                ui.label(&self.status_text);
            });
            
            // Progress bar percentage text — use theme on_primary color
            let on_primary = ui.visuals().widgets.active.fg_stroke.color;
            let pct_text = egui::RichText::new(format!("{}%", (self.progress * 100.0) as i32))
                .color(on_primary)
                .strong();

            ui.add(egui::ProgressBar::new(self.progress).text(pct_text));
            ui.add_space(15.0);
        }

        // Completed banner card
        if let Some(output_path) = &self.last_output_path {
            if !self.is_processing {
                egui::Frame::group(ui.style())
                    .fill(ui.visuals().widgets.inactive.bg_fill)
                    .stroke(egui::Stroke::NONE) // NO OUTLINE BORDER!
                    .rounding(egui::Rounding::same(16.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(137, 230, 150), format!("{} Done!", egui_material_icons::icons::ICON_CHECK_CIRCLE));
                            ui.label("Saved to:");
                            
                            // File path styled bold white
                            let file_name = output_path.file_name().unwrap_or_default().to_string_lossy();
                            ui.label(egui::RichText::new(file_name).color(egui::Color32::WHITE).strong());
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                 let open_btn = ui.button(format!("{} Open Folder", egui_material_icons::icons::ICON_FOLDER_OPEN));
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

    fn render_logs_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{} Process Log", egui_material_icons::icons::ICON_RECEIPT_LONG)).size(16.0).color(egui::Color32::WHITE));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.logs.is_empty() {
                    let clear_btn = ui.button(format!("{} Clear Logs", egui_material_icons::icons::ICON_DELETE));
                    if clear_btn.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if clear_btn.clicked() {
                        self.logs.clear();
                    }
                }
            });
        });
        ui.add_space(10.0);

        let available_height = ui.available_height() - 10.0;

        let log_frame = egui::Frame::group(ui.style())
            .fill(ui.visuals().extreme_bg_color)
            .stroke(egui::Stroke::NONE)
            .rounding(egui::Rounding::same(16.0))
            .inner_margin(egui::Margin::same(12.0));

        log_frame.show(ui, |ui| {
            if self.logs.is_empty() {
                // Empty state placeholder
                let h = available_height - 24.0;
                ui.allocate_ui(egui::vec2(ui.available_width(), h), |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("No logs yet. Start processing files to see progress here.")
                                .color(ui.visuals().weak_text_color())
                                .size(13.0),
                        );
                    });
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
                                .text_color(egui::Color32::LIGHT_GREEN)
                                .desired_width(f32::INFINITY)
                                .desired_rows(22)
                                .frame(false),
                        );
                    });
            }
        });
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new(format!("{} Anonymization Settings", egui_material_icons::icons::ICON_SETTINGS)).size(16.0).color(egui::Color32::WHITE));
        ui.add_space(10.0);

        let frame = egui::Frame::group(ui.style())
            .fill(ui.visuals().widgets.inactive.bg_fill)
            .stroke(egui::Stroke::NONE)
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(egui::Margin::same(16.0));

         frame.show(ui, |ui| {
             ui.set_min_size(egui::vec2(ui.available_width(), 0.0));
             
             ui.vertical(|ui| {
                 ui.add_space(4.0);
                 settings_row(ui, "Anonymize Participant Names: [Participant N]", &mut self.hide_names);
                 settings_row(ui, "Mask Phone Numbers: [PHONE]", &mut self.hide_phones);
                 settings_row(ui, "Mask Email Addresses: [EMAIL]", &mut self.hide_emails);
                 settings_row(ui, "Mask Web Links / URLs: [LINK]", &mut self.hide_links);
                 settings_row(ui, "Mask Credit / Debit Card Numbers: [CARD]", &mut self.hide_cards);
                 settings_row(ui, "Mask Physical Addresses: [ADDRESS]", &mut self.hide_addresses);
                 settings_row(ui, "Mask API Keys / Access Tokens: [TOKEN]", &mut self.hide_tokens);
             });
         });
     }
}

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

        // Repaint immediately during background processing to keep logs/progress bars smooth
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

        // Left SidePanel for Sidebar navigation
        egui::SidePanel::left("sidebar_panel")
            .resizable(false)
            .default_width(120.0)
            .show_separator_line(false) // Remove white separator line
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(18, 19, 21))) // Darker background
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    
                    // Title abbreviation in bold white
                    ui.label(
                        egui::RichText::new("TgHMC")
                            .font(egui::FontId::new(22.0, egui::FontFamily::Name("bold".into())))
                            .color(egui::Color32::WHITE)
                    );
                    ui.add_space(25.0);

                    // Tabs
                    let tabs = [
                        (TabState::Clean, format!("{} Clean", egui_material_icons::icons::ICON_ROCKET_LAUNCH)),
                        (TabState::Logs, format!("{} Logs", egui_material_icons::icons::ICON_RECEIPT_LONG)),
                        (TabState::Settings, format!("{} Settings", egui_material_icons::icons::ICON_SETTINGS)),
                    ];

                     for (tab, label) in tabs {
                         let is_active = self.active_tab == tab;
                         
                         let btn_color = if is_active {
                             ui.visuals().widgets.active.bg_fill
                         } else {
                             egui::Color32::TRANSPARENT
                         };
                         
                         let text_color = if is_active {
                             ui.visuals().widgets.active.fg_stroke.color
                         } else {
                             ui.visuals().widgets.inactive.fg_stroke.color
                         };
 
                         let response = ui.add(
                             egui::Button::new(
                                 egui::RichText::new(label)
                                     .size(13.0)
                                     .color(text_color)
                             )
                             .fill(btn_color)
                             .min_size(egui::vec2(100.0, 36.0))
                             .rounding(egui::Rounding::same(18.0)) // Pill rounding
                         );
 
                         if response.hovered() {
                             ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                         }
 
                         if response.clicked() {
                             self.active_tab = tab;
                         }
                         
                         ui.add_space(10.0);
                     }
                });
            });

        // Right CentralPanel for Main view
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(ctx.style().visuals.panel_fill)
                .inner_margin(egui::Margin::same(16.0))) // Normal Surface background with padding
            .show(ctx, |ui| {
                ui.add_space(10.0);
                
                match self.active_tab {
                    TabState::Clean => {
                        self.render_clean_tab(ui);
                    }
                    TabState::Logs => {
                        self.render_logs_tab(ui);
                    }
                    TabState::Settings => {
                        self.render_settings_tab(ui);
                    }
                }
            });
    }
}

pub fn settings_row(ui: &mut egui::Ui, label: &str, on: &mut bool) {
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(egui::RichText::new(label).size(15.0).color(egui::Color32::from_rgb(230, 230, 230)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            let desired_size = egui::vec2(44.0, 24.0);
            let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
            if response.clicked() {
                *on = !*on;
                response.mark_changed();
            }
            if response.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }

            if ui.is_rect_visible(rect) {
                let how_on = ui.ctx().animate_bool(response.id, *on);
                let radius = 0.5 * rect.height();
                
                let bg_color = if *on {
                    ui.visuals().widgets.active.bg_fill
                } else {
                    egui::Color32::from_rgb(60, 65, 70) // Dark grey for off state track
                };
                
                ui.painter().rect(rect, radius, bg_color, egui::Stroke::NONE);
                
                let thumb_color = if *on {
                    ui.visuals().widgets.active.fg_stroke.color
                } else {
                    ui.visuals().widgets.inactive.fg_stroke.color
                };
                let circle_x = egui::lerp((rect.left() + radius + 2.0)..=(rect.right() - radius - 2.0), how_on);
                let center = egui::pos2(circle_x, rect.center().y);
                ui.painter().circle(center, radius - 4.0, thumb_color, egui::Stroke::NONE);
            }
        });
    });
    ui.add_space(12.0);
}
