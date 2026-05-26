use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashReport {
    pub file_path: PathBuf,
    pub description: String,
    pub exception: String,
    pub stack_trace: Vec<String>,
    pub involved_mods: Vec<String>,
    pub system_details: SystemDetails,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SystemDetails {
    pub minecraft_version: Option<String>,
    pub mod_loader: Option<String>,
    pub java_version: Option<String>,
    pub operating_system: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogAnalysis {
    pub errors: Vec<LogError>,
    pub warnings: Vec<String>,
    pub suspected_mods: Vec<SuspectedMod>,
    pub crash_type: Option<CrashType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogError {
    pub message: String,
    pub source_mod: Option<String>,
    pub category: ErrorCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    OutOfMemory,
    ClassNotFound,
    ModConflict,
    MissingDependency,
    OpenGl,
    NativeLibrary,
    ModLoading,
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrashType {
    OutOfMemory,
    ModIncompatibility,
    MissingDependency,
    CorruptedInstall,
    JavaIncompatibility,
    OpenGlFailure,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuspectedMod {
    pub name: String,
    pub file_name: Option<String>,
    pub reason: String,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

pub fn parse_crash_report(path: &Path) -> std::io::Result<CrashReport> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_crash_report_content(&content, path))
}

pub fn parse_crash_report_content(content: &str, path: &Path) -> CrashReport {
    let description = extract_section(content, "Description:")
        .unwrap_or_default()
        .trim()
        .to_string();

    let exception = extract_exception(content);
    let stack_trace = extract_stack_trace(content);
    let involved_mods = extract_involved_mods(content);
    let system_details = extract_system_details(content);

    CrashReport {
        file_path: path.to_path_buf(),
        description,
        exception,
        stack_trace,
        involved_mods,
        system_details,
    }
}

pub fn parse_latest_log(path: &Path) -> std::io::Result<LogAnalysis> {
    let content = std::fs::read_to_string(path)?;
    Ok(analyze_log_content(&content))
}

pub fn analyze_log_content(content: &str) -> LogAnalysis {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut suspected_mods = Vec::new();
    let mut crash_type = None;

    for line in content.lines() {
        if line.contains("java.lang.OutOfMemoryError") {
            crash_type = Some(CrashType::OutOfMemory);
            errors.push(LogError {
                message: "Java ran out of memory".to_string(),
                source_mod: None,
                category: ErrorCategory::OutOfMemory,
            });
        } else if line.contains("java.lang.NoClassDefFoundError")
            || line.contains("java.lang.ClassNotFoundException")
        {
            let class_name = extract_class_from_line(line);
            let mod_name = guess_mod_from_class(&class_name);
            errors.push(LogError {
                message: format!("Missing class: {}", class_name),
                source_mod: mod_name.clone(),
                category: ErrorCategory::ClassNotFound,
            });
            if let Some(name) = mod_name {
                add_suspected_mod(
                    &mut suspected_mods,
                    &name,
                    "Referenced class not found",
                    Confidence::Medium,
                );
            }
        } else if line.contains("Mixin apply failed") || line.contains("MixinApplyError") {
            let mod_name = extract_mod_from_mixin_error(line);
            errors.push(LogError {
                message: line.trim().to_string(),
                source_mod: mod_name.clone(),
                category: ErrorCategory::ModConflict,
            });
            if let Some(name) = mod_name {
                add_suspected_mod(
                    &mut suspected_mods,
                    &name,
                    "Mixin conflict",
                    Confidence::High,
                );
                if crash_type.is_none() {
                    crash_type = Some(CrashType::ModIncompatibility);
                }
            }
        } else if line.contains("Missing or unsupported mandatory dependencies") {
            crash_type = Some(CrashType::MissingDependency);
            errors.push(LogError {
                message: line.trim().to_string(),
                source_mod: None,
                category: ErrorCategory::MissingDependency,
            });
        } else if line.contains("GLFW error") || line.contains("OpenGL") && line.contains("error") {
            crash_type = Some(CrashType::OpenGlFailure);
            errors.push(LogError {
                message: line.trim().to_string(),
                source_mod: None,
                category: ErrorCategory::OpenGl,
            });
        } else if line.contains("[ERROR]") || line.contains("/ERROR]") {
            if let Some(mod_name) = extract_mod_from_log_line(line) {
                add_suspected_mod(
                    &mut suspected_mods,
                    &mod_name,
                    "Logged error during startup",
                    Confidence::Low,
                );
            }
            errors.push(LogError {
                message: line.trim().to_string(),
                source_mod: extract_mod_from_log_line(line),
                category: ErrorCategory::Generic,
            });
        } else if line.contains("[WARN]") || line.contains("/WARN]") {
            warnings.push(line.trim().to_string());
        }
    }

    LogAnalysis {
        errors,
        warnings,
        suspected_mods,
        crash_type,
    }
}

fn extract_section<'a>(content: &'a str, header: &str) -> Option<&'a str> {
    let start = content.find(header)?;
    let after_header = &content[start + header.len()..];
    let end = after_header.find('\n').unwrap_or(after_header.len());
    Some(&after_header[..end])
}

fn extract_exception(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if (trimmed.contains("Exception") || trimmed.contains("Error"))
            && trimmed.contains(':')
            && !trimmed.starts_with("--")
            && !trimmed.starts_with('#')
        {
            return trimmed.to_string();
        }
    }
    String::new()
}

fn extract_stack_trace(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|line| line.trim_start().starts_with("at "))
        .take(20)
        .map(|l| l.trim().to_string())
        .collect()
}

