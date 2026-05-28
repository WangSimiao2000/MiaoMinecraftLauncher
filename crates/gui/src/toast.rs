use std::time::Instant;

use eframe::egui;

use crate::animation::easing::{MD3_EMPHASIZED_DECELERATE, MD3_STANDARD_ACCELERATE};
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Success,
    Warning,
    Error,
    Info,
}

impl ToastLevel {
    fn color(&self) -> egui::Color32 {
        match self {
            ToastLevel::Success => theme::Colors::success(),
            ToastLevel::Warning => theme::Colors::warning(),
            ToastLevel::Error => theme::Colors::danger(),
            ToastLevel::Info => theme::Colors::accent(),
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            ToastLevel::Success => crate::icons::ICON_CHECK,
            ToastLevel::Warning => crate::icons::ICON_ERROR,
            ToastLevel::Error => crate::icons::ICON_X_CIRCLE,
            ToastLevel::Info => crate::icons::ICON_GLOBE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub created_at: Instant,
    pub duration_secs: f32,
}

const ENTER_DURATION: f32 = 0.25;
const EXIT_DURATION: f32 = 0.2;

impl Toast {
    pub fn new(message: impl Into<String>, level: ToastLevel) -> Self {
        let duration_secs = match level {
            ToastLevel::Error => 5.0,
            ToastLevel::Warning => 4.0,
            _ => 3.0,
        };
        Self {
            message: message.into(),
            level,
            created_at: Instant::now(),
            duration_secs,
        }
    }

    fn elapsed(&self) -> f32 {
        self.created_at.elapsed().as_secs_f32()
    }

    fn is_expired(&self) -> bool {
        self.elapsed() >= self.duration_secs
    }

    fn enter_t(&self) -> f32 {
        let t = (self.elapsed() / ENTER_DURATION).clamp(0.0, 1.0);
        MD3_EMPHASIZED_DECELERATE.eval(t)
    }

    fn exit_t(&self) -> f32 {
        let remaining = self.duration_secs - self.elapsed();
        if remaining >= EXIT_DURATION {
            return 0.0;
        }
        let t = (1.0 - remaining / EXIT_DURATION).clamp(0.0, 1.0);
        MD3_STANDARD_ACCELERATE.eval(t)
    }
}

#[derive(Default)]
pub struct ToastQueue {
    toasts: Vec<Toast>,
}

impl ToastQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, toast: Toast) {
        const MAX_TOASTS: usize = 5;
        if self.toasts.len() >= MAX_TOASTS {
            self.toasts.remove(0);
        }
        self.toasts.push(toast);
    }

    pub fn success(&mut self, msg: impl Into<String>) {
        self.push(Toast::new(msg, ToastLevel::Success));
    }

    pub fn warning(&mut self, msg: impl Into<String>) {
        self.push(Toast::new(msg, ToastLevel::Warning));
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.push(Toast::new(msg, ToastLevel::Error));
    }

    pub fn info(&mut self, msg: impl Into<String>) {
        self.push(Toast::new(msg, ToastLevel::Info));
    }

    pub fn has_active(&self) -> bool {
        !self.toasts.is_empty()
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        self.toasts.retain(|t| !t.is_expired());

        if self.toasts.is_empty() {
            return;
        }

        ctx.request_repaint();

        let screen_rect = ctx.content_rect();
        let toast_width = 320.0_f32.min(screen_rect.width() - 32.0);
        let base_x = screen_rect.right() - toast_width - 16.0;
        let mut y = screen_rect.top() + 50.0;

        for toast in &self.toasts {
            let enter = toast.enter_t();
            let exit = toast.exit_t();

            let alpha = ((enter * (1.0 - exit)) * 255.0) as u8;
            let slide_in = (1.0 - enter) * 40.0;
            let slide_out = exit * 20.0;

            let area = egui::Area::new(egui::Id::new(&toast.message).with(toast.created_at))
                .fixed_pos(egui::pos2(base_x + slide_in + slide_out, y))
                .interactable(false)
                .order(egui::Order::Foreground);

            let response = area.show(ctx, |ui| {
                let color = toast.level.color();
                let frame = egui::Frame::NONE
                    .fill(theme::Colors::bg_elevated().gamma_multiply(alpha as f32 / 255.0))
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .stroke(egui::Stroke::new(
                        1.0,
                        color.gamma_multiply(alpha as f32 / 255.0),
                    ));

                frame.show(ui, |ui| {
                    ui.set_width(toast_width - 24.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(toast.level.icon())
                                .color(color.gamma_multiply(alpha as f32 / 255.0))
                                .size(14.0),
                        );
                        ui.label(
                            egui::RichText::new(&toast.message)
                                .color(
                                    theme::Colors::text_primary()
                                        .gamma_multiply(alpha as f32 / 255.0),
                                )
                                .size(13.0),
                        );
                    });
                });
            });

            y += response.response.rect.height() + 6.0;
        }
    }
}
