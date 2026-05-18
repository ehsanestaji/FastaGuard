pub mod html;
pub mod json;
pub mod multiqc;
pub mod tsv;

use anyhow::{anyhow, Context, Result};
use std::collections::BTreeSet;

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
        let normalized = path.to_string_lossy().into_owned();
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
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::models::{
        Artifacts, FastaguardReport, InputInfo, Summary, ToolInfo, Verdict, VerdictStatus,
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
            findings: Vec::new(),
            artifacts: Artifacts {
                html: "fastaguard_report.html".to_string(),
                tsv: "fastaguard.tsv".to_string(),
                multiqc: "fastaguard_multiqc.json".to_string(),
            },
        }
    }
}
