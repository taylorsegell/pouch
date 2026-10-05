//! `<project>/.unam/selection.yaml` (spec 2.3; M2 plan P7): the desired
//! state per target, edited by hand or (from milestone M3) by `use`, `add`,
//! and `rm`.
//!
//! `add` and `remove` are stored as text and parsed at resolution time, so an
//! invalid reference becomes a resolution problem instead of a load failure.
//! Rewriting the file does not preserve comments (known limitation).

use crate::config::Strategy;
use crate::error::{Diagnostic, Error, Result};
use crate::fsops::write_atomic;
use crate::paths::Dirs;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// A destination target (spec 2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetId {
    ClaudeCode,
    AgentsNeutral,
}

impl TargetId {
    pub fn as_str(self) -> &'static str {
        match self {
            TargetId::ClaudeCode => "claude-code",
            TargetId::AgentsNeutral => "agents-neutral",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "claude-code" => Some(TargetId::ClaudeCode),
            "agents-neutral" => Some(TargetId::AgentsNeutral),
            _ => None,
        }
    }
}

/// Ordered by name, so maps of targets serialize alphabetically.
impl Ord for TargetId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for TargetId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for TargetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The selection for one target.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TargetSelection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<Strategy>,
    pub bundles: Vec<String>,
    pub add: Vec<String>,
    pub remove: Vec<String>,
}

/// A whole selection file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    #[serde(default = "one")]
    pub version: u32,
    #[serde(default)]
    pub targets: BTreeMap<TargetId, TargetSelection>,
}

fn one() -> u32 {
    1
}

impl Default for Selection {
    fn default() -> Self {
        Selection {
            version: 1,
            targets: BTreeMap::new(),
        }
    }
}

/// `<root>/.unam/selection.yaml`.
pub fn selection_path(root: &Path) -> PathBuf {
    root.join(".unam").join("selection.yaml")
}

/// Parse selection text. Empty (or whitespace-only, or comment-only) text is
/// an empty selection.
pub fn parse_selection(text: &str, path: &Path) -> Result<Selection> {
    let invalid = |message: String| Error::SelectionInvalid {
        path: path.to_path_buf(),
        message,
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if text.trim().is_empty() {
        return Ok(Selection::default());
    }
    let value: serde_norway::Value =
        serde_norway::from_str(text).map_err(|e| invalid(e.to_string()))?;
    if value.is_null() {
        return Ok(Selection::default());
    }
    if let Some(v) = value.get("version")
        && let Some(n) = v.as_u64()
        && n != 1
    {
        return Err(Error::SelectionUnsupportedVersion {
            path: path.to_path_buf(),
            version: u32::try_from(n).unwrap_or(u32::MAX),
        });
    }
    serde_norway::from_value(value).map_err(|e| invalid(e.to_string()))
}

/// Load the selection of the project at `root`; `None` when absent.
pub fn load_selection(root: &Path) -> Result<Option<Selection>> {
    let path = selection_path(root);
    match std::fs::read_to_string(&path) {
        Ok(text) => parse_selection(&text, &path).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::SelectionRead { path, source: e }),
    }
}

/// Result of `save_selection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionSave {
    pub path: PathBuf,
    /// True when this call created the `.unam/` directory.
    pub created_dir: bool,
    /// Lines offered for the project's `.gitignore` (OWN-7): non-empty only
    /// when this call created `.unam/`. The file `.gitignore` is never read
    /// or written here; the caller prints the offer.
    pub gitignore_lines: Vec<String>,
    /// Project-index diagnostics from registering the root.
    pub diagnostics: Vec<Diagnostic>,
}

/// The `.gitignore` lines offered on the first write (OWN-7).
pub const GITIGNORE_LINES: &[&str] =
    &[".unam/applied.json", ".unam/journal.json", ".unam/history/"];

/// Render a selection as YAML text.
pub fn render_selection(selection: &Selection) -> Result<String> {
    serde_norway::to_string(selection)
        .map_err(|e| Error::io("cannot serialize the selection", std::io::Error::other(e)))
}

