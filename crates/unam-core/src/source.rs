//! Sources (spec 4.2): registration, listing, removal, relocation.

use crate::bundle::BundleSet;
use crate::catalog::{ALL_CLASSES, Catalog, CatalogDiagnostic, FlatDiagnostic, LoadOptions};
use crate::change::{ChangeRequest, Edit, Prepared, prepare};
use crate::config::{Config, SourceEntry, is_valid_alias};
use crate::error::{Diagnostic, Error, Result};
use crate::paths::Dirs;
use crate::project::{ProjectRoot, RootDetection, TargetId};
use crate::state::Clock;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// A validated source alias (`^[a-z0-9][a-z0-9-]{0,31}$`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SourceAlias(String);

impl SourceAlias {
    pub fn parse(text: &str) -> Result<Self> {
        if is_valid_alias(text) {
            Ok(SourceAlias(text.to_string()))
        } else {
            Err(Error::InvalidAlias {
                alias: text.to_string(),
            })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SourceAlias {
    type Error = Error;
    fn try_from(value: String) -> Result<Self> {
        SourceAlias::parse(&value)
    }
}

impl From<SourceAlias> for String {
    fn from(value: SourceAlias) -> Self {
        value.0
    }
}

impl fmt::Display for SourceAlias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A source root. Read-only by construction: it exposes only `as_path()`,
/// so it cannot be handed to write helpers by accident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePath(PathBuf);

impl SourcePath {
    pub fn new(path: PathBuf) -> Self {
        SourcePath(path)
    }

    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Detected kind of a source root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    Directory,
    GitRepo,
}

impl SourceKind {
    /// `GitRepo` when the root directly contains an entry named `.git`.
    pub fn detect(root: &Path) -> Self {
        if std::fs::symlink_metadata(root.join(".git")).is_ok() {
            SourceKind::GitRepo
        } else {
            SourceKind::Directory
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Directory => "directory",
            SourceKind::GitRepo => "git-repo",
        }
    }
}

/// A registered source as loaded.
#[derive(Debug, Clone)]
pub struct Source {
    pub alias: SourceAlias,
    pub path: SourcePath,
    pub raw_path: String,
    pub precedence: u32,
    pub kind: SourceKind,
}

/// The spelling stored in `config.toml` (plan P8): absolute or `~` paths
/// as typed; anything else as the canonical absolute path.
pub fn stored_spelling(raw: &str, canonical: &Path) -> String {
    if raw.starts_with('~') || Path::new(raw).is_absolute() {
        raw.to_string()
    } else {
        canonical.to_string_lossy().into_owned()
    }
}

/// Highest existing precedence plus 10, or 10 for the first source.
pub fn next_precedence(entries: &[SourceEntry]) -> u32 {
    entries
        .iter()
        .map(|e| e.precedence)
        .max()
        .map_or(10, |m| m.saturating_add(10))
}

/// Result of `remove`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveOutcome {
    pub alias: String,
    pub forced: bool,
    pub dangling_projects: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Unregister `alias`: refuse when referenced unless `force`; delete its
/// cache file. A project references the alias through a literal `add` or
/// `remove` reference, or through a selected bundle with a member from it
/// (M4-11); `catalog` and `bundles` resolve the bundle members.
pub fn remove(
    config: &mut Config,
    dirs: &Dirs,
    catalog: &Catalog,
    bundles: &BundleSet,
    alias: &str,
    force: bool,
) -> Result<RemoveOutcome> {
    let Some(pos) = config.sources.iter().position(|s| s.alias == alias) else {
        return Err(Error::SourceNotFound {
            alias: alias.to_string(),
        });
    };
    let (projects, diagnostics) =
        crate::project::index::projects_using_source(dirs, catalog, bundles, alias);
    if !projects.is_empty() && !force {
        return Err(Error::SourceReferenced {
            alias: alias.to_string(),
            projects,
        });
    }
    config.sources.remove(pos);
    config.save(dirs)?;
    // The alias is validated (it came from the config), so this path stays
    // inside `<cache dir>/catalog/`.
    let cache = dirs.catalog_cache_file(alias);
    match std::fs::remove_file(&cache) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(Error::io(
                format!("cannot delete cache {}", cache.display()),
                e,
            ));
        }
    }
    Ok(RemoveOutcome {
        alias: alias.to_string(),
        forced: force,
        dangling_projects: projects,
        diagnostics,
    })
}

/// Result of `add`: the new entry plus the scan of that source.
#[derive(Debug, Clone)]
pub struct AddOutcome {
    pub entry: SourceEntry,
    pub expanded_path: PathBuf,
    pub kind: SourceKind,
    pub skill_count: usize,
    /// Catalog for this source only (its diagnostics include `zero-skills`).
    pub catalog: Catalog,
}

fn canonical_of(entry: &SourceEntry, dirs: &Dirs) -> Option<PathBuf> {
    let p = dirs.expand(&entry.path).ok()?;
    std::fs::canonicalize(p).ok()
}

/// Validate a source root (shared by `add` and `relocate`): it expands,
/// canonicalizes, is a readable directory, and is not registered under
/// another alias (`except` names the alias being relocated). Returns the
/// expanded and the canonical path.
fn validate_root(
    config: &Config,
    dirs: &Dirs,
    raw_path: &str,
    except: Option<&str>,
) -> Result<(PathBuf, PathBuf)> {
    let invalid = |reason: String| Error::SourcePathInvalid {
        path: raw_path.to_string(),
        reason,
    };
    let expanded = dirs.expand(raw_path)?;
    let canonical = std::fs::canonicalize(&expanded).map_err(|e| invalid(e.to_string()))?;
    if !canonical.is_dir() {
        return Err(invalid("not a directory".to_string()));
    }
    std::fs::read_dir(&canonical).map_err(|e| invalid(format!("not readable: {e}")))?;
    if let Some(existing) = config.sources.iter().find(|s| {
        Some(s.alias.as_str()) != except
            && canonical_of(s, dirs).as_deref() == Some(canonical.as_path())
    }) {
        return Err(Error::SourcePathAlreadyRegistered {
            path: canonical.to_string_lossy().into_owned(),
            alias: existing.alias.clone(),
        });
    }
    Ok((expanded, canonical))
}

/// Register a source (spec 4.2): validate, canonicalize, reject duplicates,
/// scan it, warn on zero skills, then append the entry and save the config.
/// Nothing is written to the config when validation fails.
pub fn add(
    config: &mut Config,
    dirs: &Dirs,
    alias: &str,
    raw_path: &str,
    options: LoadOptions,
) -> Result<AddOutcome> {
    let alias = SourceAlias::parse(alias)?;
    if config.sources.iter().any(|s| s.alias == alias.as_str()) {
        return Err(Error::AliasExists {
            alias: alias.to_string(),
        });
    }
    let (expanded, canonical) = validate_root(config, dirs, raw_path, None)?;
    let entry = SourceEntry {
        alias: alias.to_string(),
        path: stored_spelling(raw_path, &canonical),
        precedence: next_precedence(&config.sources),
    };
    let mut catalog = Catalog::load(std::slice::from_ref(&entry), dirs, options)?;
    let skill_count = catalog.skills.len();
    if skill_count == 0 {
        catalog.diagnostics.push(CatalogDiagnostic {
            source: alias.clone(),
            diagnostic: Diagnostic::warning(
                "zero-skills",
                "no skills found (a skill is a directory containing SKILL.md); registered anyway",
            ),
        });
    }
    config.sources.push(entry.clone());
    config.save(dirs)?;
    Ok(AddOutcome {
        entry,
        expanded_path: expanded,
        kind: SourceKind::detect(&canonical),
        skill_count,
        catalog,
    })
}

/// What planning found for one dependent project.
pub enum Planned {
    /// The repair plan (`Edit::Reconcile` against the relocated catalog).
    Plan(Box<Prepared>),
    /// The project cannot be planned (a skill is missing at the new path,
    /// an unrelated reference does not resolve, ...); it stays as it is.
    Unplanned(String),
}

/// One dependent project and target.
pub struct ProjectRelocation {
    pub project: ProjectRoot,
    pub target: TargetId,
    pub planned: Planned,
}

/// A planned relocation (M4-10). Nothing is written until
/// `commit_relocation`; the caller then applies each plan with
/// `change::run` using `config` and `catalog`.
pub struct Relocation {
    pub alias: String,
    pub old_path: String,
    pub entry: SourceEntry,
    pub expanded_path: PathBuf,
    /// The configuration with the new path.
    pub config: Config,
    /// The catalog loaded from `config` (no cache read or write).
    pub catalog: Catalog,
    pub projects: Vec<ProjectRelocation>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Plan `source relocate <alias> <path>` (spec 4.2, SRC-4, EDG-4): validate
/// the new path like `add`, build the configuration and catalog as they
/// would be, and compute the repair plan of every indexed project whose
/// receipt holds an entry from the alias. Writes nothing.
pub fn plan_relocation(
    config: &Config,
    dirs: &Dirs,
    bundles: &BundleSet,
    alias: &str,
    raw_path: &str,
    options: LoadOptions,
    clock: &dyn Clock,
) -> Result<Relocation> {
    let Some(pos) = config.sources.iter().position(|s| s.alias == alias) else {
        return Err(Error::SourceNotFound {
            alias: alias.to_string(),
        });
    };
    let (expanded, canonical) = validate_root(config, dirs, raw_path, Some(alias))?;
    let mut new_config = config.clone();
    let old_path = std::mem::replace(
        &mut new_config.sources[pos].path,
        stored_spelling(raw_path, &canonical),
    );
    let entry = new_config.sources[pos].clone();
    let catalog = Catalog::load(
        &new_config.sources,
        dirs,
        LoadOptions {
            use_cache: false,
            ..options
        },
    )?;
    let (uses, mut diagnostics) = crate::project::index::projects_with_receipt_from(dirs, alias);
    let mut projects = Vec::new();
    for u in uses {
        let Ok(path) = std::fs::canonicalize(&u.root) else {
            diagnostics.push(
                Diagnostic::warning(
                    "project-missing",
                    format!("project {} cannot be resolved; it is left as it is", u.root),
                )
                .with_path(u.root.clone()),
            );
            continue;
        };
        let how = if std::fs::symlink_metadata(path.join(".git")).is_ok() {
            RootDetection::Git
        } else {
            RootDetection::Explicit
        };
        let project = ProjectRoot { path, how };
        for target in u.targets {
            let req = ChangeRequest {
                dirs,
                config: &new_config,
                catalog: &catalog,
                bundles,
                root: &project,
                target,
                edit: Edit::Reconcile,
                command: "source relocate",
                clock,
            };
            let planned = match prepare(&req) {
                Ok(p) => Planned::Plan(Box::new(p)),
                Err(f) => Planned::Unplanned(f.error.to_string()),
            };
            projects.push(ProjectRelocation {
                project: project.clone(),
                target,
                planned,
            });
        }
    }
    Ok(Relocation {
        alias: alias.to_string(),
        old_path,
        entry,
        expanded_path: expanded,
        config: new_config,
        catalog,
        projects,
        diagnostics,
    })
}

/// Save the relocated configuration and delete the alias cache file. The
/// caller applies the project plans afterwards.
pub fn commit_relocation(relocation: &Relocation, dirs: &Dirs) -> Result<()> {
    relocation.config.save(dirs)?;
    // The alias is validated (it came from the config), so this path stays
    // inside `<cache dir>/catalog/`.
    let cache = dirs.catalog_cache_file(&relocation.alias);
    match std::fs::remove_file(&cache) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::io(
            format!("cannot delete cache {}", cache.display()),
            e,
        )),
    }
}

