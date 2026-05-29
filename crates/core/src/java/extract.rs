use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

use crate::error::{MiaoError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    TarGz,
    Zip,
}

impl ArchiveFormat {
    pub fn from_filename(name: &str) -> Option<ArchiveFormat> {
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
            Some(ArchiveFormat::TarGz)
        } else if lower.ends_with(".zip") {
            Some(ArchiveFormat::Zip)
        } else {
            None
        }
    }
}

pub fn extract(archive: &Path, dest: &Path, format: ArchiveFormat, strip: usize) -> Result<()> {
    if !dest.exists() {
        std::fs::create_dir_all(dest)?;
    }
    match format {
        ArchiveFormat::TarGz => extract_tar_gz(archive, dest, strip),
        ArchiveFormat::Zip => extract_zip(archive, dest, strip),
    }
}

fn extract_tar_gz(archive: &Path, dest: &Path, strip: usize) -> Result<()> {
    let file = File::open(archive)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(gz);
    tar.set_preserve_permissions(true);
    tar.set_overwrite(true);

    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let Some(rel) = strip_prefix(&path, strip) else {
            continue;
        };
        let out_path = sanitize_join(dest, &rel)?;

        let header = entry.header().clone();
        let entry_type = header.entry_type();

        if entry_type.is_dir() {
            std::fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            let Some(link_target) = entry.link_name()? else {
                continue;
            };
            apply_link(&out_path, &link_target, entry_type.is_symlink())?;
            continue;
        }

        let mut out = File::create(&out_path)?;
        io::copy(&mut entry, &mut out)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = header.mode().unwrap_or(0o644);
            std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode))?;
        }
    }
    Ok(())
}

fn extract_zip(archive: &Path, dest: &Path, strip: usize) -> Result<()> {
    let file = File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let raw_name = entry.name().to_string();
        let entry_path = PathBuf::from(&raw_name);
        let Some(rel) = strip_prefix(&entry_path, strip) else {
            continue;
        };
        let out_path = sanitize_join(dest, &rel)?;

        if entry.is_dir() || raw_name.ends_with('/') {
            std::fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&out_path)?;
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        out.write_all(&buf)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode()
                && mode != 0
            {
                std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode))?;
            }
        }
    }
    Ok(())
}

fn strip_prefix(path: &Path, n: usize) -> Option<PathBuf> {
    let mut comps = path.components();
    for _ in 0..n {
        match comps.next()? {
            Component::Normal(_) => continue,
            _ => return None,
        }
    }
    let rest = comps.as_path();
    if rest.as_os_str().is_empty() {
        None
    } else {
        Some(rest.to_path_buf())
    }
}

fn sanitize_join(base: &Path, rel: &Path) -> Result<PathBuf> {
    let mut out = base.to_path_buf();
    for comp in rel.components() {
        match comp {
            Component::Normal(s) => out.push(s),
            Component::CurDir => continue,
            _ => {
                return Err(MiaoError::Other(format!(
                    "Refusing to extract entry with unsafe path: {}",
                    rel.display()
                )));
            }
        }
    }
    Ok(out)
}

