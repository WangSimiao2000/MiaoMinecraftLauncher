use eframe::egui;

use crate::animation::lerp_color;
use crate::app::{DetailTab, I18n, MiaoApp};
use crate::navigation::Page;
use crate::theme;
use crate::widgets::InstanceCard;

const INDICATOR_WIDTH: f32 = 3.0;
const INDICATOR_HEIGHT: f32 = 20.0;

impl MiaoApp {
    #[allow(deprecated)]
    pub fn render_sidebar(&mut self, ctx: &egui::Context) {
        let lang = self.language.clone();
        egui::Panel::left("instance_list")
            .resizable(true)
            .default_size(240.0)
            .frame(theme::panel_frame())
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(theme::subheading(I18n::t(&lang, "instances")));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(I18n::t(&lang, "import")).clicked() {
                            self.import_with_dialog();
                        }
                        if ui.small_button(I18n::t(&lang, "new")).clicked() {
                            self.open_new_instance_dialog();
                        }
                    });
                });
                ui.add_space(theme::Spacing::SMALL_GAP);
                ui.separator();
                ui.add_space(theme::Spacing::SMALL_GAP);

                if self.instances.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.label(theme::muted(I18n::t(&lang, "no_instances")));
                        ui.add_space(8.0);
                        ui.label(theme::small(I18n::t(&lang, "no_instances_hint")));
                    });
                } else {
                    let row_height = 36.0;
                    let total = self.instances.len();
                    let scroll_area_top = ui.cursor().top();
                    let mut selected_card_center_y: Option<f32> = None;

                    egui::ScrollArea::vertical().show_rows(
                        ui,
                        row_height,
                        total,
                        |ui, row_range| {
                            for i in row_range {
                                let selected = self.selected_instance == Some(i);
                                let inst = &self.instances[i];
                                let loader_label =
                                    inst.mod_loader.as_ref().map(|l| l.loader_type.to_string());

                                let response = InstanceCard::new(&inst.name, selected)
                                    .loader(loader_label.as_deref())
                                    .show(ui);

                                if selected {
                                    selected_card_center_y =
                                        Some(response.rect.center().y - scroll_area_top);
                                }

                                if response.clicked() {
                                    if selected {
                                        self.selected_instance = None;
                                    } else {
                                        self.selected_instance = Some(i);
                                        self.active_tab = DetailTab::Mods;
                                        self.confirm_delete = None;
                                        if matches!(self.nav_stack.current(), Page::Settings) {
                                            self.nav_stack.pop();
                                        }
                                    }
                                }
                            }
                        },
                    );

                    let has_selection = selected_card_center_y.is_some();
                    let indicator_opacity = ui.ctx().animate_bool_with_time_and_easing(
                        egui::Id::new("sidebar_indicator_opacity"),
                        has_selection,
                        0.25,
                        crate::animation::ease_out,
                    );

                    if let Some(center_y) = selected_card_center_y {
                        let target_y = scroll_area_top + center_y;
                        if self.sidebar_indicator_y.position() == 0.0
                            && self.sidebar_indicator_y.velocity() == 0.0
                        {
                            self.sidebar_indicator_y.snap(target_y);
                        } else {
                            self.sidebar_indicator_y.set_target(target_y);
                        }
                    }

                    if indicator_opacity > 0.0 {
                        let y = self.sidebar_indicator_y.position();
                        let panel_left = ui.min_rect().left();
                        let indicator_rect = egui::Rect::from_min_size(
                            egui::pos2(panel_left, y - INDICATOR_HEIGHT / 2.0),
                            egui::vec2(INDICATOR_WIDTH, INDICATOR_HEIGHT),
                        );
                        ui.painter().rect_filled(
                            indicator_rect,
                            egui::CornerRadius::same(2),
                            theme::Colors::accent_light().gamma_multiply(indicator_opacity),
                        );
                    }
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(6.0);
                    let btn_width = ui.available_width();
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(btn_width, 32.0), egui::Sense::click());

                    let is_active = matches!(self.nav_stack.current(), Page::Settings);

                    let hover_t = ui.ctx().animate_bool_with_time_and_easing(
                        response.id.with("settings_hover"),
                        response.hovered(),
                        0.2,
                        crate::animation::ease_out,
                    );
                    let active_t = ui.ctx().animate_bool_with_time_and_easing(
                        response.id.with("settings_active"),
                        is_active,
                        0.3,
                        crate::animation::ease_out,
                    );

                    let base_alpha = 0.3 * active_t;
                    let hover_boost = if is_active { 0.15 * hover_t } else { hover_t };
                    let bg_color = if base_alpha + hover_boost > 0.0 {
                        if active_t > 0.0 {
                            theme::Colors::bg_widget_active()
                                .gamma_multiply(base_alpha + hover_boost)
                        } else {
                            theme::Colors::bg_widget_hover().gamma_multiply(hover_boost)
                        }
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    if bg_color != egui::Color32::TRANSPARENT {
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(4), bg_color);
                    }

                    let active_color = theme::Colors::accent_light();
                    let idle_color = theme::Colors::text_secondary();
                    let text_color = if active_t > 0.0 {
                        lerp_color(idle_color, active_color, active_t)
                    } else if hover_t > 0.0 {
                        lerp_color(idle_color, theme::Colors::text_primary(), hover_t)
                    } else {
                        idle_color
                    };

                    ui.painter().text(
                        rect.left_center() + egui::vec2(12.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        format!(
                            "{}  {}",
                            crate::icons::ICON_GEAR,
                            I18n::t(&lang, "settings")
                        ),
                        egui::FontId::proportional(13.0),
                        text_color,
                    );

                    if is_active && response.hovered() {
                        egui::show_tooltip_at_pointer(
                            ui.ctx(),
                            ui.layer_id(),
                            egui::Id::new("settings_tooltip"),
                            |ui| {
                                ui.label(I18n::t(&lang, "click_to_exit_settings"));
                            },
                        );
                    }

                    if response.clicked() {
                        if is_active {
                            self.nav_stack.pop();
                        } else {
                            self.selected_instance = None;
                            self.nav_stack.push(Page::Settings);
                        }
                    }

                    ui.add_space(4.0);
                    ui.separator();
                });
            });
    }
}
