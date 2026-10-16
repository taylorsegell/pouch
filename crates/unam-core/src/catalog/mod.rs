//! Catalog: identity types, loaded skills, and the per-source scan results.

pub mod cache;
pub mod frontmatter;
pub mod lookup;
pub mod scan;
pub mod search;

pub use lookup::{Lookup, canonical_ref_text, edit_distance};
pub use scan::{FingerprintEntry, SKILL_SKIP_NAMES, ScanLimits, ScannedSkill, SourceScan};

use crate::config::SourceEntry;
use crate::error::{Diagnostic, REMEDY_SOURCE_UNREACHABLE};
use crate::hash::Hash;
use crate::paths::Dirs;
use crate::source::{SourceAlias, SourceKind};
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// A forward-slash relative path with no empty, `.`, or `..` components.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelPath(String);

impl RelPath {
    /// Validate a relative path. Returns the reason on failure.
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.is_empty() {
            return Err("empty path".to_string());
        }
        if text.starts_with('/') {
            return Err("path must be relative".to_string());
        }
        for part in text.split('/') {
            match part {
                "" => return Err("empty path component".to_string()),
                "." | ".." => return Err(format!("`{part}` is not allowed")),
                _ => {}
            }
            if part.contains('\\') || part.contains('\0') {
                return Err("invalid character in path".to_string());
            }
        }
        Ok(RelPath(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parent path as a string; empty for a top-level entry.
    pub fn parent_str(&self) -> &str {
        match self.0.rfind('/') {
            Some(i) => &self.0[..i],
            None => "",
        }
    }

    /// Last component.
    pub fn file_name(&self) -> &str {
        match self.0.rfind('/') {
            Some(i) => &self.0[i + 1..],
            None => &self.0,
        }
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable identifier `<alias>:<relative/path>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SkillId {
    pub source: SourceAlias,
    pub rel_path: RelPath,
}

impl SkillId {
    /// Parse the stored text `alias:rel/path` of a skill ID (receipts and
    /// selection texts). Anything that is not `<alias>:<relative path>` is
    /// `InvalidRef`. A top-level skill has a one-component relative path, so
    /// `fx:name` parses with `rel_path` `name`.
    pub fn parse(text: &str) -> crate::error::Result<SkillId> {
        let invalid = |reason: &str| crate::error::Error::InvalidRef {
            reference: text.to_string(),
            reason: reason.to_string(),
        };
        let (alias, rest) = text
            .split_once(':')
            .ok_or_else(|| invalid("not a qualified skill ID"))?;
        let source = SourceAlias::parse(alias).map_err(|_| invalid("invalid source alias"))?;
        let rel_path = RelPath::parse(rest).map_err(|r| invalid(&r))?;
        Ok(SkillId { source, rel_path })
    }
}

impl fmt::Display for SkillId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.source, self.rel_path)
    }
}

impl Serialize for SkillId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

/// What a user may type. Resolved to a `SkillId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillRef {
    Qualified {
        source: SourceAlias,
        rel_path: RelPath,
    },
    SourceName {
        source: SourceAlias,
        name: String,
    },
    Bare(String),
}

/// A loaded skill (spec 3).
#[derive(Debug, Clone, PartialEq)]
pub struct Skill {
    pub id: SkillId,
    pub name: String,
    pub description: String,
    /// Parent of `rel_path`; empty for a top-level skill.
    pub category: String,
    pub frontmatter: serde_json::Map<String, serde_json::Value>,
    pub supporting: Vec<String>,
    pub tree_hash: Hash,
    pub diagnostics: Vec<Diagnostic>,
}

/// Per-source status after a load.
#[derive(Debug, Clone)]
pub struct SourceStatus {
    pub alias: SourceAlias,
    pub raw_path: String,
    /// The configured path after `~` and variable expansion.
    pub expanded: PathBuf,
    /// Canonical root when reachable; the expanded path otherwise.
    pub root: PathBuf,
    pub precedence: u32,
    pub reachable: bool,
    pub scanned_at: Option<String>,
    pub skill_count: Option<usize>,
    pub kind: Option<SourceKind>,
}

/// A catalog-level diagnostic (skipped skills and source-level notices).
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogDiagnostic {
    pub source: SourceAlias,
    pub diagnostic: Diagnostic,
}

/// All loaded skills across sources.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub skills: Vec<Skill>,
    pub by_id: HashMap<SkillId, usize>,
    pub by_name: HashMap<String, Vec<usize>>,
    pub sources: Vec<SourceStatus>,
    pub diagnostics: Vec<CatalogDiagnostic>,
    pub scanned_at: String,
}

/// Options for `Catalog::load`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadOptions {
    /// Read and write the per-source cache (`--no-cache` sets this false).
    pub use_cache: bool,
    /// Consult the global gitignore inside git trees (product: true).
    pub git_global: bool,
}

impl LoadOptions {
    /// Options used by the CLI.
    pub fn product(use_cache: bool) -> Self {
        LoadOptions {
            use_cache,
            git_global: true,
        }
    }
}

fn now_rfc3339() -> String {
    jiff::Timestamp::now().to_string()
}

impl Catalog {
    /// Load every source in `(precedence, alias)` order: check reachability,
    /// read the cache, scan, write the cache when something changed, assemble.
    /// Never writes under a source; an unreachable source is reported and left
    /// untouched (no cache read, write, or delete).
    pub fn load(
        entries: &[SourceEntry],
        dirs: &Dirs,
        options: LoadOptions,
    ) -> crate::error::Result<Catalog> {
        let mut ordered: Vec<&SourceEntry> = entries.iter().collect();
        ordered.sort_by(|a, b| (a.precedence, &a.alias).cmp(&(b.precedence, &b.alias)));
        let mut catalog = Catalog {
            scanned_at: now_rfc3339(),
            ..Catalog::default()
        };
        for entry in ordered {
            let alias = SourceAlias::parse(&entry.alias)?;
            let root = dirs.expand(&entry.path)?;
            let canonical = if root.is_dir() && std::fs::read_dir(&root).is_ok() {
                std::fs::canonicalize(&root).ok()
            } else {
                None
            };
            let Some(canonical) = canonical else {
                catalog.diagnostics.push(CatalogDiagnostic {
                    source: alias.clone(),
                    diagnostic: Diagnostic::warning(
                        "source-unreachable",
                        format!("source `{alias}` at {} is unreachable", root.display()),
                    )
                    .with_remedy(REMEDY_SOURCE_UNREACHABLE),
                });
                catalog.sources.push(SourceStatus {
                    alias,
                    raw_path: entry.path.clone(),
                    expanded: root.clone(),
                    root,
                    precedence: entry.precedence,
                    reachable: false,
                    scanned_at: None,
                    skill_count: None,
                    kind: None,
                });
                continue;
            };
            let source_path = canonical.to_string_lossy().into_owned();
            let hit = if options.use_cache {
                cache::read(dirs, &alias, &source_path)
            } else {
                None
            };
            let prior = hit.as_ref().map(|h| h.prior.clone()).unwrap_or_default();
            let limits = ScanLimits::product(options.git_global);
            let result = scan::scan_source(&alias, &canonical, &prior, &limits);
            let mut scanned_at = hit.as_ref().map(|h| h.scanned_at.clone());
            if options.use_cache && (hit.is_none() || result.reparsed > 0 || result.dropped > 0) {
                let stamp = now_rfc3339();
                match cache::write(dirs, &alias, &source_path, &stamp, &result.skills) {
                    Ok(()) => scanned_at = Some(stamp),
                    Err(e) => catalog.diagnostics.push(CatalogDiagnostic {
                        source: alias.clone(),
                        diagnostic: Diagnostic::warning(
                            "cache-write-failed",
                            format!("cannot write the catalog cache: {e}"),
                        ),
                    }),
                }
            }
            for d in result.diagnostics {
                catalog.diagnostics.push(CatalogDiagnostic {
                    source: alias.clone(),
                    diagnostic: d,
                });
            }
            catalog.sources.push(SourceStatus {
                alias,
                raw_path: entry.path.clone(),
                expanded: root.clone(),
                root: canonical.clone(),
                precedence: entry.precedence,
                reachable: true,
                scanned_at,
                skill_count: Some(result.skills.len()),
                kind: Some(SourceKind::detect(&canonical)),
            });
            catalog
                .skills
                .extend(result.skills.into_iter().map(|s| s.skill));
        }
        catalog.skills.sort_by(|a, b| a.id.cmp(&b.id));
        catalog.reindex();
        Ok(catalog)
    }

    /// Rebuild `by_id` and `by_name` from `skills`.
    pub fn reindex(&mut self) {
        self.by_id.clear();
        self.by_name.clear();
        for (i, s) in self.skills.iter().enumerate() {
            self.by_id.insert(s.id.clone(), i);
            self.by_name.entry(s.name.clone()).or_default().push(i);
        }
    }

    /// Status of one source.
    pub fn source(&self, alias: &SourceAlias) -> Option<&SourceStatus> {
        self.sources.iter().find(|s| s.alias == *alias)
    }

    /// Source precedence of `alias` (orders candidate lists, plan P15);
    /// `u32::MAX` when the alias is unknown.
    pub fn source_rank(&self, alias: &SourceAlias) -> u32 {
        self.source(alias).map_or(u32::MAX, |s| s.precedence)
    }

    /// Absolute path of a skill: its source root joined with `rel_path`.
    pub fn abs_path(&self, id: &SkillId) -> Option<PathBuf> {
        self.source(&id.source)
            .map(|s| s.root.join(id.rel_path.as_str()))
    }

    /// The read-only source path of a loaded skill; `None` for an unknown ID.
    pub fn source_path(&self, id: &SkillId) -> Option<crate::source::SourcePath> {
        if !self.by_id.contains_key(id) {
            return None;
        }
        self.abs_path(id).map(crate::source::SourcePath::new)
    }

    /// The loaded skill with this ID.
    pub fn skill(&self, id: &SkillId) -> Option<&Skill> {
        self.by_id.get(id).map(|&i| &self.skills[i])
    }
}

/// Diagnostic classes (CLI Contract): which commands report which codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagClass {
    /// Attached to a loaded skill.
    Skill,
    /// A skill that was skipped.
    Skipped,
    /// About a source as a whole.
    Source,
    /// About the configuration or the project index.
    Config,
}

/// Class of a diagnostic code.
pub fn diag_class(code: &str) -> DiagClass {
    match code {
        "name-dir-mismatch" | "name-too-long" | "colon-retry-applied" => DiagClass::Skill,
        "source-unreachable" | "walk-capped" | "walk-error" | "root-skill-skipped"
        | "zero-skills" | "duplicate-name" | "cache-write-failed" => DiagClass::Source,
        "precedence-tie"
        | "project-index-unreadable"
        | "bundle-unreadable"
        | "bundle-not-utf8"
        | "bundle-invalid-yaml"
        | "bundle-not-mapping"
        | "bundle-name-blank"
        | "bundle-no-skills"
        | "bundle-empty-slug"
        | "bundle-duplicate-slug"
        | "bundle-entry-ignored"
        | "ref-invalid"
        | "ref-not-found"
        | "ref-ambiguous"
        | "bundle-missing" => DiagClass::Config,
        _ => DiagClass::Skipped,
    }
}

/// A diagnostic with its source and (for skill-level ones) its skill ID.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlatDiagnostic {
    pub source: Option<String>,
    pub skill: Option<String>,
    #[serde(flatten)]
    pub diagnostic: Diagnostic,
}

