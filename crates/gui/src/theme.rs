use egui::{Color32, Margin, RichText, Rounding, Stroke, Vec2};

pub struct Colors;

#[allow(dead_code)]
impl Colors {
    pub const BG_DARK: Color32 = Color32::from_rgb(22, 24, 30);
    pub const BG_PANEL: Color32 = Color32::from_rgb(26, 29, 36);
    pub const BG_MAIN: Color32 = Color32::from_rgb(30, 33, 40);
    pub const BG_ELEVATED: Color32 = Color32::from_rgb(38, 42, 52);
    pub const BG_WIDGET: Color32 = Color32::from_rgb(45, 50, 60);
    pub const BG_WIDGET_HOVER: Color32 = Color32::from_rgb(60, 70, 85);
    pub const BG_WIDGET_ACTIVE: Color32 = Color32::from_rgb(75, 130, 195);

    pub const ACCENT: Color32 = Color32::from_rgb(75, 130, 195);
    pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(120, 180, 255);
    pub const SUCCESS: Color32 = Color32::from_rgb(80, 180, 80);
    pub const DANGER: Color32 = Color32::from_rgb(200, 80, 80);
    pub const WARNING: Color32 = Color32::from_rgb(220, 170, 50);

    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(220, 225, 235);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 165, 180);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(120, 125, 140);
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(80, 85, 95);
}

pub struct Spacing;

impl Spacing {
    pub const ITEM: Vec2 = Vec2::new(8.0, 6.0);
    pub const BUTTON_PADDING: Vec2 = Vec2::new(10.0, 4.0);
    pub const WINDOW_MARGIN: Margin = Margin::same(12.0);
    pub const PANEL_MARGIN: Margin = Margin::same(10.0);
    pub const SECTION_GAP: f32 = 12.0;
    pub const SMALL_GAP: f32 = 4.0;
}

pub struct Radii;

impl Radii {
    pub const WIDGET: Rounding = Rounding::same(4.0);
    pub const WINDOW: Rounding = Rounding::same(8.0);
}

pub struct Fonts;

impl Fonts {
    pub const TITLE: f32 = 22.0;
    pub const HEADING: f32 = 20.0;
    pub const SUBHEADING: f32 = 14.0;
    pub const BODY: f32 = 13.0;
    pub const SMALL: f32 = 11.0;
    pub const BUTTON: f32 = 14.0;
}

pub fn heading(text: &str) -> RichText {
    RichText::new(text)
        .size(Fonts::HEADING)
        .strong()
        .color(Colors::ACCENT_LIGHT)
}

pub fn title(text: &str) -> RichText {
    RichText::new(text).size(Fonts::TITLE).strong()
}

pub fn subheading(text: &str) -> RichText {
    RichText::new(text).size(Fonts::SUBHEADING).strong()
}

pub fn body(text: &str) -> RichText {
    RichText::new(text)
        .size(Fonts::BODY)
        .color(Colors::TEXT_PRIMARY)
}

pub fn muted(text: &str) -> RichText {
    RichText::new(text)
        .size(Fonts::BODY)
        .color(Colors::TEXT_MUTED)
}

pub fn small(text: &str) -> RichText {
    RichText::new(text)
        .size(Fonts::SMALL)
        .color(Colors::TEXT_MUTED)
}

pub fn badge_mc(version: &str) -> RichText {
    RichText::new(format!("MC {}", version))
        .size(Fonts::BODY)
        .color(Colors::ACCENT_LIGHT)
}

pub fn badge_loader(text: &str) -> RichText {
    RichText::new(text).size(Fonts::BODY).color(Colors::SUCCESS)
}

pub fn status_text(text: &str) -> RichText {
    RichText::new(text)
        .size(Fonts::SMALL)
        .color(Colors::TEXT_SECONDARY)
}

pub fn launch_button() -> egui::Button<'static> {
    egui::Button::new(RichText::new("Launch").size(Fonts::BUTTON).strong())
        .fill(Colors::SUCCESS)
        .rounding(Radii::WIDGET)
}

#[allow(dead_code)]
pub fn danger_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).size(Fonts::BODY))
        .fill(Colors::DANGER)
        .rounding(Radii::WIDGET)
}

pub fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(Colors::BG_PANEL)
        .inner_margin(Spacing::PANEL_MARGIN)
}

