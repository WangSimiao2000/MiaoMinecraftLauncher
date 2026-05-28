use std::cell::RefCell;

use egui::{Color32, CornerRadius, Margin, RichText, Stroke, Vec2};
pub use miao_core::config::ThemePreset;
use miao_core::custom_theme::ThemeColors as CustomColors;

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
                bg_dark: Color32::from_rgb(24, 25, 30),
                bg_panel: Color32::from_rgb(28, 30, 35),
                bg_main: Color32::from_rgb(33, 35, 40),
                bg_elevated: Color32::from_rgb(40, 43, 50),
                bg_widget: Color32::from_rgb(48, 52, 60),
                bg_widget_hover: Color32::from_rgb(62, 67, 78),
                bg_widget_active: Color32::from_rgb(108, 142, 180),
                accent: Color32::from_rgb(108, 142, 180),
                accent_light: Color32::from_rgb(148, 178, 210),
                success: Color32::from_rgb(110, 168, 120),
                danger: Color32::from_rgb(185, 105, 100),
                warning: Color32::from_rgb(200, 170, 95),
                text_primary: Color32::from_rgb(218, 220, 228),
                text_secondary: Color32::from_rgb(155, 160, 172),
                text_muted: Color32::from_rgb(115, 120, 132),
                text_disabled: Color32::from_rgb(78, 82, 92),
            },
            ThemePreset::Ocean => Self {
                bg_dark: Color32::from_rgb(22, 28, 38),
                bg_panel: Color32::from_rgb(26, 33, 44),
                bg_main: Color32::from_rgb(30, 38, 50),
                bg_elevated: Color32::from_rgb(38, 48, 62),
                bg_widget: Color32::from_rgb(44, 56, 72),
                bg_widget_hover: Color32::from_rgb(55, 70, 88),
                bg_widget_active: Color32::from_rgb(95, 145, 180),
                accent: Color32::from_rgb(95, 145, 180),
                accent_light: Color32::from_rgb(135, 182, 215),
                success: Color32::from_rgb(100, 165, 115),
                danger: Color32::from_rgb(180, 100, 100),
                warning: Color32::from_rgb(195, 165, 85),
                text_primary: Color32::from_rgb(210, 222, 235),
                text_secondary: Color32::from_rgb(148, 165, 182),
                text_muted: Color32::from_rgb(108, 125, 145),
                text_disabled: Color32::from_rgb(72, 88, 105),
            },
            ThemePreset::Forest => Self {
                bg_dark: Color32::from_rgb(24, 28, 25),
                bg_panel: Color32::from_rgb(28, 33, 29),
                bg_main: Color32::from_rgb(33, 38, 34),
                bg_elevated: Color32::from_rgb(40, 48, 42),
                bg_widget: Color32::from_rgb(48, 58, 50),
                bg_widget_hover: Color32::from_rgb(60, 72, 62),
                bg_widget_active: Color32::from_rgb(105, 155, 115),
                accent: Color32::from_rgb(105, 155, 115),
                accent_light: Color32::from_rgb(140, 190, 148),
                success: Color32::from_rgb(105, 165, 110),
                danger: Color32::from_rgb(180, 100, 95),
                warning: Color32::from_rgb(190, 165, 80),
                text_primary: Color32::from_rgb(212, 222, 215),
                text_secondary: Color32::from_rgb(155, 168, 158),
                text_muted: Color32::from_rgb(118, 130, 120),
                text_disabled: Color32::from_rgb(78, 88, 80),
            },
            ThemePreset::Warm => Self {
                bg_dark: Color32::from_rgb(30, 27, 24),
                bg_panel: Color32::from_rgb(35, 32, 28),
                bg_main: Color32::from_rgb(40, 37, 33),
                bg_elevated: Color32::from_rgb(50, 46, 40),
                bg_widget: Color32::from_rgb(60, 54, 46),
                bg_widget_hover: Color32::from_rgb(75, 68, 58),
                bg_widget_active: Color32::from_rgb(188, 155, 95),
                accent: Color32::from_rgb(188, 155, 95),
                accent_light: Color32::from_rgb(218, 185, 125),
                success: Color32::from_rgb(110, 165, 105),
                danger: Color32::from_rgb(185, 105, 95),
                warning: Color32::from_rgb(200, 168, 85),
                text_primary: Color32::from_rgb(228, 222, 215),
                text_secondary: Color32::from_rgb(172, 165, 155),
                text_muted: Color32::from_rgb(132, 125, 115),
                text_disabled: Color32::from_rgb(90, 84, 76),
            },
            ThemePreset::Sakura => Self {
                bg_dark: Color32::from_rgb(30, 26, 29),
                bg_panel: Color32::from_rgb(35, 30, 34),
                bg_main: Color32::from_rgb(40, 35, 39),
                bg_elevated: Color32::from_rgb(50, 44, 48),
                bg_widget: Color32::from_rgb(60, 52, 57),
                bg_widget_hover: Color32::from_rgb(76, 65, 72),
                bg_widget_active: Color32::from_rgb(175, 120, 145),
                accent: Color32::from_rgb(175, 120, 145),
                accent_light: Color32::from_rgb(210, 155, 178),
                success: Color32::from_rgb(110, 165, 115),
                danger: Color32::from_rgb(185, 100, 100),
                warning: Color32::from_rgb(195, 162, 85),
                text_primary: Color32::from_rgb(232, 220, 228),
                text_secondary: Color32::from_rgb(175, 160, 168),
                text_muted: Color32::from_rgb(138, 122, 130),
                text_disabled: Color32::from_rgb(92, 78, 85),
            },
            ThemePreset::Light => Self {
                bg_dark: Color32::from_rgb(215, 218, 222),
                bg_panel: Color32::from_rgb(228, 230, 234),
                bg_main: Color32::from_rgb(238, 240, 243),
                bg_elevated: Color32::from_rgb(247, 248, 250),
                bg_widget: Color32::from_rgb(220, 223, 228),
                bg_widget_hover: Color32::from_rgb(205, 210, 216),
                bg_widget_active: Color32::from_rgb(88, 125, 162),
                accent: Color32::from_rgb(88, 125, 162),
                accent_light: Color32::from_rgb(65, 102, 140),
                success: Color32::from_rgb(75, 138, 82),
                danger: Color32::from_rgb(168, 72, 72),
                warning: Color32::from_rgb(165, 130, 55),
                text_primary: Color32::from_rgb(40, 44, 52),
                text_secondary: Color32::from_rgb(85, 92, 102),
                text_muted: Color32::from_rgb(115, 122, 132),
                text_disabled: Color32::from_rgb(158, 164, 172),
            },
        }
    }

    pub fn from_custom(c: &CustomColors) -> Self {
        Self {
            bg_dark: Color32::from_rgb(c.bg_dark[0], c.bg_dark[1], c.bg_dark[2]),
            bg_panel: Color32::from_rgb(c.bg_panel[0], c.bg_panel[1], c.bg_panel[2]),
            bg_main: Color32::from_rgb(c.bg_main[0], c.bg_main[1], c.bg_main[2]),
            bg_elevated: Color32::from_rgb(c.bg_elevated[0], c.bg_elevated[1], c.bg_elevated[2]),
            bg_widget: Color32::from_rgb(c.bg_widget[0], c.bg_widget[1], c.bg_widget[2]),
            bg_widget_hover: Color32::from_rgb(
                c.bg_widget_hover[0],
                c.bg_widget_hover[1],
                c.bg_widget_hover[2],
            ),
            bg_widget_active: Color32::from_rgb(c.accent[0], c.accent[1], c.accent[2]),
            accent: Color32::from_rgb(c.accent[0], c.accent[1], c.accent[2]),
            accent_light: Color32::from_rgb(
                c.accent_light[0],
                c.accent_light[1],
                c.accent_light[2],
            ),
            success: Color32::from_rgb(c.success[0], c.success[1], c.success[2]),
            danger: Color32::from_rgb(c.danger[0], c.danger[1], c.danger[2]),
            warning: Color32::from_rgb(c.warning[0], c.warning[1], c.warning[2]),
            text_primary: Color32::from_rgb(
                c.text_primary[0],
                c.text_primary[1],
                c.text_primary[2],
            ),
            text_secondary: Color32::from_rgb(
                c.text_secondary[0],
                c.text_secondary[1],
                c.text_secondary[2],
            ),
            text_muted: Color32::from_rgb(c.text_muted[0], c.text_muted[1], c.text_muted[2]),
            text_disabled: Color32::from_rgb(
                c.text_disabled[0],
                c.text_disabled[1],
                c.text_disabled[2],
            ),
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
    RichText::new(text)
        .size(Fonts::TITLE)
        .strong()
        .color(Colors::text_primary())
}

pub fn subheading(text: &str) -> RichText {
    RichText::new(text)
        .size(Fonts::SUBHEADING)
        .strong()
        .color(Colors::text_primary())
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
    style.animation_time = 0.2;
}

pub fn swatch_colors(preset: ThemePreset) -> (Color32, Color32) {
    let pal = Palette::for_preset(preset);
    (pal.bg_main, pal.accent)
}

pub fn apply_theme(ctx: &egui::Context, preset: ThemePreset) {
    let pal = Palette::for_preset(preset);

    ACTIVE_PALETTE.with(|p| *p.borrow_mut() = pal);

    ctx.set_visuals(build_visuals(&pal));
    ctx.global_style_mut(apply_layout);
}

pub fn apply_custom_theme(ctx: &egui::Context, colors: &CustomColors) {
    let pal = Palette::from_custom(colors);

    ACTIVE_PALETTE.with(|p| *p.borrow_mut() = pal);

    ctx.set_visuals(build_visuals(&pal));
    ctx.global_style_mut(apply_layout);
}

pub fn custom_swatch_colors(colors: &CustomColors) -> (Color32, Color32) {
    (
        Color32::from_rgb(colors.bg_main[0], colors.bg_main[1], colors.bg_main[2]),
        Color32::from_rgb(colors.accent[0], colors.accent[1], colors.accent[2]),
    )
}
