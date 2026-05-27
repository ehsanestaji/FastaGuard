pub mod html;
pub mod json;
pub mod multiqc;
pub mod tsv;

use anyhow::{anyhow, Context, Result};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use crate::cli::OutputPaths;
use crate::models::FastaguardReport;

pub fn write_all(report: &FastaguardReport, outputs: &OutputPaths) -> Result<()> {
    validate_output_paths(outputs)?;

    json::write(report, &outputs.json)?;
    tsv::write(report, &outputs.tsv)?;
    multiqc::write(report, &outputs.multiqc)?;
    html::write(report, &outputs.html)?;
    Ok(())
}

fn validate_output_paths(outputs: &OutputPaths) -> Result<()> {
    let paths = [&outputs.html, &outputs.json, &outputs.tsv, &outputs.multiqc];
    let mut seen_paths = BTreeSet::new();

    for path in paths {
        let normalized = normalize_output_path(path)?.to_string_lossy().into_owned();
        if !seen_paths.insert(normalized.clone()) {
            return Err(anyhow!("duplicate output paths: {}", normalized));
        }
    }

    for path in paths {
        let Some(parent) = path.parent() else {
            continue;
        };
        if parent.as_os_str().is_empty() {
            continue;
        }
        if !parent
            .try_exists()
            .with_context(|| format!("failed to check parent directory for {}", path.display()))?
        {
            return Err(anyhow!(
                "parent directory for output path {} does not exist: {}",
                path.display(),
                parent.display()
            ));
        }
        if !parent.is_dir() {
            return Err(anyhow!(
                "parent directory for output path {} is not a directory: {}",
                path.display(),
                parent.display()
            ));
        }
    }

    Ok(())
}

fn normalize_output_path(path: &Path) -> Result<PathBuf> {
    let anchored = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to resolve current directory for output path validation")?
            .join(path)
    };

    Ok(normalize_path_lexically(&anchored))
}

