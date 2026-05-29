# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0-beta.4] - 2026-05-29

### Added

- **Mojang JRE installer**: switched the Java download backend from Adoptium
  to the official Mojang JRE manifest. Mirrored host-for-host by BMCLAPI, so
  the existing download mirror chain handles failover for free. Streams
  files individually with SHA-1 verification — a mid-stream disconnect only
  re-fetches the affected file.
- Java source selector in Settings (extensible for future sources).
- File integrity check before launch with automatic re-download of missing
  or corrupted libraries / assets.
- Custom theme support via TOML files in `<data_dir>/themes/`.
- Two new bundled themes: **Sakura** and **Light**.
- Bundled Japanese locale (163/163 keys translated); CJK system-font fallback
  on all platforms.
- Auto-create `themes/` and `locales/` directories with example templates on
  first startup.
- Localized theme names; redesigned theme picker as equal-width grid.
- Redesigned language picker as a grid with native-name descriptions.
- `distclean.sh` to remove all build artifacts.

### Changed

- All palettes redesigned with the Morandi color system; theme swatches fixed
  to render the actual palette.
- Light theme: explicit text/border colors so titles, subheadings and widgets
  remain readable on a light background.
- Animation timings slowed to mobile-standard durations across the GUI.
- Locale loader now uses linear easing for color/opacity transitions and
  removes deleted languages on refresh.
- Theme directory is scanned only on Settings refresh — not every frame.
- New-instance dialog and Settings page layouts polished for visual
  consistency.
- Release artifacts simplified: Linux ships the AppImage directly + bare CLI
  binary; Windows ships the bare `.exe` (no zip wrapper).

### Fixed

- **Windows**: console window no longer flashes when spawning Java
  (`CREATE_NO_WINDOW` applied to all child processes).
- Avatar / cape cache is refreshed on startup instead of stale-loaded.
- `import_mrpack` now installs the base game and the required mod loader,
  not just the mods.
- NeoForge no longer reports compatible versions for Minecraft releases
  predating NeoForge support.
- Version matching and error propagation hardened across the version
  pipeline.
- `gamma_multiply` replaced with `lerp_color` / alpha for hover transitions
  (matches the actual painter capability and removes a class of color
  artifacts).
- CollapsibleCard arrow rotates around its center; collapse animation
  smoothed.
- Self-update compatibility restored after release-asset filename change.

### Internal / CI

- Renamed `tests/version_install.rs` to avoid a false-positive Windows UAC
  prompt during test execution.
- Removed code duplication and hardcoded values across GUI rendering paths.
- Theme picker grid unified; templates prefixed with `_` are now skipped on
  load and detected on deletion.

[0.2.0-beta.4]: https://github.com/WangSimiao2000/MiaoMinecraftLauncher/compare/v0.2.0-beta.3...v0.2.0-beta.4

## [0.2.0-beta.3] - 2026-05-28

### Added

- Custom animation system with spring physics (damped spring, velocity
  preservation on interruption) and CSS cubic-bezier easing (Newton-Raphson).
- Material Design 3 easing curves: standard, emphasized, decelerate, accelerate.
- Named spring presets calibrated from industry data (snappy, bouncy, gentle,
  smooth, playful).
- Spring-driven indicators: sidebar selection, detail tab underline, settings
  nav vertical bar — all smoothly follow the active item.
- Eased hover/select transitions on InstanceCard, CollapsibleCard, ModCard.
- Page slide transition with direction sense (push → right, pop → left).
- Dialog popup Y-offset + opacity easing (cubic_out 0.2s).
- Toast animations upgraded to MD3 emphasized-decelerate (enter) and
  standard-accelerate (exit) curves.
- Smooth progress bar with exponential lerp interpolation.
- Crossfade between detail view and welcome screen on instance deselect.
- `self_update` crate integration for atomic binary replacement.
- `perform_self_update()` one-shot function via GitHub Releases backend.
- Core library credits listed in Settings > About page.

