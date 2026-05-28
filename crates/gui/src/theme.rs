use std::cell::RefCell;

use egui::{Color32, CornerRadius, Margin, RichText, Stroke, Vec2};
pub use miao_core::config::ThemePreset;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg_dark: Color32,
    pub bg_panel: Color32,
    pub bg_main: Color32,
    pub bg_elevated: Color32,
    pub bg_widget: Color32,
    pub bg_widget_hover: Color32,
    pub bg_widget_active: Color32,
    pub accent: Color32,
    pub accent_light: Color32,
    pub success: Color32,
    pub danger: Color32,
    pub warning: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub text_disabled: Color32,
}

impl Palette {
    fn for_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::Dark => Self {
                bg_dark: Color32::from_rgb(22, 24, 30),
                bg_panel: Color32::from_rgb(26, 29, 36),
                bg_main: Color32::from_rgb(30, 33, 40),
                bg_elevated: Color32::from_rgb(38, 42, 52),
                bg_widget: Color32::from_rgb(45, 50, 60),
                bg_widget_hover: Color32::from_rgb(60, 70, 85),
                bg_widget_active: Color32::from_rgb(75, 130, 195),
                accent: Color32::from_rgb(75, 130, 195),
                accent_light: Color32::from_rgb(120, 180, 255),
                success: Color32::from_rgb(80, 180, 80),
                danger: Color32::from_rgb(200, 80, 80),
                warning: Color32::from_rgb(220, 170, 50),
                text_primary: Color32::from_rgb(220, 225, 235),
                text_secondary: Color32::from_rgb(160, 165, 180),
                text_muted: Color32::from_rgb(120, 125, 140),
                text_disabled: Color32::from_rgb(80, 85, 95),
            },
            ThemePreset::Ocean => Self {
                bg_dark: Color32::from_rgb(15, 25, 40),
                bg_panel: Color32::from_rgb(18, 30, 48),
                bg_main: Color32::from_rgb(22, 35, 55),
                bg_elevated: Color32::from_rgb(30, 45, 65),
                bg_widget: Color32::from_rgb(35, 55, 75),
                bg_widget_hover: Color32::from_rgb(45, 70, 95),
                bg_widget_active: Color32::from_rgb(60, 150, 220),
                accent: Color32::from_rgb(60, 150, 220),
                accent_light: Color32::from_rgb(100, 200, 255),
                success: Color32::from_rgb(80, 180, 80),
                danger: Color32::from_rgb(200, 80, 80),
                warning: Color32::from_rgb(220, 170, 50),
                text_primary: Color32::from_rgb(210, 230, 245),
                text_secondary: Color32::from_rgb(150, 175, 200),
                text_muted: Color32::from_rgb(110, 135, 160),
                text_disabled: Color32::from_rgb(70, 90, 110),
            },
            ThemePreset::Forest => Self {
                bg_dark: Color32::from_rgb(20, 28, 22),
                bg_panel: Color32::from_rgb(24, 32, 26),
                bg_main: Color32::from_rgb(28, 36, 30),
                bg_elevated: Color32::from_rgb(36, 46, 38),
                bg_widget: Color32::from_rgb(42, 54, 44),
                bg_widget_hover: Color32::from_rgb(55, 70, 58),
                bg_widget_active: Color32::from_rgb(70, 160, 90),
                accent: Color32::from_rgb(70, 160, 90),
                accent_light: Color32::from_rgb(120, 220, 130),
                success: Color32::from_rgb(80, 180, 80),
                danger: Color32::from_rgb(200, 80, 80),
                warning: Color32::from_rgb(220, 170, 50),
                text_primary: Color32::from_rgb(215, 230, 218),
                text_secondary: Color32::from_rgb(160, 175, 162),
                text_muted: Color32::from_rgb(120, 135, 122),
                text_disabled: Color32::from_rgb(80, 92, 82),
            },
            ThemePreset::Warm => Self {
                bg_dark: Color32::from_rgb(30, 26, 22),
                bg_panel: Color32::from_rgb(36, 30, 26),
                bg_main: Color32::from_rgb(40, 34, 30),
                bg_elevated: Color32::from_rgb(50, 44, 38),
                bg_widget: Color32::from_rgb(58, 50, 42),
                bg_widget_hover: Color32::from_rgb(75, 65, 55),
                bg_widget_active: Color32::from_rgb(210, 150, 60),
                accent: Color32::from_rgb(210, 150, 60),
                accent_light: Color32::from_rgb(255, 200, 100),
                success: Color32::from_rgb(80, 180, 80),
                danger: Color32::from_rgb(200, 80, 80),
                warning: Color32::from_rgb(220, 170, 50),
                text_primary: Color32::from_rgb(235, 225, 215),
                text_secondary: Color32::from_rgb(180, 168, 155),
                text_muted: Color32::from_rgb(140, 128, 115),
                text_disabled: Color32::from_rgb(95, 85, 75),
            },
            ThemePreset::Sakura => Self {
                bg_dark: Color32::from_rgb(30, 22, 28),
                bg_panel: Color32::from_rgb(36, 26, 34),
                bg_main: Color32::from_rgb(40, 30, 38),
                bg_elevated: Color32::from_rgb(52, 38, 48),
                bg_widget: Color32::from_rgb(62, 45, 58),
                bg_widget_hover: Color32::from_rgb(80, 58, 75),
                bg_widget_active: Color32::from_rgb(200, 100, 150),
                accent: Color32::from_rgb(200, 100, 150),
                accent_light: Color32::from_rgb(255, 150, 200),
                success: Color32::from_rgb(80, 180, 80),
                danger: Color32::from_rgb(200, 80, 80),
                warning: Color32::from_rgb(220, 170, 50),
                text_primary: Color32::from_rgb(240, 220, 235),
                text_secondary: Color32::from_rgb(185, 160, 178),
                text_muted: Color32::from_rgb(145, 120, 138),
                text_disabled: Color32::from_rgb(95, 75, 90),
            },
            ThemePreset::Light => Self {
                bg_dark: Color32::from_rgb(218, 220, 224),
                bg_panel: Color32::from_rgb(234, 236, 240),
                bg_main: Color32::from_rgb(241, 243, 246),
                bg_elevated: Color32::from_rgb(250, 251, 252),
                bg_widget: Color32::from_rgb(218, 222, 228),
                bg_widget_hover: Color32::from_rgb(200, 206, 214),
                bg_widget_active: Color32::from_rgb(55, 110, 190),
                accent: Color32::from_rgb(55, 110, 190),
                accent_light: Color32::from_rgb(35, 90, 165),
                success: Color32::from_rgb(40, 140, 50),
                danger: Color32::from_rgb(185, 50, 50),
                warning: Color32::from_rgb(180, 130, 20),
                text_primary: Color32::from_rgb(36, 41, 47),
                text_secondary: Color32::from_rgb(87, 96, 106),
                text_muted: Color32::from_rgb(110, 119, 129),
                text_disabled: Color32::from_rgb(160, 168, 176),
            },
        }
    }
}