fn normalize_path_lexically(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match components.last() {
                Some(Component::Normal(_)) => {
                    components.pop();
                }
                Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                _ => components.push(component),
            },
            _ => components.push(component),
        }
    }

    let mut normalized = PathBuf::new();
    for component in components {
        normalized.push(component.as_os_str());
    }

    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Mutex;

    use tempfile::TempDir;

    use super::*;
    use crate::models::{
        empty_plots, Artifacts, FastaguardReport, GateDecision, InputInfo, MachineSummary,
        Provenance, ProvenanceThresholds, Scope, Summary, ToolInfo, Verdict, VerdictStatus,
    };

    #[test]
    fn duplicate_output_paths_error_before_creating_files() {
        let temp_dir = TempDir::new().unwrap();
        let duplicate = temp_dir.path().join("report.json");
        let outputs = OutputPaths {
            html: temp_dir.path().join("report.html"),
            json: duplicate.clone(),
            tsv: duplicate.clone(),
            multiqc: temp_dir.path().join("multiqc.json"),
        };

        let error = write_all(&test_report(), &outputs).unwrap_err();

        assert!(error.to_string().contains("duplicate output paths"));
        assert!(!outputs.html.exists());
        assert!(!outputs.json.exists());
        assert!(!outputs.multiqc.exists());
    }

    #[test]
    fn missing_parent_directory_errors_before_creating_earlier_artifacts() {
        let temp_dir = TempDir::new().unwrap();
        let missing_parent = temp_dir.path().join("missing");
        let outputs = OutputPaths {
            html: temp_dir.path().join("report.html"),
            json: temp_dir.path().join("report.json"),
            tsv: missing_parent.join("report.tsv"),
            multiqc: temp_dir.path().join("multiqc.json"),
        };

        let error = write_all(&test_report(), &outputs).unwrap_err();

        assert!(error.to_string().contains("parent directory"));
        assert!(error
            .to_string()
            .contains(&outputs.tsv.display().to_string()));
        assert!(!outputs.html.exists());
        assert!(!outputs.json.exists());
        assert!(!outputs.tsv.exists());
        assert!(!outputs.multiqc.exists());
    }

    #[test]
    fn file_parent_errors_before_creating_earlier_artifacts() {
        let temp_dir = TempDir::new().unwrap();
        let parent_file = temp_dir.path().join("parent-file");
        fs::write(&parent_file, "not a directory").unwrap();
        let outputs = OutputPaths {
            html: temp_dir.path().join("report.html"),
            json: temp_dir.path().join("report.json"),
            tsv: parent_file.join("report.tsv"),
            multiqc: temp_dir.path().join("multiqc.json"),
        };

        let error = write_all(&test_report(), &outputs).unwrap_err();

        assert!(error.to_string().contains("parent directory"));
        assert!(error
            .to_string()
            .contains(&outputs.tsv.display().to_string()));
        assert!(!outputs.html.exists());
        assert!(!outputs.json.exists());
        assert!(!outputs.tsv.exists());
        assert!(!outputs.multiqc.exists());
    }

    #[test]
    fn duplicate_output_paths_detect_equivalent_dot_relative_paths() {
        let _guard = current_dir_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let current_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        let outputs = OutputPaths {
            html: "report.html".into(),
            json: "report.json".into(),
            tsv: "./report.json".into(),
            multiqc: "multiqc.json".into(),
        };

        let result = write_all(&test_report(), &outputs);
        std::env::set_current_dir(current_dir).unwrap();

        let error = result.unwrap_err();
        assert!(error.to_string().contains("duplicate output paths"));
        assert!(!temp_dir.path().join("report.html").exists());
        assert!(!temp_dir.path().join("report.json").exists());
        assert!(!temp_dir.path().join("multiqc.json").exists());
    }

    #[test]
    fn duplicate_output_paths_detect_equivalent_parent_relative_paths() {
        let _guard = current_dir_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        let current_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        let outputs = OutputPaths {
            html: "report.html".into(),
            json: "report.json".into(),
            tsv: "subdir/../report.json".into(),
            multiqc: "multiqc.json".into(),
        };

        let result = write_all(&test_report(), &outputs);
        std::env::set_current_dir(current_dir).unwrap();

        let error = result.unwrap_err();
        assert!(error.to_string().contains("duplicate output paths"));
        assert!(!temp_dir.path().join("report.html").exists());
        assert!(!temp_dir.path().join("report.json").exists());
        assert!(!temp_dir.path().join("multiqc.json").exists());
    }

    #[test]
    fn duplicate_output_paths_detect_relative_and_absolute_aliases() {
        let _guard = current_dir_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let current_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        let absolute_duplicate = std::env::current_dir().unwrap().join("report.same");
        let outputs = OutputPaths {
            html: "report.html".into(),
            json: "report.same".into(),
            tsv: absolute_duplicate,
            multiqc: "multiqc.json".into(),
        };

        let result = write_all(&test_report(), &outputs);
        std::env::set_current_dir(current_dir).unwrap();

        let error = result.unwrap_err();
        assert!(error.to_string().contains("duplicate output paths"));
        assert!(!temp_dir.path().join("report.html").exists());
        assert!(!temp_dir.path().join("report.same").exists());
        assert!(!temp_dir.path().join("multiqc.json").exists());
    }

    fn current_dir_lock() -> &'static Mutex<()> {
        static LOCK: Mutex<()> = Mutex::new(());
        &LOCK
    }

    fn test_report() -> FastaguardReport {
        FastaguardReport {
            schema_version: "0.1.0".to_string(),
            tool: ToolInfo {
                name: "FastaGuard".to_string(),
                version: "0.1.0".to_string(),
            },
            input: InputInfo {
                path: "input.fa".to_string(),
                profile: "assembly".to_string(),
                compressed: false,
            },
            verdict: Verdict {
                status: VerdictStatus::Pass,
                reasons: Vec::new(),
            },
            gate: GateDecision {
                mode: "none".to_string(),
                status: VerdictStatus::Pass,
                blocking_findings: Vec::new(),
                advisory_findings: Vec::new(),
                fail_on: Vec::new(),
            },
            machine_summary: MachineSummary {
                verdict: VerdictStatus::Pass,
                safe_for_downstream: true,
                top_findings: Vec::new(),
                recommended_next_tools: Vec::new(),
                routing_hints: Vec::new(),
            },
            scope: Scope {
                level: "fasta_preflight".to_string(),
                can_conclude: Vec::new(),
                cannot_conclude: Vec::new(),
            },
            provenance: Provenance {
                profile: "assembly".to_string(),
                threads: 1,
                fail_on: Vec::new(),
                thresholds: ProvenanceThresholds {
                    high_n_sequence_fraction: 0.2,
                    high_global_n_fraction: 0.05,
                    min_contig_length: 200,
                    max_gap_run: 100,
                    gc_outlier_zscore: 3.0,
                },
                command: "fastaguard input.fa".to_string(),
                started_at: "2026-05-23T00:00:00Z".to_string(),
                completed_at: "2026-05-23T00:00:00Z".to_string(),
                duration_ms: 0,
                input_size_bytes: 100,
                input_sha256: "0".repeat(64),
            },
            summary: Summary {
                sequence_count: 2,
                total_length: 100,
                min_length: 40,
                max_length: 60,
                mean_length: 50.0,
                median_length: 50.0,
                n50: 60,
                n90: 40,
                l50: 1,
                l90: 2,
                gc_percent: 48.5,
                at_percent: 50.0,
                n_percent: 1.5,
                ambiguity_percent: 1.5,
                duplicate_id_count: 0,
                duplicate_sequence_count: 0,
                invalid_sequence_count: 0,
                high_n_sequence_count: 0,
                tiny_contig_count: 0,
                max_gap_run: 1,
            },
            plots: empty_plots(),
            findings: Vec::new(),
            artifacts: Artifacts {
                html: "fastaguard_report.html".to_string(),
                tsv: "fastaguard.tsv".to_string(),
                multiqc: "fastaguard_mqc.json".to_string(),
            },
        }
    }
}