### Changed

- Global `animation_time` reduced from 0.15s to 0.12s (desktop-optimized).
- CollapsibleCard: animated expand/collapse (0.25s) with rotating chevron.
- Settings nav: spring-driven indicator replaces per-item fade.
- Settings content area fades in on tab switch (0.18s).
- Account cards: animated active state fill transition.
- Instance selection and Settings are now mutually exclusive.
- Clicking a selected instance deselects it (returns to welcome).
- Settings button shows hover feedback when active + tooltip hint.

### Fixed

- Sidebar indicator no longer slides in from top on first selection (snap).
- Tab/settings indicators snap on first render, spring on subsequent changes.
- Sidebar indicator fades out on deselect instead of disappearing instantly.
- Detail panel crossfades on deselect instead of hard-cutting to welcome.

[0.2.0-beta.3]: https://github.com/WangSimiao2000/MiaoMinecraftLauncher/compare/v0.2.0-beta.2...v0.2.0-beta.3

## [0.2.0-beta.2] - 2026-05-27

### Added

- Authlib-injector login UI in Settings > Account.
- Multi game folder management with folder switching.
- Built-in FAQ help page.
- Rounded window corners via transparent viewport.

### Changed

- Upgraded eframe/egui from 0.30 to 0.34.
- Unified mod search UI: shared search result cards, shared version list
  renderer, single `PendingInstall` enum for both Modrinth and CurseForge.
- Removed `async-trait` / `async-recursion` crates in favor of native
  `async fn` in trait (Rust edition 2024).
- Virtualized long lists with `show_rows()` for better scroll performance.
- Rewrote README with screenshots, badges, and structured layout.
- Consolidated icon into `assets/icon.png` as single source of truth.

### Fixed

- Mod search confirmation panel now interrupts/replaces results instead of
  appearing below them.
- Search result highlight no longer paints over card content (z-order fix).
- Blur no longer paints over dialog on backdrop click.
- Settings nav panel flush to left edge; tabs centered.
- New instance dialog constrained to 420px width.
- Window close button replaced with custom 28px hit target.
- CollapsibleCard header is full-width clickable with hover feedback.
- Language preference persists across restarts.
- Console window hidden on Windows release builds.
- About page description updated to cross-platform.

### Internal / CI

- Bumped actions/checkout v4→v6, codecov-action v4→v6,
  softprops/action-gh-release v2→v3.
- Updated `zip` crate 2→8, `toml` crate 0.8→1.1.
- Release workflow uses CHANGELOG section as release body.

[0.2.0-beta.2]: https://github.com/WangSimiao2000/MiaoMinecraftLauncher/compare/v0.2.0-beta.1...v0.2.0-beta.2

## [0.2.0-beta.1] - 2026-05-27

This is the first prerelease of the 0.2.0 line. It collects 66 commits since
the initial 0.1.0 tag and brings cross-platform support, several large
feature additions, and an architectural overhaul of the GUI runtime.

### Highlights

- **Windows is now supported.** GUI runs correctly on HiDPI displays and
  matches the Linux look under any system theme.
- **CurseForge integration** lands alongside the existing Modrinth flow,
  with a built-in API key plus user override.
- **Authlib-injector login** for third-party skin sites (LittleSkin,
  Blessing Skin, etc.) works end to end.
- **Crash analyzer** parses crash reports / logs and points at the offending
  mod when it can.
- **Self-update** can fetch and replace the running binary from GitHub
  Releases.
- **GUI runtime** moved from `Arc<Mutex<AsyncState>>` to a channel-based
  controller with a shared tokio runtime, eliminating per-operation
  runtime creation.

### Added

- Microsoft OAuth device-code login with automatic token refresh before
  game launch.
- authlib-injector third-party skin-site login.
- CurseForge mod search and install with dependency resolution; built-in
  API key is shipped, users can override it in settings.
