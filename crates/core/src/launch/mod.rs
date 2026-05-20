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
        if let Some(downloads) = &lib.downloads
            && let Some(artifact) = &downloads.artifact
        {
            let path = options.config.libraries_dir().join(&artifact.path);
            paths.push(path.to_string_lossy().to_string());
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
    let mut args = vec![
        "--username".to_string(),
        options.auth.username().to_string(),
        "--version".to_string(),
        options.version_meta.id.clone(),
        "--gameDir".to_string(),
        options.game_dir.to_string_lossy().to_string(),
        "--assetsDir".to_string(),
        options.config.assets_dir().to_string_lossy().to_string(),
        "--assetIndex".to_string(),
        options.version_meta.asset_index.id.clone(),
        "--uuid".to_string(),
        options.auth.uuid().to_string(),
        "--accessToken".to_string(),
        options.auth.access_token().to_string(),
        "--userType".to_string(),
        "msa".to_string(),
    ];

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::offline::create_offline_account;
    use crate::instance::Resolution;
    use crate::version::meta::*;

    fn make_test_options(with_resolution: bool) -> LaunchOptions {
        let mut instance = Instance::new("test", "1.20.4").with_memory(2048, 4096);
        if with_resolution {
            instance = instance.with_resolution(1920, 1080);
        }
        instance.jvm_args = vec!["-XX:+UseG1GC".to_string()];
        instance.game_args = vec!["--server".to_string(), "localhost".to_string()];

        let auth = AuthMethod::Offline(create_offline_account("TestPlayer"));
        let config = LauncherConfig {
            data_dir: PathBuf::from("/tmp/miao-test"),
            ..Default::default()
        };

        let meta = VersionMeta {
            id: "1.20.4".to_string(),
            main_class: "net.minecraft.client.main.Main".to_string(),
            minecraft_arguments: None,
            arguments: None,
            libraries: vec![
                Library {
                    name: "com.mojang:authlib:3.16".to_string(),
                    downloads: Some(LibraryDownloads {
                        artifact: Some(Artifact {
                            path: "com/mojang/authlib/3.16/authlib-3.16.jar".to_string(),
                            sha1: "abc".to_string(),
                            size: 100,
                            url: "https://example.com/authlib.jar".to_string(),
                        }),
                    }),
                    rules: None,
                },
                Library {
                    name: "windows-only:lib:1.0".to_string(),
                    downloads: Some(LibraryDownloads {
                        artifact: Some(Artifact {
                            path: "windows/only.jar".to_string(),
                            sha1: "def".to_string(),
                            size: 200,
                            url: "https://example.com/win.jar".to_string(),
                        }),
                    }),
                    rules: Some(vec![Rule {
                        action: "allow".to_string(),
                        os: Some(OsRule {
                            name: Some("windows".to_string()),
                        }),
                    }]),
                },
            ],
            asset_index: AssetIndex {
                id: "12".to_string(),
                sha1: "aaa".to_string(),
                size: 100,
                url: String::new(),
                total_size: None,
            },
            downloads: Downloads {
                client: DownloadEntry {
                    sha1: "bbb".to_string(),
                    size: 25000000,
                    url: String::new(),
                },
                server: None,
            },
            java_version: Some(JavaVersion { major_version: 17 }),
        };

        LaunchOptions {
            game_dir: PathBuf::from("/tmp/miao-test/instances/test"),
            java_path: PathBuf::from("/usr/bin/java"),
            version_meta: meta,
            instance,
            auth,
            config,
        }
    }

    #[test]
    fn build_classpath_includes_allowed_libs() {
        let options = make_test_options(false);
        let cp = build_classpath(&options).unwrap();

        assert!(cp.contains("authlib-3.16.jar"));
        assert!(!cp.contains("windows/only.jar"));
        assert!(cp.contains("1.20.4.jar"));
    }

    #[test]
    fn build_classpath_uses_colon_separator() {
        let options = make_test_options(false);
        let cp = build_classpath(&options).unwrap();
        assert!(cp.contains(':'));
    }

    #[test]
    fn build_game_args_contains_required_fields() {
        let options = make_test_options(false);
        let args = build_game_args(&options).unwrap();

        assert!(args.contains(&"--username".to_string()));
        assert!(args.contains(&"TestPlayer".to_string()));
        assert!(args.contains(&"--version".to_string()));
        assert!(args.contains(&"1.20.4".to_string()));
        assert!(args.contains(&"--gameDir".to_string()));
        assert!(args.contains(&"--assetsDir".to_string()));
        assert!(args.contains(&"--assetIndex".to_string()));
        assert!(args.contains(&"12".to_string()));
        assert!(args.contains(&"--uuid".to_string()));
        assert!(args.contains(&"--accessToken".to_string()));
        assert!(args.contains(&"0".to_string()));
        assert!(args.contains(&"--userType".to_string()));
        assert!(args.contains(&"msa".to_string()));
    }

    #[test]
    fn build_game_args_with_resolution() {
        let options = make_test_options(true);
        let args = build_game_args(&options).unwrap();

        assert!(args.contains(&"--width".to_string()));
        assert!(args.contains(&"1920".to_string()));
        assert!(args.contains(&"--height".to_string()));
        assert!(args.contains(&"1080".to_string()));
    }

    #[test]
    fn build_game_args_without_resolution() {
        let options = make_test_options(false);
        let args = build_game_args(&options).unwrap();

        assert!(!args.contains(&"--width".to_string()));
        assert!(!args.contains(&"--height".to_string()));
    }

    #[test]
    fn build_game_args_includes_custom_args() {
        let options = make_test_options(false);
        let args = build_game_args(&options).unwrap();

        assert!(args.contains(&"--server".to_string()));
        assert!(args.contains(&"localhost".to_string()));
    }

    #[test]
    fn build_launch_command_structure() {
        let options = make_test_options(true);
        let cmd = build_launch_command(&options).unwrap();

        let program = cmd.get_program().to_string_lossy().to_string();
        assert_eq!(program, "/usr/bin/java");

        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();

        assert!(args.contains(&"-Xmx4096m".to_string()));
        assert!(args.contains(&"-Xms2048m".to_string()));
        assert!(args.contains(&"-XX:+UseG1GC".to_string()));
        assert!(args.contains(&"-cp".to_string()));
        assert!(args.contains(&"net.minecraft.client.main.Main".to_string()));
    }
}
