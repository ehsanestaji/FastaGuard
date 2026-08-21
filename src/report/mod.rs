pub mod compare_html;
pub mod compare_multiqc;
pub mod compare_tsv;
pub mod html;
pub mod json;
pub mod multiqc;
pub mod tsv;

use anyhow::{anyhow, Context, Result};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tempfile::NamedTempFile;

use crate::cli::OutputPaths;
use crate::models::{CompareReport, FastaguardReport};

pub fn write_all(report: &FastaguardReport, outputs: &OutputPaths) -> Result<()> {
    validate_output_paths(outputs)?;

    let write_json = |path: &Path| json::write(report, path);
    let write_tsv = |path: &Path| tsv::write(report, path);
    let write_multiqc = |path: &Path| multiqc::write(report, path);
    let write_html = |path: &Path| html::write(report, path);
    write_staged_set(
        &[
            (&outputs.json, &write_json),
            (&outputs.tsv, &write_tsv),
            (&outputs.multiqc, &write_multiqc),
            (&outputs.html, &write_html),
        ],
        outputs.allow_overwrite,
    )
}

pub fn write_compare_all(report: &CompareReport, outputs: &OutputPaths) -> Result<()> {
    validate_output_paths(outputs)?;

    let write_json = |path: &Path| json::write_compare(report, path);
    let write_tsv = |path: &Path| compare_tsv::write(report, path);
    let write_multiqc = |path: &Path| compare_multiqc::write(report, path);
    let write_html = |path: &Path| compare_html::write(report, path);
    write_staged_set(
        &[
            (&outputs.json, &write_json),
            (&outputs.tsv, &write_tsv),
            (&outputs.multiqc, &write_multiqc),
            (&outputs.html, &write_html),
        ],
        outputs.allow_overwrite,
    )
}

type StagedSerializer<'a> = (&'a Path, &'a dyn Fn(&Path) -> Result<()>);