fn apply_link(link_path: &Path, target: &Path, is_symlink: bool) -> Result<()> {
    if link_path.exists() {
        std::fs::remove_file(link_path).ok();
    }
    if is_symlink {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link_path)?;
        }
        #[cfg(windows)]
        {
            let resolved = link_path
                .parent()
                .map(|p| p.join(target))
                .unwrap_or_else(|| target.to_path_buf());
            if resolved.exists() {
                std::fs::copy(&resolved, link_path)?;
            }
        }
    } else {
        let resolved = link_path
            .parent()
            .map(|p| p.join(target))
            .unwrap_or_else(|| target.to_path_buf());
        if resolved.exists() {
            std::fs::copy(&resolved, link_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_from_filename_tar_gz() {
        assert_eq!(
            ArchiveFormat::from_filename("OpenJDK21U-jre_x64_linux_hotspot_21.0.11_10.tar.gz"),
            Some(ArchiveFormat::TarGz)
        );
        assert_eq!(
            ArchiveFormat::from_filename("foo.TGZ"),
            Some(ArchiveFormat::TarGz)
        );
    }

    #[test]
    fn format_from_filename_zip() {
        assert_eq!(
            ArchiveFormat::from_filename("microsoft-jdk-21-windows-x64.zip"),
            Some(ArchiveFormat::Zip)
        );
    }

    #[test]
    fn format_from_filename_unknown() {
        assert_eq!(ArchiveFormat::from_filename("foo.7z"), None);
        assert_eq!(ArchiveFormat::from_filename("foo.tar.xz"), None);
    }

    #[test]
    fn strip_prefix_one_level() {
        let p = Path::new("jdk-21.0.11+10/bin/java");
        let stripped = strip_prefix(p, 1).unwrap();
        assert_eq!(stripped, PathBuf::from("bin/java"));
    }

    #[test]
    fn strip_prefix_only_top_level_returns_none() {
        let p = Path::new("jdk-21.0.11+10");
        assert!(strip_prefix(p, 1).is_none());
    }

    #[test]
    fn strip_prefix_zero_keeps_path() {
        let p = Path::new("a/b/c");
        assert_eq!(strip_prefix(p, 0).unwrap(), PathBuf::from("a/b/c"));
    }

    #[test]
    fn sanitize_rejects_parent_traversal() {
        let base = PathBuf::from("/tmp/dest");
        let rel = Path::new("../etc/passwd");
        assert!(sanitize_join(&base, rel).is_err());
    }

    #[test]
    fn sanitize_rejects_absolute() {
        let base = PathBuf::from("/tmp/dest");
        let rel = Path::new("/etc/passwd");
        assert!(sanitize_join(&base, rel).is_err());
    }

    #[test]
    fn sanitize_handles_curdir() {
        let base = PathBuf::from("/tmp/dest");
        let rel = Path::new("./bin/java");
        let out = sanitize_join(&base, rel).unwrap();
        assert_eq!(out, PathBuf::from("/tmp/dest/bin/java"));
    }

    #[test]
    fn extract_zip_strips_top_dir() {
        use std::io::Write as _;
        use zip::write::SimpleFileOptions;

        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("test.zip");
        let f = std::fs::File::create(&archive).unwrap();
        let mut zw = zip::ZipWriter::new(f);
        let opts = SimpleFileOptions::default();
        zw.add_directory("jdk-21/", opts).unwrap();
        zw.add_directory("jdk-21/bin/", opts).unwrap();
        zw.start_file("jdk-21/bin/java", opts).unwrap();
        zw.write_all(b"#!/fake/java\n").unwrap();
        zw.finish().unwrap();

        let dest = tmp.path().join("out");
        extract(&archive, &dest, ArchiveFormat::Zip, 1).unwrap();

        let java = dest.join("bin/java");
        assert!(java.exists());
        let content = std::fs::read_to_string(&java).unwrap();
        assert_eq!(content, "#!/fake/java\n");
    }

    #[test]
    fn extract_tar_gz_strips_top_dir() {
        use std::io::Write as _;

        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("test.tar.gz");
        let f = std::fs::File::create(&archive).unwrap();
        let gz = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
        let mut tw = tar::Builder::new(gz);

        let mut data = Vec::new();
        data.write_all(b"#!/fake/java\n").unwrap();
        let mut hdr = tar::Header::new_gnu();
        hdr.set_size(data.len() as u64);
        hdr.set_mode(0o755);
        hdr.set_cksum();
        tw.append_data(&mut hdr, "jdk-21.0.11+10/bin/java", &data[..])
            .unwrap();
        tw.into_inner().unwrap().finish().unwrap();

        let dest = tmp.path().join("out");
        extract(&archive, &dest, ArchiveFormat::TarGz, 1).unwrap();

        let java = dest.join("bin/java");
        assert!(java.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perm = std::fs::metadata(&java).unwrap().permissions();
            assert_eq!(perm.mode() & 0o777, 0o755);
        }
    }
}