impl FlatDiagnostic {
    /// A diagnostic that belongs to no source (config-level).
    pub fn global(diagnostic: Diagnostic) -> Self {
        FlatDiagnostic {
            source: None,
            skill: None,
            diagnostic,
        }
    }
}

impl Catalog {
    /// Every diagnostic (skill-level first, in skill order, then catalog-level),
    /// optionally limited to one source and to the given classes.
    pub fn flat_diagnostics(
        &self,
        source: Option<&SourceAlias>,
        classes: &[DiagClass],
    ) -> Vec<FlatDiagnostic> {
        let wanted = |code: &str| classes.contains(&diag_class(code));
        let mut out = Vec::new();
        for s in &self.skills {
            if source.is_some_and(|a| s.id.source != *a) {
                continue;
            }
            for d in &s.diagnostics {
                if wanted(&d.code) {
                    out.push(FlatDiagnostic {
                        source: Some(s.id.source.to_string()),
                        skill: Some(s.id.to_string()),
                        diagnostic: d.clone(),
                    });
                }
            }
        }
        for c in &self.diagnostics {
            if source.is_some_and(|a| c.source != *a) || !wanted(&c.diagnostic.code) {
                continue;
            }
            out.push(FlatDiagnostic {
                source: Some(c.source.to_string()),
                skill: None,
                diagnostic: c.diagnostic.clone(),
            });
        }
        out
    }
}