/// One row of `source list`.
#[derive(Debug, Clone)]
pub struct SourceSummary {
    pub alias: String,
    pub raw_path: String,
    pub expanded_path: PathBuf,
    pub kind: Option<SourceKind>,
    pub precedence: u32,
    pub reachable: bool,
    pub skill_count: Option<usize>,
    pub last_scan: Option<String>,
    pub diagnostics: Vec<FlatDiagnostic>,
}

/// List sources ordered by `(precedence, alias)`, scanning each one.
pub fn list(config: &Config, dirs: &Dirs, options: LoadOptions) -> Result<Vec<SourceSummary>> {
    let catalog = Catalog::load(&config.sources, dirs, options)?;
    Ok(catalog
        .sources
        .iter()
        .map(|s| SourceSummary {
            alias: s.alias.to_string(),
            raw_path: s.raw_path.clone(),
            expanded_path: s.expanded.clone(),
            kind: s.kind,
            precedence: s.precedence,
            reachable: s.reachable,
            skill_count: s.skill_count,
            last_scan: s.scanned_at.clone(),
            diagnostics: catalog.flat_diagnostics(Some(&s.alias), ALL_CLASSES),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Env;
    use std::collections::HashMap;
    use std::ffi::OsString;

    pub(crate) fn dirs_in(root: &Path) -> Dirs {
        let mut m: HashMap<String, OsString> = HashMap::new();
        m.insert("HOME".into(), root.join("home").into());
        m.insert("UNAM_CONFIG_DIR".into(), root.join("config").into());
        m.insert("UNAM_CACHE_DIR".into(), root.join("cache").into());
        m.insert("XDG_DATA_HOME".into(), root.join("data").into());
        Dirs::resolve(Env::from_map(m)).unwrap()
    }

    fn entry(alias: &str, precedence: u32) -> SourceEntry {
        SourceEntry {
            alias: alias.into(),
            path: format!("/{alias}"),
            precedence,
        }
    }

    #[test]
    fn alias_validation() {
        assert!(SourceAlias::parse("fx").is_ok());
        assert_eq!(
            SourceAlias::parse("Fx").unwrap_err().code(),
            "invalid-alias"
        );
        let a: SourceAlias = serde_json::from_str("\"ok-1\"").unwrap();
        assert_eq!(a.to_string(), "ok-1");
        assert!(serde_json::from_str::<SourceAlias>("\"Bad\"").is_err());
    }

    #[test]
    fn next_precedence_rule() {
        assert_eq!(next_precedence(&[]), 10);
        assert_eq!(next_precedence(&[entry("a", 10)]), 20);
        assert_eq!(next_precedence(&[entry("a", 30), entry("b", 10)]), 40);
    }

    #[test]
    fn spelling_rule() {
        let canon = Path::new("/real/lib");
        assert_eq!(stored_spelling("/typed/lib", canon), "/typed/lib");
        assert_eq!(stored_spelling("~/lib", canon), "~/lib");
        assert_eq!(stored_spelling("lib", canon), "/real/lib");
        assert_eq!(stored_spelling("./lib", canon), "/real/lib");
    }

    #[test]
    fn kind_detection() {
        let t = tempfile::tempdir().unwrap();
        assert_eq!(SourceKind::detect(t.path()), SourceKind::Directory);
        std::fs::write(t.path().join(".git"), b"gitdir: x").unwrap();
        assert_eq!(SourceKind::detect(t.path()), SourceKind::GitRepo);
    }

    fn write_index(d: &Dirs, text: &str) {
        std::fs::create_dir_all(&d.data).unwrap();
        std::fs::write(d.project_index_file(), text).unwrap();
    }

    fn write_selection(root: &Path, text: &str) {
        std::fs::create_dir_all(root.join(".unam")).unwrap();
        std::fs::write(root.join(".unam/selection.yaml"), text).unwrap();
    }

    fn config_with(d: &Dirs, aliases: &[&str]) -> Config {
        let mut c = Config::default();
        for (i, a) in aliases.iter().enumerate() {
            c.sources.push(entry(a, 10 * (i as u32 + 1)));
        }
        c.save(d).unwrap();
        c
    }

    #[test]
    fn remove_deletes_entry_and_cache() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        let mut c = config_with(&d, &["fx", "other"]);
        use crate::fsops::{ToolPath, write_atomic};
        write_atomic(&ToolPath::new(d.catalog_cache_file("fx")), b"{}").unwrap();
        write_atomic(&ToolPath::new(d.catalog_cache_file("other")), b"{}").unwrap();
        let out = remove(
            &mut c,
            &d,
            &Catalog::default(),
            &BundleSet::default(),
            "fx",
            false,
        )
        .unwrap();
        assert!(out.dangling_projects.is_empty());
        assert!(!d.catalog_cache_file("fx").exists());
        assert!(d.catalog_cache_file("other").exists());
        let (reloaded, _) = Config::load_or_create(&d).unwrap();
        assert_eq!(reloaded.sources.len(), 1);
        assert_eq!(reloaded.sources[0].alias, "other");
    }

    #[test]
    fn remove_unknown_alias() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        let mut c = config_with(&d, &["fx"]);
        let e = remove(
            &mut c,
            &d,
            &Catalog::default(),
            &BundleSet::default(),
            "nope",
            false,
        )
        .unwrap_err();
        assert_eq!(e.code(), "source-not-found");
        assert_eq!(e.exit_code(), 4);
    }

    #[test]
    fn remove_referenced_requires_force() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        let mut c = config_with(&d, &["fx"]);
        let proj = t.path().join("proj");
        write_selection(&proj, "targets:\n  claude-code:\n    add: [\"fx:a/b\"]\n");
        write_index(
            &d,
            &serde_json::json!({"version": 1, "projects": [proj]}).to_string(),
        );
        let e = remove(
            &mut c,
            &d,
            &Catalog::default(),
            &BundleSet::default(),
            "fx",
            false,
        )
        .unwrap_err();
        assert_eq!(e.code(), "source-referenced");
        assert_eq!(e.exit_code(), 3);
        assert_eq!(c.sources.len(), 1);
        let out = remove(
            &mut c,
            &d,
            &Catalog::default(),
            &BundleSet::default(),
            "fx",
            true,
        )
        .unwrap();
        assert!(out.forced);
        assert_eq!(
            out.dangling_projects,
            vec![proj.to_string_lossy().into_owned()]
        );
        assert!(c.sources.is_empty());
    }

    // ---- relocate (milestone M4) ----

    #[cfg(unix)]
    #[test]
    fn relocate_plans_each_dependent_project() {
        use crate::verify::fixture::*;
        let mut f = fx();
        f.add(&["fx:sdlc/code-review"]);
        let new_lib = f.base.join("moved");
        std::fs::rename(&f.lib, &new_lib).unwrap();
        // A second indexed project whose directory is gone.
        crate::project::index::register(&f.dirs, &f.base.join("work/p")).unwrap();
        let gone = f.base.join("work/gone");
        std::fs::create_dir_all(&gone).unwrap();
        crate::project::index::register(&f.dirs, &gone).unwrap();
        std::fs::remove_dir_all(&gone).unwrap();
        // Unknown alias and a path registered under another alias.
        let unknown = plan_relocation(
            &f.config,
            &f.dirs,
            &f.bundles,
            "nope",
            "/x",
            opts(),
            &f.clock,
        );
        assert_eq!(unknown.err().unwrap().code(), "source-not-found");
        let other = f.base.join("other");
        std::fs::create_dir_all(&other).unwrap();
        f.config.sources.push(SourceEntry {
            alias: "other".into(),
            path: other.to_string_lossy().into_owned(),
            precedence: 20,
        });
        let clash = plan_relocation(
            &f.config,
            &f.dirs,
            &f.bundles,
            "fx",
            other.to_str().unwrap(),
            opts(),
            &f.clock,
        );
        assert_eq!(clash.err().unwrap().code(), "source-path-registered");
        f.config.sources.pop();
        f.config.save(&f.dirs).unwrap();
        write_atomic_cache(&f.dirs);
        // The plan: one project, remove then add, nothing written yet.
        let rel = plan_relocation(
            &f.config,
            &f.dirs,
            &f.bundles,
            "fx",
            new_lib.to_str().unwrap(),
            opts(),
            &f.clock,
        )
        .unwrap();
        assert_eq!(rel.projects.len(), 1);
        let Planned::Plan(p) = &rel.projects[0].planned else {
            panic!("expected a plan");
        };
        let ops: Vec<&str> = p.plan.steps.iter().map(|s| s.op()).collect();
        assert_eq!(ops, vec!["remove", "add"]);
        assert!(rel.diagnostics.iter().any(|d| d.code == "project-missing"));
        assert_eq!(rel.entry.path, new_lib.to_str().unwrap());
        let (before, _) = Config::load_or_create(&f.dirs).unwrap();
        assert_eq!(before.sources[0].path, f.lib.to_str().unwrap());
        assert!(f.dirs.catalog_cache_file("fx").exists());
        commit_relocation(&rel, &f.dirs).unwrap();
        let (after, _) = Config::load_or_create(&f.dirs).unwrap();
        assert_eq!(after.sources[0].path, new_lib.to_str().unwrap());
        assert!(!f.dirs.catalog_cache_file("fx").exists());
    }

    fn write_atomic_cache(d: &Dirs) {
        use crate::fsops::{ToolPath, write_atomic};
        write_atomic(&ToolPath::new(d.catalog_cache_file("fx")), b"{}").unwrap();
    }

    // ---- add and list (step D3) ----

    fn opts() -> LoadOptions {
        LoadOptions {
            use_cache: true,
            git_global: false,
        }
    }

    fn dirs_with(root: &Path, extra: &[(&str, &Path)]) -> Dirs {
        let mut m: HashMap<String, OsString> = HashMap::new();
        m.insert("HOME".into(), root.join("home").into());
        m.insert("UNAM_CONFIG_DIR".into(), root.join("config").into());
        m.insert("UNAM_CACHE_DIR".into(), root.join("cache").into());
        m.insert("XDG_DATA_HOME".into(), root.join("data").into());
        for (k, v) in extra {
            m.insert(k.to_string(), v.as_os_str().to_owned());
        }
        Dirs::resolve(Env::from_map(m)).unwrap()
    }

    fn lib_with_skill(dir: &Path) {
        let p = dir.join("cat/one/SKILL.md");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "---\nname: one\ndescription: First.\n---\n").unwrap();
    }

    #[test]
    fn add_writes_config_and_cache() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        let lib = t.path().join("lib");
        lib_with_skill(&lib);
        let mut c = Config::default();
        let out = add(&mut c, &d, "fx", lib.to_str().unwrap(), opts()).unwrap();
        assert_eq!(out.skill_count, 1);
        assert_eq!(out.entry.precedence, 10);
        assert_eq!(out.kind, SourceKind::Directory);
        assert!(d.catalog_cache_file("fx").exists());
        let (reloaded, _) = Config::load_or_create(&d).unwrap();
        assert_eq!(reloaded.sources, vec![out.entry.clone()]);
        assert_eq!(reloaded.sources[0].path, lib.to_str().unwrap());
        let rows = list(&reloaded, &d, opts()).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].reachable);
        assert_eq!(rows[0].skill_count, Some(1));
        assert!(rows[0].last_scan.is_some());
    }

    #[test]
    fn zero_skills_warns_and_registers() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        let lib = t.path().join("empty");
        std::fs::create_dir_all(&lib).unwrap();
        let mut c = Config::default();
        let out = add(&mut c, &d, "fx", lib.to_str().unwrap(), opts()).unwrap();
        assert_eq!(out.skill_count, 0);
        assert!(
            out.catalog
                .diagnostics
                .iter()
                .any(|x| x.diagnostic.code == "zero-skills")
        );
        assert_eq!(c.sources.len(), 1);
    }

    #[test]
    fn duplicate_alias_and_canonical_path() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        let lib = t.path().join("lib");
        lib_with_skill(&lib);
        let mut c = Config::default();
        add(&mut c, &d, "fx", lib.to_str().unwrap(), opts()).unwrap();
        let other = t.path().join("other");
        std::fs::create_dir_all(&other).unwrap();
        let e = add(&mut c, &d, "fx", other.to_str().unwrap(), opts()).unwrap_err();
        assert_eq!(e.code(), "alias-exists");
        let spelled = format!("{}/./lib/", t.path().display());
        let e = add(&mut c, &d, "fx2", &spelled, opts()).unwrap_err();
        assert_eq!(e.code(), "source-path-registered");
        let e = add(&mut c, &d, "Bad", other.to_str().unwrap(), opts()).unwrap_err();
        assert_eq!(e.code(), "invalid-alias");
        let e = add(&mut c, &d, "nope", "/definitely/not/here", opts()).unwrap_err();
        assert_eq!(e.code(), "source-path-invalid");
        assert_eq!(c.sources.len(), 1);
        let (reloaded, _) = Config::load_or_create(&d).unwrap();
        assert_eq!(reloaded.sources.len(), 1);
    }

    #[test]
    fn tilde_and_var_expand_through_injected_env() {
        let t = tempfile::tempdir().unwrap();
        let home = t.path().join("home");
        lib_with_skill(&home.join("skills"));
        let libs = t.path().join("libs");
        lib_with_skill(&libs.join("team"));
        let d = dirs_with(t.path(), &[("UNAM_TEST_LIBS", &libs)]);
        let mut c = Config::default();
        let a = add(&mut c, &d, "home", "~/skills", opts()).unwrap();
        assert_eq!(a.entry.path, "~/skills");
        assert_eq!(a.skill_count, 1);
        let b = add(&mut c, &d, "team", "$UNAM_TEST_LIBS/team", opts()).unwrap();
        assert_eq!(b.skill_count, 1);
        assert_eq!(b.entry.precedence, 20);
        let e = add(&mut c, &d, "x", "$UNAM_TEST_UNSET_VAR/x", opts()).unwrap_err();
        assert_eq!(e.code(), "config-unset-variable");
    }
}
