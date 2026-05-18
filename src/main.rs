use clap::Parser;
use fastaguard::cli::Cli;

fn main() {
    let cli = Cli::parse();
    match fastaguard::run(cli) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("fastaguard error: {error}");
            std::process::exit(3);
        }
    }
}
