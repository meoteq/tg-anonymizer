use eframe::egui;

fn load_custom_fonts(fonts: &mut egui::FontDefinitions) {
    let font_bytes = include_bytes!("assets/GoogleSansFlex.ttf");
    fonts.font_data.insert(
        "Google Sans Flex".to_owned(),
        egui::FontData::from_owned(font_bytes.to_vec()).into(),
    );

    let bold_bytes = include_bytes!("assets/GoogleSans-Bold.ttf");
    fonts.font_data.insert(
        "Google Sans Bold".to_owned(),
        egui::FontData::from_owned(bold_bytes.to_vec()).into(),
    );

    if let Some(vec) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        vec.insert(0, "Google Sans Flex".to_owned());
    }

    if let Some(vec) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        vec.insert(0, "Google Sans Flex".to_owned());
    }

    // Custom bold family
    fonts.families.insert(
        egui::FontFamily::Name("bold".into()),
        vec!["Google Sans Bold".to_owned()],
    );

    // Add material icons font
    let mut data = egui::FontData::from_static(egui_material_icons::FONT_DATA);
    data.tweak.y_offset_factor = 0.05;
    fonts.font_data.insert("material-icons".to_owned(), data.into());
    if let Some(vec) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        vec.push("material-icons".to_owned());
    }
    if let Some(vec) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        vec.push("material-icons".to_owned());
    }
}

pub fn setup_custom_style(ctx: &egui::Context) {
    // 1. Setup fonts
    let mut fonts = egui::FontDefinitions::default();
    load_custom_fonts(&mut fonts);
    ctx.set_fonts(fonts);

    // 2. Setup theme and styles
    let mut style = (*ctx.style()).clone();

    // Rounding configuration (Material Design 3 style)
    let card_rounding = egui::Rounding::same(20.0);
    let button_rounding = egui::Rounding::same(24.0);
    
    style.visuals.window_rounding = card_rounding;
    style.visuals.menu_rounding = card_rounding;
    
    style.visuals.widgets.inactive.rounding = button_rounding;
    style.visuals.widgets.hovered.rounding = button_rounding;
    style.visuals.widgets.active.rounding = button_rounding;

    // Color tokens
    let primary = egui::Color32::from_rgb(168, 199, 250);        // Light Blue Accent
    let on_primary = egui::Color32::from_rgb(6, 46, 111);        // Dark Blue Text
    
    let surface = egui::Color32::from_rgb(26, 27, 30);           // Background Color
    let on_surface = egui::Color32::from_rgb(227, 226, 230);     // Primary Text
    
    let surface_variant = egui::Color32::from_rgb(45, 48, 53);   // Card Background
    let _on_surface_variant = egui::Color32::from_rgb(194, 201, 212); // Secondary Text
    
    let outline = egui::Color32::from_rgb(142, 145, 154);        // Border Colors
    let text_bg = egui::Color32::from_rgb(32, 33, 36);           // Inputs Background

    style.visuals.panel_fill = surface;
    style.visuals.extreme_bg_color = text_bg;
    style.visuals.selection.bg_fill = primary;

    style.visuals.widgets.inactive.bg_fill = surface_variant;
    style.visuals.widgets.inactive.weak_bg_fill = surface_variant;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, on_surface);

    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 65, 72);
    style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(60, 65, 72);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, primary);

    style.visuals.widgets.active.bg_fill = primary;
    style.visuals.widgets.active.weak_bg_fill = primary;
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(2.0, on_primary);

    style.visuals.widgets.noninteractive.bg_fill = surface;
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, outline);

    style.spacing.item_spacing = egui::vec2(12.0, 14.0);
    style.spacing.button_padding = egui::vec2(20.0, 10.0);
    
    // Set scroll bar width on style.spacing.scroll (if it exists in this egui version) or configure scroll bars through visuals.
    style.spacing.scroll.bar_width = 4.0;
    style.spacing.scroll.bar_inner_margin = 2.0;

    ctx.set_style(style);
}
