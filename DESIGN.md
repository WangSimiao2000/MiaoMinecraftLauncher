# MiaoMinecraftLauncher - Design Document

## Overview

A feature-rich Minecraft launcher for Linux, built in Rust with dual interface support (TUI via ratatui, GUI via egui).

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