/// All four classes.
pub const ALL_CLASSES: &[DiagClass] = &[
    DiagClass::Skill,
    DiagClass::Skipped,
    DiagClass::Source,
    DiagClass::Config,
];

/// Classes reported by `search` and `show` in their envelope.
pub const SOURCE_AND_CONFIG: &[DiagClass] = &[DiagClass::Source, DiagClass::Config];

#[cfg(test)]
pub(crate) mod tests_support {
    use super::*;

    /// A catalog built from `(alias, rel_path, name)` triples plus unreachable aliases.
    pub fn catalog_of(skills: &[(&str, &str, &str)], unreachable: &[&str]) -> Catalog {
        let mut c = Catalog::default();
        let mut aliases: Vec<&str> = skills.iter().map(|s| s.0).collect();
        aliases.sort();
        aliases.dedup();
        for a in aliases {
            c.sources.push(SourceStatus {
                alias: SourceAlias::parse(a).unwrap(),
                raw_path: format!("/{a}"),
                expanded: PathBuf::from(format!("/{a}")),
                root: PathBuf::from(format!("/{a}")),
                precedence: 10,
                reachable: true,
                scanned_at: None,
                skill_count: None,
                kind: Some(SourceKind::Directory),
            });
        }
        for a in unreachable {
            c.sources.push(SourceStatus {
                alias: SourceAlias::parse(a).unwrap(),
                raw_path: format!("/{a}"),
                expanded: PathBuf::from(format!("/{a}")),
                root: PathBuf::from(format!("/{a}")),
                precedence: 20,
                reachable: false,
                scanned_at: None,
                skill_count: None,
                kind: None,
            });
        }
        for (alias, rel, name) in skills {
            let rel_path = RelPath::parse(rel).unwrap();
            c.skills.push(Skill {
                id: SkillId {
                    source: SourceAlias::parse(alias).unwrap(),
                    rel_path: rel_path.clone(),
                },
                name: name.to_string(),
                description: format!("About {name}."),
                category: rel_path.parent_str().to_string(),
                frontmatter: serde_json::Map::new(),
                supporting: vec![],
                tree_hash: Hash::of_bytes(rel.as_bytes()),
                diagnostics: vec![],
            });
        }
        c.skills.sort_by(|a, b| a.id.cmp(&b.id));
        c.reindex();
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relpath_validation() {
        for ok in ["a", "a/b", "a/b/c-d", "x.y/z"] {
            assert!(RelPath::parse(ok).is_ok(), "{ok}");
        }
        for bad in ["", "/a", "a//b", "a/", "./a", "a/../b", "..", "a\\b"] {
            assert!(RelPath::parse(bad).is_err(), "{bad}");
        }
        let r = RelPath::parse("engineering/backend/api/error-handling").unwrap();
        assert_eq!(r.parent_str(), "engineering/backend/api");
        assert_eq!(r.file_name(), "error-handling");
        assert_eq!(RelPath::parse("top").unwrap().parent_str(), "");
    }

    #[test]
    fn m2_codes_are_config_class() {
        for code in [
            "bundle-unreadable",
            "bundle-not-utf8",
            "bundle-invalid-yaml",
            "bundle-not-mapping",
            "bundle-name-blank",
            "bundle-no-skills",
            "bundle-empty-slug",
            "bundle-duplicate-slug",
            "bundle-entry-ignored",
            "ref-invalid",
            "ref-not-found",
            "ref-ambiguous",
            "bundle-missing",
        ] {
            assert_eq!(diag_class(code), DiagClass::Config, "{code}");
        }
        assert_eq!(diag_class("bad-yaml"), DiagClass::Skipped);
    }

    #[test]
    fn source_path_known_and_unknown_id() {
        let c = tests_support::catalog_of(&[("fx", "sdlc/code-review", "code-review")], &[]);
        let id = SkillId::parse("fx:sdlc/code-review").unwrap();
        assert_eq!(
            c.source_path(&id).unwrap().as_path(),
            Path::new("/fx/sdlc/code-review")
        );
        let unknown = SkillId::parse("fx:sdlc/other").unwrap();
        assert!(c.source_path(&unknown).is_none());
        assert_eq!(c.skill(&id).unwrap().name, "code-review");
    }

    #[test]
    fn skill_id_parse_forms() {
        let id = SkillId::parse("fx:a/b").unwrap();
        assert_eq!(id.to_string(), "fx:a/b");
        assert_eq!(id.rel_path.as_str(), "a/b");
        // A top-level skill: one-component relative path.
        let top = SkillId::parse("fx:name").unwrap();
        assert_eq!(top.rel_path.as_str(), "name");
        for bad in ["bare-name", "Fx:a/b", "fx:", "fx:/a", "fx:a/../b", ""] {
            assert_eq!(
                SkillId::parse(bad).unwrap_err().code(),
                "invalid-ref",
                "{bad:?}"
            );
        }
    }

    #[test]
    fn skill_id_display_and_serialize() {
        let id = SkillId {
            source: SourceAlias::parse("fx").unwrap(),
            rel_path: RelPath::parse("sdlc/code-review").unwrap(),
        };
        assert_eq!(id.to_string(), "fx:sdlc/code-review");
        assert_eq!(
            serde_json::to_string(&id).unwrap(),
            "\"fx:sdlc/code-review\""
        );
    }

    // ---- Catalog::load and cache ----

    use crate::config::SourceEntry;
    use crate::paths::{Dirs, Env};
    use std::collections::HashMap as Map;
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;

    fn dirs_in(root: &Path) -> Dirs {
        let mut m: Map<String, OsString> = Map::new();
        m.insert("HOME".into(), root.join("home").into());
        m.insert("UNAM_CONFIG_DIR".into(), root.join("config").into());
        m.insert("UNAM_CACHE_DIR".into(), root.join("cache").into());
        m.insert("XDG_DATA_HOME".into(), root.join("data").into());
        Dirs::resolve(Env::from_map(m)).unwrap()
    }

    fn opts(use_cache: bool) -> LoadOptions {
        LoadOptions {
            use_cache,
            git_global: false,
        }
    }

    fn write_skill(lib: &Path, rel: &str, name: &str, desc: &str) {
        let p = lib.join(rel).join("SKILL.md");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, format!("---\nname: {name}\ndescription: {desc}\n---\n")).unwrap();
    }