fn extract_involved_mods(content: &str) -> Vec<String> {
    let mut mods = Vec::new();
    let mut in_mods_section = false;

    for line in content.lines() {
        if line.contains("Mod List:") || line.contains("-- MOD") {
            in_mods_section = true;
            continue;
        }
        if in_mods_section {
            if line.trim().is_empty() || line.starts_with("--") {
                in_mods_section = false;
                continue;
            }
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                if let Some(mod_name) = extract_mod_name_from_list_line(trimmed) {
                    mods.push(mod_name);
                }
            }
        }
    }

    mods
}

fn extract_mod_name_from_list_line(line: &str) -> Option<String> {
    // Fabric format: "  modid x.y.z   modname.jar"
    // Forge format: "  modid (modfile.jar)  modname x.y.z"
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    Some(parts[0].to_string())
}

fn extract_system_details(content: &str) -> SystemDetails {
    let mut details = SystemDetails::default();
    let mut in_system = false;

    for line in content.lines() {
        if line.contains("System Details") || line.contains("-- System Details --") {
            in_system = true;
            continue;
        }
        if !in_system {
            continue;
        }
        if line.starts_with("--") && !line.contains("System Details") {
            break;
        }

        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("Minecraft Version:") {
            details.minecraft_version = Some(val.trim().to_string());
        } else if let Some(val) = trimmed.strip_prefix("Java Version:") {
            details.java_version = Some(val.trim().to_string());
        } else if let Some(val) = trimmed.strip_prefix("Operating System:") {
            details.operating_system = Some(val.trim().to_string());
        } else if trimmed.contains("Fabric Loader") || trimmed.contains("FabricLoader") {
            details.mod_loader = Some("fabric".to_string());
        } else if trimmed.contains("Forge") && trimmed.contains("Version") {
            details.mod_loader = Some("forge".to_string());
        }
    }

    details
}

fn extract_class_from_line(line: &str) -> String {
    if let Some(idx) = line.rfind(':') {
        line[idx + 1..].trim().to_string()
    } else {
        line.trim().to_string()
    }
}

fn guess_mod_from_class(class_name: &str) -> Option<String> {
    let lower = class_name.to_lowercase();
    let parts: Vec<&str> = lower.split('.').collect();

    let skip_packages = [
        "com",
        "net",
        "org",
        "io",
        "me",
        "dev",
        "gg",
        "cc",
        "example",
        "test",
        "mojang",
        "minecraft",
        "java",
        "sun",
        "javax",
        "google",
        "apache",
        "slf4j",
        "log4j",
        "gson",
        "guava",
    ];

    let mc_internal = [
        "client",
        "server",
        "world",
        "nbt",
        "util",
        "block",
        "item",
        "entity",
        "render",
        "resources",
        "network",
        "commands",
    ];

    for &part in parts.iter() {
        if skip_packages.contains(&part) || mc_internal.contains(&part) {
            continue;
        }
        if part.len() > 2 {
            return Some(part.to_string());
        }
    }
    None
}

fn extract_mod_from_mixin_error(line: &str) -> Option<String> {
    // Pattern: "from mod <modid>" or "(<modid>.mixins.json)"
    if let Some(idx) = line.find("from mod ") {
        let after = &line[idx + 9..];
        let end = after
            .find(|c: char| c.is_whitespace() || c == ')' || c == ']')
            .unwrap_or(after.len());
        return Some(after[..end].to_string());
    }
    if let Some(idx) = line.find(".mixins.json") {
        let before = &line[..idx];
        let start = before
            .rfind(|c: char| c == '(' || c == ' ' || c == '/')
            .map(|i| i + 1)
            .unwrap_or(0);
        return Some(before[start..].to_string());
    }
    None
}

