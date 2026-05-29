use crate::config::{DownloadMirror, JavaSource, LauncherConfig};
use crate::error::Result;

use super::install::JavaInstallPlan;

pub async fn plan_install(
    http: &reqwest::Client,
    config: &LauncherConfig,
    major_version: u32,
) -> Result<JavaInstallPlan> {
    let forced_config = LauncherConfig {
        download_mirror: DownloadMirror::Bmclapi,
        ..config.clone()
    };
    let mut plan = super::mojang::plan_install(http, &forced_config, major_version).await?;
    plan.source_name = JavaSource::Bmclapi.display_name();
    plan.install_dir = super::install::install_dir_for(config, JavaSource::Bmclapi, &plan.variant);

    let prev_mojang_dir =
        super::install::install_dir_for(config, JavaSource::Mojang, &plan.variant);
    for task in &mut plan.tasks {
        if let Ok(rel) = task.dest.strip_prefix(&prev_mojang_dir) {
            task.dest = plan.install_dir.join(rel);
        }
    }
    #[cfg(unix)]
    for path in &mut plan.executables {
        if let Ok(rel) = path.strip_prefix(&prev_mojang_dir) {
            *path = plan.install_dir.join(rel);
        }
    }
    for (link, _) in &mut plan.links {
        if let Ok(rel) = link.strip_prefix(&prev_mojang_dir) {
            *link = plan.install_dir.join(rel);
        }
    }

    Ok(plan)
}

#[cfg(test)]
mod tests {
    use crate::config::JavaSource;

    #[test]
    fn bmclapi_id_and_display_name() {
        assert_eq!(JavaSource::Bmclapi.id(), "bmclapi");
        assert_eq!(JavaSource::Bmclapi.display_name(), "BMCLAPI");
    }
}