    struct Fx {
        _t: tempfile::TempDir,
        lib: PathBuf,
        dirs: Dirs,
        entries: Vec<SourceEntry>,
    }

    fn fx() -> Fx {
        let t = tempfile::tempdir().unwrap();
        let lib = t.path().join("lib");
        write_skill(&lib, "sdlc/code-review", "code-review", "Review code.");
        write_skill(&lib, "a/error-handling", "error-handling", "A.");
        write_skill(&lib, "b/error-handling", "error-handling", "B.");
        let dirs = dirs_in(t.path());
        let entries = vec![SourceEntry {
            alias: "fx".into(),
            path: lib.to_string_lossy().into_owned(),
            precedence: 10,
        }];
        Fx {
            _t: t,
            lib,
            dirs,
            entries,
        }
    }

    fn cache_bytes(f: &Fx) -> Option<Vec<u8>> {
        fs::read(f.dirs.catalog_cache_file("fx")).ok()
    }

    fn desc(c: &Catalog, rel: &str) -> String {
        c.skills
            .iter()
            .find(|s| s.id.rel_path.as_str() == rel)
            .unwrap()
            .description
            .clone()
    }

    fn poison(f: &Fx) {
        let p = f.dirs.catalog_cache_file("fx");
        let text = fs::read_to_string(&p).unwrap();
        fs::write(&p, text.replace("Review code.", "MARKER")).unwrap();
    }

