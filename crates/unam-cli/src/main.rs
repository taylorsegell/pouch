//! The `unam` command-line interface.

mod cli;
mod output;

use clap::Parser;
use std::process::ExitCode;
use std::str::FromStr;
use tracing_subscriber::filter::LevelFilter;

fn log_level(verbose: u8, quiet: bool) -> LevelFilter {
    if let Some(level) = std::env::var("UNAM_LOG")
        .ok()
        .and_then(|v| LevelFilter::from_str(v.trim()).ok())
    {
        return level;
    }
    if quiet {
        return LevelFilter::ERROR;
    }
    match verbose {
        0 => LevelFilter::WARN,
        1 => LevelFilter::INFO,
        _ => LevelFilter::DEBUG,
    }
}

fn main() -> ExitCode {
    let args = cli::Cli::parse();
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(log_level(args.verbose, args.quiet))
        .init();
    let code = cli::run(args);
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
