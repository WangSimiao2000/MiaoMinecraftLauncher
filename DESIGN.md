# MiaoMinecraftLauncher - Design Document

## Overview

A feature-rich Minecraft launcher for Linux, built in Rust with CLI and GUI (egui) interfaces.

---

## UI Design Rules

### 1. Control Height — The Fundamental Rule

All interactive controls on the same horizontal row MUST share the same total height.

| Token | Height | Use Case |
|-------|--------|----------|
| `CONTROL_HEIGHT` | **28px** | All buttons, text inputs, combo boxes |
| `CONTROL_HEIGHT_SMALL` | 22px | Only for inline list actions (Del in table rows) |

### 2. Grid System — 4px Base

Every dimension (height, width, padding, margin, gap) must be a multiple of 4px.

```
Scale: 4, 8, 12, 16, 20, 24, 28, 32, 40, 48
```

### 3. Spacing Tokens

| Token | Value | Use |
|-------|-------|-----|
| `ITEM_SPACING` | 8×8 | Default gap between adjacent widgets |
| `BUTTON_PADDING` | 12×6 | Internal padding for buttons |
| `SECTION_GAP` | 16 | Between section cards |
| `SMALL_GAP` | 6 | Between related items |
| `PANEL_MARGIN` | 12 | Panel internal margin |
| `WINDOW_MARGIN` | 14 | Window internal margin |

### 4. Horizontal Row Alignment Rules

**Rule A**: All widgets in `ui.horizontal()` resolve to the same height via `interact_size.y = 28px`.

**Rule B**: TextEdit must match button height:
```rust
egui::TextEdit::singleline(&mut text)
    .min_size(ui.spacing().interact_size)
    .margin(ui.spacing().button_padding)
```

**Rule C**: No mixing `small_button` and `button` in same row. Use consistent sizing, differentiate by text color if needed.

**Rule D**: Label-to-control gap = 8px (1 item_spacing unit).

### 5. Typography

| Style | Size | Use |
|-------|------|-----|
| Title | 22px Bold | Instance name, page title |
| Heading | 20px Bold | Section heading |
| Subheading | 14px Bold | Card headers |
| Body | 13px | Primary content |
| Small | 11px | Hints, secondary info |
| Button | 14px | Button labels |

### 6. Color Layers (Dark Theme)

| Layer | RGB | Use |
|-------|-----|-----|
| BG_DARK | 22,24,30 | Top/bottom bars |
| BG_PANEL | 26,29,36 | Sidebar |
| BG_MAIN | 30,33,40 | Central panel |
| BG_ELEVATED | 38,42,52 | Cards |
| BG_WIDGET | 45,50,60 | Inputs, inactive buttons |
| BG_WIDGET_HOVER | 60,70,85 | Hover state |

### 7. Corner Radius

| Element | Radius |
|---------|--------|
| Widgets | 4px |
| Cards | 8px |
| List items | 6px |

### 8. Shadow

