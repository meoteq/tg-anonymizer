use eframe::egui;

// ─── Material Design 3 – Color Tokens (Dark Scheme) ─────────────────────────
// Generated from seed color #A8C7FA (Primary-80) using the MD3 tonal palette.
// All roles follow the MD3 color system spec with proper on-* pairing.

#[allow(dead_code)]
pub struct Md3Colors {
    // Accent – Primary
    pub primary: egui::Color32,
    pub on_primary: egui::Color32,
    pub primary_container: egui::Color32,
    pub on_primary_container: egui::Color32,

    // Accent – Secondary
    pub secondary: egui::Color32,
    pub on_secondary: egui::Color32,
    pub secondary_container: egui::Color32,
    pub on_secondary_container: egui::Color32,

    // Accent – Tertiary
    pub tertiary: egui::Color32,
    pub on_tertiary: egui::Color32,
    pub tertiary_container: egui::Color32,
    pub on_tertiary_container: egui::Color32,

    // Error
    pub error: egui::Color32,
    pub on_error: egui::Color32,
    pub error_container: egui::Color32,
    pub on_error_container: egui::Color32,

    // Surface hierarchy
    pub surface: egui::Color32,
    pub on_surface: egui::Color32,
    pub on_surface_variant: egui::Color32,
    pub surface_container_lowest: egui::Color32,
    pub surface_container_low: egui::Color32,
    pub surface_container: egui::Color32,
    pub surface_container_high: egui::Color32,
    pub surface_container_highest: egui::Color32,
    pub surface_dim: egui::Color32,
    pub surface_bright: egui::Color32,

    // Inverse
    pub inverse_surface: egui::Color32,
    pub inverse_on_surface: egui::Color32,
    pub inverse_primary: egui::Color32,

    // Outline
    pub outline: egui::Color32,
    pub outline_variant: egui::Color32,

    // Utility (MD3 success / good – not in core spec but common)
    pub success: egui::Color32,
    pub on_success: egui::Color32,
}

impl Default for Md3Colors {
    fn default() -> Self {
        Self::dark()
    }
}

impl Md3Colors {
    /// Material Design 3 dark scheme generated from seed #A8C7FA.
    pub fn dark() -> Self {
        Self {
            // Primary – P-80 / P-20 / P-30 / P-90
            primary:                egui::Color32::from_rgb(168, 199, 250),
            on_primary:             egui::Color32::from_rgb(  6,  46, 111),
            primary_container:      egui::Color32::from_rgb( 30,  70, 136),
            on_primary_container:   egui::Color32::from_rgb(212, 227, 255),

            // Secondary – S-80 / S-20 / S-30 / S-90
            secondary:              egui::Color32::from_rgb(189, 195, 215),
            on_secondary:           egui::Color32::from_rgb( 39,  45,  63),
            secondary_container:    egui::Color32::from_rgb( 62,  67,  86),
            on_secondary_container: egui::Color32::from_rgb(217, 224, 245),

            // Tertiary – T-80 / T-20 / T-30 / T-90
            tertiary:               egui::Color32::from_rgb(218, 189, 226),
            on_tertiary:            egui::Color32::from_rgb( 62,  40,  72),
            tertiary_container:     egui::Color32::from_rgb( 86,  63,  96),
            on_tertiary_container:  egui::Color32::from_rgb(247, 217, 255),

            // Error
            error:                  egui::Color32::from_rgb(255, 180, 171),
            on_error:               egui::Color32::from_rgb(105,   0,   5),
            error_container:        egui::Color32::from_rgb(147,   0,  10),
            on_error_container:     egui::Color32::from_rgb(255, 218, 214),

            // Surface hierarchy (N-6 / N-87 / NV-80 / N-4 / N-10 / N-12 / N-17 / N-22 / N-6 / N-24)
            surface:                    egui::Color32::from_rgb( 18,  19,  24),
            on_surface:                 egui::Color32::from_rgb(227, 225, 230),
            on_surface_variant:         egui::Color32::from_rgb(196, 199, 208),
            surface_container_lowest:   egui::Color32::from_rgb( 13,  14,  18),
            surface_container_low:      egui::Color32::from_rgb( 28,  29,  33),
            surface_container:          egui::Color32::from_rgb( 32,  33,  38),
            surface_container_high:     egui::Color32::from_rgb( 42,  43,  48),
            surface_container_highest:  egui::Color32::from_rgb( 53,  54,  59),
            surface_dim:                egui::Color32::from_rgb( 18,  19,  24),
            surface_bright:             egui::Color32::from_rgb( 56,  57,  63),

            // Inverse
            inverse_surface:        egui::Color32::from_rgb(227, 225, 230),
            inverse_on_surface:     egui::Color32::from_rgb( 48,  49,  53),
            inverse_primary:        egui::Color32::from_rgb( 54,  93, 160),

            // Outline
            outline:                egui::Color32::from_rgb(142, 145, 154),
            outline_variant:        egui::Color32::from_rgb( 68,  71,  79),

            // Success
            success:                egui::Color32::from_rgb(137, 230, 150),
            on_success:             egui::Color32::from_rgb(  0,  57,  14),
        }
    }
}

