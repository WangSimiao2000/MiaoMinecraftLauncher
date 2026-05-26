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
| `miao-core` | Core library: auth, version management, downloads, instance management, Java detection, mod loader support, Modrinth/CurseForge integration, crash analysis, self-update |
| `miao-cli` | CLI entry point with subcommands |
| `miao-gui` | Graphical UI using egui/eframe |

## Core Modules

### Authentication (`auth`)

- **Microsoft OAuth**: Device code flow → Xbox Live → XSTS → Minecraft Services
- **Offline mode**: UUID v3 generation from username
- **authlib-injector**: Third-party skin site login via Yggdrasil protocol (LittleSkin, etc.)
- Token persistence and auto-refresh

### Version Management (`version`)

- Fetch version manifest from Mojang/BMCLAPI
- Parse version metadata (libraries, assets, JVM args)
- Platform-specific library filtering (Linux/macOS/Windows via `current_os_name()`)

### Download Manager (`download`)

- Concurrent downloads with configurable parallelism (default: 64)
- SHA1 integrity verification
- Multi-source failover: mirror chain with automatic fallback (BMCLAPI ↔ Official ↔ Custom)
- Exponential backoff retry (max 3 attempts per mirror)
- HTTP timeout configuration (connect: 10s, request: 30s)
- Progress tracking (bytes + file count)
- Skip already-verified files

### Instance Management (`instance`)

- Isolated game directories per instance
- Per-instance configuration (Java path, JVM args, memory, resolution)
- Mod loader binding per instance
- Timestamps for sorting (created, last played)

### Java Management (`java`)

- Auto-detect system Java from platform-specific paths + `JAVA_HOME`
- Parse version strings (Java 8 `1.8.x` and modern `17.x` formats)
- Select minimum compatible version for game requirements
- Auto-download from Adoptium (tar.gz on Unix, zip on Windows)

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

### CurseForge Integration (`curseforge`)

- `CurseForgeClient` with API key authentication (`x-api-key` header)
- Mod search with game version and mod loader type filters
- File listing with loader compatibility filtering
- Download with null-URL detection (some mods restrict third-party distribution)
- Dependency resolution (BFS, required-only, with cycle detection)
- Full install pipeline: search → resolve deps → download all

### Crash Analysis (`crash`)

- **Parser** (`crash/parser.rs`): Parse Minecraft crash-report files and latest.log
- **Analyzer** (`crash/analyzer.rs`): Aggregate findings into actionable diagnosis
- Detect crash types: OOM, ModIncompatibility, MissingDependency, OpenGL, Java incompatibility
- Identify suspected mods with confidence levels (High/Medium/Low)
- Generate fix suggestions based on crash category

### Self-Update (`update`)

- Check GitHub Releases for new versions
- Platform-specific asset selection (OS × arch)
- Download and apply with atomic backup → replace → chmod
- Automatic rollback on failure

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
- Classpath assembly with cross-platform library filtering
- Game argument injection (auth, dirs, resolution)

### Service Layer (`service/`)

`LauncherService` provides high-level operations shared by CLI and GUI, split into domain modules:

| Module | Responsibility |
|--------|---------------|
| `auth.rs` | Token refresh, account management |
| `instance.rs` | Create, launch, list, delete, export, import |
| `java.rs` | Detection and Adoptium download |
| `loaders.rs` | Fabric/Quilt/NeoForge/Forge operations |
| `mods.rs` | Modrinth search and install |

## Data Layout

Config and data directories are determined by `dirs::config_dir()` / `dirs::data_dir()`:

| Platform | Config | Data |
|----------|--------|------|
| Linux | `~/.config/miao-minecraft-launcher/` | `~/.local/share/miao-minecraft-launcher/` |
| macOS | `~/Library/Application Support/miao-minecraft-launcher/` | same |
| Windows | `%APPDATA%\miao-minecraft-launcher\` | same |
| Portable | `<exe_dir>/mmcl-data/` (auto-detected) | same |

```
<data_dir>/
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