    #[test]
    fn cache_miss_then_hit() {
        let f = fx();
        assert!(cache_bytes(&f).is_none());
        let c1 = Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        assert_eq!(c1.skills.len(), 3);
        let first = cache_bytes(&f).unwrap();
        assert!(c1.sources[0].scanned_at.is_some());
        let c2 = Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        assert_eq!(
            cache_bytes(&f).unwrap(),
            first,
            "unchanged scan must not rewrite"
        );
        assert_eq!(c2.sources[0].scanned_at, c1.sources[0].scanned_at);
        // A hit really reuses cached metadata.
        poison(&f);
        let c3 = Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        assert_eq!(desc(&c3, "sdlc/code-review"), "MARKER");
    }

    #[test]
    fn no_cache_neither_reads_nor_writes() {
        let f = fx();
        Catalog::load(&f.entries, &f.dirs, opts(false)).unwrap();
        assert!(cache_bytes(&f).is_none());
        Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        poison(&f);
        let before = cache_bytes(&f).unwrap();
        let c = Catalog::load(&f.entries, &f.dirs, opts(false)).unwrap();
        assert_eq!(desc(&c, "sdlc/code-review"), "Review code.");
        assert_eq!(cache_bytes(&f).unwrap(), before);
    }

    #[test]
    fn dropped_and_new_nested_skill() {
        let f = fx();
        Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        fs::remove_dir_all(f.lib.join("b")).unwrap();
        write_skill(&f.lib, "deep/x/y/new-one", "new-one", "New.");
        let c = Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        let ids: Vec<String> = c.skills.iter().map(|s| s.id.to_string()).collect();
        assert!(ids.contains(&"fx:deep/x/y/new-one".to_string()));
        assert!(!ids.contains(&"fx:b/error-handling".to_string()));
        let text = String::from_utf8(cache_bytes(&f).unwrap()).unwrap();
        assert!(text.contains("deep/x/y/new-one"));
        assert!(!text.contains("b/error-handling"));
    }