fn write_staged_set(serializers: &[StagedSerializer<'_>], allow_overwrite: bool) -> Result<()> {
    let mut staged = Vec::with_capacity(serializers.len());

    for (final_path, serializer) in serializers {
        let parent = final_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut temporary = NamedTempFile::new_in(parent).with_context(|| {
            format!(
                "failed to create staged report for {}",
                final_path.display()
            )
        })?;
        serializer(temporary.path())?;
        temporary.as_file_mut().flush().with_context(|| {
            format!("failed to flush staged report for {}", final_path.display())
        })?;
        staged.push((temporary, *final_path));
    }

    // Publication is sequential: each file is staged before its rename, but the
    // complete set of final filenames is not atomic as a unit.
    for (temporary, final_path) in staged {
        if allow_overwrite {
            temporary
                .persist(final_path)
                .map_err(|error| error.error)
                .with_context(|| format!("failed to publish report {}", final_path.display()))?;
        } else {
            temporary.persist_noclobber(final_path).map_err(|error| {
                if error.error.kind() == std::io::ErrorKind::AlreadyExists {
                    anyhow!(
                        "output path {} already exists; use --force to replace it",
                        final_path.display()
                    )
                } else {
                    anyhow!(error.error).context(format!(
                        "failed to publish report {} without overwriting an existing entry",
                        final_path.display()
                    ))
                }
            })?;
        }
    }
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

    for path in paths {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_dir() {
                    return Err(anyhow!(
                        "output path {} is a directory, not a file",
                        path.display()
                    ));
                }
                if !outputs.allow_overwrite {
                    return Err(anyhow!(
                        "output path {} already exists; use --force to replace it",
                        path.display()
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(anyhow!(
                    "failed to check output path {}: {}",
                    path.display(),
                    error
                ));
            }
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
    fn staged_serializer_error_removes_temporary_files_without_publishing() {
        let temp_dir = TempDir::new().unwrap();
        let first_final = temp_dir.path().join("first.json");
        let second_final = temp_dir.path().join("second.tsv");
        let first_serializer = |path: &Path| -> Result<()> {
            fs::write(path, "complete temporary report")?;
            Ok(())
        };
        let failing_serializer = |path: &Path| -> Result<()> {
            fs::write(path, "incomplete temporary report")?;
            Err(anyhow!("injected serializer failure"))
        };
        let serializers: [StagedSerializer<'_>; 2] = [
            (&first_final, &first_serializer),
            (&second_final, &failing_serializer),
        ];

        let error = write_staged_set(&serializers, false).unwrap_err();

        assert!(error.to_string().contains("injected serializer failure"));
        assert!(!first_final.exists());
        assert!(!second_final.exists());
        assert_eq!(fs::read_dir(temp_dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn no_clobber_publication_rejects_collision_created_after_preflight() {
        let temp_dir = TempDir::new().unwrap();
        let first_final = temp_dir.path().join("first.json");
        let second_final = temp_dir.path().join("second.tsv");
        let first_serializer = |path: &Path| -> Result<()> {
            fs::write(path, "first staged report")?;
            Ok(())
        };
        let second_serializer = |path: &Path| -> Result<()> {
            fs::write(&first_final, "concurrent writer")?;
            fs::write(path, "second staged report")?;
            Ok(())
        };
        let serializers: [StagedSerializer<'_>; 2] = [
            (&first_final, &first_serializer),
            (&second_final, &second_serializer),
        ];

        let error = write_staged_set(&serializers, false).unwrap_err();

        assert!(error.to_string().contains("already exists"), "{error:#}");
        assert_eq!(
            fs::read_to_string(&first_final).unwrap(),
            "concurrent writer"
        );
        assert!(!second_final.exists());
    }

    #[cfg(unix)]
    #[test]
    fn no_clobber_rejects_dangling_symlink_output_entry() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let dangling_target = temp_dir.path().join("missing-target");
        let dangling_output = temp_dir.path().join("report.json");
        symlink(&dangling_target, &dangling_output).unwrap();
        let outputs = OutputPaths {
            html: temp_dir.path().join("report.html"),
            json: dangling_output.clone(),
            tsv: temp_dir.path().join("report.tsv"),
            multiqc: temp_dir.path().join("multiqc.json"),
            allow_overwrite: false,
        };

        let error = write_all(&test_report(), &outputs).unwrap_err();

        assert!(error.to_string().contains("already exists"), "{error:#}");
        assert!(fs::symlink_metadata(&dangling_output)
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(!dangling_target.exists());
        assert!(!outputs.html.exists());
        assert!(!outputs.tsv.exists());
        assert!(!outputs.multiqc.exists());
    }

    #[test]
    fn duplicate_output_paths_error_before_creating_files() {
        let temp_dir = TempDir::new().unwrap();
        let duplicate = temp_dir.path().join("report.json");
        let outputs = OutputPaths {
            html: temp_dir.path().join("report.html"),
            json: duplicate.clone(),
            tsv: duplicate.clone(),
            multiqc: temp_dir.path().join("multiqc.json"),
            allow_overwrite: false,
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
            allow_overwrite: false,
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
            allow_overwrite: false,
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
    fn directory_output_target_errors_even_when_overwrite_is_allowed() {
        let temp_dir = TempDir::new().unwrap();
        let directory_target = temp_dir.path().join("report.json");
        fs::create_dir(&directory_target).unwrap();
        let outputs = OutputPaths {
            html: temp_dir.path().join("report.html"),
            json: directory_target,
            tsv: temp_dir.path().join("report.tsv"),
            multiqc: temp_dir.path().join("multiqc.json"),
            allow_overwrite: true,
        };

        let error = write_all(&test_report(), &outputs).unwrap_err();

        assert!(error.to_string().contains("is a directory, not a file"));
        assert!(!outputs.html.exists());
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
            allow_overwrite: false,
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
            allow_overwrite: false,
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
            allow_overwrite: false,
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
                submission_target: None,
                submission_policy: None,
                status: VerdictStatus::Pass,
                can_continue: true,
                blocking_findings: Vec::new(),
                advisory_findings: Vec::new(),
                fail_on: Vec::new(),
            },
            readiness: crate::readiness::build_readiness(
                VerdictStatus::Pass,
                &[],
                &[],
                crate::readiness::ReadinessScope::Single,
                None,
            ),
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
                submission_target: None,
                submission_policy: None,
                threads: 1,
                fail_on: Vec::new(),
                thresholds: ProvenanceThresholds {
                    high_n_sequence_fraction: 0.2,
                    high_global_n_fraction: 0.05,
                    min_contig_length: 200,
                    max_gap_run: 100,
                    gc_outlier_zscore: 3.0,
                    expected_size_bases: None,
                    expected_size_tolerance: None,
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
                duplicate_first_token_id_count: 0,
                duplicate_sequence_count: 0,
                unsafe_id_count: 0,
                long_header_count: 0,
                reserved_header_char_count: 0,
                invalid_sequence_count: 0,
                high_n_sequence_count: 0,
                tiny_contig_count: 0,
                terminal_n_sequence_count: 0,
                repeated_gap_pattern_sequence_count: 0,
                max_gap_run: 1,
                ungapped_total_length: 100,
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
