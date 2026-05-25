# Cross-Platform Support Roadmap

The launcher currently targets **Linux only**. This document catalogs all platform-specific code that must be changed to support Windows and macOS.

## Summary

| Area | Files | Effort |
|------|-------|--------|
| Java detection | `core/src/java/mod.rs` | Medium |
| Java download | `core/src/java/download.rs` | Medium |
| Version meta library filtering | `core/src/version/meta.rs` | Low |
| Config default paths | `core/src/config.rs` | Low |
| GUI font loading | `gui/src/main.rs` | Low |
| Native library extraction | `core/src/version/install.rs` | Low |

## Detailed Changes Required

### 1. Java Detection (`crates/core/src/java/mod.rs`)

**Current:** Hardcoded Linux paths `/usr/lib/jvm`, `/usr/local/lib/jvm`, `/usr/java`.

**Required:**
```rust
fn system_java_search_paths() -> Vec<&'static str> {
    match std::env::consts::OS {
        "linux" => vec!["/usr/lib/jvm", "/usr/local/lib/jvm", "/usr/java"],
        "macos" => vec!["/Library/Java/JavaVirtualMachines", "/usr/local/opt/openjdk"],
        "windows" => vec!["C:\\Program Files\\Java", "C:\\Program Files (x86)\\Java"],
        _ => vec![],
    }
}
```

Additionally on Windows:
- Check `JAVA_HOME` environment variable
- Query Windows Registry: `HKLM\SOFTWARE\JavaSoft\Java Runtime Environment`
- Binary name is `java.exe` not `java`

### 2. Java Download (`crates/core/src/java/download.rs`)

**Current:** Line 69 hardcodes `os=linux` in Adoptium API URL. Archive extraction uses `tar::Archive` + `flate2` (tar.gz).

**Required:**
```rust
let os = match std::env::consts::OS {
    "linux" => "linux",
    "macos" => "mac",
    "windows" => "windows",
    other => other,
};
// URL: ...&os={os}
```

Archive extraction must branch:
- Linux/macOS: `.tar.gz` → `tar` + `flate2` (current)
- Windows: `.zip` → `zip` crate

Binary discovery:
- Linux/macOS: `bin/java`
- Windows: `bin/java.exe`

### 3. Version Meta Library Filtering (`crates/core/src/version/meta.rs`)

**Current:** Line 97 hardcodes `os.name == Some("linux")`.

**Required:**
```rust
fn current_os_name() -> &'static str {
    match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "osx",      // Mojang uses "osx" not "macos"
        "windows" => "windows",
        other => other,
    }
}
// Then: os.name.as_deref() == Some(current_os_name())
```

Note: Mojang's metadata uses `"osx"` for macOS, not `"macos"`.

### 4. Config Default Paths (`crates/core/src/config.rs`)

**Current:** Fallback is `~/.local/share` (Linux XDG).

**Required:** The `dirs::data_dir()` call already handles cross-platform correctly:
- Linux: `~/.local/share`
- macOS: `~/Library/Application Support`
- Windows: `C:\Users\<user>\AppData\Roaming`

Only change needed: remove the hardcoded `PathBuf::from("~/.local/share")` fallback and use a truly cross-platform fallback (e.g., current directory).

### 5. GUI Font Loading (`crates/gui/src/main.rs`)

**Current:** CJK font search paths are all Linux:
```
/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc
/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc
...
```

**Required:**
```rust
fn cjk_font_paths() -> Vec<&'static str> {
    match std::env::consts::OS {
        "linux" => vec![
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        ],
        "windows" => vec![
            "C:\\Windows\\Fonts\\msyh.ttc",      // Microsoft YaHei
            "C:\\Windows\\Fonts\\simsun.ttc",     // SimSun
        ],
        "macos" => vec![
            "/System/Library/Fonts/PingFang.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ],
        _ => vec![],
    }
}
```

### 6. Native Libraries (`crates/core/src/version/install.rs`)

Minecraft includes platform-specific native libraries (LWJGL, etc.) with classifier names like:
- `natives-linux`
- `natives-windows`
- `natives-osx` / `natives-macos`

The download task collection must select the correct classifier for the current OS. This is partially handled by the library rule filtering (item 3 above), but the native extraction path may also need attention.

## Dependencies Affected

| Crate | Linux-only? | Cross-platform alternative |
|-------|-------------|---------------------------|
| `tar` | ✓ (used for .tar.gz Java archives) | Keep + add `zip` branch for Windows |
| `flate2` | ✓ (gzip decompression) | Keep + add `zip` branch for Windows |
| `open` | ✓ Cross-platform already | No change needed |
| `dirs` | ✓ Cross-platform already | No change needed |
| `rfd` | ✓ Cross-platform already | No change needed |

## CI Changes for Cross-Platform

Add matrix builds to `.github/workflows/ci.yml`:
```yaml
strategy:
  matrix:
    os: [ubuntu-latest, windows-latest, macos-latest]
```

Remove the `libgtk-3-dev` etc. apt installs (only needed on Linux).

## Estimated Effort

- **Minimal viable** (builds + runs on all platforms): ~4-6 hours
- **Full parity** (Java detection + download works everywhere): ~8-12 hours
- **Polish** (native font rendering, platform shortcuts): ~2-4 hours additional
