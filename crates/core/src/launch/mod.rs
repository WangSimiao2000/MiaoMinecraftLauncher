use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;

use crate::auth::AuthMethod;
use crate::config::LauncherConfig;
use crate::instance::Instance;
use crate::version::meta::VersionMeta;

pub struct LaunchOptions {
    pub game_dir: PathBuf,
    pub java_path: PathBuf,
    pub version_meta: VersionMeta,
    pub instance: Instance,
    pub auth: AuthMethod,
    pub config: LauncherConfig,
}

pub fn build_launch_command(options: &LaunchOptions) -> Result<Command> {
    let mut cmd = Command::new(&options.java_path);

    cmd.arg(format!("-Xmx{}m", options.instance.memory_max_mb));
    cmd.arg(format!("-Xms{}m", options.instance.memory_min_mb));

    for arg in &options.instance.jvm_args {
        cmd.arg(arg);
    }

    let classpath = build_classpath(options)?;
    cmd.arg("-cp").arg(classpath);

    cmd.arg(&options.version_meta.main_class);

    let game_args = build_game_args(options)?;
    for arg in game_args {
        cmd.arg(arg);
    }

    cmd.current_dir(&options.game_dir);

    Ok(cmd)
}

fn build_classpath(options: &LaunchOptions) -> Result<String> {
    let mut paths: Vec<String> = Vec::new();

    for lib in &options.version_meta.libraries {
        if !VersionMeta::is_library_allowed(lib) {
            continue;
        }
        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                let path = options.config.libraries_dir().join(&artifact.path);
                paths.push(path.to_string_lossy().to_string());
            }
        }
    }

    let client_jar = options
        .config
        .versions_dir()
        .join(&options.version_meta.id)
        .join(format!("{}.jar", options.version_meta.id));
    paths.push(client_jar.to_string_lossy().to_string());

    Ok(paths.join(":"))
}

fn build_game_args(options: &LaunchOptions) -> Result<Vec<String>> {
    let mut args = Vec::new();

    args.push("--username".to_string());
    args.push(options.auth.username().to_string());

    args.push("--version".to_string());
    args.push(options.version_meta.id.clone());

    args.push("--gameDir".to_string());
    args.push(options.game_dir.to_string_lossy().to_string());

    args.push("--assetsDir".to_string());
    args.push(options.config.assets_dir().to_string_lossy().to_string());

    args.push("--assetIndex".to_string());
    args.push(options.version_meta.asset_index.id.clone());

    args.push("--uuid".to_string());
    args.push(options.auth.uuid().to_string());

    args.push("--accessToken".to_string());
    args.push(options.auth.access_token().to_string());

    args.push("--userType".to_string());
    args.push("msa".to_string());

    if let Some(res) = &options.instance.resolution {
        args.push("--width".to_string());
        args.push(res.width.to_string());
        args.push("--height".to_string());
        args.push(res.height.to_string());
    }

    for arg in &options.instance.game_args {
        args.push(arg.clone());
    }

    Ok(args)
}
