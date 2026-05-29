pub mod cache;
pub mod http;
pub mod manifest;
pub mod resolver;

pub use cache::{Cache, CacheEntry, CacheError, CacheMeta, Freshness};
pub use http::{HttpError, ProgressSink, RateLimitedClient};
pub use manifest::{
    ConfigOverlay, Criticality, Loader, Manifest, ManifestError, ManifestPackEntry, ModPolicy,
    ModSource, OverlayFile, Pack, PackMod, ReplacementRef, SupportLevel, parse_manifest,
    parse_pack,
};
pub use resolver::{
    CandidateVersion, ConflictReport, CycleNode, DependencyEdge, DependencyKind, ProjectMeta,
    ReleaseType, ResolutionReport, ResolveError, ResolvedMod, ResolvedStatus, ResolverDataSource,
    resolve,
};
