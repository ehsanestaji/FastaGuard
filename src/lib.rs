pub mod cli;
pub mod contract;
pub mod findings;
pub mod gate;
pub mod metrics;
pub mod models;
pub mod parser;
pub mod profile;
pub mod readiness;
pub mod report;
pub mod stats;

use anyhow::Result;
use cli::Cli;
use std::time::Instant;

pub fn run(cli: Cli) -> Result<i32> {
    if cli.schema {
        println!("{}", contract::schema_json().trim_end());
        return Ok(0);
    }
    if cli.finding_catalog {
        println!("{}", contract::finding_catalog_json().trim_end());
        return Ok(0);
    }
    if let Some(finding_id) = &cli.explain_finding {
        println!("{}", contract::explain_finding_json(finding_id)?);
        return Ok(0);
    }

    let config = cli.to_run_config()?;
    let run_started = Instant::now();
    let profile = profile::ProfileConfig::assembly(config.thresholds);
    let metrics = match metrics::AssemblyMetrics::from_path(&config.input, &profile) {
        Ok(metrics) => metrics,
        Err(error) if parser::is_structural_fasta_error(&error) => {
            let output = models::FastaguardReport::from_invalid_fasta(
                config.clone(),
                &profile,
                error.to_string(),
                measured_duration_ms(&config, run_started),
            )?;
            report::write_all(&output, &config.outputs)?;
            return Ok(output.exit_code());
        }
        Err(error) => return Err(error),
    };
    let analysis = findings::analyze(&metrics, &profile, &config.rules);
    let duration_ms = measured_duration_ms(&config, run_started);
    let output = models::FastaguardReport::from_analysis(
        config.clone(),
        &profile,
        metrics,
        analysis,
        duration_ms,
    )?;
    report::write_all(&output, &config.outputs)?;
    Ok(output.exit_code())
}

fn measured_duration_ms(config: &cli::RunConfig, started: Instant) -> u64 {
    if config.provenance_timestamp_override.is_some() {
        return 0;
    }

    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{OutputPaths, RuleConfig, RunConfig};
    use crate::gate::GateMode;
    use crate::profile::ThresholdOverrides;
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn measured_duration_uses_elapsed_time_without_fixture_override() {
        let config = test_config(None);
        let started = Instant::now()
            .checked_sub(Duration::from_millis(5))
            .unwrap();

        assert!(measured_duration_ms(&config, started) >= 5);
    }

    #[test]
    fn measured_duration_is_stable_for_fixture_timestamp_override() {
        let config = test_config(Some("2026-05-23T00:00:00Z".to_string()));
        let started = Instant::now()
            .checked_sub(Duration::from_millis(5))
            .unwrap();

        assert_eq!(measured_duration_ms(&config, started), 0);
    }

    fn test_config(provenance_timestamp_override: Option<String>) -> RunConfig {
        RunConfig {
            input: PathBuf::from("input.fa"),
            profile: "assembly".to_string(),
            gate_mode: GateMode::None,
            outputs: OutputPaths {
                html: PathBuf::from("fastaguard_report.html"),
                json: PathBuf::from("fastaguard.json"),
                tsv: PathBuf::from("fastaguard.tsv"),
                multiqc: PathBuf::from("fastaguard_mqc.json"),
            },
            rules: RuleConfig {
                fail_on: BTreeSet::new(),
            },
            thresholds: ThresholdOverrides {
                max_n_rate: None,
                min_contig_length: None,
            },
            threads: 1,
            command: "fastaguard input.fa".to_string(),
            started_at: "2026-05-23T00:00:00Z".to_string(),
            provenance_timestamp_override,
        }
    }
}
