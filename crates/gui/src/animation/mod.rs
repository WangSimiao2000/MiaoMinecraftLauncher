//! Animation system for MMCL GUI.
//!
//! This module provides two complementary animation approaches:
//!
//! - **Easing-based** (via `egui_animation` + custom cubic-bezier): Time-driven
//!   animations with a fixed duration and a shaping curve. Ideal for hover states,
//!   page transitions, and any animation with a known start/end.
//!
//! - **Spring physics** (custom implementation): Physically-simulated motion that
//!   preserves velocity on interruption and feels natural. Ideal for interactive
//!   elements (indicators, drag targets, follow-the-cursor).
//!
//! # Architecture
//!
//! ```text
//! animation/
//! ├── mod.rs       ← Public API, re-exports
//! ├── spring.rs    ← Spring physics (self-built, ~80 lines)
//! ├── easing.rs    ← Cubic-bezier + utility wrappers (self-built)
//! └── preset.rs    ← Named animation configs for consistent feel
//! ```
//!
//! # Usage with egui
//!
//! ```rust,ignore
//! // Spring (store in your App struct, tick every frame):
//! let dt = ctx.input(|i| i.stable_dt);
//! self.indicator_y.set_target(selected_y);
//! let y = self.indicator_y.tick(dt);
//! if !self.indicator_y.is_settled() { ctx.request_repaint(); }
//!
//! // Easing (stateless, via egui's Id-based system):
//! let t = egui_animation::animate_eased(
//!     ctx, "panel_open", if open { 1.0 } else { 0.0 },
//!     0.25, emath::easing::cubic_out,
//! );
//! ```

pub mod easing;
pub mod preset;
pub mod spring;

pub use easing::{
    CubicBezier, MD3_EMPHASIZED_ACCELERATE, MD3_EMPHASIZED_DECELERATE, MD3_STANDARD,
    MD3_STANDARD_ACCELERATE, MD3_STANDARD_DECELERATE, SNAPPY,
};
pub use preset::{
    AnimPreset, PRESET_BOUNCY, PRESET_GENTLE, PRESET_PLAYFUL, PRESET_SMOOTH, PRESET_SNAPPY,
};
pub use spring::{Spring, SpringConfig};

pub use eframe::emath::easing::cubic_out as ease_out;
pub use eframe::emath::easing::linear as ease_linear;

pub const DURATION_HOVER: f32 = 0.2;
pub const DURATION_SELECT: f32 = 0.25;
pub const DURATION_FADE: f32 = 0.3;
pub const DURATION_DIALOG: f32 = 0.3;
pub const DURATION_EXPAND: f32 = 0.35;
pub const DURATION_PAGE: f32 = 0.35;

pub fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

use eframe::egui;