- Unified mod search with source selector (Modrinth / CurseForge), version
  flow, i18n.
- Modrinth `.mrpack` import / export.
- Multi-source download manager with BMCLAPI mirror failover and retry.
- Cross-platform Java detection and download via Adoptium.
- Self-update flow: detect a newer GitHub Release, download the matching
  asset, replace the running binary.
- Intelligent crash analysis that parses crash reports and game logs.
- Player skin / cape support: model selection, avatar display, cape on the
  account card, self-hosted skin/cape cropping and caching.
- Custom title bar with painted window-control icons; rounded window
  corners on supported platforms.
- Dynamic theme palette (Dark, Ocean, Forest, Warm).
- GPU blur shader for dialogs, with open / close animations.
- Bootstrap Icons font embedded; MiSans Medium for unified CJK + Latin
  rendering.
- Toast notifications, including on game crash.
- Settings panel pinned to the sidebar bottom.
- Parallel instance creation (per-task progress instead of a global
  installing flag).
- Security audit job in CI plus a Dependabot configuration for cargo and
  GitHub Actions.
- Windows release build in the release workflow; `check-windows` job in
  CI to keep Windows compilation green.
- AppImage build for Linux releases.

### Changed

- Refactored `LauncherService` into per-domain modules (auth, instance,
  java, loaders, mods).
- Migrated core errors from `anyhow` to typed `thiserror` variants;
  added coverage CI.
- GUI uses a shared tokio runtime instead of creating a fresh runtime
  per async call site (was 14 places).
- Settings rendered as an inline panel rather than replacing the full
  page.
- Background image removed; full i18n coverage; avatar loading fixed.
- Theme application refactored: `apply_theme` now builds an
  `egui::Visuals` from a dark baseline and applies layout tweaks
  separately, instead of patching a cloned `Style`. This prevents the
  host system's light theme from bleeding into widget fills.
- Release profile gets LTO, strip, and `codegen-units = 1`.
- Pinned Rust toolchain to `nightly-2026-04-14` for reproducibility.

### Fixed

- Windows: process is now per-monitor DPI aware (v2), so HiDPI rendering
  is no longer bitmap-stretched / blurry.
- Windows: widgets such as `+ New` no longer pick up the OS light theme
  and render with the configured palette in every theme preset.
- Account is required before launching the game; no fallback `Player`
  account in `LauncherService`.
- Microsoft tokens auto-refresh before launch.
- Tests no longer corrupt the user's real config file.
- Pre-1.13 launch arguments and natives extraction.
- Settings toggle behaviour and redundant header in the dialog.
- HTTP image loading is enabled so avatars and capes display.
- GNOME Wayland: removed the unreliable transparent window mode.
- Many smaller GUI interaction fixes (title-bar buttons clickable,
  filesystem errors surfaced to the user, etc.).
- Clippy nightly compatibility (redundant `&` in `format!` args, float
  literal annotations).

### Internal / CI

- Controller integration tests, graceful shutdown, parallel test API.
- Channel-based GUI architecture replaces `Arc<Mutex<...>>`.
- Cancellation support across long-running tasks.
- Pre-commit and pre-push git hooks for fmt / clippy.
- Release pipeline split into `build-linux`, `build-windows`, and a
  separate `release` job that publishes both artifacts on tag pushes.

### Known limitations

- Windows builds have not yet been validated on every HiDPI / multi-
  monitor / mixed-DPI configuration; please report visual issues.
- The Windows artifact ships only `miao-gui.exe`; the CLI is not
  published on Windows.

## [0.1.0] - 2026-05-21

- Initial release.

[0.2.0-beta.1]: https://github.com/WangSimiao2000/MiaoMinecraftLauncher/compare/v0.1.0...v0.2.0-beta.1
[0.1.0]: https://github.com/WangSimiao2000/MiaoMinecraftLauncher/releases/tag/v0.1.0
