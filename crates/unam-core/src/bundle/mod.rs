//! Bundles (spec 2.2, 4.4): Hermes-compatible manifests, the loader, CRUD,
//! and the in-place line editor for the `skills:` block.
//!
//! The loader mirrors Hermes `_load_bundle_file` (M2 plan D7): `*.yaml`
//! sorted, then `*.yml` sorted; a leading BOM stripped; the first file for a
//! slug wins; entries stringified and blanks dropped; the description
//! defaults to `Load {n} skills as a bundle`. A broken manifest is skipped
//! with a named diagnostic, never fatal.

pub mod crud;
pub mod edit;
pub mod slug;
pub mod word_table;

pub use crud::{
    DeleteOutcome, EditOutcome, EntriesOutcome, SaveOutcome, add_refs, delete, duplicate,
    edit_in_editor, remove_refs, render_manifest, save,
};
pub use slug::slugify;

use crate::error::Diagnostic;
use crate::paths::Dirs;
use serde_norway::Value;
use std::ffi::OsString;
use std::path::PathBuf;

/// The lenient `metadata.unambiguous` view (wrong types are ignored).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BundleMetadata {
    pub version: Option<u32>,
    pub materialize_instruction: Option<bool>,
}

/// A loaded bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub skills: Vec<String>,
    pub instruction: Option<String>,
    pub metadata: BundleMetadata,
    /// The bundles directory joined with `file_name`.
    pub path: PathBuf,
    pub file_name: String,
}

/// A parsed manifest (no file identity yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedBundle {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub skills: Vec<String>,
    pub instruction: Option<String>,
    pub metadata: BundleMetadata,
    pub warnings: Vec<Diagnostic>,
}

/// Every accepted bundle in load order, plus the loader diagnostics (each
/// with `path` set to the manifest file name).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BundleSet {
    pub bundles: Vec<Bundle>,
    pub diagnostics: Vec<Diagnostic>,
}

impl BundleSet {
    /// Find a bundle by name or slug (the text is slugified first).
    pub fn get(&self, name_or_slug: &str) -> Option<&Bundle> {
        let slug = slugify(name_or_slug);
        if slug.is_empty() {
            return None;
        }
        self.bundles.iter().find(|b| b.slug == slug)
    }

    /// Every slug, sorted.
    pub fn slugs(&self) -> Vec<String> {
        let mut out: Vec<String> = self.bundles.iter().map(|b| b.slug.clone()).collect();
        out.sort();
        out
    }
}

/// Default description (Hermes): `Load {n} skills as a bundle`.
pub fn default_description(n: usize) -> String {
    format!("Load {n} skills as a bundle")
}

/// A scalar as text: strings as is, numbers and booleans as text.
fn scalar_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Parse manifest text (M2 plan, `bundle/mod.rs` steps 1 to 5).
pub fn parse_bundle_text(text: &str, file_stem: &str) -> Result<ParsedBundle, Diagnostic> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let value: Value = if text.trim().is_empty() {
        Value::Null
    } else {
        serde_norway::from_str(text)
            .map_err(|e| Diagnostic::error("bundle-invalid-yaml", format!("invalid YAML: {e}")))?
    };
    let Value::Mapping(map) = &value else {
        return Err(Diagnostic::error(
            "bundle-not-mapping",
            "the manifest is not a YAML mapping",
        ));
    };
    let get = |k: &str| map.get(Value::String(k.to_string()));

    let name = match get("name") {
        None | Some(Value::Null) => file_stem.trim().to_string(),
        Some(v) => match scalar_text(v) {
            Some(s) => s.trim().to_string(),
            None => {
                return Err(Diagnostic::error("bundle-name-blank", "`name` is not text"));
            }
        },
    };
    if name.is_empty() {
        return Err(Diagnostic::error(
            "bundle-name-blank",
            "the bundle name is blank",
        ));
    }

    let Some(Value::Sequence(raw)) = get("skills") else {
        return Err(Diagnostic::error(
            "bundle-no-skills",
            "`skills` is missing, not a list, or empty",
        ));
    };
    if raw.is_empty() {
        return Err(Diagnostic::error(
            "bundle-no-skills",
            "`skills` is missing, not a list, or empty",
        ));
    }
    let mut skills = Vec::new();
    let mut ignored = 0usize;
    for entry in raw {
        match entry {
            Value::Null => {}
            Value::Mapping(_) | Value::Sequence(_) | Value::Tagged(_) => ignored += 1,
            other => {
                if let Some(s) = scalar_text(other) {
                    let s = s.trim();
                    if !s.is_empty() {
                        skills.push(s.to_string());
                    }
                }
            }
        }
    }
    let mut warnings = Vec::new();
    if ignored > 0 {
        warnings.push(Diagnostic::warning(
            "bundle-entry-ignored",
            format!(
                "{ignored} skills entr{} not text and ignored",
                if ignored == 1 { "y is" } else { "ies are" }
            ),
        ));
    }
    if skills.is_empty() {
        return Err(Diagnostic::error("bundle-no-skills", "empty skills list"));
    }

    let slug = slugify(&name);
    if slug.is_empty() {
        return Err(Diagnostic::error(
            "bundle-empty-slug",
            format!("the name `{name}` has no letters or digits, so its slug is empty"),
        ));
    }

    let text_of = |k: &str| -> String {
        get(k)
            .and_then(scalar_text)
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    let mut description = text_of("description");
    if description.is_empty() {
        description = default_description(skills.len());
    }
    let instruction = Some(text_of("instruction")).filter(|s| !s.is_empty());
    let unam = get("metadata").and_then(|m| m.get("unambiguous"));
    let metadata = BundleMetadata {
        version: unam
            .and_then(|u| u.get("version"))
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok()),
        materialize_instruction: unam
            .and_then(|u| u.get("materialize_instruction"))
            .and_then(Value::as_bool),
    };

    Ok(ParsedBundle {
        slug,
        name,
        description,
        skills,
        instruction,
        metadata,
        warnings,
    })
}