// ─── Material Design 3 – Shape Tokens ───────────────────────────────────────

#[allow(dead_code)]
pub struct Md3Shape {
    pub none: f32,
    pub extra_small: f32,
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub large_increased: f32,
    pub extra_large: f32,
    pub extra_large_increased: f32,
    pub extra_extra_large: f32,
    pub full: f32,
}

impl Default for Md3Shape {
    fn default() -> Self {
        Self {
            none: 0.0,
            extra_small: 4.0,
            small: 8.0,
            medium: 12.0,
            large: 16.0,
            large_increased: 20.0,
            extra_large: 28.0,
            extra_large_increased: 32.0,
            extra_extra_large: 48.0,
            full: 9999.0,
        }
    }
}

// ─── Material Design 3 – Motion Tokens ──────────────────────────────────────
// egui doesn't have CSS cubic-bezier, so we expose durations + spring params.

#[allow(dead_code)]
pub struct Md3Motion {
    // Duration tokens (seconds for egui animate_*)
    pub short1: f32,
    pub short2: f32,
    pub short3: f32,
    pub short4: f32,
    pub medium1: f32,
    pub medium2: f32,
    pub medium3: f32,
    pub medium4: f32,
    pub long2: f32,
}

impl Default for Md3Motion {
    fn default() -> Self {
        Self {
            short1:  0.05,
            short2:  0.10,
            short3:  0.15,
            short4:  0.20,
            medium1: 0.25,
            medium2: 0.30,
            medium3: 0.35,
            medium4: 0.40,
            long2:   0.50,
        }
    }
}

// ─── Material Design 3 – Typography Scale ───────────────────────────────────
// Maps MD3 type-scale to egui font sizes. We substitute Google Sans as brand.

#[allow(dead_code)]
pub struct Md3Type {
    pub display_large: f32,
    pub display_medium: f32,
    pub display_small: f32,
    pub headline_large: f32,
    pub headline_medium: f32,
    pub headline_small: f32,
    pub title_large: f32,
    pub title_medium: f32,
    pub title_small: f32,
    pub body_large: f32,
    pub body_medium: f32,
    pub body_small: f32,
    pub label_large: f32,
    pub label_medium: f32,
    pub label_small: f32,
}

impl Default for Md3Type {
    fn default() -> Self {
        Self {
            display_large:  57.0,
            display_medium: 45.0,
            display_small:  36.0,
            headline_large: 32.0,
            headline_medium: 28.0,
            headline_small: 24.0,
            title_large:    22.0,
            title_medium:   16.0,
            title_small:    14.0,
            body_large:     16.0,
            body_medium:    14.0,
            body_small:     12.0,
            label_large:    14.0,
            label_medium:   12.0,
            label_small:    11.0,
        }
    }
}

// ─── Material Design 3 – Spacing (8dp grid) ────────────────────────────────

#[allow(dead_code)]
pub struct Md3Spacing {
    pub xs: f32,    // 4dp
    pub sm: f32,    // 8dp
    pub md: f32,    // 12dp
    pub lg: f32,    // 16dp
    pub xl: f32,    // 24dp
    pub xxl: f32,   // 32dp
}

impl Default for Md3Spacing {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 12.0,
            lg: 16.0,
            xl: 24.0,
            xxl: 32.0,
        }
    }
}

// ─── Unified Token Accessor ─────────────────────────────────────────────────