fn extract_mod_from_log_line(line: &str) -> Option<String> {
    // Pattern: "[modname/ERROR]" or "[modname]: ERROR"
    if let Some(start) = line.find('[') {
        if let Some(end) = line[start..].find(']') {
            let bracket_content = &line[start + 1..start + end];
            let parts: Vec<&str> = bracket_content.split('/').collect();
            if parts.len() >= 2 {
                let candidate = parts[0].trim();
                if !candidate.is_empty()
                    && candidate != "main"
                    && candidate != "Server thread"
                    && candidate != "Render thread"
                {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    None
}

fn add_suspected_mod(
    mods: &mut Vec<SuspectedMod>,
    name: &str,
    reason: &str,
    confidence: Confidence,
) {
    if let Some(existing) = mods.iter_mut().find(|m| m.name == name) {
        if confidence_value(&confidence) > confidence_value(&existing.confidence) {
            existing.confidence = confidence;
            existing.reason = reason.to_string();
        }
    } else {
        mods.push(SuspectedMod {
            name: name.to_string(),
            file_name: None,
            reason: reason.to_string(),
            confidence,
        });
    }
}

fn confidence_value(c: &Confidence) -> u8 {
    match c {
        Confidence::Low => 1,
        Confidence::Medium => 2,
        Confidence::High => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_oom_from_log() {
        let log = "[12:00:00] [main/ERROR]: java.lang.OutOfMemoryError: Java heap space\n";
        let analysis = analyze_log_content(log);
        assert_eq!(analysis.crash_type, Some(CrashType::OutOfMemory));
        assert_eq!(analysis.errors.len(), 1);
        assert_eq!(analysis.errors[0].category, ErrorCategory::OutOfMemory);
    }

    #[test]
    fn parse_class_not_found() {
        let log = "[12:00:00] [main/ERROR]: java.lang.ClassNotFoundException: com.example.sodium.SodiumMod\n";
        let analysis = analyze_log_content(log);
        assert_eq!(analysis.errors.len(), 1);
        assert_eq!(analysis.errors[0].category, ErrorCategory::ClassNotFound);
        assert_eq!(analysis.suspected_mods.len(), 1);
        assert_eq!(analysis.suspected_mods[0].name, "sodium");
    }

    #[test]
    fn parse_mixin_error() {
        let log = "[12:00:00] [main/ERROR]: Mixin apply failed from mod iris (iris.mixins.json)\n";
        let analysis = analyze_log_content(log);
        assert_eq!(analysis.errors.len(), 1);
        assert_eq!(analysis.errors[0].category, ErrorCategory::ModConflict);
        assert_eq!(analysis.suspected_mods.len(), 1);
        assert_eq!(analysis.suspected_mods[0].name, "iris");
        assert_eq!(analysis.suspected_mods[0].confidence, Confidence::High);
    }

    #[test]
    fn parse_missing_dependency() {
        let log =
            "[12:00:00] [main/ERROR]: Missing or unsupported mandatory dependencies: fabric-api\n";
        let analysis = analyze_log_content(log);
        assert_eq!(analysis.crash_type, Some(CrashType::MissingDependency));
    }

    #[test]
    fn parse_opengl_error() {
        let log = "[12:00:00] [main/ERROR]: GLFW error 65543: GLX: Failed to create context\n";
        let analysis = analyze_log_content(log);
        assert_eq!(analysis.crash_type, Some(CrashType::OpenGlFailure));
    }

    #[test]
    fn extract_exception_from_crash() {
        let content = "---- Minecraft Crash Report ----\n\
            Description: Initializing game\n\n\
            java.lang.RuntimeException: Mod loading failed\n\
            \tat net.fabricmc.loader.impl.FabricLoaderImpl.load(FabricLoaderImpl.java:185)\n";
        let report = parse_crash_report_content(content, Path::new("crash.txt"));
        assert!(report.exception.contains("RuntimeException"));
        assert_eq!(report.description, "Initializing game");
    }

    #[test]
    fn extract_system_details_from_crash() {
        let content = "-- System Details --\n\
            Minecraft Version: 1.20.4\n\
            Java Version: 17.0.9\n\
            Operating System: Linux amd64\n\
            Fabric Loader: 0.15.3\n\
            --\n";
        let report = parse_crash_report_content(content, Path::new("crash.txt"));
        assert_eq!(
            report.system_details.minecraft_version,
            Some("1.20.4".to_string())
        );
        assert_eq!(
            report.system_details.java_version,
            Some("17.0.9".to_string())
        );
        assert_eq!(report.system_details.mod_loader, Some("fabric".to_string()));
    }

    #[test]
    fn guess_mod_from_class_name() {
        assert_eq!(
            guess_mod_from_class("com.example.sodium.SodiumMod"),
            Some("sodium".to_string())
        );
        assert_eq!(guess_mod_from_class("net.minecraft.client.Minecraft"), None);
    }

    #[test]
    fn extract_mixin_mod_name() {
        assert_eq!(
            extract_mod_from_mixin_error("Mixin apply failed from mod iris"),
            Some("iris".to_string())
        );
        assert_eq!(
            extract_mod_from_mixin_error("(sodium.mixins.json): target mixin"),
            Some("sodium".to_string())
        );
    }

    #[test]
    fn empty_log_produces_no_errors() {
        let analysis = analyze_log_content("");
        assert!(analysis.errors.is_empty());
        assert!(analysis.warnings.is_empty());
        assert!(analysis.suspected_mods.is_empty());
        assert_eq!(analysis.crash_type, None);
    }

    #[test]
    fn multiple_errors_accumulate() {
        let log = "[12:00:00] [main/ERROR]: java.lang.OutOfMemoryError\n\
                   [12:00:01] [main/ERROR]: java.lang.ClassNotFoundException: com.test.modx.Main\n";
        let analysis = analyze_log_content(log);
        assert_eq!(analysis.errors.len(), 2);
        assert_eq!(analysis.crash_type, Some(CrashType::OutOfMemory));
        assert_eq!(analysis.suspected_mods.len(), 1);
        assert_eq!(analysis.suspected_mods[0].name, "modx");
    }
}
