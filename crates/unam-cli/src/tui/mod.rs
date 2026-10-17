//! The interactive picker (spec 6; milestone M6). A renderer over
//! `unam-core`: it calls the same core functions as the CLI and never
//! writes a file itself. Single-threaded: a blocking key loop in `term`, a
//! pure state machine in `app`, and the drawing code in `picker` and
//! `views`.

mod actions;
mod app;
mod picker;
mod term;
#[cfg(test)]
mod tests;
mod views;

use crate::output::ErrorInfo;
use std::io::IsTerminal;
use std::path::PathBuf;
use unam_core::config::Config;
use unam_core::paths::Dirs;

pub use app::Opts;

/// Bare `unam` opens the picker only when both stdin and stdout are
/// terminals (M6-10).
pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// True when a project root resolves for the working directory (and
/// `--project`); bare `unam` falls back to usage plus status otherwise.
pub fn root_resolves(project: Option<&std::path::Path>) -> bool {
    std::env::current_dir()
        .ok()
        .is_some_and(|cwd| unam_core::change::resolve_root(project, &cwd).is_ok())
}

/// Run the picker; returns the process exit code. Errors print in the CLI
/// style after the terminal is restored.
pub fn run(use_cache: bool, project: Option<PathBuf>, target: Option<String>) -> i32 {
    match run_inner(use_cache, project, target) {
        Ok(()) => 0,
        Err(e) => {
            let info = ErrorInfo::from_anyhow(&e);
            eprint!("{}", info.human());
            info.exit_code
        }
    }
}

fn run_inner(
    use_cache: bool,
    project: Option<PathBuf>,
    target: Option<String>,
) -> anyhow::Result<()> {
    let target = match target {
        None => None,
        Some(t) => Some(
            unam_core::project::TargetId::parse(&t)
                .ok_or_else(|| unam_core::error::Error::InvalidTarget { value: t.clone() })?,
        ),
    };
    let cwd = std::env::current_dir()
        .map_err(|e| unam_core::error::Error::io("cannot read the working directory", e))?;
    let dirs = Dirs::from_process_env()?;
    let (config, _diags) = Config::load_or_create(&dirs)?;
    let opts = Opts {
        use_cache,
        project,
        target,
        cwd,
    };
    let mut app = app::App::loading(dirs, config, opts);
    term::run_terminal(&mut app)
}
