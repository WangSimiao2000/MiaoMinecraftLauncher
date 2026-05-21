# Architecture

## Crate Structure

```
┌─────────────────────────────────────────────────┐
│                  User Interface                   │
├──────────────────┬──────────────────────────────┤
│     miao-cli     │          miao-gui             │
│     (clap)       │       (egui/eframe)           │
├──────────────────┴──────────────────────────────┤
│                   miao-core                      │
├─────────┬──────┬───────┬──────┬─────┬───────────┤
│  auth   │ ver  │ down  │ inst │java │ modloader │
│         │ mgmt │ load  │ ance │     │           │
└─────────┴──────┴───────┴──────┴─────┴───────────┘
```

| Crate | Role |
|-------|------|
| `miao-core` | Core library: auth, version management, downloads, instance management, Java detection, mod loader support, Modrinth integration |
| `miao-cli` | CLI entry point with subcommands |
| `miao-gui` | Graphical UI using egui/eframe |

## Core Modules

### Authentication (`auth`)

- **Microsoft OAuth**: Device code flow → Xbox Live → XSTS → Minecraft Services
- **Offline mode**: UUID v3 generation from username
- Token persistence and auto-refresh

### Version Management (`version`)

- Fetch version manifest from Mojang/BMCLAPI
- Parse version metadata (libraries, assets, JVM args)
- Platform-specific library filtering (Linux only)

### Download Manager (`download`)

- Concurrent downloads with configurable parallelism (default: 64)
- SHA1 integrity verification
- BMCLAPI mirror support with URL transformation
- Progress tracking (bytes + file count)
- Skip already-verified files

### Instance Management (`instance`)

- Isolated game directories per instance
- Per-instance configuration (Java path, JVM args, memory, resolution)
- Mod loader binding per instance
- Timestamps for sorting (created, last played)

### Java Management (`java`)

- Auto-detect system Java installations from standard paths
- Parse version strings (Java 8 `1.8.x` and modern `17.x` formats)
- Select minimum compatible version for game requirements
- Auto-download from Adoptium if needed

### Mod Loader Support (`modloader`)

- Forge / NeoForge / Fabric / Quilt
- Loader version listing and installation
- Per-instance loader configuration
- `ResolvedProfile` abstraction for modloader profiles

### Modrinth Integration (`modrinth`)

- Mod search with faceted filtering (version, loader)
- Dependency resolution with cycle detection
- Mod installation with automatic dependency handling
- Mrpack modpack import/export

### Mod Management (`modmanager`)

- Scan mods directory
- Enable/disable mods (`.jar` ↔ `.jar.disabled`)

### Resource Packs & Shaders (`resource`)

- Scan resourcepacks/shaderpacks directories
- Support both `.zip` and directory-based packs

### Server List (`server_list`)

- Persistent server entries (name, address, port)

### Launch (`launch`)

- Build JVM command line from version metadata
- Classpath assembly with Linux platform filtering
- Game argument injection (auth, dirs, resolution)

### Service Layer (`service`)

- `LauncherService` provides high-level operations shared by CLI and GUI
- Orchestrates multi-step workflows (create instance, install loader, etc.)

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
├── java/
│   └── jdk-21/
└── instances/
    └── my-instance/
        ├── instance.toml
        ├── mods/
        ├── resourcepacks/
        ├── shaderpacks/
        ├── saves/
        └── config/
```

## Download Mirror Strategy

| Source | URL Pattern |
|--------|------------|
| Official | `libraries.minecraft.net`, `resources.download.minecraft.net`, `piston-meta.mojang.com` |
| BMCLAPI | `bmclapi2.bangbang93.com/maven`, `bmclapi2.bangbang93.com/assets` |
| Custom | User-defined base URL with same path structure |

## Technology Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust | Performance, safety, single binary distribution |
| Async | tokio | De facto standard for async Rust |
| HTTP | reqwest | Feature-rich, tokio-native |
| GUI | egui/eframe | Pure Rust, immediate mode, lightweight |
| CLI | clap | Derive-based, best Rust CLI framework |
| Serialization | serde + toml/json | Standard Rust serialization |
| Hashing | sha1/sha2 | Download integrity verification |
| Zip | zip | Mrpack modpack handling |
