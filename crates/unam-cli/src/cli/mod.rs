//! Argument parsing and dispatch. Command modules are thin: they call
//! `unam-core` and build the JSON data plus the human text.

mod bundle;
mod change;
mod doctor;
mod search;
mod show;
mod source;
mod status;

use crate::output::{self, ErrorInfo, warnings_only};
use clap::{Args, CommandFactory, Parser, Subcommand};
use serde_json::Value;
use std::io::IsTerminal;
use std::path::PathBuf;
use unam_core::catalog::{Catalog, FlatDiagnostic, LoadOptions, SOURCE_AND_CONFIG};
use unam_core::config::Config;
use unam_core::paths::Dirs;
use unam_core::project::{ProjectRoot, TargetId};

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
    /// Use this directory as the project root (no search for `.git`).
    #[arg(long, global = true, value_name = "PATH")]
    pub project: Option<PathBuf>,
    /// The target to change: `claude-code` or `agents-neutral`.
    #[arg(long, global = true, value_name = "ID")]
    pub target: Option<String>,
    /// Apply without a prompt (the plan is still printed).
    #[arg(long, global = true)]
    pub yes: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Exactly one of `--replace` and `--add`.
#[derive(Debug, Args)]
#[group(required = true, multiple = false)]
pub struct UseMode {
    /// Make this bundle the whole selection (clears add and remove).
    #[arg(long)]
    pub replace: bool,
    /// Add this bundle to the selection.
    #[arg(long)]
    pub add: bool,
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
    /// Manage skill bundles.
    Bundle {
        #[command(subcommand)]
        cmd: BundleCmd,
    },
    /// Add skills to this project's selection and apply them.
    Add {
        /// Skill references.
        #[arg(required = true)]
        references: Vec<String>,
    },
    /// Remove skills from this project's selection and apply the change.
    Rm {
        /// Skill references.
        #[arg(required = true)]
        references: Vec<String>,
    },
    /// Use a bundle in this project (with --replace or --add) and apply it.
    Use {
        /// Bundle name or slug.
        bundle: String,
        #[command(flatten)]
        mode: UseMode,
    },
    /// Remove every skill unam placed for the target and clear its selection.
    Deactivate {
        /// Every target (the same as the default while one adapter exists).
        #[arg(long)]
        all: bool,
    },
    /// Finish an interrupted apply.
    Resume,
    /// Undo an interrupted apply, or restore the previous applied state.
    Rollback,
    /// Show the six-state table of this project's target.
    Status,
    /// Report where the destination diverges from the selection.
    Reconcile {
        /// Repair the divergences through the journaled executor.
        #[arg(long)]
        repair: bool,
    },
    /// Check symlinks, the destination, sources, the receipt, and the journal.
    Doctor {
        /// Also list every adapter claim with its evidence.
        #[arg(long)]
        evidence: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum BundleCmd {
    /// Save the current project's selection as a bundle.
    Save {
        name: String,
        /// Overwrite an existing bundle with the same slug.
        #[arg(long)]
        force: bool,
    },
    /// List bundles.
    Ls,
    /// Show a bundle and how each entry resolves.
    Show { name: String },
    /// Edit a bundle in $EDITOR (committed only when valid).
    Edit { name: String },
    /// Copy a bundle under a new name.
    Duplicate {
        name: String,
        new_name: String,
        /// Overwrite an existing bundle with the same slug.
        #[arg(long)]
        force: bool,
    },
    /// Delete a bundle, or remove entries when references are given.
    Rm {
        name: String,
        /// Skill references to remove (none deletes the bundle).
        references: Vec<String>,
        /// Rewrite the whole file instead of editing lines (drops comments).
        #[arg(long, requires = "references")]
        rewrite: bool,
    },
    /// Add skills to a bundle.
    Add {
        name: String,
        /// Skill references to add.
        #[arg(required = true)]
        references: Vec<String>,
        /// Rewrite the whole file instead of editing lines (drops comments).
        #[arg(long)]
        rewrite: bool,
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
    /// Point a source at a new path and repair the projects that use it.
    Relocate { alias: String, path: String },
}

/// The global flags a command may use.
#[derive(Debug, Clone, Default)]
pub struct Flags {
    pub no_cache: bool,
    pub project: Option<PathBuf>,
    pub target: Option<String>,
    pub yes: bool,
    pub json: bool,
}

/// Shared state for a command run.
pub struct Ctx {
    pub dirs: Dirs,
    pub config: Config,
    /// Config-level diagnostics (precedence ties).
    pub config_diags: Vec<FlatDiagnostic>,
    pub options: LoadOptions,
    pub project: Option<PathBuf>,
    pub target: Option<String>,
    pub yes: bool,
    pub json: bool,
    /// stdin is a terminal and `--json` is absent (plan P13).
    pub interactive: bool,
}

impl Ctx {
    fn load(flags: &Flags) -> anyhow::Result<Ctx> {
        let dirs = Dirs::from_process_env()?;
        let (config, diags) = Config::load_or_create(&dirs)?;
        tracing::debug!(config = %dirs.config_file().display(), sources = config.sources.len(), "config loaded");
        Ok(Ctx {
            dirs,
            config,
            config_diags: diags.into_iter().map(FlatDiagnostic::global).collect(),
            options: LoadOptions::product(!flags.no_cache),
            project: flags.project.clone(),
            target: flags.target.clone(),
            yes: flags.yes,
            json: flags.json,
            interactive: std::io::stdin().is_terminal() && !flags.json,
        })
    }

    /// Every bundle manifest in the bundles directory.
    pub fn bundles(&self) -> unam_core::bundle::BundleSet {
        unam_core::bundle::load_dir(&self.dirs)
    }

    pub fn catalog(&self) -> anyhow::Result<Catalog> {
        Ok(Catalog::load(
            &self.config.sources,
            &self.dirs,
            self.options,
        )?)
    }

    /// The project root: `--project`, else the nearest `.git` from the
    /// working directory.
    pub fn root(&self) -> unam_core::error::Result<ProjectRoot> {
        let cwd = std::env::current_dir()
            .map_err(|e| unam_core::error::Error::io("cannot read the working directory", e))?;
        unam_core::change::resolve_root(self.project.as_deref(), &cwd)
    }

    /// The parsed `--target` flag (`invalid-target` for unknown text).
    pub fn target_flag(&self) -> unam_core::error::Result<Option<TargetId>> {
        match &self.target {
            None => Ok(None),
            Some(t) => TargetId::parse(t)
                .map(Some)
                .ok_or_else(|| unam_core::error::Error::InvalidTarget { value: t.clone() }),
        }
    }
}

/// Config-level plus source-level warnings of a catalog load.
pub(super) fn catalog_warnings(ctx: &Ctx, catalog: &Catalog) -> Vec<FlatDiagnostic> {
    let mut all = ctx.config_diags.clone();
    all.extend(catalog.flat_diagnostics(None, SOURCE_AND_CONFIG));
    warnings_only(&all)
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

/// A failed command, with any diagnostics gathered before the failure. A
/// failure payload (plan P15, the `--json` `data` of exits 2, 6, 7, 10,
/// 11) travels inside `error` as a `DataError`; the default is the empty
/// object, so M1 and M2 failure output is unchanged.
pub struct Failed {
    pub error: anyhow::Error,
    pub warnings: Vec<FlatDiagnostic>,
}

/// A core error plus the `--json` `data` of its failure envelope.
#[derive(Debug)]
pub struct DataError {
    pub error: unam_core::error::Error,
    pub data: Value,
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

impl std::error::Error for DataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl Failed {
    /// A core error with a failure payload.
    pub fn with_data(
        error: unam_core::error::Error,
        data: Value,
        warnings: Vec<FlatDiagnostic>,
    ) -> Failed {
        Failed {
            error: anyhow::Error::new(DataError { error, data }),
            warnings,
        }
    }

    /// The failure payload, when one was attached.
    pub fn data(&self) -> Option<&Value> {
        self.error.downcast_ref::<DataError>().map(|d| &d.data)
    }
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
    Cli::command().render_help().to_string()
}

/// Run the parsed command; returns the process exit code.
pub fn run(cli: Cli) -> i32 {
    let flags = Flags {
        no_cache: cli.no_cache,
        project: cli.project.clone(),
        target: cli.target.clone(),
        yes: cli.yes,
        json: cli.json,
    };
    let Some(command) = cli.command else {
        // Bare `unam` opens the picker on a terminal (M6-10); anywhere else
        // it prints the usage text and the status table, exit 0.
        if !cli.json
            && crate::tui::is_interactive()
            && crate::tui::root_resolves(cli.project.as_deref())
        {
            return crate::tui::run(!cli.no_cache, cli.project.clone(), cli.target.clone());
        }
        print!("{}", usage_text());
        if let Some(table) = status::bare(&flags) {
            print!("\n{table}");
        }
        return 0;
    };
    let (name, result) = match Ctx::load(&flags) {
        Err(e) => (command_name(&command), Err(Failed::from(e))),
        Ok(ctx) => {
            let name = command_name(&command);
            let r = match command {
                Command::Source { cmd } => match cmd {
                    SourceCmd::Add { alias, path } => source::add(&ctx, &alias, &path),
                    SourceCmd::List => source::list(&ctx),
                    SourceCmd::Rm { alias, force } => source::rm(&ctx, &alias, force),
                    SourceCmd::Relocate { alias, path } => source::relocate(&ctx, &alias, &path),
                },
                Command::Search {
                    query,
                    category,
                    source,
                } => search::run(&ctx, &query, category.as_deref(), source.as_deref()),
                Command::Show { reference } => show::run(&ctx, &reference),
                Command::Bundle { cmd } => match cmd {
                    BundleCmd::Save { name, force } => bundle::save(&ctx, &name, force),
                    BundleCmd::Ls => bundle::ls(&ctx),
                    BundleCmd::Show { name } => bundle::show(&ctx, &name),
                    BundleCmd::Edit { name } => bundle::edit(&ctx, &name),
                    BundleCmd::Duplicate {
                        name,
                        new_name,
                        force,
                    } => bundle::duplicate(&ctx, &name, &new_name, force),
                    BundleCmd::Rm {
                        name,
                        references,
                        rewrite,
                    } => bundle::rm(&ctx, &name, &references, rewrite),
                    BundleCmd::Add {
                        name,
                        references,
                        rewrite,
                    } => bundle::add(&ctx, &name, &references, rewrite),
                },
                Command::Add { references } => change::add(&ctx, references),
                Command::Rm { references } => change::rm(&ctx, references),
                Command::Use { bundle, mode } => change::use_bundle(&ctx, bundle, mode.replace),
                Command::Deactivate { all } => change::deactivate(&ctx, all),
                Command::Resume => change::resume(&ctx),
                Command::Rollback => change::rollback(&ctx),
                Command::Status => status::status(&ctx),
                Command::Reconcile { repair } => status::reconcile(&ctx, repair),
                Command::Doctor { evidence } => doctor::run(&ctx, evidence),
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
            SourceCmd::Relocate { .. } => "source relocate",
        },
        Command::Search { .. } => "search",
        Command::Show { .. } => "show",
        Command::Bundle { cmd } => match cmd {
            BundleCmd::Save { .. } => "bundle save",
            BundleCmd::Ls => "bundle ls",
            BundleCmd::Show { .. } => "bundle show",
            BundleCmd::Edit { .. } => "bundle edit",
            BundleCmd::Duplicate { .. } => "bundle duplicate",
            BundleCmd::Rm { .. } => "bundle rm",
            BundleCmd::Add { .. } => "bundle add",
        },
        Command::Add { .. } => "add",
        Command::Rm { .. } => "rm",
        Command::Use { .. } => "use",
        Command::Deactivate { .. } => "deactivate",
        Command::Resume => "resume",
        Command::Rollback => "rollback",
        Command::Status => "status",
        Command::Reconcile { .. } => "reconcile",
        Command::Doctor { .. } => "doctor",
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
                let data = failed
                    .data()
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                print_json(&output::envelope(
                    false,
                    command,
                    data,
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