thread_local! {
    static ACTIVE_PALETTE: RefCell<Palette> = RefCell::new(Palette::for_preset(ThemePreset::Dark));
}

fn with_palette<R>(f: impl FnOnce(&Palette) -> R) -> R {
    ACTIVE_PALETTE.with(|p| f(&p.borrow()))
}

pub struct Colors;

#[allow(dead_code)]
impl Colors {
    pub fn bg_dark() -> Color32 {
        with_palette(|p| p.bg_dark)
    }
    pub fn bg_panel() -> Color32 {
        with_palette(|p| p.bg_panel)
    }
    pub fn bg_main() -> Color32 {
        with_palette(|p| p.bg_main)
    }
    pub fn bg_elevated() -> Color32 {
        with_palette(|p| p.bg_elevated)
    }
    pub fn bg_widget() -> Color32 {
        with_palette(|p| p.bg_widget)
    }
    pub fn bg_widget_hover() -> Color32 {
        with_palette(|p| p.bg_widget_hover)
    }
    pub fn bg_widget_active() -> Color32 {
        with_palette(|p| p.bg_widget_active)
    }
    pub fn accent() -> Color32 {
        with_palette(|p| p.accent)
    }
    pub fn accent_light() -> Color32 {
        with_palette(|p| p.accent_light)
    }
    pub fn success() -> Color32 {
        with_palette(|p| p.success)
    }
    pub fn danger() -> Color32 {
        with_palette(|p| p.danger)
    }
    pub fn warning() -> Color32 {
        with_palette(|p| p.warning)
    }
    pub fn text_primary() -> Color32 {
        with_palette(|p| p.text_primary)
    }
    pub fn text_secondary() -> Color32 {
        with_palette(|p| p.text_secondary)
    }
    pub fn text_muted() -> Color32 {
        with_palette(|p| p.text_muted)
    }
    pub fn text_disabled() -> Color32 {
        with_palette(|p| p.text_disabled)
    }
    pub fn is_light() -> bool {
        with_palette(|p| p.bg_main.r() > 180)
    }
    pub fn subtle_border() -> Color32 {
        if Self::is_light() {
            Color32::from_black_alpha(12)
        } else {
            Color32::from_white_alpha(6)
        }
    }
    pub fn subtle_border_strong() -> Color32 {
        if Self::is_light() {
            Color32::from_black_alpha(20)
        } else {
            Color32::from_white_alpha(15)
        }
    }
}

