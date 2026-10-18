//! The `unam` command-line interface.

/// Write formatted text to stdout. A closed pipe (`unam ... | head`) ends the
/// process quietly with exit 0, the usual behavior for CLI tools; any other
/// write error is reported and exits 1. Never panics, unlike `print!`.
fn write_stdout(args: std::fmt::Arguments<'_>) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    if let Err(e) = out.write_fmt(args) {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        eprintln!("error: cannot write to stdout: {e}");
        std::process::exit(1);
    }
}

macro_rules! out {
    ($($arg:tt)*) => { $crate::write_stdout(format_args!($($arg)*)) };
}

macro_rules! outln {
    ($($arg:tt)*) => { $crate::write_stdout(format_args!("{}\n", format_args!($($arg)*))) };
}

mod cli;
mod output;
mod tui;

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
