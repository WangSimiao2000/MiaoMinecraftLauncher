# MMCL (MiaoMinecraftLauncher)

A feature-rich Minecraft launcher built in Rust with cross-platform support. Ships with both CLI and native GUI interfaces.

## Features

- **Instance Management** — Create, configure, and launch isolated Minecraft instances
- **Mod Loader Support** — Fabric, Quilt, NeoForge, and Forge with one-click install
- **Modrinth Integration** — Search, install mods with automatic dependency resolution
- **CurseForge Integration** — Search, install mods with dependency resolution (API key required)
- **Modpack Support** — Import/export `.mrpack` modpacks
- **Java Auto-Detection** — Finds compatible system Java or downloads from Adoptium
- **Download Mirrors** — BMCLAPI mirror with multi-source failover and automatic retry
- **Microsoft Login** — Full OAuth device code flow (+ offline mode)
- **Third-Party Skin Sites** — authlib-injector support (LittleSkin, Blessing Skin, etc.)
- **Crash Analysis** — Intelligent crash-report and log parsing with mod identification
- **Self-Update** — Check and apply updates from GitHub Releases
- **Resource Management** — Resource packs, shader packs, world saves

## Screenshots

*Coming soon*

## Installation

### Build from Source

Requires Rust nightly (pinned in `rust-toolchain.toml`).

```bash
git clone https://github.com/WangSimiao2000/MiaoMinecraftLauncher.git
cd MiaoMinecraftLauncher
cargo build --release
```

Binaries output to `target/release/`:
- `miao` — Command-line interface
- `miao-gui` — Native GUI (egui)

### System Dependencies (GUI)

On Debian/Ubuntu:
```bash
sudo apt install libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

## Usage

### GUI

```bash
cargo run -p miao-gui --release
```

### CLI

```bash
# List available versions
miao versions

# Create an instance
miao create 1.21.4 --name "My Server" --loader fabric

# Launch
miao launch "My Server"

# Search and install mods
miao mod-search sodium --mc 1.21.4 --loader fabric
miao mod-install "My Server" sodium
```

## Project Structure

```
crates/
├── core/       # Core library (auth, downloads, instance, modloader, modrinth)
├── cli/        # CLI interface (clap)
└── gui/        # Native GUI (egui/eframe)
docs/
├── architecture.md    # System architecture & modules
├── cross-platform.md  # Cross-platform support roadmap
├── ui-design.md       # UI design system & rules
└── testing.md         # Testing strategy & coverage
```

See [docs/architecture.md](docs/architecture.md) for detailed module documentation.

## Development

```bash
# Check everything compiles
cargo check --workspace

# Run all tests
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all

# Coverage (requires cargo-tarpaulin)
cargo tarpaulin -p miao-core --skip-clean
```

## Tech Stack

| Component | Choice |
|-----------|--------|
| Language | Rust (nightly) |
| Async | tokio |
| HTTP | reqwest |
| GUI | egui 0.30 / eframe |
| CLI | clap (derive) |
| Serialization | serde + toml/json |

## Contributing

1. Fork the repository
2. Run `sh .githooks/install.sh` to enable pre-commit/pre-push hooks
3. Create a feature branch
4. Ensure `cargo clippy -- -D warnings` and `cargo test` pass
5. Submit a pull request

## Author

**MickeyMiao**
- Blog: [blog.mickeymiao.cn](https://blog.mickeymiao.cn)
- Bilibili: [鄙人米奇喵](https://space.bilibili.com/)

## License

[GPL-3.0-or-later](LICENSE)
