# Testing Strategy

## Overview

- **221 tests** across the workspace
- **Core coverage**: 67% (testable code ~82%)
- Framework: Rust built-in `#[test]` + `#[tokio::test]`
- Dev dependencies: `tempfile`, `tokio`

## Test Organization

### Unit Tests (inline `#[cfg(test)]`)

Located alongside source code in each module:

| Module | Tests | Focus |
|--------|-------|-------|
| `java/mod.rs` | 28 | Version parsing, detection, compatibility |
| `modrinth/api.rs` | 16 | Search, versions, deps resolution, install |
| `instance/mod.rs` | 12 | CRUD, builder pattern, save/load |
| `server_list.rs` | 11 | Add/remove/save/load |
| `service.rs` | 11 | Config accessors, instance ops, accounts |
| `auth/microsoft.rs` | 10 | Request serialization, UUID parsing |
| `resource/mod.rs` | 10 | Pack scanning, filtering |
| `modmanager/mod.rs` | 9 | Mod scanning, toggle |
| `modrinth/mrpack.rs` | 9 | Export zip, hashes, overrides, loader deps |
| `version/meta.rs` | 8 | Library rules, Java version, deserialization |
| `launch/mod.rs` | 7 | Classpath, game args, JVM args |
| `modloader/*.rs` | 31 | Maven paths, downloads, deserialization |
| Others | 19 | Config, download, version parsing |

### Integration Tests (`crates/core/tests/`)

| File | Tests | Description |
|------|-------|-------------|
| `mock_http.rs` | — | Shared `MockHttpClient` helper |
| `version_manifest.rs` | 4 | Manifest fetching with mirrors |
| `version_install.rs` | 3 | Version meta fetching |
| `modloader_fetch.rs` | 19 | All modloader version fetching + install |

## Mocking Pattern

### HttpClient Trait Injection

```rust
#[async_trait::async_trait]
pub trait HttpClient: Send + Sync {
    async fn get_json<T: DeserializeOwned + Send>(&self, url: &str) -> Result<T>;
    async fn get_bytes(&self, url: &str) -> Result<Vec<u8>>;
}
```

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

# With coverage (requires cargo-tarpaulin)
cargo tarpaulin -p miao-core --skip-clean --out Stdout
```

## Coverage Targets

| Scope | Target | Current |
|-------|--------|---------|
| Core (testable code) | 80% | ~82% |
| Core (raw) | — | 67% |
| Network-only code | Excluded | — |

### Untestable Without External Mocks

These modules require `wiremock` or similar for full coverage:

- `auth/microsoft.rs` — Multi-step OAuth chain
- `java/download.rs` — Adoptium binary download
- `service.rs` async methods — Full orchestration flows
- `modrinth/mrpack.rs` import — Network download phase

## Adding New Tests

1. Pure logic → inline `#[cfg(test)] mod tests` in the source file
2. Network-dependent → use `&impl HttpClient` parameter + mock
3. File I/O → `tempfile::tempdir()` for isolation
4. Multi-module integration → `crates/core/tests/` with shared `mock_http.rs`