pub struct Spacing;

impl Spacing {
    pub const ITEM: Vec2 = Vec2::new(8.0, 8.0);
    pub const BUTTON_PADDING: Vec2 = Vec2::new(12.0, 6.0);
    pub const INTERACT_SIZE: Vec2 = Vec2::new(40.0, 28.0);
    pub const WINDOW_MARGIN: Margin = Margin::same(14);
    pub const PANEL_MARGIN: Margin = Margin::same(12);
    pub const SECTION_GAP: f32 = 16.0;
    pub const SMALL_GAP: f32 = 6.0;
}

pub struct Radii;

impl Radii {
    pub const WIDGET: CornerRadius = CornerRadius::same(4);
    pub const WINDOW: CornerRadius = CornerRadius::same(8);
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
        .color(Colors::accent_light())
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
        .color(Colors::text_primary())
}

pub fn muted(text: &str) -> RichText {
    RichText::new(text)
        .size(Fonts::BODY)
        .color(Colors::text_muted())
}

pub fn small(text: &str) -> RichText {
    RichText::new(text)
        .size(Fonts::SMALL)
        .color(Colors::text_muted())
}

pub fn badge_mc(version: &str) -> RichText {
    RichText::new(format!("MC {}", version))
        .size(Fonts::BODY)
        .color(Colors::accent_light())
}

pub fn badge_loader(text: &str) -> RichText {
    RichText::new(text)
        .size(Fonts::BODY)
        .color(Colors::success())
}

pub fn status_text(text: &str) -> RichText {
    RichText::new(text)
        .size(Fonts::SMALL)
        .color(Colors::text_secondary())
}

pub const LIST_ITEM_ROUNDING: CornerRadius = CornerRadius::same(6);
pub const TAB_UNDERLINE_HEIGHT: f32 = 2.5;

#[allow(dead_code)]
pub fn list_item_frame(hovered: bool, selected: bool) -> egui::Frame {
    let fill = if selected {
        Colors::bg_widget_active().gamma_multiply(0.3)
    } else if hovered {
        Colors::bg_widget_hover()
    } else {
        Color32::TRANSPARENT
    };
    egui::Frame::NONE
        .fill(fill)
        .corner_radius(LIST_ITEM_ROUNDING)
        .inner_margin(Margin::symmetric(10, 6))
}

pub fn launch_button() -> egui::Button<'static> {
    egui::Button::new(RichText::new("▶ Launch").size(Fonts::BUTTON).strong())
        .fill(Colors::success())
        .corner_radius(CornerRadius::same(6))
}

pub fn danger_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).size(Fonts::BODY))
        .fill(Colors::danger())
        .corner_radius(CornerRadius::same(6))
}

#[allow(dead_code)]
pub fn card_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(Colors::bg_elevated())
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
        .stroke(Stroke::new(1.0_f32, Colors::subtle_border()))
}

#[allow(dead_code)]
pub fn subtle_separator(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    let rect = ui.available_rect_before_wrap();
    let y = rect.top();
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        Stroke::new(0.5_f32, Colors::subtle_border_strong()),
    );
    ui.add_space(4.0);
}

pub fn panel_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(Colors::bg_panel().gamma_multiply(0.92))
        .inner_margin(Spacing::PANEL_MARGIN)
}

pub fn top_bar_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(Colors::bg_dark().gamma_multiply(0.94))
        .corner_radius(CornerRadius {
            nw: Radii::WINDOW.nw,
            ne: Radii::WINDOW.ne,
            sw: 0,
            se: 0,
        })
        .inner_margin(Margin::symmetric(12, 6))
        .stroke(Stroke::new(0.5, Colors::subtle_border()))
}

pub fn bottom_bar_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(Colors::bg_dark().gamma_multiply(0.94))
        .corner_radius(CornerRadius {
            nw: 0,
            ne: 0,
            sw: Radii::WINDOW.sw,
            se: Radii::WINDOW.se,
        })
        .inner_margin(Margin::symmetric(12, 6))
}

