//! Platform-specific GUI behavior, centralized.
//!
//! Every conditional that affects window chrome, native font fallback, or
//! Win32/AppKit calls lives here. Callers in `main.rs` and `app.rs` should
//! never reach for `#[cfg]` or `std::env::consts::OS` directly — call into
//! this module instead so the rules stay consistent and testable as new
//! platforms are added.

use eframe::egui;

/// Apply OS-appropriate window chrome to the eframe viewport builder.
///
/// - **macOS**: keep the native title bar so the traffic-light buttons
///   (close / minimize / zoom) remain in their conventional position with
///   their usual behaviors (Cmd+W, double-click to maximize, full-screen,
///   accessibility). Hide the title bar's painted area and the title text
///   so our own header content can extend up to the very top of the
///   window via `fullsize_content_view`.
/// - **Windows / Linux**: drop the OS chrome entirely so we can paint our
///   own title bar (`render_title_bar` in `app.rs`). Linux DEs vary too
///   much for native chrome to look consistent, and Windows native chrome
///   doesn't blend with our themes.
pub fn apply_window_chrome(builder: egui::ViewportBuilder) -> egui::ViewportBuilder {
    #[cfg(target_os = "macos")]
    {
        builder
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        builder.with_decorations(false).with_transparent(true)
    }
}

/// Whether the app should paint its own close / minimize / maximize
/// buttons in the title bar. macOS uses the native traffic lights, so we
/// only paint our own buttons elsewhere.
pub const fn should_render_custom_window_buttons() -> bool {
    !cfg!(target_os = "macos")
}

/// Left-edge inset (in egui points) that title-bar content must respect
/// so it doesn't paint underneath the macOS traffic lights. The traffic
/// light cluster is roughly 70 px wide; we leave a small gap on top.
pub const fn title_bar_left_padding() -> f32 {
    if cfg!(target_os = "macos") { 78.0 } else { 4.0 }
}

/// Tell Windows that this process is per-monitor DPI aware (v2) so winit
/// reads the correct scale factor instead of getting a bitmap-stretched
/// window on HiDPI displays. Must run before any window is created. No-op
/// on every other platform.
#[cfg(target_os = "windows")]
pub fn enable_high_dpi_awareness() {
    use std::ffi::c_void;

    type DpiContext = *mut c_void;
    const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: DpiContext = -4_isize as DpiContext;

    unsafe extern "system" {
        fn SetProcessDpiAwarenessContext(value: DpiContext) -> i32;
    }

    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn enable_high_dpi_awareness() {}

/// Filesystem paths to probe for a CJK / Latin fallback font.
///
/// Used after MiSans Medium has been registered as the primary font, to
/// fill in any glyphs MiSans doesn't cover (Japanese, Korean, etc.). The
/// caller loads the first path that exists and silently continues if
/// none are present.
pub fn system_fallback_font_paths() -> &'static [&'static str] {
    #[cfg(target_os = "linux")]
    {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/OTF/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ]
    }
    #[cfg(target_os = "windows")]
    {
        &[
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\malgun.ttf",
            "C:\\Windows\\Fonts\\yugothic.ttf",
            "C:\\Windows\\Fonts\\meiryo.ttc",
            "C:\\Windows\\Fonts\\simsun.ttc",
        ]
    }
    #[cfg(target_os = "macos")]
    {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/AppleSDGothicNeo.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_window_buttons_disabled_on_macos() {
        if cfg!(target_os = "macos") {
            assert!(!should_render_custom_window_buttons());
        } else {
            assert!(should_render_custom_window_buttons());
        }
    }

    #[test]
    fn title_bar_left_padding_reserves_space_for_traffic_lights_on_macos() {
        if cfg!(target_os = "macos") {
            assert!(title_bar_left_padding() >= 70.0);
        } else {
            assert!(title_bar_left_padding() < 16.0);
        }
    }

    #[test]
    fn system_fallback_font_paths_nonempty_on_supported_platforms() {
        if cfg!(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "macos"
        )) {
            assert!(!system_fallback_font_paths().is_empty());
        }
    }

    #[test]
    fn enable_high_dpi_awareness_is_safe_to_call() {
        enable_high_dpi_awareness();
    }
}