pub fn top_bar_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(Colors::BG_DARK)
        .inner_margin(Margin::symmetric(12.0, 8.0))
}

pub fn bottom_bar_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(Colors::BG_DARK)
        .inner_margin(Margin::symmetric(12.0, 6.0))
}

pub fn section_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(Colors::BG_ELEVATED)
        .rounding(Radii::WINDOW)
        .inner_margin(Margin::same(16.0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    Dark,
    Ocean,
    Forest,
    Warm,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 4] = [
        ThemePreset::Dark,
        ThemePreset::Ocean,
        ThemePreset::Forest,
        ThemePreset::Warm,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ThemePreset::Dark => "Dark (Default)",
            ThemePreset::Ocean => "Ocean Blue",
            ThemePreset::Forest => "Forest Green",
            ThemePreset::Warm => "Warm Amber",
        }
    }

    pub fn accent(&self) -> Color32 {
        match self {
            ThemePreset::Dark => Color32::from_rgb(75, 130, 195),
            ThemePreset::Ocean => Color32::from_rgb(60, 150, 220),
            ThemePreset::Forest => Color32::from_rgb(70, 160, 90),
            ThemePreset::Warm => Color32::from_rgb(210, 150, 60),
        }
    }

    pub fn accent_light(&self) -> Color32 {
        match self {
            ThemePreset::Dark => Color32::from_rgb(120, 180, 255),
            ThemePreset::Ocean => Color32::from_rgb(100, 200, 255),
            ThemePreset::Forest => Color32::from_rgb(120, 220, 130),
            ThemePreset::Warm => Color32::from_rgb(255, 200, 100),
        }
    }
}

pub fn apply_theme(ctx: &egui::Context, preset: ThemePreset) {
    let accent = preset.accent();
    let mut style = (*ctx.style()).clone();

    style.spacing.item_spacing = Spacing::ITEM;
    style.spacing.button_padding = Spacing::BUTTON_PADDING;
    style.spacing.window_margin = Spacing::WINDOW_MARGIN;

    style.visuals.widgets.inactive.rounding = Radii::WIDGET;
    style.visuals.widgets.hovered.rounding = Radii::WIDGET;
    style.visuals.widgets.active.rounding = Radii::WIDGET;

    style.visuals.widgets.inactive.bg_fill = Colors::BG_WIDGET;
    style.visuals.widgets.hovered.bg_fill = Colors::BG_WIDGET_HOVER;
    style.visuals.widgets.active.bg_fill = accent;

    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Colors::TEXT_PRIMARY);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    style.visuals.selection.bg_fill = accent;
    style.visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);

    style.visuals.window_rounding = Radii::WINDOW;
    style.visuals.window_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 4.0),
        blur: 12.0,
        spread: 0.0,
        color: Color32::from_black_alpha(60),
    };

    style.visuals.panel_fill = Colors::BG_MAIN;
    style.visuals.extreme_bg_color = Colors::BG_DARK;
    style.visuals.faint_bg_color = Colors::BG_ELEVATED;

    ctx.set_style(style);
}

pub fn apply_global_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.spacing.item_spacing = Spacing::ITEM;
    style.spacing.button_padding = Spacing::BUTTON_PADDING;
    style.spacing.window_margin = Spacing::WINDOW_MARGIN;

    style.visuals.widgets.inactive.rounding = Radii::WIDGET;
    style.visuals.widgets.hovered.rounding = Radii::WIDGET;
    style.visuals.widgets.active.rounding = Radii::WIDGET;

    style.visuals.widgets.inactive.bg_fill = Colors::BG_WIDGET;
    style.visuals.widgets.hovered.bg_fill = Colors::BG_WIDGET_HOVER;
    style.visuals.widgets.active.bg_fill = Colors::BG_WIDGET_ACTIVE;

    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Colors::TEXT_PRIMARY);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    style.visuals.selection.bg_fill = Colors::ACCENT;
    style.visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);

    style.visuals.window_rounding = Radii::WINDOW;
    style.visuals.window_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 4.0),
        blur: 12.0,
        spread: 0.0,
        color: Color32::from_black_alpha(60),
    };

    style.visuals.panel_fill = Colors::BG_MAIN;
    style.visuals.extreme_bg_color = Colors::BG_DARK;
    style.visuals.faint_bg_color = Colors::BG_ELEVATED;

    ctx.set_style(style);
}
