//! Colors, type scale, and fonts. Everything visual is decided here so the
//! layout code in `app.rs` only refers to named roles.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle, Theme, Visuals,
};
use std::{fs, sync::Arc};

/// Type scale. No sizes outside this list.
pub const SIZE_SMALL: f32 = 12.0;
pub const SIZE_BODY: f32 = 14.0;
pub const SIZE_READING: f32 = 16.0;
pub const SIZE_SECTION: f32 = 20.0;
pub const SIZE_TITLE: f32 = 28.0;

pub const RADIUS: u8 = 8;
pub const SEMIBOLD: &str = "semibold";

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub sidebar: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub row_selected: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub separator: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub warning: Color32,
    formats: [Color32; 7],
}

const LIGHT: Palette = Palette {
    background: Color32::from_rgb(0xff, 0xff, 0xff),
    sidebar: Color32::from_rgb(0xf5, 0xf5, 0xf7),
    surface: Color32::from_rgb(0xe8, 0xe8, 0xed),
    surface_hover: Color32::from_rgb(0xdc, 0xdc, 0xe1),
    row_selected: Color32::from_rgb(0xe3, 0xe3, 0xe8),
    text: Color32::from_rgb(0x1d, 0x1d, 0x1f),
    secondary: Color32::from_rgb(0x6e, 0x6e, 0x73),
    separator: Color32::from_rgb(0xd2, 0xd2, 0xd7),
    accent: Color32::from_rgb(0x00, 0x71, 0xe3),
    accent_hover: Color32::from_rgb(0x00, 0x66, 0xcc),
    on_accent: Color32::WHITE,
    danger: Color32::from_rgb(0xd7, 0x00, 0x15),
    warning: Color32::from_rgb(0xb2, 0x50, 0x00),
    // text, markdown, csv, tsv, json, xml, pdf
    formats: [
        Color32::from_rgb(0x8e, 0x8e, 0x93),
        Color32::from_rgb(0x30, 0xb0, 0xc7),
        Color32::from_rgb(0x34, 0xc7, 0x59),
        Color32::from_rgb(0x00, 0xc7, 0xbe),
        Color32::from_rgb(0xff, 0x95, 0x00),
        Color32::from_rgb(0x58, 0x56, 0xd6),
        Color32::from_rgb(0xaf, 0x52, 0xde),
    ],
};

const DARK: Palette = Palette {
    background: Color32::from_rgb(0x1c, 0x1c, 0x1e),
    sidebar: Color32::from_rgb(0x24, 0x24, 0x26),
    surface: Color32::from_rgb(0x3a, 0x3a, 0x3c),
    surface_hover: Color32::from_rgb(0x48, 0x48, 0x4a),
    row_selected: Color32::from_rgb(0x3a, 0x3a, 0x3c),
    text: Color32::from_rgb(0xf5, 0xf5, 0xf7),
    secondary: Color32::from_rgb(0x98, 0x98, 0x9d),
    separator: Color32::from_rgb(0x38, 0x38, 0x3a),
    accent: Color32::from_rgb(0x0a, 0x84, 0xff),
    accent_hover: Color32::from_rgb(0x40, 0x9c, 0xff),
    on_accent: Color32::WHITE,
    danger: Color32::from_rgb(0xff, 0x45, 0x3a),
    warning: Color32::from_rgb(0xff, 0x9f, 0x0a),
    formats: [
        Color32::from_rgb(0x98, 0x98, 0x9d),
        Color32::from_rgb(0x40, 0xc8, 0xe0),
        Color32::from_rgb(0x30, 0xd1, 0x58),
        Color32::from_rgb(0x63, 0xe6, 0xe2),
        Color32::from_rgb(0xff, 0x9f, 0x0a),
        Color32::from_rgb(0x5e, 0x5c, 0xe6),
        Color32::from_rgb(0xbf, 0x5a, 0xf2),
    ],
};

impl Palette {
    pub fn of(ctx: &egui::Context) -> Self {
        match ctx.theme() {
            Theme::Light => LIGHT,
            Theme::Dark => DARK,
        }
    }