#[allow(dead_code)]
pub struct Md3Theme {
    pub colors: Md3Colors,
    pub shape: Md3Shape,
    pub motion: Md3Motion,
    pub typo: Md3Type,
    pub spacing: Md3Spacing,
}

impl Default for Md3Theme {
    fn default() -> Self {
        Self {
            colors: Md3Colors::default(),
            shape: Md3Shape::default(),
            motion: Md3Motion::default(),
            typo: Md3Type::default(),
            spacing: Md3Spacing::default(),
        }
    }
}

// ─── Font Loading ───────────────────────────────────────────────────────────

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
        vec!["Google Sans Bold".to_owned(), "material-icons".to_owned()],
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

// ─── Apply MD3 Style to egui Context ────────────────────────────────────────

pub fn setup_custom_style(ctx: &egui::Context) {
    // 1. Fonts
    let mut fonts = egui::FontDefinitions::default();
    load_custom_fonts(&mut fonts);
    ctx.set_fonts(fonts);

    // 2. Color tokens
    let c = Md3Colors::dark();
    let s = Md3Shape::default();

    let mut style = (*ctx.style()).clone();

    // --- Shape: MD3 fully-rounded buttons, medium-rounded cards/windows ---
    let button_rounding = egui::Rounding::same(s.full.min(100.0)); // effectively pill shape
    let card_rounding = egui::Rounding::same(s.medium);

    style.visuals.window_rounding = egui::Rounding::same(s.extra_large);
    style.visuals.menu_rounding = egui::Rounding::same(s.small);

    style.visuals.widgets.inactive.rounding = button_rounding;
    style.visuals.widgets.hovered.rounding = button_rounding;
    style.visuals.widgets.active.rounding = button_rounding;
    style.visuals.widgets.noninteractive.rounding = card_rounding;

    // --- Surface colors ---
    style.visuals.panel_fill = c.surface;
    style.visuals.extreme_bg_color = c.surface_container_lowest;
    style.visuals.faint_bg_color = c.surface_container_low;
    style.visuals.code_bg_color = c.surface_container;

    // Selection uses primary
    style.visuals.selection.bg_fill = c.primary;
    style.visuals.selection.stroke = egui::Stroke::new(1.0, c.on_primary);

    // --- Widget states (MD3 state layer model) ---

    // Inactive: surface-container-high fill, on-surface text
    style.visuals.widgets.inactive.bg_fill = c.surface_container_high;
    style.visuals.widgets.inactive.weak_bg_fill = c.surface_container_high;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, c.on_surface);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;

    // Hovered: surface-container-highest + primary tinted text
    style.visuals.widgets.hovered.bg_fill = c.surface_container_highest;
    style.visuals.widgets.hovered.weak_bg_fill = c.surface_container_highest;
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, c.primary);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;

    // Active: surface-container-highest (same as hovered to keep scrollbar grey when clicked/pressed)
    style.visuals.widgets.active.bg_fill = c.surface_container_highest;
    style.visuals.widgets.active.weak_bg_fill = c.surface_container_highest;
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(2.0, c.primary);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;

    // Non-interactive: surface fill, on-surface-variant text
    style.visuals.widgets.noninteractive.bg_fill = c.surface;
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, c.on_surface_variant);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;

    // Open (combo-boxes, etc.)
    style.visuals.widgets.open.bg_fill = c.surface_container_highest;
    style.visuals.widgets.open.weak_bg_fill = c.surface_container_highest;
    style.visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, c.primary);

    // Hyperlinks
    style.visuals.hyperlink_color = c.primary;

    // Warning text
    style.visuals.warn_fg_color = egui::Color32::from_rgb(255, 183, 77); // amber-ish

    // Error text
    style.visuals.error_fg_color = c.error;

    // --- Spacing: 8dp grid system ---
    style.spacing.item_spacing = egui::vec2(12.0, 12.0);
    style.spacing.button_padding = egui::vec2(24.0, 12.0);

    // Scrollbar
    style.spacing.scroll.bar_width = 4.0;
    style.spacing.scroll.bar_inner_margin = 2.0;

    // Text cursor
    style.visuals.text_cursor.stroke = egui::Stroke::new(2.0, c.primary);

    // Animation speed – slightly slower for MD3 emphasized feel
    style.animation_time = 0.25;

    ctx.set_style(style);
}
