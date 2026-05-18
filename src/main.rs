use clap::Parser;
use fastaguard::cli::Cli;

fn main() {
    let _cli = Cli::parse();
    eprintln!("fastaguard implementation is not wired yet");
    std::process::exit(3);
}
