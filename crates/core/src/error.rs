use std::path::PathBuf;

use thiserror::Error;

/// Top-level error type for miao-core operations.
#[derive(Debug, Error)]
pub enum MiaoError {
    #[error(transparent)]
    Instance(#[from] InstanceError),

    #[error(transparent)]
    Download(#[from] DownloadError),

    #[error(transparent)]
    ModLoader(#[from] ModLoaderError),

    #[error(transparent)]
    Java(#[from] JavaError),

    #[error(transparent)]
    Auth(#[from] AuthError),

    #[error(transparent)]
    Launch(#[from] LaunchError),

    #[error(transparent)]
    Version(#[from] VersionError),

    #[error(transparent)]
    Modrinth(#[from] ModrinthError),

    #[error(transparent)]
    CurseForge(#[from] CurseForgeError),

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("UUID parse error: {0}")]
    Uuid(#[from] uuid::Error),

    #[error("Task join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("{0}")]
    Other(String),
}

/// Instance management errors.
#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("instance '{name}' not found")]
    NotFound { name: String },

    #[error("instance '{name}' already exists")]
    AlreadyExists { name: String },

    #[error("failed to read instance config at {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse instance config: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("failed to write instance config: {0}")]
    ConfigWrite(std::io::Error),

    #[error("instance '{name}' has no mod loader installed")]
    NoModLoader { name: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Download errors.
#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("SHA1 mismatch for {path}: expected {expected}, got {actual}")]
    Sha1Mismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[error("download failed for {url}: {source}")]
    Network { url: String, source: reqwest::Error },

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error writing {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("no files to download")]
    EmptyTaskList,
}

/// Mod loader errors.
#[derive(Debug, Error)]
pub enum ModLoaderError {
    #[error("unknown loader type: {0}")]
    UnknownType(String),

    #[error("no {loader} versions available for MC {mc_version}")]
    NoVersionsAvailable { loader: String, mc_version: String },

    #[error("failed to fetch {loader} profile: {source}")]
    ProfileFetch {
        loader: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to install {loader} {version}: {reason}")]
    InstallFailed {
        loader: String,
        version: String,
        reason: String,
    },

    #[error("download error: {0}")]
    Download(#[from] DownloadError),

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
}

/// Java detection/download errors.
#[derive(Debug, Error)]
pub enum JavaError {
    #[error("no compatible Java {required_major} found")]
    NotFound { required_major: u32 },

    #[error("failed to probe Java at {path}: {reason}")]
    ProbeFailed { path: PathBuf, reason: String },

    #[error("failed to download Java {major}: {source}")]
    DownloadFailed {
        major: u32,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to extract Java archive: {0}")]
    ExtractFailed(String),

    #[error("no Adoptium release found for Java {major}")]
    NoRelease { major: u32 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Authentication errors.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("no account configured")]
    NoAccount,

    #[error("Microsoft auth failed: {0}")]
    MicrosoftAuth(String),

    #[error("token expired for user '{username}'")]
    TokenExpired { username: String },

    #[error("token refresh failed: {0}")]
    RefreshFailed(String),
}

/// Version-related errors.
#[derive(Debug, Error)]
pub enum VersionError {
    #[error("version '{version}' not found in manifest")]
    NotFound { version: String },

    #[error("version metadata file not found for MC {version}")]
    MetaNotFound { version: String },

    #[error("failed to parse version metadata: {0}")]
    ParseFailed(#[from] serde_json::Error),
}

/// Game launch errors.
#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("version metadata not found for MC {version}")]
    VersionMetaNotFound { version: String },

    #[error("no compatible Java found (requires Java {required})")]
    JavaNotFound { required: u32 },

    #[error("game process failed with exit code: {code:?}")]
    ProcessFailed { code: Option<i32> },

    #[error("failed to build launch command: {0}")]
    CommandBuild(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Modrinth API errors.
#[derive(Debug, Error)]
pub enum ModrinthError {
    #[error("mod '{project}' not found on Modrinth")]
    ProjectNotFound { project: String },

    #[error("no compatible version of '{project}' for MC {mc_version} ({loader})")]
    NoCompatibleVersion {
        project: String,
        mc_version: String,
        loader: String,
    },

    #[error("Modrinth API error: {0}")]
    Api(#[from] reqwest::Error),

    #[error("failed to download mod file: {0}")]
    DownloadFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// CurseForge API errors.
#[derive(Debug, Error)]
pub enum CurseForgeError {
    #[error("mod ID {mod_id} not found on CurseForge")]
    ModNotFound { mod_id: u32 },

    #[error("no compatible file for mod {mod_id} (MC {mc_version}, {loader})")]
    NoCompatibleFile {
        mod_id: u32,
        mc_version: String,
        loader: String,
    },

    #[error("CurseForge API error: {0}")]
    Api(#[from] reqwest::Error),

    #[error("failed to download mod file: {0}")]
    DownloadFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CurseForge API key not configured")]
    NoApiKey,

    #[error("CurseForge API rate-limited; retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
}

/// Configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to load config from {path}: {reason}")]
    Load { path: PathBuf, reason: String },

    #[error("failed to save config to {path}: {reason}")]
    Save { path: PathBuf, reason: String },

    #[error("invalid data directory: {0}")]
    InvalidDataDir(PathBuf),
}

/// Convenience result type for miao-core.
pub type Result<T> = std::result::Result<T, MiaoError>;
