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
    let metrics = metrics::AssemblyMetrics::from_path(&config.input, &profile)?;
    let analysis = findings::analyze(&metrics, &profile, &config.rules);
    let output = models::FastaguardReport::from_analysis(config.clone(), metrics, analysis);
    report::write_all(&output, &config.outputs)?;
    Ok(output.exit_code())
}
