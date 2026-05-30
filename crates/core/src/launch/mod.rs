use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::Result;

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
    crate::process::no_window(&mut cmd);

    cmd.arg(format!("-Xmx{}m", options.instance.memory_max_mb));
    cmd.arg(format!("-Xms{}m", options.instance.memory_min_mb));

    let natives_path =
        crate::version::install::natives_dir(&options.config, &options.version_meta.id);
    cmd.arg(format!("-Djava.library.path={}", natives_path.display()));

    for arg in &options.instance.jvm_args {
        cmd.arg(arg);
    }

    let classpath = build_classpath(options)?;
    cmd.arg("-cp").arg(classpath);

    let main_class = options
        .instance
        .mod_loader
        .as_ref()
        .and_then(|l| l.main_class.as_deref())
        .unwrap_or(&options.version_meta.main_class);
    cmd.arg(main_class);

    let game_args = build_game_args(options)?;
    for arg in game_args {
        cmd.arg(arg);
    }

    cmd.current_dir(&options.game_dir);

    Ok(cmd)
}

fn build_classpath(options: &LaunchOptions) -> Result<String> {
    build_classpath_for_os(options, super::version::meta::current_os_name())
}

fn build_classpath_for_os(options: &LaunchOptions, os: &str) -> Result<String> {
    let libraries_dir = options.config.libraries_dir();
    let mut paths: Vec<String> = Vec::new();
    let mut coord_index: HashMap<String, usize> = HashMap::new();

    let push_lib =
        |paths: &mut Vec<String>, coord_index: &mut HashMap<String, usize>, path_str: String| {
            let Some(key) = maven_coord_key(Path::new(&path_str), &libraries_dir) else {
                paths.push(path_str);
                return;
            };
            match coord_index.get(&key).copied() {
                None => {
                    coord_index.insert(key, paths.len());
                    paths.push(path_str);
                }
                Some(existing_idx) => {
                    let existing_path = &paths[existing_idx];
                    let new_ver = maven_coord_version(Path::new(&path_str), &libraries_dir);
                    let old_ver = maven_coord_version(Path::new(existing_path), &libraries_dir);
                    if let (Some(n), Some(o)) = (new_ver, old_ver)
                        && compare_maven_version(&n, &o) == Ordering::Greater
                    {
                        paths[existing_idx] = path_str;
                    }
                }
            }
        };

    if let Some(loader) = &options.instance.mod_loader {
        for lib_path in &loader.extra_libraries {
            push_lib(&mut paths, &mut coord_index, lib_path.clone());
        }
    }

    for lib in &options.version_meta.libraries {
        if !VersionMeta::is_library_allowed_for_os(lib, os) {
            continue;
        }
        if lib.natives.is_some()
            && lib
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .is_none()
        {
            continue;
        }
        if let Some(downloads) = &lib.downloads
            && let Some(artifact) = &downloads.artifact
        {
            let path = libraries_dir.join(&artifact.path);
            push_lib(
                &mut paths,
                &mut coord_index,
                path.to_string_lossy().to_string(),
            );
        }
    }

    let client_jar = options
        .config
        .versions_dir()
        .join(&options.version_meta.id)
        .join(format!("{}.jar", options.version_meta.id));
    paths.push(client_jar.to_string_lossy().to_string());

    let separator = if os == "windows" { ";" } else { ":" };
    Ok(paths.join(separator))
}

fn maven_coord_version(jar_path: &Path, libraries_dir: &Path) -> Option<String> {
    let rel = jar_path.strip_prefix(libraries_dir).ok()?;
    let segments: Vec<&str> = rel.iter().map(|s| s.to_str()).collect::<Option<Vec<_>>>()?;
    if segments.len() < 4 {
        return None;
    }
    Some(segments[segments.len() - 2].to_string())
}

fn compare_maven_version(a: &str, b: &str) -> Ordering {
    let split = |s: &str| -> Vec<String> {
        s.split(['.', '-', '+', '_'])
            .map(|p| p.to_string())
            .collect()
    };
    let a_parts = split(a);
    let b_parts = split(b);
    for i in 0..a_parts.len().max(b_parts.len()) {
        let ap = a_parts.get(i).map(String::as_str).unwrap_or("0");
        let bp = b_parts.get(i).map(String::as_str).unwrap_or("0");
        let an = ap.parse::<u64>().ok();
        let bn = bp.parse::<u64>().ok();
        let cmp = match (an, bn) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => ap.cmp(bp),
        };
        if cmp != Ordering::Equal {
            return cmp;
        }
    }
    Ordering::Equal
}