/// Write the selection atomically (creating `.unam/` if needed) and register
/// the project root in the project index.
pub fn save_selection(dirs: &Dirs, root: &Path, selection: &Selection) -> Result<SelectionSave> {
    let dir = root.join(".unam");
    let created_dir = !dir.exists();
    if created_dir {
        std::fs::create_dir_all(&dir)
            .map_err(|e| Error::io(format!("cannot create {}", dir.display()), e))?;
    }
    let path = selection_path(root);
    write_atomic(&path, render_selection(selection)?.as_bytes())?;
    let diagnostics = super::index::register(dirs, root)?;
    Ok(SelectionSave {
        path,
        created_dir,
        gitignore_lines: if created_dir {
            GITIGNORE_LINES.iter().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        },
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::index::read_index;

    fn p() -> PathBuf {
        PathBuf::from("/p/.unam/selection.yaml")
    }

    const SPEC_EXAMPLE: &str = "version: 1
targets:
  claude-code:
    strategy: symlink
    bundles: [backend-dev, debugging]
    add: [skillzone:sdlc/code-review]
    remove: [skillzone:testing/pytest-fixtures]
  agents-neutral:
    bundles: [backend-dev]
";

    #[test]
    fn spec_example_parses() {
        let s = parse_selection(SPEC_EXAMPLE, &p()).unwrap();
        assert_eq!(s.version, 1);
        let cc = &s.targets[&TargetId::ClaudeCode];
        assert_eq!(cc.strategy, Some(Strategy::Symlink));
        assert_eq!(cc.bundles, vec!["backend-dev", "debugging"]);
        assert_eq!(cc.add, vec!["skillzone:sdlc/code-review"]);
        assert_eq!(cc.remove, vec!["skillzone:testing/pytest-fixtures"]);
        let an = &s.targets[&TargetId::AgentsNeutral];
        assert_eq!(an.strategy, None);
        assert!(an.add.is_empty());
        let order: Vec<&str> = s.targets.keys().map(|t| t.as_str()).collect();
        assert_eq!(order, vec!["agents-neutral", "claude-code"]);
    }

    #[test]
    fn version_rules() {
        let s = parse_selection("targets:\n  claude-code:\n    add: [x]\n", &p()).unwrap();
        assert_eq!(s.version, 1);
        let e = parse_selection("version: 2\ntargets: {}\n", &p()).unwrap_err();
        assert_eq!(e.code(), "selection-version-unsupported");
        assert_eq!(e.exit_code(), 1);
    }

    #[test]
    fn unknown_key_and_unknown_target_are_errors() {
        for text in [
            "version: 1\nbundels: []\n",
            "targets:\n  claude-code:\n    bundels: [x]\n",
            "targets:\n  cursor:\n    add: [x]\n",
            "targets: [1, 2]\n",
            "- just\n- a list\n",
        ] {
            let e = parse_selection(text, &p()).unwrap_err();
            assert_eq!(e.code(), "selection-invalid", "{text:?}");
        }
        let e = parse_selection("targets: [unclosed\n", &p()).unwrap_err();
        assert_eq!(e.code(), "selection-invalid");
    }

    #[test]
    fn empty_and_whitespace_files_are_empty_selections() {
        for text in ["", "   \n\n", "# only a comment\n", "\u{feff}"] {
            let s = parse_selection(text, &p()).unwrap();
            assert_eq!(s, Selection::default(), "{text:?}");
        }
    }

    #[test]
    fn add_and_remove_kept_as_text_including_invalid() {
        let s = parse_selection(
            "targets:\n  claude-code:\n    add: [\"fx:\", \"Bad:x\", ok]\n    remove: [\"\"]\n",
            &p(),
        )
        .unwrap();
        let cc = &s.targets[&TargetId::ClaudeCode];
        assert_eq!(cc.add, vec!["fx:", "Bad:x", "ok"]);
        assert_eq!(cc.remove, vec![""]);
    }

    #[test]
    fn target_id_round_trip() {
        for t in [TargetId::ClaudeCode, TargetId::AgentsNeutral] {
            assert_eq!(TargetId::parse(t.as_str()), Some(t));
            assert_eq!(t.to_string(), t.as_str());
        }
        assert_eq!(TargetId::parse("cursor"), None);
        assert!(TargetId::AgentsNeutral < TargetId::ClaudeCode);
    }

    #[test]
    fn round_trip_and_save() {
        let t = tempfile::tempdir().unwrap();
        let d = crate::project::index::tests_support::dirs_in(t.path());
        let root = t.path().join("proj");
        std::fs::create_dir_all(&root).unwrap();
        // Load of an absent file.
        assert_eq!(load_selection(&root).unwrap(), None);
        let mut s = parse_selection(SPEC_EXAMPLE, &p()).unwrap();
        s.targets
            .get_mut(&TargetId::AgentsNeutral)
            .unwrap()
            .strategy = Some(Strategy::Copy);
        let first = save_selection(&d, &root, &s).unwrap();
        assert!(first.created_dir);
        assert_eq!(
            first.gitignore_lines,
            vec![".unam/applied.json", ".unam/journal.json", ".unam/history/"]
        );
        assert!(!root.join(".gitignore").exists());
        assert_eq!(load_selection(&root).unwrap(), Some(s.clone()));
        let text = std::fs::read_to_string(&first.path).unwrap();
        assert!(
            text.starts_with("version: 1\ntargets:\n  agents-neutral:\n    strategy: copy\n"),
            "{text}"
        );
        // Second save offers nothing.
        let second = save_selection(&d, &root, &s).unwrap();
        assert!(!second.created_dir);
        assert!(second.gitignore_lines.is_empty());
        // The root is registered (canonical).
        let canon = std::fs::canonicalize(&root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(read_index(&d).projects, vec![canon]);
    }

    #[test]
    fn load_reports_invalid_yaml() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        std::fs::create_dir_all(root.join(".unam")).unwrap();
        std::fs::write(selection_path(root), "targets: [\n").unwrap();
        assert_eq!(
            load_selection(root).unwrap_err().code(),
            "selection-invalid"
        );
        std::fs::remove_file(selection_path(root)).unwrap();
        std::fs::create_dir_all(selection_path(root)).unwrap();
        assert_eq!(
            load_selection(root).unwrap_err().code(),
            "selection-read-failed"
        );
    }
}
