# Cross-Platform Support

The launcher supports **Linux, macOS, and Windows**. This document describes
how each platform-specific concern is handled in code and what remains to be
done at the CI/release level.

## Status Summary

| Area | Files | Status |
|------|-------|--------|
| Java detection | `core/src/java/mod.rs` | ✅ Implemented |
| Java installation | `core/src/java/install.rs`, `core/src/java/mojang.rs` | ✅ Implemented (Mojang JRE source) |
| Version meta library filtering | `core/src/version/meta.rs` | ✅ Implemented |
| Native library extraction | `core/src/version/install.rs` | ✅ Implemented (driven by library rules) |
| Config / data directories | `core/src/config.rs` | ✅ Implemented |
| GUI font loading & CJK fallback | `crates/gui/src/main.rs` | ✅ Implemented |
| Console window suppression on Windows | `core/src/process.rs` | ✅ Implemented |
| Per-monitor DPI awareness on Windows | `crates/gui/src/main.rs` | ✅ Implemented |
| CI Windows compilation check | `.github/workflows/ci.yml` (`check-windows`), `release.yml` (`build-windows`) | ✅ Implemented |
| CI macOS compilation check | `.github/workflows/ci.yml` (`check-macos`), `release.yml` (`build-macos`) | ✅ Implemented (aarch64) |
| CI Linux build + AppImage | `.github/workflows/release.yml` | ✅ Implemented |
| CI macOS build | `.github/workflows/release.yml` | ✅ Implemented (aarch64 / Apple Silicon, .app bundle). Intel (`x86_64-apple-darwin`) was tried once and pulled — the macos-13 GitHub-hosted runner queue regularly exceeds one hour, blocking releases. Apple stopped selling Intel Macs in 2023. |
| Released CLI binary on Windows | `.github/workflows/release.yml` | ❌ Intentionally skipped — adding a console binary to `PATH` on Windows is awkward; WSL users can use the Linux binary, and the CLI is a strict subset of the GUI (no MS OAuth / CurseForge), so it isn't useful enough for interactive Windows users. Re-enable if real demand surfaces. |

## Implementation Details

### 1. Java Detection (`crates/core/src/java/mod.rs`)

Search paths are dispatched on `std::env::consts::OS`:

```rust
fn system_java_search_paths() -> Vec<String> {
    match std::env::consts::OS {
        "linux" => /* /usr/lib/jvm, /usr/local/lib/jvm, /usr/java, ... */,
        "macos" => /* /Library/Java/JavaVirtualMachines, /usr/local/opt/openjdk, ... */,
        "windows" => /* C:\Program Files\Java, C:\Program Files (x86)\Java, ... */,
        _ => vec![],
    }
}
```

`JAVA_HOME` is always honored. The Java executable name is `java.exe` on
Windows and `java` elsewhere.

### 2. Java Installation (`crates/core/src/java/install.rs` + `mojang.rs`)

The launcher fetches the **official Mojang JRE manifest** from
`piston-meta.mojang.com/v1/products/java-runtime/.../all.json`. This replaces
the earlier Adoptium-based flow and avoids the GitHub-redirect issues common
in mainland China — BMCLAPI mirrors `piston-meta.mojang.com` and
`piston-data.mojang.com` host-for-host, so the existing download mirror chain
handles failover for free.

Files are downloaded individually with SHA-1 verification: a mid-stream
disconnect re-fetches only the affected file, not the whole archive. The
installer is source-neutral via `JavaSource` + `JavaInstallPlan`, so adding
another upstream (e.g. Adoptium fallback) only requires implementing one
`async fn(&Client, &LauncherConfig, u32) -> Result<JavaInstallPlan>`.

### 3. Version Meta Library Filtering (`crates/core/src/version/meta.rs`)

`current_os_name()` maps `std::env::consts::OS` to Mojang's metadata
vocabulary:

```rust
pub fn current_os_name() -> &'static str {
    match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "osx",      // Mojang uses "osx", not "macos"
        "windows" => "windows",
        other => other,
    }
}
```

