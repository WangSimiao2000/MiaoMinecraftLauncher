pub mod cache;
pub mod http;
pub mod installer;
pub mod live;
pub mod manifest;
pub mod registry;
pub mod resolver;

pub use cache::{Cache, CacheEntry, CacheError, CacheMeta, Freshness};
pub use http::{HttpError, ProgressSink, RateLimitedClient};
pub use installer::{
    InstallError, InstallExecutor, InstallOutcome, InstallPlan, cleanup_orphan_staging, install,
};
pub use live::{LiveInstallExecutor, LiveResolverDataSource};
pub use manifest::{
    ConfigOverlay, Criticality, Loader, Manifest, ManifestError, ManifestPackEntry, ModPolicy,
    ModSource, OverlayFile, Pack, PackMod, ReplacementRef, SupportLevel, parse_manifest,
    parse_pack,
};
pub use registry::{BUILT_IN_SOURCES, ModpackSource, RegistryError, load_sources};
pub use resolver::{
    CandidateVersion, ConflictReport, CycleNode, DependencyEdge, DependencyKind, ProjectMeta,
    ReleaseType, ResolutionReport, ResolveError, ResolvedMod, ResolvedOverlay, ResolvedStatus,
    ResolverDataSource, resolve,
};
