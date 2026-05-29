<div align="center">

<img src="assets/icon.png" width="128" alt="MMCL Logo">

# MMCL (MiaoMinecraftLauncher)

A feature-rich Minecraft launcher built in Rust — fast, native, and open source.

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-nightly-orange.svg)](rust-toolchain.toml)
[![Tests](https://img.shields.io/badge/Tests-314%20passing-brightgreen.svg)](#development)

</div>

---

## Screenshots

<p align="center">
  <img src="screenshots/instance.png" width="720" alt="Instance view with mod management">
  <br>
  <em>Instance detail — Fabric 1.21.11 with installed mods</em>
</p>

<p align="center">
  <img src="screenshots/about.png" width="720" alt="Settings — About page">
  <br>
  <em>Settings — About</em>
</p>

## Features

| Category | Highlights |
|----------|-----------|
| **Instances** | Create, configure, and launch isolated Minecraft instances |
| **Mod Loaders** | One-click install for Fabric, Quilt, NeoForge, and Forge |
| **Modrinth** | Search & install mods with dependency resolution, pagination, update detection |
| **CurseForge** | Search & install mods with built-in API key, pagination |
| **Modpacks** | Import/export `.mrpack` modpacks |
| **Drag & Drop** | Drop `.jar` / `.zip` files into the window to install mods or resource packs |
| **Mod Updates** | One-click check for newer versions of installed mods via Modrinth |
| **Java** | Auto-detect compatible JVM or install from Mojang / BMCLAPI / Adoptium Temurin / Microsoft Build of OpenJDK |
| **Downloads** | BMCLAPI mirror, multi-source failover, concurrent downloads |
| **Auth** | Microsoft OAuth device-code flow, offline mode, authlib-injector (LittleSkin, etc.) |
| **Diagnostics** | Crash-report & log parsing with mod-level identification |
| **Updates** | New-version notifications via GitHub Releases (one-click jump to download page) |
| **Resources** | Resource packs, shader packs, world saves |
| **i18n** | External JSON locale files — community-contributed translations without code changes |

## Quick Start

### Build from Source

Requires Rust nightly (pinned in `rust-toolchain.toml`).

```bash
git clone https://github.com/WangSimiao2000/MiaoMinecraftLauncher.git
cd MiaoMinecraftLauncher
cargo build --release
```

Binaries output to `target/release/`:

| Binary | Description |
|--------|-------------|
| `miao-gui` | Native GUI (egui) — primary frontend, all platforms |
| `miao` | CLI frontend — Linux / macOS only in official releases |

The CLI is for headless servers, SSH workflows and scripted batch operations
(create/launch/list/mod-install). It is a strict **subset** of the GUI:
Microsoft OAuth login, CurseForge, crash analysis, mod-update detection and
several settings are GUI-only. Use the GUI as your daily driver and reach
for the CLI when you need to drive the launcher from a script or over a
shell. We do not ship `miao.exe` on Windows because adding a console binary
to `PATH` on Windows is awkward enough that the script-automation use case
is better served by the Linux binary inside WSL.

### System Dependencies (Linux GUI)

```bash
# Debian / Ubuntu
sudo apt install libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

### Run

```bash
# GUI
cargo run -p miao-gui --release

# CLI examples
miao versions
miao new 1.21.1 --name "My Instance" --loader fabric
miao launch "My Instance"
miao mod-search sodium --mc-version 1.21.1 --loader fabric
miao mod-install "My Instance" sodium
```

## Architecture

```
crates/
├── core/    Core library — auth, downloads, instance, modloaders, Modrinth, CurseForge
├── cli/     CLI frontend (clap)
└── gui/     Native GUI frontend (egui / eframe)
```

See [`docs/architecture.md`](docs/architecture.md) for detailed module documentation.

## Tech Stack

| Component | Choice |
|-----------|--------|
| Language | Rust (nightly) |
| Async | tokio |
| HTTP | reqwest |
| GUI | egui 0.34 / eframe |
| Animation | egui_animation + custom Spring physics |
| CLI | clap (derive) |
| Serialization | serde + toml / json |
| Update Check | GitHub Releases API (manual download) |
| Font | MiSans Medium (bundled) |
| Icons | Bootstrap Icons (bundled TTF) |

## Development

```bash
cargo check --workspace          # Compile check
cargo test --workspace           # Run all tests
cargo clippy --workspace -- -D warnings  # Lint
cargo fmt --all                  # Format
cargo tarpaulin -p miao-core --skip-clean  # Coverage
```

## Contributing

1. Fork the repository
2. Run `sh .githooks/install.sh` to enable pre-commit/pre-push hooks
3. Create a feature branch
4. Ensure `cargo clippy -- -D warnings` and `cargo test` pass
5. Submit a pull request

### Adding a Translation

No Rust knowledge required — just create a JSON file:

1. Copy `crates/gui/locales/en.json` to a new file (e.g. `ko.json`, `de.json`)
2. Add a `"_name"` field with the language's native name (e.g. `"한국어"`)
3. Translate all values (keys stay in English)
4. Submit a PR — or place the file in `<data-dir>/locales/` for personal use

Bundled languages: `en` (English), `zh` (中文), `ja` (日本語).

Users can also hot-reload new translations at runtime via Settings > Appearance > Refresh.

## Credits

| Dependency | License | Source |
|------------|---------|--------|
| [egui / eframe](https://github.com/emilk/egui) | MIT OR Apache-2.0 | emilk |
| [tokio](https://github.com/tokio-rs/tokio) | MIT | Tokio Contributors |
| [reqwest](https://github.com/seanmonstar/reqwest) | MIT OR Apache-2.0 | seanmonstar |
| [serde](https://github.com/serde-rs/serde) | MIT OR Apache-2.0 | David Tolnay |
| [clap](https://github.com/clap-rs/clap) | MIT OR Apache-2.0 | Kevin K. |
| [egui_animation](https://github.com/lucasmerlin/hello_egui) | MIT | lucasmerlin |
| [MiSans](https://hyperos.mi.com/font/en) | SIL OFL 1.1 | Xiaomi |
| [Bootstrap Icons](https://icons.getbootstrap.com/) | MIT | The Bootstrap Authors |

## Author

**MickeyMiao** — [GitHub](https://github.com/WangSimiao2000) · [Bilibili](https://space.bilibili.com/36913332)

## License

[GPL-3.0-or-later](LICENSE)
