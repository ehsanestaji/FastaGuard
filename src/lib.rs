pub mod cli;
pub mod findings;
pub mod metrics;
pub mod models;
pub mod parser;
pub mod profile;
pub mod report;
pub mod stats;

use anyhow::Result;
use cli::Cli;

pub fn run(cli: Cli) -> Result<i32> {
    let config = cli.to_run_config()?;
    let profile = profile::ProfileConfig::assembly(config.thresholds);
    let metrics = match metrics::AssemblyMetrics::from_path(&config.input, &profile) {
        Ok(metrics) => metrics,
        Err(error) if parser::is_structural_fasta_error(&error) => {
            let output =
                models::FastaguardReport::from_invalid_fasta(config.clone(), error.to_string());
            report::write_all(&output, &config.outputs)?;
            return Ok(output.exit_code());
        }
        Err(error) => return Err(error),
    };
    let analysis = findings::analyze(&metrics, &profile, &config.rules);
    let output = models::FastaguardReport::from_analysis(config.clone(), metrics, analysis);
    report::write_all(&output, &config.outputs)?;
    Ok(output.exit_code())
}