fn maven_coord_key(jar_path: &Path, libraries_dir: &Path) -> Option<String> {
    let rel = jar_path.strip_prefix(libraries_dir).ok()?;
    let segments: Vec<&str> = rel.iter().map(|s| s.to_str()).collect::<Option<Vec<_>>>()?;
    if segments.len() < 4 {
        return None;
    }
    let filename = segments[segments.len() - 1];
    let version = segments[segments.len() - 2];
    let artifact_id = segments[segments.len() - 3];
    let group_segments = &segments[..segments.len() - 3];
    let group_id = group_segments.join(".");

    let stem = filename.strip_suffix(".jar")?;
    let prefix = format!("{artifact_id}-{version}");
    let classifier = stem.strip_prefix(&prefix).and_then(|rest| {
        if rest.is_empty() {
            Some("")
        } else {
            rest.strip_prefix('-')
        }
    })?;

    if classifier.is_empty() {
        Some(format!("{group_id}:{artifact_id}"))
    } else {
        Some(format!("{group_id}:{artifact_id}::{classifier}"))
    }
}

fn build_game_args(options: &LaunchOptions) -> Result<Vec<String>> {
    if let Some(ref mc_args) = options.version_meta.minecraft_arguments {
        return build_game_args_legacy(options, mc_args);
    }

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

fn build_game_args_legacy(options: &LaunchOptions, template: &str) -> Result<Vec<String>> {
    let assets_dir = options.config.assets_dir();
    let game_dir = &options.game_dir;

    let resolved = template
        .replace("${auth_player_name}", options.auth.username())
        .replace("${version_name}", &options.version_meta.id)
        .replace("${game_directory}", &game_dir.to_string_lossy())
        .replace("${assets_root}", &assets_dir.to_string_lossy())
        .replace(
            "${game_assets}",
            &assets_dir.join("virtual").join("legacy").to_string_lossy(),
        )
        .replace("${assets_index_name}", &options.version_meta.asset_index.id)
        .replace("${auth_uuid}", &options.auth.uuid().to_string())
        .replace("${auth_access_token}", options.auth.access_token())
        .replace("${auth_session}", options.auth.access_token())
        .replace("${user_properties}", "{}")
        .replace("${user_type}", "msa");

    let mut args: Vec<String> = resolved.split_whitespace().map(|s| s.to_string()).collect();

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
                        classifiers: None,
                    }),
                    rules: None,
                    natives: None,
                    extract: None,
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
                        classifiers: None,
                    }),
                    rules: Some(vec![Rule {
                        action: "allow".to_string(),
                        os: Some(OsRule {
                            name: Some("windows".to_string()),
                        }),
                    }]),
                    natives: None,
                    extract: None,
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
        let cp = build_classpath_for_os(&options, "linux").unwrap();

        assert!(cp.contains("authlib-3.16.jar"));
        assert!(!cp.contains("windows/only.jar"));
        assert!(cp.contains("1.20.4.jar"));
    }

    #[test]
    fn build_classpath_uses_colon_separator() {
        let options = make_test_options(false);
        let cp = build_classpath_for_os(&options, "linux").unwrap();
        assert!(cp.contains(':'));
    }

    #[test]
    fn build_classpath_uses_semicolon_on_windows() {
        let options = make_test_options(false);
        let cp = build_classpath_for_os(&options, "windows").unwrap();
        assert!(cp.contains(';'));
    }

    #[test]
    fn maven_coord_key_extracts_group_and_artifact_no_classifier() {
        let lib_dir = PathBuf::from("/data/libraries");
        let key = maven_coord_key(&lib_dir.join("org/ow2/asm/asm/9.9/asm-9.9.jar"), &lib_dir);
        assert_eq!(key, Some("org.ow2.asm:asm".to_string()));

        let key2 = maven_coord_key(
            &lib_dir.join("com/mojang/authlib/3.16/authlib-3.16.jar"),
            &lib_dir,
        );
        assert_eq!(key2, Some("com.mojang:authlib".to_string()));
    }

    #[test]
    fn maven_coord_key_includes_classifier_for_lwjgl_natives() {
        let lib_dir = PathBuf::from("/data/libraries");
        let regular = maven_coord_key(
            &lib_dir.join("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar"),
            &lib_dir,
        );
        let native = maven_coord_key(
            &lib_dir.join("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar"),
            &lib_dir,
        );
        assert_eq!(regular, Some("org.lwjgl:lwjgl".to_string()));
        assert_eq!(native, Some("org.lwjgl:lwjgl::natives-linux".to_string()));
        assert_ne!(
            regular, native,
            "regular jar and natives jar must not collide"
        );
    }

    #[test]
    fn maven_coord_key_includes_classifier_with_hyphens() {
        let lib_dir = PathBuf::from("/data/libraries");
        let key = maven_coord_key(
            &lib_dir.join("io/netty/netty-transport-native-epoll/4.1.115.Final/netty-transport-native-epoll-4.1.115.Final-linux-x86_64.jar"),
            &lib_dir,
        );
        assert_eq!(
            key,
            Some("io.netty:netty-transport-native-epoll::linux-x86_64".to_string())
        );
    }

    #[test]
    fn maven_coord_key_returns_none_for_paths_outside_libraries_dir() {
        let lib_dir = PathBuf::from("/data/libraries");
        assert_eq!(
            maven_coord_key(Path::new("/elsewhere/foo.jar"), &lib_dir),
            None
        );
    }

    #[test]
    fn maven_coord_key_returns_none_for_too_short_path() {
        let lib_dir = PathBuf::from("/data/libraries");
        assert_eq!(maven_coord_key(&lib_dir.join("flat.jar"), &lib_dir), None);
    }

    #[test]
    fn maven_coord_key_returns_none_for_filename_not_matching_artifact_version() {
        let lib_dir = PathBuf::from("/data/libraries");
        assert_eq!(
            maven_coord_key(
                &lib_dir.join("org/foo/bar/1.0/UNRELATED-NAME.jar"),
                &lib_dir
            ),
            None
        );
    }

    #[test]
    fn compare_maven_version_numeric_segments() {
        assert_eq!(compare_maven_version("9.9", "9.6"), Ordering::Greater);
        assert_eq!(compare_maven_version("9.6", "9.9"), Ordering::Less);
        assert_eq!(compare_maven_version("9.10", "9.9"), Ordering::Greater);
        assert_eq!(compare_maven_version("9.9", "9.9"), Ordering::Equal);
    }

    #[test]
    fn compare_maven_version_handles_qualifiers() {
        assert_eq!(
            compare_maven_version("4.1.115.Final", "4.1.116"),
            Ordering::Less,
            "numeric segment 115 < 116 should dominate over later qualifier"
        );
        assert_eq!(
            compare_maven_version("4.1.115.Final", "4.1.115.Final"),
            Ordering::Equal
        );
    }

    #[test]
    fn maven_coord_version_extracts_from_path() {
        let lib_dir = PathBuf::from("/data/libraries");
        assert_eq!(
            maven_coord_version(&lib_dir.join("org/ow2/asm/asm/9.9/asm-9.9.jar"), &lib_dir),
            Some("9.9".to_string())
        );
        assert_eq!(
            maven_coord_version(&lib_dir.join("flat.jar"), &lib_dir),
            None
        );
    }

    #[test]
    fn build_classpath_dedup_keeps_higher_version_when_loader_first() {
        let mut options = make_test_options(false);
        let lib_dir = options.config.libraries_dir();
        options.instance.mod_loader = Some(crate::instance::ModLoaderConfig {
            loader_type: crate::modloader::ModLoaderType::Fabric,
            version: "0.16.10".to_string(),
            main_class: None,
            extra_libraries: vec![
                lib_dir
                    .join("org/ow2/asm/asm/9.9/asm-9.9.jar")
                    .to_string_lossy()
                    .to_string(),
            ],
        });
        options.version_meta.libraries.push(Library {
            name: "org.ow2.asm:asm:9.6".to_string(),
            downloads: Some(LibraryDownloads {
                artifact: Some(Artifact {
                    path: "org/ow2/asm/asm/9.6/asm-9.6.jar".to_string(),
                    sha1: "ccc".to_string(),
                    size: 100,
                    url: String::new(),
                }),
                classifiers: None,
            }),
            rules: None,
            natives: None,
            extract: None,
        });

        let cp = build_classpath_for_os(&options, "linux").unwrap();

        assert!(cp.contains("asm-9.9.jar"));
        assert!(!cp.contains("asm-9.6.jar"));
    }

    #[test]
    fn build_classpath_dedup_keeps_higher_version_when_vanilla_first() {
        let mut options = make_test_options(false);
        let lib_dir = options.config.libraries_dir();
        options.instance.mod_loader = Some(crate::instance::ModLoaderConfig {
            loader_type: crate::modloader::ModLoaderType::Fabric,
            version: "0.16.10".to_string(),
            main_class: None,
            extra_libraries: vec![
                lib_dir
                    .join("org/ow2/asm/asm/9.6/asm-9.6.jar")
                    .to_string_lossy()
                    .to_string(),
            ],
        });
        options.version_meta.libraries.push(Library {
            name: "org.ow2.asm:asm:9.9".to_string(),
            downloads: Some(LibraryDownloads {
                artifact: Some(Artifact {
                    path: "org/ow2/asm/asm/9.9/asm-9.9.jar".to_string(),
                    sha1: "ccc".to_string(),
                    size: 100,
                    url: String::new(),
                }),
                classifiers: None,
            }),
            rules: None,
            natives: None,
            extract: None,
        });

        let cp = build_classpath_for_os(&options, "linux").unwrap();

        assert!(
            cp.contains("asm-9.9.jar"),
            "version comparison should choose 9.9 even when 9.6 was inserted first"
        );
        assert!(!cp.contains("asm-9.6.jar"));
    }

    #[test]
    fn build_classpath_dedupes_loader_vs_vanilla_asm() {
        let mut options = make_test_options(false);
        let lib_dir = options.config.libraries_dir();
        options.instance.mod_loader = Some(crate::instance::ModLoaderConfig {
            loader_type: crate::modloader::ModLoaderType::Fabric,
            version: "0.16.10".to_string(),
            main_class: Some("net.fabricmc.loader.impl.launch.knot.KnotClient".to_string()),
            extra_libraries: vec![
                lib_dir
                    .join("org/ow2/asm/asm/9.9/asm-9.9.jar")
                    .to_string_lossy()
                    .to_string(),
            ],
        });
        options.version_meta.libraries.push(Library {
            name: "org.ow2.asm:asm:9.6".to_string(),
            downloads: Some(LibraryDownloads {
                artifact: Some(Artifact {
                    path: "org/ow2/asm/asm/9.6/asm-9.6.jar".to_string(),
                    sha1: "ccc".to_string(),
                    size: 100,
                    url: String::new(),
                }),
                classifiers: None,
            }),
            rules: None,
            natives: None,
            extract: None,
        });

        let cp = build_classpath_for_os(&options, "linux").unwrap();

        assert!(cp.contains("asm-9.9.jar"), "loader's ASM 9.9 must remain");
        assert!(
            !cp.contains("asm-9.6.jar"),
            "vanilla's ASM 9.6 must be dropped (would cause Fabric duplicate-class crash)"
        );
    }

    #[test]
    fn build_classpath_keeps_lwjgl_native_classifier_alongside_regular_jar() {
        let mut options = make_test_options(false);
        options.version_meta.libraries.extend(vec![
            Library {
                name: "org.lwjgl:lwjgl:3.3.3".to_string(),
                downloads: Some(LibraryDownloads {
                    artifact: Some(Artifact {
                        path: "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar".to_string(),
                        sha1: "a".to_string(),
                        size: 100,
                        url: String::new(),
                    }),
                    classifiers: None,
                }),
                rules: None,
                natives: None,
                extract: None,
            },
            Library {
                name: "org.lwjgl:lwjgl:3.3.3:natives-linux".to_string(),
                downloads: Some(LibraryDownloads {
                    artifact: Some(Artifact {
                        path: "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar".to_string(),
                        sha1: "b".to_string(),
                        size: 100,
                        url: String::new(),
                    }),
                    classifiers: None,
                }),
                rules: None,
                natives: None,
                extract: None,
            },
        ]);

        let cp = build_classpath_for_os(&options, "linux").unwrap();

        assert!(
            cp.contains("lwjgl-3.3.3.jar"),
            "regular lwjgl jar must remain"
        );
        assert!(
            cp.contains("lwjgl-3.3.3-natives-linux.jar"),
            "lwjgl native classifier must NOT be deduped against the regular jar"
        );
    }

    #[test]
    fn build_classpath_keeps_distinct_artifacts_in_same_group() {
        let mut options = make_test_options(false);
        let lib_dir = options.config.libraries_dir();
        options.instance.mod_loader = Some(crate::instance::ModLoaderConfig {
            loader_type: crate::modloader::ModLoaderType::Fabric,
            version: "0.16.10".to_string(),
            main_class: None,
            extra_libraries: vec![
                lib_dir
                    .join("org/ow2/asm/asm/9.9/asm-9.9.jar")
                    .to_string_lossy()
                    .to_string(),
                lib_dir
                    .join("org/ow2/asm/asm-tree/9.9/asm-tree-9.9.jar")
                    .to_string_lossy()
                    .to_string(),
                lib_dir
                    .join("org/ow2/asm/asm-commons/9.9/asm-commons-9.9.jar")
                    .to_string_lossy()
                    .to_string(),
            ],
        });

        let cp = build_classpath_for_os(&options, "linux").unwrap();

        assert!(cp.contains("asm-9.9.jar"));
        assert!(cp.contains("asm-tree-9.9.jar"));
        assert!(cp.contains("asm-commons-9.9.jar"));
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
