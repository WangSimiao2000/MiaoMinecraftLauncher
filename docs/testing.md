# Testing Strategy

## Overview

- **319 tests** across the workspace (270 core unit + 4 GUI unit + 12 CLI integration + 27 core integration + 6 GUI controller integration; 5 of the GUI controller tests are `#[ignore]`'d as they hit live network endpoints, so default `cargo test` runs 314)
- **Core coverage**: ~67% raw / ~82% of testable code
- Framework: Rust built-in `#[test]` + `#[tokio::test]`
- Dev dependencies: `tempfile`, `tokio`, `assert_cmd`, `predicates`

## Test Organization

### Unit Tests (inline `#[cfg(test)]`)

Located alongside source code in each module. Major test concentrations:

| Module | Tests | Focus |
|--------|-------|-------|
| `java/mod.rs` | 30 | Version parsing, detection, compatibility, caching |
| `modrinth/api.rs` | 16 | Search, versions, deps resolution, install |
| `instance/mod.rs` | 12 | CRUD, builder pattern, save/load |
| `crash/parser.rs` | 11 | Crash report + log parsing |
| `service/mod.rs` | 11 | Config accessors, instance ops, accounts |
| `server_list.rs` | 11 | Add/remove/save/load |
| `auth/microsoft.rs` | 10 | Request serialization, UUID parsing |
| `resource/mod.rs` | 10 | Pack scanning, filtering |
| `modmanager/mod.rs` | 9 | Mod scanning, toggle |
| `modrinth/mrpack.rs` | 9 | Export zip, hashes, overrides, loader deps |
| `version/meta.rs` | 8 | Library rules, Java version, deserialization |
| `launch/mod.rs` | 8 | Classpath, game args, JVM args |
| `modloader/{fabric,forge,neoforge}` | 7 each | Maven paths, downloads, deserialization |
| `modloader/quilt.rs` | 5 | Same as above |
| `modloader/mod.rs` | 5 | Loader dispatch / shared logic |
| `config.rs` | 7 | Config defaults, save/load roundtrip |
| `download/mirror.rs` | 7 | Mirror chain URL transformation |
| `crash/analyzer.rs` | 5 | Crash type classification, mod identification |
| `version/install.rs` | 5 | Install plan construction |
| `java/mojang.rs` | 6 | Mojang JRE manifest parsing |
| `curseforge/api.rs` | 6 | CurseForge search/install requests |
| `auth/authlib_injector.rs` | 6 | Yggdrasil flow |
| `download/manager.rs` | 4 | Concurrent download orchestration |
| `update/`, `auth/offline`, `version/assets`, `java/install`, … | 3 each | |
| Other modules | remainder | Config, download, integrity, http, skin, process |

### Integration Tests (`crates/core/tests/`)

| File | Tests | Description |
|------|-------|-------------|
| `mock_http.rs` | — | Shared `MockHttpClient` helper |
| `version_manifest.rs` | 4 | Manifest fetching with mirrors |
| `version_meta_fetch.rs` | 3 | Version meta fetching |
| `modloader_fetch.rs` | 20 | All modloader version fetching + install |

### GUI Integration Tests (`crates/gui/tests/`)

| File | Tests | Description |
|------|-------|-------------|
| `controller_tests.rs` | 6 | Controller event loop, command dispatch, graceful shutdown, parallel task API |

Five of the six controller tests hit live Mojang / Fabric / Forge / Modrinth /
GitHub endpoints and are marked `#[ignore]` so the default `cargo test`
doesn't fail on transient network issues. The CI `integration` job runs
them explicitly with `cargo test -p miao-gui --test controller_tests --
--ignored` and is allowed to fail on network blips. The remaining test
(`cancel_task_when_no_active_task_is_noop`) is purely local and runs by
default.

### CLI Integration Tests (`crates/cli/tests/`)

| File | Tests | Description |
|------|-------|-------------|
| `cli.rs` | 12 | Subcommand discovery, version flag, argument parsing, account creation, isolated config |

CLI tests use `assert_cmd` to spawn the compiled `miao` binary and override
`HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `APPDATA`, and `LOCALAPPDATA`
per-test so the test process **never** touches the developer's real config
directory. Each test gets a fresh `tempfile::tempdir()` as its sandbox. No
network calls are made in the CLI integration suite.

## Mocking Pattern

### HttpClient Trait Injection

```rust
pub trait HttpClient: Send + Sync {
    async fn get_json<T: DeserializeOwned + Send>(&self, url: &str) -> Result<T>;
    async fn get_bytes(&self, url: &str) -> Result<Vec<u8>>;
}
```

Native `async fn` in trait is used (Rust edition 2024, nightly toolchain) —
no `async-trait` macro is required.

Production code accepts `&impl HttpClient`. Tests provide a mock with HashMap-based URL pattern matching:

```rust
struct MockHttp {
    json_responses: Mutex<HashMap<String, String>>,
    byte_responses: Mutex<HashMap<String, Vec<u8>>>,
}

mock.on_json("/project/sodium/version", r#"[...]"#);
mock.on_bytes("cdn.example.com/test.jar", b"content".to_vec());
```

### Filesystem Tests

Use `tempfile::tempdir()` for isolated filesystem operations:

```rust
let tmp = tempfile::tempdir().unwrap();
let inst = Instance::new("test", "1.20.4");
inst.save_to(tmp.path()).unwrap();
```

## Running Tests

```bash
# All tests
cargo test --workspace

# Single crate
cargo test -p miao-core

# Specific module
cargo test -p miao-core -- modrinth::api::tests

# GUI controller tests
cargo test -p miao-gui --test controller_tests

# With coverage (requires cargo-tarpaulin)
cargo tarpaulin -p miao-core --skip-clean --out Stdout
```

## Coverage Targets

| Scope | Target | Current |
|-------|--------|---------|
| Core (testable code) | 80% | ~82% |
| Core (raw) | — | ~67% |
| Network-only code | Excluded | — |

### Untestable Without External Mocks

These modules require `wiremock` or similar for full coverage:

- `auth/microsoft.rs` — Multi-step OAuth chain
- `java/install.rs` — Mojang JRE binary download
- `service/*` async methods — Full orchestration flows
- `modrinth/mrpack.rs` import — Network download phase

## Adding New Tests

1. Pure logic → inline `#[cfg(test)] mod tests` in the source file
2. Network-dependent → use `&impl HttpClient` parameter + mock
3. File I/O → `tempfile::tempdir()` for isolation
4. Multi-module integration → `crates/core/tests/` with shared `mock_http.rs`
5. GUI controller / channel-based logic → `crates/gui/tests/controller_tests.rs`