fn manifest_files(dirs: &Dirs) -> Result<Vec<OsString>, std::io::Error> {
    let mut yaml = Vec::new();
    let mut yml = Vec::new();
    for entry in std::fs::read_dir(&dirs.bundles)? {
        let entry = entry?;
        let name = entry.file_name();
        // `is_file` follows symlinks (a dangling link is not a file).
        if !entry.path().is_file() {
            continue;
        }
        let lossy = name.to_string_lossy();
        if lossy.ends_with(".yaml") {
            yaml.push(name);
        } else if lossy.ends_with(".yml") {
            yml.push(name);
        }
    }
    yaml.sort();
    yml.sort();
    yaml.extend(yml);
    Ok(yaml)
}

/// Load every manifest in the bundles directory. Never fails: a missing
/// directory is an empty set; a broken manifest is a diagnostic.
pub fn load_dir(dirs: &Dirs) -> BundleSet {
    let mut set = BundleSet::default();
    if !dirs.bundles.is_dir() {
        return set;
    }
    let files = match manifest_files(dirs) {
        Ok(f) => f,
        Err(e) => {
            set.diagnostics.push(
                Diagnostic::error(
                    "bundle-unreadable",
                    format!("cannot list the bundles directory: {e}"),
                )
                .with_path(dirs.bundles.to_string_lossy()),
            );
            return set;
        }
    };
    for name in files {
        let file_name = name.to_string_lossy().into_owned();
        let path = dirs.bundles.join(&name);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                set.diagnostics.push(
                    Diagnostic::error("bundle-unreadable", format!("cannot read: {e}"))
                        .with_path(&file_name),
                );
                continue;
            }
        };
        let Ok(text) = String::from_utf8(bytes) else {
            set.diagnostics.push(
                Diagnostic::error("bundle-not-utf8", "the manifest is not valid UTF-8")
                    .with_path(&file_name),
            );
            continue;
        };
        let stem = std::path::Path::new(&name)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        match parse_bundle_text(&text, &stem) {
            Err(d) => set.diagnostics.push(d.with_path(&file_name)),
            Ok(parsed) => {
                for w in parsed.warnings {
                    set.diagnostics.push(w.with_path(&file_name));
                }
                if let Some(first) = set.bundles.iter().find(|b| b.slug == parsed.slug) {
                    set.diagnostics.push(
                        Diagnostic::warning(
                            "bundle-duplicate-slug",
                            format!(
                                "slug `{}` is already used by {}; {} is ignored",
                                parsed.slug, first.file_name, file_name
                            ),
                        )
                        .with_path(&file_name),
                    );
                    continue;
                }
                set.bundles.push(Bundle {
                    slug: parsed.slug,
                    name: parsed.name,
                    description: parsed.description,
                    skills: parsed.skills,
                    instruction: parsed.instruction,
                    metadata: parsed.metadata,
                    path,
                    file_name,
                });
            }
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::index::tests_support::dirs_in;
    use std::fs;

    fn parse(text: &str) -> Result<ParsedBundle, Diagnostic> {
        parse_bundle_text(text, "stem")
    }

    fn code(text: &str) -> String {
        parse(text).unwrap_err().code
    }

    const HERMES: &str = "name: hermes-only\nskills:\n  - code-review\n  - pytest-fixtures\n  - alpha-root-skill\ninstruction: |\n  Do it.\n\n";

    #[test]
    fn hermes_only_file() {
        let b = parse(HERMES).unwrap();
        assert_eq!(b.slug, "hermes-only");
        assert_eq!(b.name, "hermes-only");
        assert_eq!(b.description, "Load 3 skills as a bundle");
        assert_eq!(
            b.skills,
            vec!["code-review", "pytest-fixtures", "alpha-root-skill"]
        );
        assert_eq!(b.instruction.as_deref(), Some("Do it."));
        assert_eq!(b.metadata, BundleMetadata::default());
        assert!(b.warnings.is_empty());
    }

    #[test]
    fn bom_and_crlf() {
        let t = format!("\u{feff}{}", HERMES.replace('\n', "\r\n"));
        let b = parse(&t).unwrap();
        assert_eq!(b.skills.len(), 3);
        assert_eq!(b.instruction.as_deref(), Some("Do it."));
    }

    #[test]
    fn name_rules() {
        assert_eq!(parse("skills: [a]\n").unwrap().name, "stem");
        assert_eq!(parse("name: null\nskills: [a]\n").unwrap().name, "stem");
        assert_eq!(parse("name: ~\nskills: [a]\n").unwrap().name, "stem");
        assert_eq!(code("name: ''\nskills: [a]\n"), "bundle-name-blank");
        assert_eq!(code("name: '   '\nskills: [a]\n"), "bundle-name-blank");
        assert_eq!(code("name: [x]\nskills: [a]\n"), "bundle-name-blank");
        let b = parse("name: 42\nskills: [a]\n").unwrap();
        assert_eq!((b.name.as_str(), b.slug.as_str()), ("42", "42"));
        assert_eq!(
            parse("name: '  Backend Dev '\nskills: [a]\n").unwrap().slug,
            "backend-dev"
        );
        assert_eq!(code("name: '!!!'\nskills: [a]\n"), "bundle-empty-slug");
        assert!(parse_bundle_text("skills: [a]\n", "  ").is_err());
    }

    #[test]
    fn description_and_instruction() {
        let b = parse("skills: [a, b]\ndescription: '  Hi.  '\ninstruction: '   '\n").unwrap();
        assert_eq!(b.description, "Hi.");
        assert_eq!(b.instruction, None);
        let b = parse("skills: [a]\ndescription: ''\n").unwrap();
        assert_eq!(b.description, "Load 1 skills as a bundle");
        let b = parse("skills: [a]\ndescription: null\n").unwrap();
        assert_eq!(b.description, "Load 1 skills as a bundle");
    }

    #[test]
    fn skills_rules() {
        for t in [
            "name: x\n",
            "skills: a\n",
            "skills: []\n",
            "skills: {a: b}\n",
        ] {
            let d = parse(t).unwrap_err();
            assert_eq!(d.code, "bundle-no-skills", "{t}");
            assert!(d.message.contains("missing, not a list, or empty"));
        }
        let d = parse("skills: ['  ', '', null]\n").unwrap_err();
        assert_eq!(d.code, "bundle-no-skills");
        assert_eq!(d.message, "empty skills list");
        let b = parse("skills: [7, true, false, 1.5, null, ' x ', {a: b}, [c], s, '']\n").unwrap();
        assert_eq!(b.skills, vec!["7", "true", "false", "1.5", "x", "s"]);
        assert_eq!(b.warnings.len(), 1);
        assert_eq!(b.warnings[0].code, "bundle-entry-ignored");
        assert!(
            b.warnings[0].message.starts_with("2 "),
            "{}",
            b.warnings[0].message
        );
        assert_eq!(b.description, "Load 6 skills as a bundle");
    }

    #[test]
    fn invalid_yaml_non_mapping_and_empty_file() {
        assert_eq!(code("skills: [a\n"), "bundle-invalid-yaml");
        assert_eq!(code("- a\n- b\n"), "bundle-not-mapping");
        assert_eq!(code("just text\n"), "bundle-not-mapping");
        assert_eq!(code(""), "bundle-not-mapping");
        assert_eq!(code("# comment only\n"), "bundle-not-mapping");
    }

    #[test]
    fn metadata_view() {
        let b = parse("skills: [a]\nmetadata:\n  unambiguous:\n    version: 1\n    materialize_instruction: true\n").unwrap();
        assert_eq!(b.metadata.version, Some(1));
        assert_eq!(b.metadata.materialize_instruction, Some(true));
        let b = parse("skills: [a]\nmetadata:\n  unambiguous:\n    version: x\n    materialize_instruction: 3\n").unwrap();
        assert_eq!(b.metadata, BundleMetadata::default());
        let b = parse("skills: [a]\nmetadata: 5\n").unwrap();
        assert_eq!(b.metadata, BundleMetadata::default());
    }

    fn write(dir: &std::path::Path, name: &str, text: &[u8]) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(name), text).unwrap();
    }

    #[test]
    fn load_order_and_duplicates() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        write(&d.bundles, "b.yaml", b"skills: [x]\n");
        write(&d.bundles, "c.yaml", b"skills: [x]\n");
        write(&d.bundles, "a.yml", b"skills: [x]\n");
        write(&d.bundles, "z.txt", b"skills: [x]\n");
        fs::create_dir_all(d.bundles.join("dir.yaml")).unwrap();
        write(&d.bundles, "dup.yaml", b"name: B\nskills: [y]\n");
        write(&d.bundles, ".hidden.yaml", b"skills: [x]\n");
        let set = load_dir(&d);
        let slugs: Vec<&str> = set.bundles.iter().map(|b| b.slug.as_str()).collect();
        assert_eq!(slugs, vec!["hidden", "b", "c", "a"]);
        assert_eq!(set.diagnostics.len(), 1);
        let dup = &set.diagnostics[0];
        assert_eq!(dup.code, "bundle-duplicate-slug");
        assert_eq!(dup.path.as_deref(), Some("dup.yaml"));
        assert!(dup.message.contains("b.yaml"));
        let b = set.get("B").unwrap();
        assert_eq!(b.file_name, "b.yaml");
        assert_eq!(b.path, d.bundles.join("b.yaml"));
        assert_eq!(set.get("b").unwrap().slug, "b");
        assert!(set.get("!!!").is_none());
        assert_eq!(set.slugs(), vec!["a", "b", "c", "hidden"]);
    }

    #[test]
    fn stem_in_both_extensions_yaml_first() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        write(&d.bundles, "a.yml", b"skills: [from-yml]\n");
        write(&d.bundles, "a.yaml", b"skills: [from-yaml]\n");
        let set = load_dir(&d);
        assert_eq!(set.bundles.len(), 1);
        assert_eq!(set.bundles[0].skills, vec!["from-yaml"]);
        assert_eq!(set.diagnostics[0].code, "bundle-duplicate-slug");
    }

    #[test]
    fn broken_files_are_reported_and_skipped() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        write(&d.bundles, "bad.yaml", b"skills: [a\n");
        write(&d.bundles, "latin1.yaml", b"name: caf\xe9\nskills: [a]\n");
        write(&d.bundles, "ok.yaml", b"skills: [a]\n");
        let set = load_dir(&d);
        assert_eq!(set.bundles.len(), 1);
        let codes: Vec<(&str, &str)> = set
            .diagnostics
            .iter()
            .map(|x| (x.code.as_str(), x.path.as_deref().unwrap()))
            .collect();
        assert_eq!(
            codes,
            vec![
                ("bundle-invalid-yaml", "bad.yaml"),
                ("bundle-not-utf8", "latin1.yaml")
            ]
        );
    }

    #[test]
    fn missing_directory_is_empty() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        assert_eq!(load_dir(&d), BundleSet::default());
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_manifest_hybrid() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        write(&d.bundles, "locked.yaml", b"skills: [a]\n");
        write(&d.bundles, "ok.yaml", b"skills: [a]\n");
        let p = d.bundles.join("locked.yaml");
        fs::set_permissions(&p, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read(&p).is_ok() {
            println!("SKIP-ROOT: running as root; chmod 000 does not block reads");
            fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
            return;
        }
        let set = load_dir(&d);
        fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(set.bundles.len(), 1);
        assert_eq!(set.diagnostics[0].code, "bundle-unreadable");
        assert_eq!(set.diagnostics[0].path.as_deref(), Some("locked.yaml"));
    }
}
