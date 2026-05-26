use std::path::{Path, PathBuf};

use super::parser::{
    Confidence, CrashReport, CrashType, LogAnalysis, SuspectedMod, analyze_log_content,
    parse_crash_report, parse_latest_log,
};

#[derive(Debug, Clone)]
pub struct CrashDiagnosis {
    pub crash_report: Option<CrashReport>,
    pub log_analysis: Option<LogAnalysis>,
    pub summary: String,
    pub suggestions: Vec<String>,
    pub suspected_mods: Vec<SuspectedMod>,
}

pub fn diagnose_crash(game_dir: &Path) -> CrashDiagnosis {
    let crash_report = find_latest_crash_report(game_dir).and_then(|p| parse_crash_report(&p).ok());

    let log_analysis = {
        let log_path = game_dir.join("logs").join("latest.log");
        if log_path.exists() {
            parse_latest_log(&log_path).ok()
        } else {
            None
        }
    };

    let mut suspected_mods = Vec::new();
    let mut suggestions = Vec::new();

    if let Some(report) = &crash_report {
        for mod_name in &report.involved_mods {
            add_mod_if_absent(
                &mut suspected_mods,
                mod_name,
                "Listed in crash report",
                Confidence::Medium,
            );
        }
    }

    if let Some(analysis) = &log_analysis {
        for suspect in &analysis.suspected_mods {
            add_mod_if_absent(
                &mut suspected_mods,
                &suspect.name,
                &suspect.reason,
                suspect.confidence.clone(),
            );
        }

        match analysis.crash_type {
            Some(CrashType::OutOfMemory) => {
                suggestions.push(
                    "Increase allocated RAM in instance settings (recommended: 4GB+)".to_string(),
                );
                suggestions
                    .push("Remove performance-heavy mods or reduce render distance".to_string());
            }
            Some(CrashType::MissingDependency) => {
                suggestions
                    .push("Install missing mod dependencies (check mod requirements)".to_string());
            }
            Some(CrashType::ModIncompatibility) => {
                suggestions
                    .push("Remove or update conflicting mods (see suspected mods)".to_string());
                suggestions
                    .push("Check if mods are compatible with your Minecraft version".to_string());
            }
            Some(CrashType::OpenGlFailure) => {
                suggestions.push("Update graphics drivers".to_string());
                suggestions.push("Try removing shader mods (Iris/OptiFine)".to_string());
            }
            Some(CrashType::JavaIncompatibility) => {
                suggestions.push(
                    "Switch to a compatible Java version for this Minecraft version".to_string(),
                );
            }
            Some(CrashType::CorruptedInstall) => {
                suggestions.push("Verify game files or reinstall the instance".to_string());
            }
            Some(CrashType::Unknown) | None => {}
        }
    }

    if suggestions.is_empty() && !suspected_mods.is_empty() {
        suggestions
            .push("Try removing the suspected mods one by one to identify the culprit".to_string());
    }

    let summary = build_summary(&crash_report, &log_analysis, &suspected_mods);

    CrashDiagnosis {
        crash_report,
        log_analysis,
        summary,
        suggestions,
        suspected_mods,
    }
}

pub fn analyze_game_output(output_lines: &[String]) -> LogAnalysis {
    let combined = output_lines.join("\n");
    analyze_log_content(&combined)
}

fn find_latest_crash_report(game_dir: &Path) -> Option<PathBuf> {
    let crash_dir = game_dir.join("crash-reports");
    if !crash_dir.exists() {
        return None;
    }

    std::fs::read_dir(&crash_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("crash-"))
        .max_by_key(|entry| entry.metadata().ok().and_then(|m| m.modified().ok()))
        .map(|entry| entry.path())
}