    #[test]
    fn in_place_edit_with_new_mtime() {
        let f = fx();
        Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        poison(&f);
        let p = f.lib.join("sdlc/code-review/SKILL.md");
        fs::write(
            &p,
            "---\nname: code-review\ndescription: Review diff.\n---\n",
        )
        .unwrap();
        fs::File::options()
            .write(true)
            .open(&p)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(12345))
            .unwrap();
        let c = Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        assert_eq!(desc(&c, "sdlc/code-review"), "Review diff.");
    }

    #[test]
    fn corrupt_wrong_version_and_relocated_cache_are_misses() {
        let f = fx();
        Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        let p = f.dirs.catalog_cache_file("fx");
        let good = fs::read_to_string(&p).unwrap();
        // Corrupt.
        fs::write(&p, "{not json").unwrap();
        let c = Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        assert_eq!(c.skills.len(), 3);
        assert!(serde_json::from_slice::<serde_json::Value>(&fs::read(&p).unwrap()).is_ok());
        // Wrong version (poisoned too, so a hit would show the marker).
        fs::write(
            &p,
            good.replace("Review code.", "MARKER")
                .replace("\"version\": 1", "\"version\": 9"),
        )
        .unwrap();
        let c = Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        assert_eq!(desc(&c, "sdlc/code-review"), "Review code.");
        // Different source_path.
        let canon = fs::canonicalize(&f.lib)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        fs::write(
            &p,
            good.replace("Review code.", "MARKER")
                .replace(&canon, "/elsewhere"),
        )
        .unwrap();
        let c = Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        assert_eq!(desc(&c, "sdlc/code-review"), "Review code.");
    }

    #[test]
    fn unreachable_source_touches_nothing() {
        let f = fx();
        Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        let before = cache_bytes(&f).unwrap();
        fs::remove_dir_all(&f.lib).unwrap();
        let c = Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        assert!(c.skills.is_empty());
        assert!(!c.sources[0].reachable);
        assert_eq!(c.diagnostics[0].diagnostic.code, "source-unreachable");
        assert_eq!(
            c.diagnostics[0].diagnostic.remedy.as_deref(),
            Some(crate::error::REMEDY_SOURCE_UNREACHABLE)
        );
        assert_eq!(cache_bytes(&f).unwrap(), before);
    }

    #[test]
    fn duplicate_name_info_and_index() {
        let f = fx();
        let c = Catalog::load(&f.entries, &f.dirs, opts(true)).unwrap();
        assert!(
            c.diagnostics
                .iter()
                .any(|d| d.diagnostic.code == "duplicate-name")
        );
        assert_eq!(c.by_name["error-handling"].len(), 2);
        let id = &c.skills[0].id;
        assert_eq!(
            c.abs_path(id).unwrap(),
            fs::canonicalize(&f.lib).unwrap().join(id.rel_path.as_str())
        );
    }

    #[test]
    fn sources_ordered_by_precedence_then_alias() {
        let f = fx();
        let mut entries = f.entries.clone();
        entries.push(SourceEntry {
            alias: "aa".into(),
            path: "/definitely/missing".into(),
            precedence: 10,
        });
        entries.push(SourceEntry {
            alias: "first".into(),
            path: "/definitely/missing2".into(),
            precedence: 5,
        });
        let c = Catalog::load(&entries, &f.dirs, opts(false)).unwrap();
        let order: Vec<&str> = c.sources.iter().map(|s| s.alias.as_str()).collect();
        assert_eq!(order, vec!["first", "aa", "fx"]);
    }
}
