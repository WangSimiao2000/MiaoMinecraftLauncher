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
| `miao-core` | Core library: auth, version management, downloads, instance management, Java detection, mod loader support, Modrinth/CurseForge integration, crash analysis |
| `miao-cli` | CLI entry point with subcommands |
| `miao-gui` | Graphical UI using egui/eframe, with custom animation system (spring physics + MD3 easing) |

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
- Source-neutral installer architecture (`java/install.rs`):
  - `JavaInstallPlan` — describes per-file downloads, archive downloads,
    symlinks, and executable bits. Independent of upstream.
  - `JavaSource` enum dispatches to per-source planners; each planner returns
    a `JavaInstallPlan` and `execute()` runs them uniformly (download
    everything → extract archives → recreate links → set exec bits).
- `java/extract.rs` — tar.gz / zip extraction with `strip_components` and
  path-traversal protection. Reusable beyond Java if needed.
- Four built-in sources:
  - **Mojang** (`mojang.rs`) — official Minecraft JRE components from
    `piston-meta.mojang.com`, per-file with SHA-1, mirrored host-for-host by
    BMCLAPI through the global download mirror chain.
  - **BMCLAPI** (`bmclapi.rs`) — same manifest as Mojang but forced through
    the BMCLAPI mirror, decoupled from the user's global mirror setting.
  - **Adoptium Temurin** (`adoptium.rs`) — `api.adoptium.net` feature_releases
    endpoint, prefers JRE, falls back to JDK. Single-archive download
    (tar.gz/zip) with SHA-256. Supports Java 8/11/17/21/25.
  - **Microsoft Build of OpenJDK** (`microsoft.rs`) — `aka.ms/download-jdk`
    URL pattern + `.sha256sum.txt` sibling for checksum and version
    discovery. JDK only, Java 11/17/21/25.
- GUI exposes the Java source selector in Settings (data-driven from
  `JavaSource::all()`)

### Mod Loader Support (`modloader`)

- Forge / NeoForge / Fabric / Quilt
- Loader version listing and installation
- Per-instance loader configuration
- `ResolvedProfile` abstraction for modloader profiles

### Modrinth Integration (`modrinth`)

- Mod search with faceted filtering (version, loader) and pagination (offset)
- Dependency resolution with cycle detection
- Mod installation with automatic dependency handling
- Mod update detection via `/v2/version_files/update` (SHA-1 hash lookup)
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

### Update Notification

The GUI polls the GitHub Releases API on startup
([`gui/src/controller/versions.rs::handle_check_updates`](../crates/gui/src/controller/versions.rs))
and surfaces a banner in Settings > About when the latest tag does not match
the embedded `CARGO_PKG_VERSION`. Clicking *Download* opens the Releases
page in the default browser; the launcher does not replace its own binary.

### Mod Management (`modmanager`)

- Scan mods directory
- Enable/disable mods (`.jar` ↔ `.jar.disabled`)
- SHA-1 hash computation for update detection

### Resource Packs & Shaders (`resource`)

- Scan resourcepacks/shaderpacks directories
- Support both `.zip` and directory-based packs

### Server List (`server_list`)

- Persistent server entries (name, address, port)

### Player Skin (`skin`)

- Fetch player profile from Mojang session server
- Decode base64 skin/cape textures from profile
- Crop 8×8 face region from skin texture for UI avatars
- Cache skin/cape locally under `<data_dir>/cache/` for `file://` rendering by egui

### File Integrity (`integrity`)

- SHA-1 verification of game files before launch
- Auto-repair: re-download any missing or corrupted libraries/assets
- Triggered automatically by the launch flow; surfaces missing files to the
  user when they cannot be recovered

### Process Helpers (`process`)

- Cross-platform child process spawning
- Suppresses stray console windows on Windows when launching console-subsystem
  children (Java mostly) — the launcher itself is `windows_subsystem = "windows"`
  and has no console of its own

### Custom Theme (`custom_theme`)

- User-defined theme palette persistence (RGB triplets for every layer + accent)
- Loaded from `<data_dir>/themes/*.json`; the GUI hot-reloads on Settings refresh

### HTTP Client (`http`)

- `HttpClient` trait — abstraction over reqwest for testability
- Production implementation wraps `reqwest::Client` with retries and timeouts
- Tests inject mock implementations via `&impl HttpClient` parameter

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
| `java.rs` | Detection and JRE installation (Mojang / BMCLAPI / Adoptium / Microsoft) |
| `loaders.rs` | Fabric/Quilt/NeoForge/Forge operations |
| `mods.rs` | Modrinth + CurseForge search, install, update detection |

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
│   ├── mojang/
│   │   └── java-runtime-gamma/   (per-source / per-variant directory)
│   ├── bmclapi/
│   ├── adoptium/
│   │   └── 21.0.11+10.0.LTS/
│   └── microsoft/
│       └── 21.0.11/
├── locales/            (optional: user-contributed translations)
│   └── ja.json
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
| Animation | egui_animation + custom Spring | MD3 easing curves, physics-based spring indicators |
| CLI | clap | Derive-based, best Rust CLI framework |
| Serialization | serde + toml/json | Standard Rust serialization |
| Hashing | sha1/sha2 | Download integrity verification |
| Zip | zip | Mrpack modpack handling |


## Build-time Secrets

The launcher uses third-party services that need a key or client id. Both are
injected at compile time via `option_env!` and fall back to a hardcoded
default if the env var is not set or empty. This keeps `cargo build` working
out of the box for contributors while letting CI release builds inject
production credentials.

| Variable | Service | Default behavior | How to obtain |
|----------|---------|------------------|---------------|
| `CURSEFORGE_API_KEY` | CurseForge mod search/install | Falls back to a public key shipped with the source, override per-user via Settings > Data | Register at [console.curseforge.com](https://console.curseforge.com), create a project, copy the API key |
| `MS_CLIENT_ID` | Microsoft OAuth (Xbox Live → Minecraft) login | Falls back to the upstream MMCL Azure App registration | Register an Azure app at [portal.azure.com](https://portal.azure.com) → App registrations → "MMCL fork" → Authentication → add public client redirect → copy the Application (client) ID |

Build with keys:

```bash
CURSEFORGE_API_KEY="$2a$10$..." \
MS_CLIENT_ID="00000000-0000-0000-0000-000000000000" \
  cargo build --release
```

Forks publishing their own builds **should** override `MS_CLIENT_ID` so user
auth tokens are scoped to their own Azure app, not the upstream MMCL one.
Users can also override `curseforge_api_key` at runtime via Settings.

If no `CURSEFORGE_API_KEY` is provided at build time and the user doesn't set
one, CurseForge falls back to the bundled default. If neither is valid the
search will return a 401 and the UI will surface that to the user. Modrinth
works without any key.

## Logging

Both the CLI and the GUI initialize `tracing-subscriber` at startup. Logs go
to two destinations:

| Sink | CLI | GUI | Default level |
|------|-----|-----|---------------|
| stderr | ✅ | ✅ | `warn` (CLI), `info` (GUI) |
| `<data_dir>/logs/mmcl.log.<YYYY-MM-DD>` (daily-rolled) | — | ✅ | `info` |

Use the standard `RUST_LOG` env var to override
(e.g. `RUST_LOG=miao_gui=debug,miao_core=debug`). The GUI also installs a
panic hook that funnels panics through `tracing::error!` before re-raising,
so unexpected crashes leave a record in the log file even when stderr was
not visible (Windows GUI subsystem has no console).