fn build_summary(
    crash_report: &Option<CrashReport>,
    log_analysis: &Option<LogAnalysis>,
    suspected_mods: &[SuspectedMod],
) -> String {
    let mut parts = Vec::new();

    if let Some(report) = crash_report {
        if !report.description.is_empty() {
            parts.push(format!("Crash: {}", report.description));
        }
        if !report.exception.is_empty() {
            parts.push(format!("Exception: {}", report.exception));
        }
    }

    if let Some(analysis) = log_analysis {
        if let Some(crash_type) = &analysis.crash_type {
            parts.push(format!("Type: {:?}", crash_type));
        }
        if !analysis.errors.is_empty() {
            parts.push(format!("{} error(s) found in log", analysis.errors.len()));
        }
    }

    if !suspected_mods.is_empty() {
        let high_confidence: Vec<&str> = suspected_mods
            .iter()
            .filter(|m| m.confidence == Confidence::High)
            .map(|m| m.name.as_str())
            .collect();
        if !high_confidence.is_empty() {
            parts.push(format!("Likely culprit(s): {}", high_confidence.join(", ")));
        }
    }

    if parts.is_empty() {
        "No crash information available".to_string()
    } else {
        parts.join(" | ")
    }
}

fn add_mod_if_absent(
    mods: &mut Vec<SuspectedMod>,
    name: &str,
    reason: &str,
    confidence: Confidence,
) {
    if !mods.iter().any(|m| m.name == name) {
        mods.push(SuspectedMod {
            name: name.to_string(),
            file_name: None,
            reason: reason.to_string(),
            confidence,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_game_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("logs")).unwrap();
        fs::create_dir_all(dir.path().join("crash-reports")).unwrap();
        dir
    }

    #[test]
    fn diagnose_empty_game_dir() {
        let dir = setup_game_dir();
        let diagnosis = diagnose_crash(dir.path());
        assert!(diagnosis.crash_report.is_none());
        assert!(diagnosis.log_analysis.is_none());
        assert!(diagnosis.suspected_mods.is_empty());
    }

    #[test]
    fn diagnose_oom_from_log() {
        let dir = setup_game_dir();
        fs::write(
            dir.path().join("logs/latest.log"),
            "[12:00:00] [main/ERROR]: java.lang.OutOfMemoryError: Java heap space\n",
        )
        .unwrap();

        let diagnosis = diagnose_crash(dir.path());
        assert!(diagnosis.log_analysis.is_some());
        assert!(!diagnosis.suggestions.is_empty());
        assert!(diagnosis.suggestions[0].contains("RAM"));
    }

    #[test]
    fn diagnose_with_crash_report() {
        let dir = setup_game_dir();
        let crash_content = "---- Minecraft Crash Report ----\n\
            Description: Rendering screen\n\n\
            java.lang.NullPointerException: Cannot invoke method\n\
            \tat com.example.badmod.Renderer.render(Renderer.java:42)\n\
            -- System Details --\n\
            Minecraft Version: 1.20.4\n";
        fs::write(
            dir.path().join("crash-reports/crash-2024-01-01.txt"),
            crash_content,
        )
        .unwrap();

        let diagnosis = diagnose_crash(dir.path());
        assert!(diagnosis.crash_report.is_some());
        let report = diagnosis.crash_report.unwrap();
        assert_eq!(report.description, "Rendering screen");
        assert!(report.exception.contains("NullPointerException"));
    }

    #[test]
    fn find_latest_crash_picks_newest() {
        let dir = setup_game_dir();
        let crash_dir = dir.path().join("crash-reports");
        fs::write(crash_dir.join("crash-2024-01-01.txt"), "old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(crash_dir.join("crash-2024-01-02.txt"), "new").unwrap();

        let latest = find_latest_crash_report(dir.path()).unwrap();
        assert!(latest.to_string_lossy().contains("crash-2024-01-02"));
    }

    #[test]
    fn analyze_game_output_detects_errors() {
        let lines = vec![
            "[12:00:00] [main/INFO]: Loading mods...".to_string(),
            "[12:00:01] [main/ERROR]: java.lang.OutOfMemoryError".to_string(),
        ];
        let analysis = analyze_game_output(&lines);
        assert_eq!(analysis.crash_type, Some(CrashType::OutOfMemory));
    }
}
