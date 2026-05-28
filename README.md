<div align="center">

<img src="assets/icon.png" width="128" alt="MMCL Logo">

# MMCL (MiaoMinecraftLauncher)

A feature-rich Minecraft launcher built in Rust — fast, native, and open source.

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-nightly-orange.svg)](rust-toolchain.toml)
[![Tests](https://img.shields.io/badge/Tests-265%20passing-brightgreen.svg)](#development)

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
| **Java** | Auto-detect compatible JVM or download from Adoptium |
| **Downloads** | BMCLAPI mirror, multi-source failover, concurrent downloads |
| **Auth** | Microsoft OAuth device-code flow, offline mode, authlib-injector (LittleSkin, etc.) |
| **Diagnostics** | Crash-report & log parsing with mod-level identification |
| **Updates** | Self-update from GitHub Releases |
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
| `miao` | Command-line interface |
| `miao-gui` | Native GUI (egui) |

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
| Self-Update | self_update (GitHub Releases) |
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

1. Copy `crates/gui/locales/en.json` to a new file (e.g. `ja.json`, `ko.json`, `de.json`)
2. Add a `"_name"` field with the language's native name (e.g. `"日本語"`)
3. Translate all values (keys stay in English)
4. Submit a PR — or place the file in `<data-dir>/locales/` for personal use

Users can also hot-reload new translations at runtime via Settings > Appearance > Refresh.

## Credits

| Dependency | License | Source |
|------------|---------|--------|
| [egui / eframe](https://github.com/emilk/egui) | MIT OR Apache-2.0 | emilk |
| [tokio](https://github.com/tokio-rs/tokio) | MIT | Tokio Contributors |
| [reqwest](https://github.com/seanmonstar/reqwest) | MIT OR Apache-2.0 | seanmonstar |
| [serde](https://github.com/serde-rs/serde) | MIT OR Apache-2.0 | David Tolnay |
| [clap](https://github.com/clap-rs/clap) | MIT OR Apache-2.0 | Kevin K. |
| [self_update](https://github.com/jaemk/self_update) | MIT | jaemk |
| [egui_animation](https://github.com/lucasmerlin/hello_egui) | MIT | lucasmerlin |
| [MiSans](https://hyperos.mi.com/font/en) | SIL OFL 1.1 | Xiaomi |
| [Bootstrap Icons](https://icons.getbootstrap.com/) | MIT | The Bootstrap Authors |

## Author

**MickeyMiao** — [GitHub](https://github.com/WangSimiao2000) · [Bilibili](https://space.bilibili.com/36913332)

## License

[GPL-3.0-or-later](LICENSE)