This drives both library-rule filtering and native classifier selection
(`natives-linux` / `natives-windows` / `natives-osx`).

### 4. Config / Data Directories (`crates/core/src/config.rs`)

Resolved by `dirs::config_dir()` / `dirs::data_dir()`:

| Platform | Config | Data |
|----------|--------|------|
| Linux | `~/.config/miao-minecraft-launcher/` | `~/.local/share/miao-minecraft-launcher/` |
| macOS | `~/Library/Application Support/miao-minecraft-launcher/` | same |
| Windows | `%APPDATA%\miao-minecraft-launcher\` | same |
| Portable | `<exe_dir>/mmcl-data/` (auto-detected) | same |

### 5. GUI Font Loading (`crates/gui/src/main.rs`)

The launcher bundles **MiSans Medium** (Latin + CJK Simplified) for primary
rendering. For glyphs MiSans does not cover (Japanese/Korean specifically),
`find_system_fallback_font()` probes platform-specific paths:

| OS | Probed paths |
|----|--------------|
| Linux | Noto Sans CJK (`/usr/share/fonts/opentype/noto/`, `noto-cjk/`, `google-noto-cjk/`, `truetype/noto/`), DejaVu Sans |
| Windows | Microsoft YaHei (`msyh.ttc`), Malgun Gothic, Yu Gothic, Meiryo, SimSun |
| macOS | PingFang, Apple SD Gothic Neo, Hiragino Sans GB, Arial Unicode |

The first existing file is loaded as a fallback face. If none are found, the
launcher still runs with bundled MiSans only.

### 6. Native Libraries (`crates/core/src/version/install.rs`)

Minecraft ships per-OS native classifiers (`natives-linux`,
`natives-windows`, `natives-osx`). The download task collection picks the
correct classifier through the rule filter described in section 3 — no
separate platform branch is needed in `install.rs`.

### 7. Windows-Specific Fixes

- **Per-monitor DPI awareness v2** (`crates/gui/src/main.rs`,
  `enable_windows_dpi_awareness`): calls `SetProcessDpiAwarenessContext`
  before window creation so HiDPI rendering is sharp instead of bitmap-
  stretched.
- **Console suppression** (`core/src/process.rs`): the launcher itself is
  built with `windows_subsystem = "windows"`, but spawning Java otherwise
  flashes a console. The helper applies `CREATE_NO_WINDOW` on Windows and
  is a no-op on other platforms.

## CI / Release Status

`.github/workflows/`:

- `ci.yml` — Linux build + clippy + tests on every push and PR.
  `check-windows` and `check-macos` jobs keep Windows and macOS
  (Apple Silicon) compilation green on every push, so platform-specific
  branches behind `#[cfg(target_os = "...")]` are exercised before tag
  push instead of only at release time.
- `release.yml` — On `v*` tag push:
  - `build-linux` — CLI binary + AppImage
  - `build-windows` — `miao-gui.exe` only
  - `build-macos` — `aarch64-apple-darwin` on macos-14 (Apple Silicon).
    Produces a tarred `.app` bundle plus a bare CLI binary.
  - `release` — collects all artifacts and creates the GitHub Release.

### Outstanding CI Work

| Item | Effort | Notes |
|------|--------|-------|
| `cargo-dist` / unified packaging | unscoped | Long-term, replace per-OS shell snippets |
| macOS notarization / code signing | unscoped | Unsigned `.app` bundle currently triggers Gatekeeper warning |
| Windows code signing | unscoped | Unsigned binaries trigger SmartScreen warning |

## Dependencies (cross-platform notes)

| Crate | Notes |
|-------|-------|
| `tar` + `flate2` | Used for `.tar.gz` Java archives on Linux/macOS |
| `zip` | Used for `.zip` archives (Windows Java, mrpack import) |
| `open` | Cross-platform; opens URLs / paths in the OS default handler |
| `dirs` | Cross-platform user directory resolution |
| `rfd` | Cross-platform native file dialogs |