    /// Identity color for a format, used only for the small dot beside a file.
    pub fn format(&self, format: &str) -> Color32 {
        let index = match format {
            "markdown" => 1,
            "csv" => 2,
            "tsv" => 3,
            "json" => 4,
            "xml" => 5,
            "pdf" => 6,
            _ => 0,
        };
        self.formats[index]
    }
}

pub fn format_label(format: &str) -> &'static str {
    match format {
        "markdown" => "MD",
        "csv" => "CSV",
        "tsv" => "TSV",
        "json" => "JSON",
        "xml" => "XML",
        "pdf" => "PDF",
        _ => "TXT",
    }
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.options_mut(|options| options.theme_preference = egui::ThemePreference::System);
    for (theme, palette) in [(Theme::Light, LIGHT), (Theme::Dark, DARK)] {
        ctx.set_visuals_of(theme, visuals(theme, &palette));
    }
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(SIZE_SMALL)),
            (TextStyle::Body, FontId::proportional(SIZE_BODY)),
            (TextStyle::Button, FontId::proportional(SIZE_BODY)),
            (TextStyle::Monospace, FontId::monospace(SIZE_BODY)),
            (TextStyle::Heading, semibold(SIZE_SECTION)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.interact_size.y = 30.0;
        style.spacing.window_margin = Margin::same(16);
        style.interaction.selectable_labels = false;
    });
}

fn visuals(theme: Theme, palette: &Palette) -> Visuals {
    let mut visuals = match theme {
        Theme::Light => Visuals::light(),
        Theme::Dark => Visuals::dark(),
    };
    let radius = CornerRadius::same(RADIUS);
    visuals.panel_fill = palette.background;
    visuals.window_fill = palette.background;
    visuals.extreme_bg_color = palette.background;
    visuals.faint_bg_color = palette.sidebar;
    visuals.code_bg_color = palette.sidebar;
    visuals.hyperlink_color = palette.accent;
    visuals.override_text_color = None;
    visuals.selection.bg_fill = palette.accent.gamma_multiply(0.25);
    visuals.window_stroke = Stroke::new(1.0, palette.separator);
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 1],
        blur: 2,
        spread: 0,
        color: Color32::from_black_alpha(10),
    };
    visuals.window_corner_radius = CornerRadius::same(12);
    visuals.menu_corner_radius = radius;

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.separator);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);
    for (state, fill) in [
        (&mut widgets.inactive, palette.surface),
        (&mut widgets.hovered, palette.surface_hover),
        (&mut widgets.active, palette.surface_hover),
        (&mut widgets.open, palette.surface),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::NONE;
        state.fg_stroke = Stroke::new(1.0, palette.text);
        state.corner_radius = radius;
        state.expansion = 0.0;
    }
    // Keyboard focus is drawn with the hovered stroke; keep it clearly visible.
    widgets.hovered.bg_stroke = Stroke::NONE;
    visuals.selection.stroke = Stroke::new(2.0, palette.accent);
    visuals
}

/// Segoe UI on Windows, egui's bundled fonts as fallback everywhere else.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let fonts_dir = std::env::var_os("WINDIR")
        .map(|dir| std::path::PathBuf::from(dir).join("Fonts"))
        .unwrap_or_default();
    let load = |file: &str| fs::read(fonts_dir.join(file)).ok();

    prepend(
        &mut fonts,
        FontFamily::Proportional,
        "segoe-ui",
        load("segoeui.ttf"),
    );
    prepend(
        &mut fonts,
        FontFamily::Monospace,
        "mono",
        load("CascadiaMono.ttf").or_else(|| load("consola.ttf")),
    );

    let fallback = fonts.families[&FontFamily::Proportional].clone();
    fonts
        .families
        .insert(FontFamily::Name(SEMIBOLD.into()), fallback);
    prepend(
        &mut fonts,
        FontFamily::Name(SEMIBOLD.into()),
        "segoe-ui-semibold",
        load("seguisb.ttf"),
    );
    ctx.set_fonts(fonts);
}

fn prepend(fonts: &mut FontDefinitions, family: FontFamily, key: &str, bytes: Option<Vec<u8>>) {
    if let Some(bytes) = bytes {
        fonts
            .font_data
            .insert(key.to_owned(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, key.to_owned());
    }
}
