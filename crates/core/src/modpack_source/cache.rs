use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

const CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMeta {
    pub fetched_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub etag: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub raw: Vec<u8>,
    pub meta: CacheMeta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale,
    Missing,
}

pub struct Cache {
    root: PathBuf,
    clock: Box<dyn Clock + Send + Sync>,
}

pub trait Clock {
    fn now(&self) -> chrono::DateTime<chrono::Utc>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}

impl Cache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            clock: Box::new(SystemClock),
        }
    }

    #[cfg(test)]
    pub fn with_clock(root: impl Into<PathBuf>, clock: Box<dyn Clock + Send + Sync>) -> Self {
        Self {
            root: root.into(),
            clock,
        }
    }

    pub fn manifest_paths(&self, source_id: &str) -> (PathBuf, PathBuf) {
        let dir = self.root.join(source_id);
        (dir.join("manifest.json"), dir.join("manifest.meta.json"))
    }

    pub fn pack_paths(&self, source_id: &str, pack_id: &str) -> (PathBuf, PathBuf) {
        let dir = self.root.join(source_id).join("packs").join(pack_id);
        (dir.join("pack.json"), dir.join("pack.meta.json"))
    }

    pub fn read_manifest(&self, source_id: &str) -> Result<Option<CacheEntry>, CacheError> {
        let (body, meta) = self.manifest_paths(source_id);
        self.read_pair(&body, &meta)
    }

    pub fn read_pack(
        &self,
        source_id: &str,
        pack_id: &str,
    ) -> Result<Option<CacheEntry>, CacheError> {
        let (body, meta) = self.pack_paths(source_id, pack_id);
        self.read_pair(&body, &meta)
    }

    pub fn write_manifest(
        &self,
        source_id: &str,
        raw: &[u8],
        meta: &CacheMeta,
    ) -> Result<(), CacheError> {
        let (body, meta_path) = self.manifest_paths(source_id);
        self.write_pair(&body, &meta_path, raw, meta)
    }

    pub fn write_pack(
        &self,
        source_id: &str,
        pack_id: &str,
        raw: &[u8],
        meta: &CacheMeta,
    ) -> Result<(), CacheError> {
        let (body, meta_path) = self.pack_paths(source_id, pack_id);
        self.write_pair(&body, &meta_path, raw, meta)
    }

    pub fn freshness_of(&self, meta: &CacheMeta) -> Freshness {
        let age = self.clock.now().signed_duration_since(meta.fetched_at);
        if age.num_seconds() < 0 {
            return Freshness::Fresh;
        }
        if age.to_std().unwrap_or(Duration::ZERO) < CACHE_TTL {
            Freshness::Fresh
        } else {
            Freshness::Stale
        }
    }

    pub fn touch_meta(&self, source_id: &str, etag: Option<String>) -> Result<(), CacheError> {
        let (_, meta_path) = self.manifest_paths(source_id);
        if !meta_path.exists() {
            return Ok(());
        }
        let mut meta: CacheMeta = serde_json::from_slice(&std::fs::read(&meta_path)?)?;
        meta.fetched_at = self.clock.now();
        if etag.is_some() {
            meta.etag = etag;
        }
        std::fs::write(&meta_path, serde_json::to_vec_pretty(&meta)?)?;
        Ok(())
    }

    fn read_pair(
        &self,
        body_path: &Path,
        meta_path: &Path,
    ) -> Result<Option<CacheEntry>, CacheError> {
        if !body_path.exists() || !meta_path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read(body_path)?;
        let meta: CacheMeta = serde_json::from_slice(&std::fs::read(meta_path)?)?;
        Ok(Some(CacheEntry { raw, meta }))
    }

    fn write_pair(
        &self,
        body_path: &Path,
        meta_path: &Path,
        raw: &[u8],
        meta: &CacheMeta,
    ) -> Result<(), CacheError> {
        if let Some(parent) = body_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(body_path, raw)?;
        std::fs::write(meta_path, serde_json::to_vec_pretty(meta)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct FakeClock {
        t: Arc<Mutex<chrono::DateTime<chrono::Utc>>>,
    }

    impl FakeClock {
        fn at(
            t: chrono::DateTime<chrono::Utc>,
        ) -> (Self, Arc<Mutex<chrono::DateTime<chrono::Utc>>>) {
            let arc = Arc::new(Mutex::new(t));
            (FakeClock { t: arc.clone() }, arc)
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            *self.t.lock().unwrap()
        }
    }

    fn make_cache() -> (
        tempfile::TempDir,
        Cache,
        Arc<Mutex<chrono::DateTime<chrono::Utc>>>,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let now = chrono::DateTime::parse_from_rfc3339("2026-05-29T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let (clock, t_handle) = FakeClock::at(now);
        let cache = Cache::with_clock(dir.path(), Box::new(clock));
        (dir, cache, t_handle)
    }

    fn meta_at(now: chrono::DateTime<chrono::Utc>) -> CacheMeta {
        CacheMeta {
            fetched_at: now,
            etag: Some("\"abc\"".to_string()),
            url: "https://example.com/manifest.json".to_string(),
        }
    }

    #[test]
    fn t_cache_01_fresh_cache_skips_network() {
        let (_dir, cache, t) = make_cache();
        let now = *t.lock().unwrap();
        cache.write_manifest("miao", b"{}", &meta_at(now)).unwrap();

        let entry = cache.read_manifest("miao").unwrap().expect("present");
        assert_eq!(cache.freshness_of(&entry.meta), Freshness::Fresh);
    }

    #[test]
    fn t_cache_02_stale_cache_refetches() {
        let (_dir, cache, t) = make_cache();
        let now = *t.lock().unwrap();
        cache.write_manifest("miao", b"{}", &meta_at(now)).unwrap();

        *t.lock().unwrap() = now + chrono::Duration::hours(7);

        let entry = cache.read_manifest("miao").unwrap().expect("present");
        assert_eq!(cache.freshness_of(&entry.meta), Freshness::Stale);
    }

    #[test]
    fn t_cache_03_offline_falls_back_to_any_age() {
        let (_dir, cache, t) = make_cache();
        let now = *t.lock().unwrap();
        cache
            .write_manifest("miao", b"{\"old\":true}", &meta_at(now))
            .unwrap();

        *t.lock().unwrap() = now + chrono::Duration::days(30);

        let entry = cache
            .read_manifest("miao")
            .unwrap()
            .expect("still readable");
        assert_eq!(entry.raw, b"{\"old\":true}");
        assert_eq!(cache.freshness_of(&entry.meta), Freshness::Stale);
    }

    #[test]
    fn t_cache_04_etag_304_keeps_cached_body() {
        let (_dir, cache, t) = make_cache();
        let initial = *t.lock().unwrap();
        cache
            .write_manifest("miao", b"{\"v\":1}", &meta_at(initial))
            .unwrap();

        let later = initial + chrono::Duration::hours(7);
        *t.lock().unwrap() = later;

        cache
            .touch_meta("miao", Some("\"new-etag\"".to_string()))
            .unwrap();

        let entry = cache.read_manifest("miao").unwrap().expect("present");
        assert_eq!(entry.raw, b"{\"v\":1}");
        assert_eq!(entry.meta.fetched_at, later);
        assert_eq!(entry.meta.etag.as_deref(), Some("\"new-etag\""));
        assert_eq!(cache.freshness_of(&entry.meta), Freshness::Fresh);
    }

    #[test]
    fn t_cache_05_missing_returns_none() {
        let (_dir, cache, _) = make_cache();
        assert!(cache.read_manifest("never-written").unwrap().is_none());
    }

    #[test]
    fn t_cache_06_pack_paths_isolated_per_source() {
        let (dir, cache, t) = make_cache();
        let now = *t.lock().unwrap();
        cache
            .write_pack("miao", "p1", b"{\"a\":1}", &meta_at(now))
            .unwrap();
        cache
            .write_pack("other", "p1", b"{\"b\":2}", &meta_at(now))
            .unwrap();

        let miao = cache.read_pack("miao", "p1").unwrap().expect("present");
        let other = cache.read_pack("other", "p1").unwrap().expect("present");
        assert_eq!(miao.raw, b"{\"a\":1}");
        assert_eq!(other.raw, b"{\"b\":2}");
        assert!(dir.path().join("miao/packs/p1/pack.json").exists());
        assert!(dir.path().join("other/packs/p1/pack.json").exists());
    }
}
