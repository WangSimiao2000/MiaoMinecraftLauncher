use std::collections::{HashMap, HashSet, VecDeque};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::modpack_source::manifest::{
    Criticality, Loader, ModPolicy, ModSource, Pack, PackMod, ReplacementRef,
};

pub const ABANDONED_THRESHOLD_DAYS: u32 = 180;
pub const BACKTRACK_MAX_DEPTH: usize = 5;
pub const BACKTRACK_MAX_ATTEMPTS_PER_MOD: usize = 3;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("mc_version '{got}' is not declared in pack.mc_versions {available:?}")]
    McVersionNotInPack { got: String, available: Vec<String> },

    #[error("hard incompatibility: {a} declares it cannot run with {b}")]
    HardIncompatible { a: String, b: String },

    #[error("cycle in required dependencies: {path}")]
    DependencyCycle { path: String },

    #[error("data source error: {0}")]
    DataSource(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionReport {
    pub pack_id: String,
    pub pack_version: String,
    pub pack_content_hash: String,
    pub mc_version: String,
    pub loader: Loader,
    pub loader_version: Option<String>,
    pub resolved_at: chrono::DateTime<chrono::Utc>,
    pub mods: Vec<ResolvedMod>,
    pub conflicts: Vec<ConflictReport>,
    pub cycle: Option<Vec<CycleNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedMod {
    pub source: ModSource,
    pub project_id: String,
    pub display_name: String,
    pub criticality: Criticality,
    pub status: ResolvedStatus,
    pub version_id: Option<String>,
    pub file_url: Option<String>,
    pub file_sha1: Option<String>,
    pub file_sha512: Option<String>,
    pub file_size: Option<u64>,
    pub file_name: Option<String>,
    pub auto_added_for: Option<String>,
    pub replacement_hint: Option<ReplacementRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ResolvedStatus {
    Compatible,
    Pending {
        last_release: chrono::DateTime<chrono::Utc>,
    },
    Abandoned {
        last_release: Option<chrono::DateTime<chrono::Utc>>,
    },
    Conflict {
        reason: String,
    },
    Deprecated {
        since: String,
    },
    Ambiguous {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub a_project_id: String,
    pub b_project_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleNode {
    pub project_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct CandidateVersion {
    pub version_id: String,
    pub project_id: String,
    pub display_name: String,
    pub mc_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub release_type: ReleaseType,
    pub date_published: chrono::DateTime<chrono::Utc>,
    pub file_url: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_sha1: Option<String>,
    pub file_sha512: Option<String>,
    pub dependencies: Vec<DependencyEdge>,
    pub loader_dep_version_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseType {
    Release,
    Beta,
    Alpha,
}

#[derive(Debug, Clone)]
pub struct DependencyEdge {
    pub project_id: String,
    pub kind: DependencyKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyKind {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

#[derive(Debug, Clone)]
pub struct ProjectMeta {
    pub project_id: String,
    pub display_name: String,
    pub last_release: Option<chrono::DateTime<chrono::Utc>>,
}

#[async_trait]
pub trait ResolverDataSource: Send + Sync {
    async fn fetch_project_meta(
        &self,
        source: ModSource,
        project_id: &str,
    ) -> Result<ProjectMeta, String>;

    async fn fetch_compatible_versions(
        &self,
        source: ModSource,
        project_id: &str,
        mc_version: &str,
        loader: Loader,
    ) -> Result<Vec<CandidateVersion>, String>;

    async fn fetch_locked_version(
        &self,
        source: ModSource,
        version_id: &str,
    ) -> Result<CandidateVersion, String>;

    async fn fetch_loader_supported_mc_versions(
        &self,
        loader_version_id: &str,
    ) -> Result<Vec<String>, String>;
}

pub async fn resolve(
    pack: &Pack,
    pack_raw: &[u8],
    mc_version: &str,
    src: &dyn ResolverDataSource,
) -> Result<ResolutionReport, ResolveError> {
    if !pack.mc_versions.iter().any(|v| v == mc_version) {
        return Err(ResolveError::McVersionNotInPack {
            got: mc_version.to_string(),
            available: pack.mc_versions.clone(),
        });
    }

    let pack_content_hash = sha256_hex(pack_raw);
    let mut mods: Vec<ResolvedMod> = Vec::new();
    let mut conflicts: Vec<ConflictReport> = Vec::new();
    let mut visited: HashSet<(ModSource, String)> = HashSet::new();
    let mut depth_at: HashMap<(ModSource, String), usize> = HashMap::new();
    let mut graph: HashMap<(ModSource, String), Vec<(ModSource, String)>> = HashMap::new();

    let mut queue: VecDeque<PendingMod> = VecDeque::new();
    for mod_ref in &pack.mods {
        if let Some(dep_after) = &mod_ref.deprecated_after
            && version_ge(mc_version, dep_after)
        {
            mods.push(ResolvedMod {
                source: mod_ref.source,
                project_id: mod_ref.id.clone(),
                display_name: mod_ref.name.clone(),
                criticality: mod_ref.criticality,
                status: ResolvedStatus::Deprecated {
                    since: dep_after.clone(),
                },
                version_id: None,
                file_url: None,
                file_sha1: None,
                file_sha512: None,
                file_size: None,
                file_name: None,
                auto_added_for: None,
                replacement_hint: mod_ref.replacement.clone(),
            });
            continue;
        }
        queue.push_back(PendingMod::from_pack(mod_ref));
    }

    while let Some(item) = queue.pop_front() {
        let key = (item.source, item.project_id.clone());
        if !visited.insert(key.clone()) {
            continue;
        }

        let (resolved, chosen_deps) = resolve_one(&item, mc_version, pack.loader, src).await?;

        if let ResolvedStatus::Compatible = &resolved.status {
            for edge in &chosen_deps {
                graph
                    .entry(key.clone())
                    .or_default()
                    .push((item.source, edge.project_id.clone()));
                match edge.kind {
                    DependencyKind::Required => {
                        if !visited.contains(&(item.source, edge.project_id.clone())) {
                            let next_depth = depth_at.get(&key).copied().unwrap_or(0) + 1;
                            if next_depth > BACKTRACK_MAX_DEPTH {
                                continue;
                            }
                            depth_at.insert((item.source, edge.project_id.clone()), next_depth);
                            queue.push_back(PendingMod::dependency(
                                item.source,
                                &edge.project_id,
                                &item.project_id,
                            ));
                        }
                    }
                    DependencyKind::Incompatible => {
                        let already_visited =
                            visited.contains(&(item.source, edge.project_id.clone()));
                        let in_pack = pack
                            .mods
                            .iter()
                            .any(|m| m.source == item.source && m.id == edge.project_id);
                        if already_visited || in_pack {
                            return Err(ResolveError::HardIncompatible {
                                a: item.project_id.clone(),
                                b: edge.project_id.clone(),
                            });
                        }
                        conflicts.push(ConflictReport {
                            a_project_id: item.project_id.clone(),
                            b_project_id: edge.project_id.clone(),
                            reason: "manifest declares incompatible".to_string(),
                        });
                    }
                    DependencyKind::Optional | DependencyKind::Embedded => {}
                }
            }
        }

        mods.push(resolved);
    }

    let _ = depth_at;

    if let Some(cycle) = detect_cycle(&mods, &graph) {
        return Err(ResolveError::DependencyCycle {
            path: cycle
                .iter()
                .map(|n| n.project_id.clone())
                .collect::<Vec<_>>()
                .join(" -> "),
        });
    }

    Ok(ResolutionReport {
        pack_id: pack.id.clone(),
        pack_version: pack.version.clone(),
        pack_content_hash,
        mc_version: mc_version.to_string(),
        loader: pack.loader,
        loader_version: pack.loader_versions.get(mc_version).cloned(),
        resolved_at: chrono::Utc::now(),
        mods,
        conflicts,
        cycle: None,
    })
}

struct PendingMod {
    source: ModSource,
    project_id: String,
    display_name: Option<String>,
    criticality: Criticality,
    policy: ModPolicy,
    locked_version: Option<String>,
    replacement_hint: Option<ReplacementRef>,
    auto_added_for: Option<String>,
}

impl PendingMod {
    fn from_pack(p: &PackMod) -> Self {
        Self {
            source: p.source,
            project_id: p.id.clone(),
            display_name: Some(p.name.clone()),
            criticality: p.criticality,
            policy: p.policy,
            locked_version: p.locked_version.clone(),
            replacement_hint: p.replacement.clone(),
            auto_added_for: None,
        }
    }

    fn dependency(source: ModSource, project_id: &str, parent: &str) -> Self {
        Self {
            source,
            project_id: project_id.to_string(),
            display_name: None,
            criticality: Criticality::Optional,
            policy: ModPolicy::Auto,
            locked_version: None,
            replacement_hint: None,
            auto_added_for: Some(parent.to_string()),
        }
    }
}

async fn resolve_one(
    item: &PendingMod,
    mc_version: &str,
    loader: Loader,
    src: &dyn ResolverDataSource,
) -> Result<(ResolvedMod, Vec<DependencyEdge>), ResolveError> {
    let display_name = match &item.display_name {
        Some(n) => n.clone(),
        None => match src.fetch_project_meta(item.source, &item.project_id).await {
            Ok(m) => m.display_name,
            Err(_) => item.project_id.clone(),
        },
    };

    let base = ResolvedMod {
        source: item.source,
        project_id: item.project_id.clone(),
        display_name: display_name.clone(),
        criticality: item.criticality,
        status: ResolvedStatus::Compatible,
        version_id: None,
        file_url: None,
        file_sha1: None,
        file_sha512: None,
        file_size: None,
        file_name: None,
        auto_added_for: item.auto_added_for.clone(),
        replacement_hint: item.replacement_hint.clone(),
    };

    if item.policy == ModPolicy::Lock {
        let locked = item
            .locked_version
            .as_ref()
            .ok_or_else(|| ResolveError::DataSource("lock without locked_version".to_string()))?;
        let v = src
            .fetch_locked_version(item.source, locked)
            .await
            .map_err(ResolveError::DataSource)?;
        if !v.mc_versions.iter().any(|x| x == mc_version)
            || !v
                .loaders
                .iter()
                .any(|l| l.eq_ignore_ascii_case(loader.canonical()))
        {
            return Ok((
                ResolvedMod {
                    status: ResolvedStatus::Conflict {
                        reason: format!(
                            "locked version {locked} does not support mc {mc_version} with loader {l}",
                            l = loader.canonical()
                        ),
                    },
                    ..base
                },
                Vec::new(),
            ));
        }
        let deps = v.dependencies.clone();
        return Ok((fill(base, &v), deps));
    }

    let candidates = src
        .fetch_compatible_versions(item.source, &item.project_id, mc_version, loader)
        .await
        .map_err(ResolveError::DataSource)?;

    if let Some(chosen) = pick_best(&candidates, mc_version) {
        if loader == Loader::Neoforge
            && let Some(verdict) = neoforge_recheck(chosen, mc_version, src).await?
        {
            return Ok((
                ResolvedMod {
                    status: ResolvedStatus::Ambiguous { reason: verdict },
                    ..base
                },
                Vec::new(),
            ));
        }
        let deps = chosen.dependencies.clone();
        return Ok((fill(base, chosen), deps));
    }

    let meta = src
        .fetch_project_meta(item.source, &item.project_id)
        .await
        .map_err(ResolveError::DataSource)?;

    let now = chrono::Utc::now();
    let resolved = match meta.last_release {
        Some(last) => {
            let age_days = (now - last).num_days();
            if age_days > ABANDONED_THRESHOLD_DAYS as i64 {
                ResolvedMod {
                    status: ResolvedStatus::Abandoned {
                        last_release: Some(last),
                    },
                    ..base
                }
            } else {
                ResolvedMod {
                    status: ResolvedStatus::Pending { last_release: last },
                    ..base
                }
            }
        }
        None => ResolvedMod {
            status: ResolvedStatus::Abandoned { last_release: None },
            ..base
        },
    };
    Ok((resolved, Vec::new()))
}

fn pick_best<'a>(candidates: &'a [CandidateVersion], _mc: &str) -> Option<&'a CandidateVersion> {
    candidates
        .iter()
        .filter(|c| c.release_type == ReleaseType::Release)
        .max_by_key(|c| c.date_published)
        .or_else(|| candidates.iter().max_by_key(|c| c.date_published))
}

fn fill(base: ResolvedMod, v: &CandidateVersion) -> ResolvedMod {
    ResolvedMod {
        version_id: Some(v.version_id.clone()),
        file_url: Some(v.file_url.clone()),
        file_sha1: v.file_sha1.clone(),
        file_sha512: v.file_sha512.clone(),
        file_size: Some(v.file_size),
        file_name: Some(v.file_name.clone()),
        ..base
    }
}

async fn neoforge_recheck(
    chosen: &CandidateVersion,
    mc_version: &str,
    src: &dyn ResolverDataSource,
) -> Result<Option<String>, ResolveError> {
    if chosen.mc_versions.iter().any(|v| v == mc_version) {
        return Ok(None);
    }
    let Some(loader_dep) = &chosen.loader_dep_version_id else {
        return Ok(Some(format!(
            "NeoForge candidate {vid} has game_versions {gv:?} (does not contain {mc_version}) \
             and no loader dependency to cross-check; metadata likely returns NeoForge versions \
             instead of MC versions (modrinth/code#6068)",
            vid = chosen.version_id,
            gv = chosen.mc_versions
        )));
    };
    let supported = src
        .fetch_loader_supported_mc_versions(loader_dep)
        .await
        .map_err(ResolveError::DataSource)?;
    if supported.iter().any(|v| v == mc_version) {
        Ok(None)
    } else {
        Ok(Some(format!(
            "NeoForge candidate {vid}: cross-check via loader dep {loader_dep} \
             reports MC versions {supported:?} (does not contain {mc_version})",
            vid = chosen.version_id
        )))
    }
}

fn detect_cycle(
    mods: &[ResolvedMod],
    graph: &HashMap<(ModSource, String), Vec<(ModSource, String)>>,
) -> Option<Vec<CycleNode>> {
    let display_of: HashMap<(ModSource, String), String> = mods
        .iter()
        .map(|m| ((m.source, m.project_id.clone()), m.display_name.clone()))
        .collect();

    let mut color: HashMap<(ModSource, String), Color> = HashMap::new();
    let mut stack: Vec<(ModSource, String)> = Vec::new();

    for start in graph.keys() {
        if color.get(start).copied().unwrap_or(Color::White) != Color::White {
            continue;
        }
        if let Some(cycle) = dfs(start, graph, &mut color, &mut stack) {
            return Some(
                cycle
                    .into_iter()
                    .map(|k| CycleNode {
                        project_id: k.1.clone(),
                        display_name: display_of.get(&k).cloned().unwrap_or_else(|| k.1.clone()),
                    })
                    .collect(),
            );
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    White,
    Gray,
    Black,
}

fn dfs(
    node: &(ModSource, String),
    graph: &HashMap<(ModSource, String), Vec<(ModSource, String)>>,
    color: &mut HashMap<(ModSource, String), Color>,
    stack: &mut Vec<(ModSource, String)>,
) -> Option<Vec<(ModSource, String)>> {
    color.insert(node.clone(), Color::Gray);
    stack.push(node.clone());
    if let Some(neighbors) = graph.get(node) {
        for next in neighbors {
            match color.get(next).copied().unwrap_or(Color::White) {
                Color::White => {
                    if let Some(c) = dfs(next, graph, color, stack) {
                        return Some(c);
                    }
                }
                Color::Gray => {
                    let start = stack.iter().position(|s| s == next).unwrap_or(0);
                    let mut cycle = stack[start..].to_vec();
                    cycle.push(next.clone());
                    return Some(cycle);
                }
                Color::Black => {}
            }
        }
    }
    color.insert(node.clone(), Color::Black);
    stack.pop();
    None
}

fn version_ge(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> { s.split('.').filter_map(|p| p.parse().ok()).collect() };
    let pa = parse(a);
    let pb = parse(b);
    let len = pa.len().max(pb.len());
    for i in 0..len {
        let xa = pa.get(i).copied().unwrap_or(0);
        let xb = pb.get(i).copied().unwrap_or(0);
        if xa > xb {
            return true;
        }
        if xa < xb {
            return false;
        }
    }
    true
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

impl Loader {
    pub fn canonical(self) -> &'static str {
        match self {
            Loader::Fabric => "fabric",
            Loader::Forge => "forge",
            Loader::Neoforge => "neoforge",
            Loader::Quilt => "quilt",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack_with(mods: Vec<PackMod>) -> Pack {
        Pack {
            pack_format: "miao:1.0.0".to_string(),
            id: "p1".to_string(),
            display_name: "Pack 1".to_string(),
            version: "0.1.0".to_string(),
            summary: None,
            description: None,
            loader: Loader::Fabric,
            mc_versions: vec!["1.21.5".to_string()],
            loader_versions: HashMap::new(),
            mods,
            config_overlay: None,
        }
    }

    fn mod_auto(id: &str) -> PackMod {
        PackMod {
            source: ModSource::Modrinth,
            id: id.to_string(),
            name: id.to_string(),
            policy: ModPolicy::Auto,
            locked_version: None,
            criticality: Criticality::Core,
            deprecated_after: None,
            replacement: None,
        }
    }

    fn cv(id: &str, project: &str, age_days: i64) -> CandidateVersion {
        CandidateVersion {
            version_id: format!("v-{id}"),
            project_id: project.to_string(),
            display_name: project.to_string(),
            mc_versions: vec!["1.21.5".to_string()],
            loaders: vec!["fabric".to_string()],
            release_type: ReleaseType::Release,
            date_published: chrono::Utc::now() - chrono::Duration::days(age_days),
            file_url: format!("https://example.com/{id}.jar"),
            file_name: format!("{id}.jar"),
            file_size: 1024,
            file_sha1: Some("abc1".to_string()),
            file_sha512: Some("def512".to_string()),
            dependencies: Vec::new(),
            loader_dep_version_id: None,
        }
    }

    #[derive(Default)]
    struct FakeSource {
        compatible: HashMap<String, Vec<CandidateVersion>>,
        locked: HashMap<String, CandidateVersion>,
        meta: HashMap<String, ProjectMeta>,
        loader_mc: HashMap<String, Vec<String>>,
    }

    #[async_trait]
    impl ResolverDataSource for FakeSource {
        async fn fetch_project_meta(
            &self,
            _source: ModSource,
            project_id: &str,
        ) -> Result<ProjectMeta, String> {
            self.meta
                .get(project_id)
                .cloned()
                .ok_or_else(|| format!("no meta for {project_id}"))
        }

        async fn fetch_compatible_versions(
            &self,
            _source: ModSource,
            project_id: &str,
            _mc_version: &str,
            _loader: Loader,
        ) -> Result<Vec<CandidateVersion>, String> {
            Ok(self.compatible.get(project_id).cloned().unwrap_or_default())
        }

        async fn fetch_locked_version(
            &self,
            _source: ModSource,
            version_id: &str,
        ) -> Result<CandidateVersion, String> {
            self.locked
                .get(version_id)
                .cloned()
                .ok_or_else(|| format!("no locked {version_id}"))
        }

        async fn fetch_loader_supported_mc_versions(
            &self,
            loader_version_id: &str,
        ) -> Result<Vec<String>, String> {
            self.loader_mc
                .get(loader_version_id)
                .cloned()
                .ok_or_else(|| format!("no loader mc for {loader_version_id}"))
        }
    }

    #[tokio::test]
    async fn t_resolve_01_single_mod_auto_compatible() {
        let mut src = FakeSource::default();
        src.compatible
            .insert("sodium".to_string(), vec![cv("s1", "sodium", 5)]);
        let pack = pack_with(vec![mod_auto("sodium")]);
        let r = resolve(&pack, b"{}", "1.21.5", &src).await.unwrap();
        assert_eq!(r.mods.len(), 1);
        assert!(matches!(r.mods[0].status, ResolvedStatus::Compatible));
        assert_eq!(
            r.mods[0].file_url.as_deref(),
            Some("https://example.com/s1.jar")
        );
    }

    #[tokio::test]
    async fn t_resolve_02_single_mod_auto_no_version_pending() {
        let mut src = FakeSource::default();
        src.meta.insert(
            "newmod".to_string(),
            ProjectMeta {
                project_id: "newmod".to_string(),
                display_name: "newmod".to_string(),
                last_release: Some(chrono::Utc::now() - chrono::Duration::days(30)),
            },
        );
        let pack = pack_with(vec![mod_auto("newmod")]);
        let r = resolve(&pack, b"{}", "1.21.5", &src).await.unwrap();
        assert!(matches!(r.mods[0].status, ResolvedStatus::Pending { .. }));
    }

    #[tokio::test]
    async fn t_resolve_03_single_mod_auto_abandoned() {
        let mut src = FakeSource::default();
        src.meta.insert(
            "deadmod".to_string(),
            ProjectMeta {
                project_id: "deadmod".to_string(),
                display_name: "deadmod".to_string(),
                last_release: Some(chrono::Utc::now() - chrono::Duration::days(365)),
            },
        );
        let pack = pack_with(vec![mod_auto("deadmod")]);
        let r = resolve(&pack, b"{}", "1.21.5", &src).await.unwrap();
        assert!(matches!(r.mods[0].status, ResolvedStatus::Abandoned { .. }));
    }

    #[tokio::test]
    async fn t_resolve_04_lock_version_compatible() {
        let mut src = FakeSource::default();
        src.locked
            .insert("v-locked".to_string(), cv("locked", "sodium", 1));
        let mut m = mod_auto("sodium");
        m.policy = ModPolicy::Lock;
        m.locked_version = Some("v-locked".to_string());
        let pack = pack_with(vec![m]);
        let r = resolve(&pack, b"{}", "1.21.5", &src).await.unwrap();
        assert!(matches!(r.mods[0].status, ResolvedStatus::Compatible));
        assert_eq!(r.mods[0].version_id.as_deref(), Some("v-locked"));
    }

    #[tokio::test]
    async fn t_resolve_05_lock_version_unsupported_conflict() {
        let mut src = FakeSource::default();
        let mut wrong = cv("locked", "sodium", 1);
        wrong.mc_versions = vec!["1.20.1".to_string()];
        src.locked.insert("v-locked".to_string(), wrong);
        let mut m = mod_auto("sodium");
        m.policy = ModPolicy::Lock;
        m.locked_version = Some("v-locked".to_string());
        let pack = pack_with(vec![m]);
        let r = resolve(&pack, b"{}", "1.21.5", &src).await.unwrap();
        assert!(matches!(r.mods[0].status, ResolvedStatus::Conflict { .. }));
    }

    #[tokio::test]
    async fn t_resolve_06_dep_required_auto_added() {
        let mut src = FakeSource::default();
        let mut sodium_v = cv("s1", "sodium", 5);
        sodium_v.dependencies = vec![DependencyEdge {
            project_id: "fabric-api".to_string(),
            kind: DependencyKind::Required,
        }];
        src.compatible.insert("sodium".to_string(), vec![sodium_v]);
        src.compatible
            .insert("fabric-api".to_string(), vec![cv("f1", "fabric-api", 2)]);
        src.meta.insert(
            "fabric-api".to_string(),
            ProjectMeta {
                project_id: "fabric-api".to_string(),
                display_name: "fabric-api".to_string(),
                last_release: Some(chrono::Utc::now() - chrono::Duration::days(2)),
            },
        );

        let pack = pack_with(vec![mod_auto("sodium")]);
        let report = resolve(&pack, b"{}", "1.21.5", &src).await.unwrap();
        assert_eq!(report.mods.len(), 2);
        assert!(
            report
                .mods
                .iter()
                .any(|m| m.project_id == "fabric-api"
                    && m.auto_added_for.as_deref() == Some("sodium"))
        );
    }

    #[tokio::test]
    async fn t_resolve_07_dep_chain_within_depth_limit() {
        let mut src = FakeSource::default();
        let chain = ["a", "b", "c", "d", "e"];
        for w in chain.windows(2) {
            let mut v = cv(w[0], w[0], 1);
            v.dependencies = vec![DependencyEdge {
                project_id: w[1].to_string(),
                kind: DependencyKind::Required,
            }];
            src.compatible.insert(w[0].to_string(), vec![v]);
            src.meta.insert(
                w[0].to_string(),
                ProjectMeta {
                    project_id: w[0].to_string(),
                    display_name: w[0].to_string(),
                    last_release: Some(chrono::Utc::now()),
                },
            );
        }
        src.compatible
            .insert("e".to_string(), vec![cv("e1", "e", 1)]);
        src.meta.insert(
            "e".to_string(),
            ProjectMeta {
                project_id: "e".to_string(),
                display_name: "e".to_string(),
                last_release: Some(chrono::Utc::now()),
            },
        );

        let pack = pack_with(vec![mod_auto("a")]);
        let r = resolve(&pack, b"{}", "1.21.5", &src).await.unwrap();
        assert_eq!(r.mods.len(), 5);
    }

    #[tokio::test]
    async fn t_resolve_08_dep_incompatible_blocks_install() {
        let mut src = FakeSource::default();
        let mut sodium_v = cv("s1", "sodium", 1);
        sodium_v.dependencies = vec![DependencyEdge {
            project_id: "rivals".to_string(),
            kind: DependencyKind::Incompatible,
        }];
        src.compatible.insert("sodium".to_string(), vec![sodium_v]);
        src.compatible
            .insert("rivals".to_string(), vec![cv("r1", "rivals", 1)]);

        let pack = pack_with(vec![mod_auto("sodium"), mod_auto("rivals")]);
        let result = resolve(&pack, b"{}", "1.21.5", &src).await;
        assert!(matches!(result, Err(ResolveError::HardIncompatible { .. })));
    }

    #[tokio::test]
    async fn t_resolve_09_dep_cycle_detected_returns_cycle_nodes() {
        let mut src = FakeSource::default();
        let mut a = cv("a1", "a", 1);
        a.dependencies = vec![DependencyEdge {
            project_id: "b".to_string(),
            kind: DependencyKind::Required,
        }];
        let mut b = cv("b1", "b", 1);
        b.dependencies = vec![DependencyEdge {
            project_id: "a".to_string(),
            kind: DependencyKind::Required,
        }];
        src.compatible.insert("a".to_string(), vec![a]);
        src.compatible.insert("b".to_string(), vec![b]);

        let pack = pack_with(vec![mod_auto("a")]);
        let result = resolve(&pack, b"{}", "1.21.5", &src).await;
        assert!(matches!(result, Err(ResolveError::DependencyCycle { .. })));
    }

    #[tokio::test]
    async fn t_resolve_10_mc_version_not_in_pack_versions_pre_rejected() {
        let src = FakeSource::default();
        let pack = pack_with(vec![mod_auto("anything")]);
        let result = resolve(&pack, b"{}", "1.20.1", &src).await;
        assert!(matches!(
            result,
            Err(ResolveError::McVersionNotInPack { .. })
        ));
    }

    #[tokio::test]
    async fn t_resolve_11_deprecated_after_skips_mod() {
        let src = FakeSource::default();
        let mut m = mod_auto("sodium");
        m.deprecated_after = Some("1.21.5".to_string());
        let pack = pack_with(vec![m]);
        let r = resolve(&pack, b"{}", "1.21.5", &src).await.unwrap();
        assert_eq!(r.mods.len(), 1);
        assert!(matches!(
            r.mods[0].status,
            ResolvedStatus::Deprecated { .. }
        ));
    }

    #[tokio::test]
    async fn t_resolve_12_replacement_hint_recorded_not_installed() {
        let src = FakeSource::default();
        let mut m = mod_auto("sodium");
        m.deprecated_after = Some("1.21.5".to_string());
        m.replacement = Some(ReplacementRef {
            source: ModSource::Modrinth,
            id: "newsodium".to_string(),
        });
        let pack = pack_with(vec![m]);
        let r = resolve(&pack, b"{}", "1.21.5", &src).await.unwrap();
        assert_eq!(r.mods.len(), 1);
        let hint = r.mods[0].replacement_hint.as_ref().unwrap();
        assert_eq!(hint.id, "newsodium");
    }

    #[tokio::test]
    async fn t_resolve_13_neoforge_game_versions_ambiguous_detected() {
        let mut src = FakeSource::default();
        let mut bad = cv("iris1", "iris", 1);
        bad.mc_versions = vec!["26.1".to_string()];
        bad.loaders = vec!["neoforge".to_string()];
        bad.loader_dep_version_id = Some("neoforge-26.1.0".to_string());
        src.compatible.insert("iris".to_string(), vec![bad]);
        src.loader_mc
            .insert("neoforge-26.1.0".to_string(), vec!["1.21.10".to_string()]);

        let mut pack = pack_with(vec![mod_auto("iris")]);
        pack.loader = Loader::Neoforge;
        pack.mc_versions = vec!["1.21.5".to_string()];
        let r = resolve(&pack, b"{}", "1.21.5", &src).await.unwrap();
        assert_eq!(r.mods.len(), 1);
        assert!(
            matches!(r.mods[0].status, ResolvedStatus::Ambiguous { .. }),
            "expected Ambiguous, got {:?}",
            r.mods[0].status
        );
    }

    #[tokio::test]
    async fn t_resolve_14_pack_content_hash_computed_from_raw_json() {
        let mut src = FakeSource::default();
        src.compatible
            .insert("sodium".to_string(), vec![cv("s1", "sodium", 5)]);
        let pack = pack_with(vec![mod_auto("sodium")]);
        let raw = b"any-bytes-here";
        let r = resolve(&pack, raw, "1.21.5", &src).await.unwrap();
        let expected = sha256_hex(raw);
        assert_eq!(r.pack_content_hash, expected);
    }
}
