use clap::Parser;
use fastaguard::cli::Cli;

fn main() {
    let cli = Cli::parse();
    if let Err(error) = cli.to_run_config() {
        eprintln!("fastaguard error: {error}");
        std::process::exit(3);
    }

    eprintln!("fastaguard implementation is not wired yet");
    std::process::exit(3);
}