pub fn section_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(Colors::bg_elevated())
        .corner_radius(Radii::WINDOW)
        .inner_margin(Margin::same(16))
}

pub fn list_item_card() -> egui::Frame {
    egui::Frame::NONE
        .fill(Colors::bg_elevated())
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(12, 8))
}

#[allow(dead_code)]
pub trait ThemeColors {
    fn accent(&self) -> Color32;
    fn accent_light(&self) -> Color32;
}

impl ThemeColors for ThemePreset {
    fn accent(&self) -> Color32 {
        Palette::for_preset(*self).accent
    }

    fn accent_light(&self) -> Color32 {
        Palette::for_preset(*self).accent_light
    }
}

/// Build the [`egui::Visuals`] for a palette.
///
/// Always starts from [`egui::Visuals::dark`] so that platform-default fields
/// (notably on Windows under a light system theme) cannot bleed into the UI.
fn build_visuals(pal: &Palette) -> egui::Visuals {
    let is_light = pal.bg_main.r() > 180;
    let mut visuals = if is_light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };

    let fg_on_accent = Color32::WHITE;
    let fg_on_hover = if is_light {
        pal.text_primary
    } else {
        Color32::WHITE
    };

    let widgets = &mut visuals.widgets;

    widgets.noninteractive.bg_fill = pal.bg_main;
    widgets.noninteractive.weak_bg_fill = pal.bg_main;
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, pal.text_secondary);

    widgets.inactive.bg_fill = pal.bg_widget;
    widgets.inactive.weak_bg_fill = pal.bg_widget;
    widgets.inactive.fg_stroke = Stroke::new(1.0, pal.text_primary);
    widgets.inactive.corner_radius = Radii::WIDGET;

    widgets.hovered.bg_fill = pal.bg_widget_hover;
    widgets.hovered.weak_bg_fill = pal.bg_widget_hover;
    widgets.hovered.fg_stroke = Stroke::new(1.0, fg_on_hover);
    widgets.hovered.corner_radius = Radii::WIDGET;

    widgets.active.bg_fill = pal.accent;
    widgets.active.weak_bg_fill = pal.accent;
    widgets.active.fg_stroke = Stroke::new(1.0, fg_on_accent);
    widgets.active.corner_radius = Radii::WIDGET;

    widgets.open.bg_fill = pal.bg_widget_hover;
    widgets.open.weak_bg_fill = pal.bg_widget_hover;
    widgets.open.fg_stroke = Stroke::new(1.0, pal.text_primary);

    visuals.override_text_color = Some(pal.text_primary);
    visuals.hyperlink_color = pal.accent_light;
    visuals.code_bg_color = pal.bg_dark;

    visuals.selection.bg_fill = pal.accent;
    visuals.selection.stroke = Stroke::new(1.0, fg_on_accent);

    visuals.panel_fill = pal.bg_main;
    visuals.window_fill = pal.bg_elevated;
    visuals.window_stroke = if is_light {
        Stroke::new(1.0, Color32::from_black_alpha(30))
    } else {
        Stroke::new(1.0, pal.accent.gamma_multiply(0.3))
    };
    visuals.window_corner_radius = Radii::WINDOW;
    visuals.extreme_bg_color = pal.bg_dark;
    visuals.faint_bg_color = pal.bg_elevated;

    let shadow_alpha = if is_light { 25 } else { 60 };
    visuals.window_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(shadow_alpha),
    };
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 2,
        color: Color32::from_black_alpha(shadow_alpha + 20),
    };

    // Misc.
    visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    visuals.slider_trailing_fill = true;

    visuals
}

/// Apply layout-related (non-visual) tweaks to the global style.
fn apply_layout(style: &mut egui::Style) {
    style.spacing.item_spacing = Spacing::ITEM;
    style.spacing.interact_size = Spacing::INTERACT_SIZE;
    style.spacing.button_padding = Spacing::BUTTON_PADDING;
    style.spacing.window_margin = Spacing::WINDOW_MARGIN;
    style.animation_time = 0.12;
}

pub fn apply_theme(ctx: &egui::Context, preset: ThemePreset) {
    let pal = Palette::for_preset(preset);

    ACTIVE_PALETTE.with(|p| *p.borrow_mut() = pal);

    ctx.set_visuals(build_visuals(&pal));
    ctx.global_style_mut(apply_layout);
}
