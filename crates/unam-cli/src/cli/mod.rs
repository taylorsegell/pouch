//! Argument parsing and dispatch. Command modules are thin: they call
//! `unam-core` and build the JSON data plus the human text.

mod search;
mod show;
mod source;

use crate::output::{self, ErrorInfo};
use clap::{CommandFactory, Parser, Subcommand};
use serde_json::Value;
use unam_core::catalog::{Catalog, FlatDiagnostic, LoadOptions};
use unam_core::config::Config;
use unam_core::paths::Dirs;

#[derive(Debug, Parser)]
#[command(
    name = "unam",
    version,
    about = "Activate the right agent skills per project, unambiguously."
)]
pub struct Cli {
    /// Print one JSON object on stdout.
    #[arg(long, global = true)]
    pub json: bool,
    /// Neither read nor write the catalog cache.
    #[arg(long, global = true)]
    pub no_cache: bool,
    /// More log output on stderr (repeatable).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Less output: errors only in logs, no notes.
    #[arg(short, long, global = true)]
    pub quiet: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage skill sources.
    Source {
        #[command(subcommand)]
        cmd: SourceCmd,
    },
    /// Search skills across sources.
    Search {
        /// Search words (empty lists everything).
        query: String,
        /// Category path prefix filter.
        #[arg(long)]
        category: Option<String>,
        /// Only this source alias.
        #[arg(long)]
        source: Option<String>,
    },
    /// Show one skill: metadata, supporting files, absolute path.
    Show {
        /// `alias:path/to/skill`, `alias:name`, or a bare name.
        reference: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum SourceCmd {
    /// Register a source directory and scan it.
    Add { alias: String, path: String },
    /// List registered sources.
    List,
    /// Unregister a source.
    Rm {
        alias: String,
        /// Remove even when project selections reference it.
        #[arg(long)]
        force: bool,
    },
}

/// Shared state for a command run.
pub struct Ctx {
    pub dirs: Dirs,
    pub config: Config,
    /// Config-level diagnostics (precedence ties).
    pub config_diags: Vec<FlatDiagnostic>,
    pub options: LoadOptions,
}

impl Ctx {
    fn load(no_cache: bool) -> anyhow::Result<Ctx> {
        let dirs = Dirs::from_process_env()?;
        let (config, diags) = Config::load_or_create(&dirs)?;
        tracing::debug!(config = %dirs.config_file().display(), sources = config.sources.len(), "config loaded");
        Ok(Ctx {
            dirs,
            config,
            config_diags: diags.into_iter().map(FlatDiagnostic::global).collect(),
            options: LoadOptions::product(!no_cache),
        })
    }

    pub fn catalog(&self) -> anyhow::Result<Catalog> {
        Ok(Catalog::load(
            &self.config.sources,
            &self.dirs,
            self.options,
        )?)
    }
}

/// A successful command.
pub struct Done {
    pub data: Value,
    /// Diagnostics for the envelope `warnings` (filtered to warning/error).
    pub warnings: Vec<FlatDiagnostic>,
    /// Diagnostics printed to stderr in human mode (may include notes).
    pub human_diags: Vec<FlatDiagnostic>,
    /// Human text for stdout.
    pub human: String,
}

/// A failed command, with any diagnostics gathered before the failure.
pub struct Failed {
    pub error: anyhow::Error,
    pub warnings: Vec<FlatDiagnostic>,
}

impl From<anyhow::Error> for Failed {
    fn from(error: anyhow::Error) -> Self {
        Failed {
            error,
            warnings: Vec::new(),
        }
    }
}

impl From<unam_core::error::Error> for Failed {
    fn from(e: unam_core::error::Error) -> Self {
        Failed::from(anyhow::Error::new(e))
    }
}

pub type CmdResult = Result<Done, Failed>;

fn usage_text() -> String {
    let mut text = Cli::command().render_help().to_string();
    text.push_str("\nstatus arrives in a later milestone\n");
    text
}

/// Run the parsed command; returns the process exit code.
pub fn run(cli: Cli) -> i32 {
    let Some(command) = cli.command else {
        print!("{}", usage_text());
        return 0;
    };
    let (name, result) = match Ctx::load(cli.no_cache) {
        Err(e) => (command_name(&command), Err(Failed::from(e))),
        Ok(ctx) => {
            let name = command_name(&command);
            let r = match command {
                Command::Source { cmd } => match cmd {
                    SourceCmd::Add { alias, path } => source::add(&ctx, &alias, &path),
                    SourceCmd::List => source::list(&ctx),
                    SourceCmd::Rm { alias, force } => source::rm(&ctx, &alias, force),
                },
                Command::Search {
                    query,
                    category,
                    source,
                } => search::run(&ctx, &query, category.as_deref(), source.as_deref()),
                Command::Show { reference } => show::run(&ctx, &reference),
            };
            (name, r)
        }
    };
    render(name, result, cli.json, cli.quiet)
}

fn command_name(c: &Command) -> &'static str {
    match c {
        Command::Source { cmd } => match cmd {
            SourceCmd::Add { .. } => "source add",
            SourceCmd::List => "source list",
            SourceCmd::Rm { .. } => "source rm",
        },
        Command::Search { .. } => "search",
        Command::Show { .. } => "show",
    }
}

fn print_json(v: &Value) {
    match serde_json::to_string_pretty(v) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("error: cannot serialize output: {e}"),
    }
}

fn render(command: &str, result: CmdResult, json: bool, quiet: bool) -> i32 {
    match result {
        Ok(done) => {
            if json {
                print_json(&output::envelope(
                    true,
                    command,
                    done.data,
                    &done.warnings,
                    &[],
                ));
            } else {
                print!("{}", done.human);
                for d in &done.human_diags {
                    if quiet && d.diagnostic.severity == unam_core::error::Severity::Info {
                        continue;
                    }
                    eprintln!("{}", output::diag_line(d));
                }
            }
            0
        }
        Err(failed) => {
            let info = ErrorInfo::from_anyhow(&failed.error);
            if json {
                print_json(&output::envelope(
                    false,
                    command,
                    serde_json::json!({}),
                    &failed.warnings,
                    std::slice::from_ref(&info),
                ));
            } else {
                for d in &failed.warnings {
                    eprintln!("{}", output::diag_line(d));
                }
                eprint!("{}", info.human());
            }
            info.exit_code
        }
    }
}