| Element | Offset | Blur | Alpha |
|---------|--------|------|-------|
| Window | 0,6 | 20 | 80 |
| Popup | 0,8 | 24 | 100 |

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  User Interface                   │
├──────────────┬──────────────┬───────────────────┤
│   miao-cli   │   miao-tui   │     miao-gui      │
│   (clap)     │  (ratatui)   │  (egui/eframe)    │
├──────────────┴──────────────┴───────────────────┤
│                   miao-core                      │
├─────────┬──────┬───────┬──────┬─────┬───────────┤
│  auth   │ ver  │ down  │ inst │java │ modloader │
│         │ mgmt │ load  │ ance │     │           │
└─────────┴──────┴───────┴──────┴─────┴───────────┘
```

### Crate Structure

| Crate | Role |
|-------|------|
| `miao-core` | Core library: auth, version management, downloads, instance management, Java detection, mod loader support |
| `miao-cli` | CLI entry point with subcommands (launch, install, list, tui, gui) |
| `miao-tui` | Terminal UI using ratatui + crossterm |
| `miao-gui` | Graphical UI using egui/eframe |

## Core Modules

### 1. Authentication (`auth`)

- **Microsoft OAuth**: Device code flow → Xbox Live → XSTS → Minecraft Services
- **Offline mode**: UUID v3 generation from username
- Token persistence and auto-refresh

### 2. Version Management (`version`)

- Fetch version manifest from Mojang/BMCLAPI
- Parse version metadata (libraries, assets, JVM args)
- Platform-specific library filtering (Linux only)

### 3. Download Manager (`download`)

- Concurrent downloads with configurable parallelism (default: 64)
- SHA1 integrity verification
- BMCLAPI mirror support with URL transformation
- Progress tracking (bytes + file count)
- Skip already-verified files

### 4. Instance Management (`instance`)

- Isolated game directories per instance
- Per-instance configuration (Java path, JVM args, memory, resolution)
- Mod loader binding per instance
- Timestamps for sorting (created, last played)

### 5. Java Management (`java`)

- Auto-detect system Java installations from standard paths
- Parse version strings (Java 8 `1.8.x` and modern `17.x` formats)
- Select minimum compatible version for game requirements

### 6. Mod Loader Support (`modloader`)

- Forge / NeoForge / Fabric / Quilt
- Loader version listing and installation
- Per-instance loader configuration

### 7. Mod Management (`modmanager`)

- Scan mods directory
- Enable/disable mods (`.jar` ↔ `.jar.disabled`)
- Mod metadata extraction (future)

### 8. Resource Packs & Shaders (`resource`)

- Scan resourcepacks/shaderpacks directories
- Support both `.zip` and directory-based packs

### 9. Server List (`server_list`)

- Persistent server entries (name, address, port)

### 10. Launch (`launch`)

- Build JVM command line from version metadata
- Classpath assembly with Linux platform filtering
- Game argument injection (auth, dirs, resolution)

## Data Layout

```
~/.config/miao-minecraft-launcher/
└── config.toml

~/.local/share/miao-minecraft-launcher/
├── versions/
│   └── 1.20.4/
│       ├── 1.20.4.json
│       └── 1.20.4.jar
├── libraries/
│   └── com/mojang/...
├── assets/
│   ├── indexes/
│   └── objects/
└── instances/
    └── my-instance/
        ├── instance.toml
        ├── mods/
        ├── resourcepacks/
        ├── shaderpacks/
        ├── saves/
        └── .minecraft/
```

## Download Mirror Strategy

| Source | URL Pattern |
|--------|------------|
| Official | `libraries.minecraft.net`, `resources.download.minecraft.net`, `piston-meta.mojang.com` |
| BMCLAPI | `bmclapi2.bangbang93.com/maven`, `bmclapi2.bangbang93.com/assets`, `bmclapi2.bangbang93.com` |
| Custom | User-defined base URL with same path structure |

## Testing Strategy

Each module includes:
- **Unit tests**: Pure logic (URL transforms, version parsing, Java detection)
- **Integration tests**: File I/O, instance creation, mod scanning
- **Mock-based tests**: HTTP responses for download/auth flows

Test execution: `cargo test` runs all tests; individual crate tests via `cargo test -p miao-core`.

## Technology Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust | Performance, safety, single binary distribution |
| Async | tokio | De facto standard for async Rust |
| HTTP | reqwest | Feature-rich, tokio-native |
| TUI | ratatui + crossterm | Most mature Rust TUI ecosystem |
| GUI | egui/eframe | Pure Rust, immediate mode, lightweight |
| CLI | clap | Derive-based, best Rust CLI framework |
| Serialization | serde + toml/json | Standard Rust serialization |
| Hashing | sha1/sha2 | For download integrity verification |

## Build & Run

```bash
# Check compilation
cargo check

# Run tests
cargo test

# Build all binaries
cargo build --release

# Run CLI
cargo run -p miao-cli -- launch my-instance

# Run TUI
cargo run -p miao-tui

# Run GUI
cargo run -p miao-gui
```

## License

GPL-3.0-or-later
