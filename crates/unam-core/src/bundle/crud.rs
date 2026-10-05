//! Bundle CRUD (spec 4.4, 5; M2 plan D6, P10 to P14, P21, P22): save,
//! duplicate, delete, add and remove entries (line editor or `--rewrite`),
//! and `$EDITOR` editing. Every write goes through `fsops::write_atomic`
//! after the new text has been reparsed and verified; a manifest that is a
//! symbolic link is never written through.

use super::edit::{self, EditError, yaml_scalar};
use super::{Bundle, BundleSet, parse_bundle_text, slugify};
use crate::catalog::{Catalog, canonical_ref_text};
use crate::error::{Error, Result};
use crate::fsops::write_atomic;
use crate::paths::Dirs;
use crate::resolve::{resolve_entries, resolve_text};
use serde_norway::Value;
use std::path::{Path, PathBuf};

/// Result of `save` and `duplicate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveOutcome {
    pub slug: String,
    pub name: String,
    pub path: PathBuf,
    pub overwritten: bool,
}

/// Result of `delete`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteOutcome {
    pub slug: String,
    pub path: PathBuf,
}

/// Result of `add_refs` and `remove_refs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntriesOutcome {
    pub slug: String,
    pub path: PathBuf,
    pub added: Vec<String>,
    pub skipped: Vec<String>,
    /// Removed entry values, one per removed line.
    pub removed: Vec<String>,
    pub skill_count: usize,
    pub rewrite: bool,
}

/// Result of `edit_in_editor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditOutcome {
    pub slug: String,
    pub path: PathBuf,
    pub changed: bool,
}

fn not_found(name: &str) -> Error {
    let slug = slugify(name);
    Error::BundleNotFound {
        slug: if slug.is_empty() {
            name.trim().to_string()
        } else {
            slug
        },
    }
}

fn find<'a>(set: &'a BundleSet, name: &str) -> Result<&'a Bundle> {
    set.get(name).ok_or_else(|| not_found(name))
}

/// P21: refuse to write through a symbolic link.
fn refuse_symlink(path: &Path) -> Result<()> {
    if let Ok(meta) = std::fs::symlink_metadata(path)
        && meta.file_type().is_symlink()
    {
        return Err(Error::BundleEditUnsupported {
            path: path.to_path_buf(),
            reason: "the manifest is a symbolic link; edit its target directly (`--rewrite` does not bypass this)".to_string(),
        });
    }
    Ok(())
}

fn validated_name(name: &str) -> Result<(String, String)> {
    let invalid = |reason: &str| Error::InvalidBundleName {
        name: name.to_string(),
        reason: reason.to_string(),
    };
    if name.chars().any(char::is_control) {
        return Err(invalid("control character"));
    }
    let name = name.trim();
    let slug = slugify(name);
    if slug.is_empty() {
        return Err(invalid("no letters or digits (the slug is empty)"));
    }
    Ok((name.to_string(), slug))
}

/// Render a new manifest: `name:`, `skills:`, one `  - <scalar>` line per
/// skill (LF, no BOM). Names or entries with control characters are refused.
pub fn render_manifest(name: &str, skills: &[String]) -> Result<String> {
    if name.chars().any(char::is_control) {
        return Err(Error::InvalidBundleName {
            name: name.to_string(),
            reason: "control character".to_string(),
        });
    }
    if let Some(bad) = skills.iter().find(|s| s.chars().any(char::is_control)) {
        return Err(Error::InvalidRef {
            reference: bad.clone(),
            reason: "control character".to_string(),
        });
    }
    let mut out = format!("name: {}\nskills:\n", yaml_scalar(name));
    for s in skills {
        out.push_str(&format!("  - {}\n", yaml_scalar(s)));
    }
    Ok(out)
}

/// Where a bundle with `slug` is written, honoring `force` (plan P14).
/// Returns (path, overwritten).
fn target_for(dirs: &Dirs, set: &BundleSet, slug: &str, force: bool) -> Result<(PathBuf, bool)> {
    let default = dirs.bundle_file(slug)?;
    let existing = set
        .bundles
        .iter()
        .find(|b| b.slug == slug)
        .map(|b| b.path.clone());
    let occupied = existing.clone().or_else(|| {
        std::fs::symlink_metadata(&default)
            .is_ok()
            .then(|| default.clone())
    });
    match occupied {
        None => Ok((default, false)),
        Some(path) if !force => Err(Error::BundleExists {
            slug: slug.to_string(),
            path,
        }),
        Some(path) => {
            refuse_symlink(&path)?;
            Ok((path, true))
        }
    }
}

/// Save a new bundle with `skills` (`bundle save`, plan P10, P14).
pub fn save(
    dirs: &Dirs,
    set: &BundleSet,
    name: &str,
    skills: &[String],
    force: bool,
) -> Result<SaveOutcome> {
    let (name, slug) = validated_name(name)?;
    let (path, overwritten) = target_for(dirs, set, &slug, force)?;
    if skills.is_empty() {
        return Err(Error::BundleInvalid {
            path,
            message: "a bundle needs at least one skill".to_string(),
        });
    }
    let text = render_manifest(&name, skills)?;
    let parsed = parse_bundle_text(&text, &slug).map_err(|d| Error::BundleEditFailed {
        path: path.clone(),
        reason: d.message,
    })?;
    let cleaned: Vec<String> = skills
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if parsed.slug != slug || parsed.name != name || parsed.skills != cleaned {
        return Err(Error::BundleEditFailed {
            path,
            reason: "the rendered manifest does not reparse to the same bundle".to_string(),
        });
    }
    write_atomic(&path, text.as_bytes())?;
    Ok(SaveOutcome {
        slug,
        name,
        path,
        overwritten,
    })
}

fn line_term(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// Replace the top-level `name:` line, or insert one first (after a BOM).
fn with_name(text: &str, name: &str) -> String {
    let (bom, body) = match text.strip_prefix('\u{feff}') {
        Some(r) => ("\u{feff}", r),
        None => ("", text),
    };
    let new_line = format!("name: {}", yaml_scalar(name));
    let mut out = String::from(bom);
    let mut replaced = false;
    for line in body.split_inclusive('\n') {
        if !replaced && line.starts_with("name:") {
            let term = if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            out.push_str(&new_line);
            out.push_str(term);
            replaced = true;
        } else {
            out.push_str(line);
        }
    }
    if !replaced {
        return format!("{bom}{new_line}{}{body}", line_term(body));
    }
    out
}

/// Copy a bundle under a new name (`bundle duplicate`).
pub fn duplicate(
    dirs: &Dirs,
    set: &BundleSet,
    source: &str,
    new_name: &str,
    force: bool,
) -> Result<SaveOutcome> {
    let src = find(set, source)?;
    let (name, slug) = validated_name(new_name)?;
    let (path, overwritten) = target_for(dirs, set, &slug, force)?;
    let text = std::fs::read_to_string(&src.path)
        .map_err(|e| Error::io(format!("cannot read {}", src.path.display()), e))?;
    let new_text = with_name(&text, &name);
    match parse_bundle_text(&new_text, &slug) {
        Ok(p) if p.name == name && p.slug == slug && p.skills == src.skills => {}
        Ok(_) => {
            return Err(Error::BundleEditFailed {
                path,
                reason: "the copy does not reparse to the new name with the same skills"
                    .to_string(),
            });
        }
        Err(d) => {
            return Err(Error::BundleEditFailed {
                path,
                reason: d.message,
            });
        }
    }
    write_atomic(&path, new_text.as_bytes())?;
    Ok(SaveOutcome {
        slug,
        name,
        path,
        overwritten,
    })
}

/// Delete exactly the loaded manifest file (plan P22).
pub fn delete(dirs: &Dirs, set: &BundleSet, name: &str) -> Result<DeleteOutcome> {
    let b = find(set, name)?;
    let failed = |reason: &str| Error::BundleEditFailed {
        path: b.path.clone(),
        reason: reason.to_string(),
    };
    if b.path.parent() != Some(dirs.bundles.as_path()) {
        return Err(failed(
            "the manifest is not directly inside the bundles directory",
        ));
    }
    let meta = std::fs::symlink_metadata(&b.path)
        .map_err(|e| Error::io(format!("cannot inspect {}", b.path.display()), e))?;
    let ft = meta.file_type();
    if !(ft.is_file() || ft.is_symlink()) {
        return Err(failed(
            "the manifest is not a regular file or a symbolic link",
        ));
    }
    std::fs::remove_file(&b.path)
        .map_err(|e| Error::io(format!("cannot delete {}", b.path.display()), e))?;
    Ok(DeleteOutcome {
        slug: b.slug.clone(),
        path: b.path.clone(),
    })
}

fn unsupported(path: &Path) -> impl Fn(EditError) -> Error + '_ {
    move |e| Error::BundleEditUnsupported {
        path: path.to_path_buf(),
        reason: e.0,
    }
}

fn read_manifest(b: &Bundle) -> Result<String> {
    std::fs::read_to_string(&b.path)
        .map_err(|e| Error::io(format!("cannot read {}", b.path.display()), e))
}

fn file_stem(b: &Bundle) -> String {
    Path::new(&b.file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn top_level_without_skills(text: &str) -> Option<Value> {
    let t = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut v: Value = serde_norway::from_str(t).ok()?;
    v.as_mapping_mut()?.remove("skills");
    Some(v)
}

/// Verify an edit (plan D3): the new text parses to the same slug and name,
/// `skills` equals `expected`, and every other top-level key is unchanged.
fn verify_edit(b: &Bundle, old: &str, new: &str, expected: &[String]) -> Result<()> {
    let failed = |reason: String| Error::BundleEditFailed {
        path: b.path.clone(),
        reason,
    };
    let parsed = parse_bundle_text(new, &file_stem(b)).map_err(|d| failed(d.message))?;
    if parsed.slug != b.slug || parsed.name != b.name {
        return Err(failed("the edit would change the bundle name".to_string()));
    }
    if parsed.skills != expected {
        return Err(failed(format!(
            "the edited `skills` list does not match the expected list ({} entries instead of {})",
            parsed.skills.len(),
            expected.len()
        )));
    }
    if top_level_without_skills(old) != top_level_without_skills(new) {
        return Err(failed(
            "the edit would change another top-level key".to_string(),
        ));
    }
    Ok(())
}

/// Full rewrite (`--rewrite`): set `skills` and reserialize; the BOM and the
/// line ending are kept, comments are lost.
fn rewrite(b: &Bundle, text: &str, skills: &[String]) -> Result<String> {
    let bom = text.starts_with('\u{feff}');
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut v: Value = serde_norway::from_str(body).map_err(|e| Error::BundleEditFailed {
        path: b.path.clone(),
        reason: e.to_string(),
    })?;
    let Some(map) = v.as_mapping_mut() else {
        return Err(Error::BundleEditFailed {
            path: b.path.clone(),
            reason: "the manifest is not a mapping".to_string(),
        });
    };
    map.insert(
        Value::String("skills".to_string()),
        Value::Sequence(skills.iter().map(|s| Value::String(s.clone())).collect()),
    );
    let mut out = serde_norway::to_string(&v).map_err(|e| Error::BundleEditFailed {
        path: b.path.clone(),
        reason: e.to_string(),
    })?;
    if line_term(body) == "\r\n" {
        out = out.replace('\n', "\r\n");
    }
    if bom {
        out.insert(0, '\u{feff}');
    }
    Ok(out)
}

fn clean(values: impl IntoIterator<Item = String>) -> Vec<String> {
    values
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Add references to a bundle (`bundle add`, plan P11): each reference must
/// resolve; one already present (same canonical text) is skipped.
pub fn add_refs(
    _dirs: &Dirs,
    set: &BundleSet,
    catalog: &Catalog,
    name: &str,
    refs: &[String],
    rewrite_all: bool,
) -> Result<EntriesOutcome> {
    let b = find(set, name)?;
    refuse_symlink(&b.path)?;
    let existing: Vec<String> = b.skills.iter().map(|s| canonical_ref_text(s)).collect();
    let mut to_add: Vec<String> = Vec::new();
    let mut skipped = Vec::new();
    for r in refs {
        if r.chars().any(char::is_control) {
            return Err(Error::InvalidRef {
                reference: r.clone(),
                reason: "control character".to_string(),
            });
        }
        resolve_text(catalog, r)?;
        let canon = canonical_ref_text(r);
        if existing.contains(&canon) || to_add.contains(&canon) {
            skipped.push(canon);
        } else {
            to_add.push(canon);
        }
    }
    let outcome = |added: Vec<String>, count: usize| EntriesOutcome {
        slug: b.slug.clone(),
        path: b.path.clone(),
        added,
        skipped: skipped.clone(),
        removed: Vec::new(),
        skill_count: count,
        rewrite: rewrite_all,
    };
    if to_add.is_empty() {
        return Ok(outcome(Vec::new(), b.skills.len()));
    }
    let text = read_manifest(b)?;
    let mut expected = b.skills.clone();
    expected.extend(to_add.iter().cloned());
    let new_text = if rewrite_all {
        rewrite(b, &text, &expected)?
    } else {
        edit::add_values(&text, &to_add)
            .map_err(unsupported(&b.path))?
            .0
    };
    verify_edit(b, &text, &new_text, &expected)?;
    write_atomic(&b.path, new_text.as_bytes())?;
    Ok(outcome(to_add, expected.len()))
}

/// Remove references from a bundle (`bundle rm <name> <ref>...`), matching
/// by canonical text first, then by resolved `SkillId`.
pub fn remove_refs(
    _dirs: &Dirs,
    set: &BundleSet,
    catalog: &Catalog,
    name: &str,
    refs: &[String],
    rewrite_all: bool,
) -> Result<EntriesOutcome> {
    let b = find(set, name)?;
    refuse_symlink(&b.path)?;
    let text = read_manifest(b)?;
    // Raw edit-layer values (line edit) or the loaded list (rewrite).
    let entries: Vec<String> = if rewrite_all {
        b.skills.clone()
    } else {
        edit::list_entries(&text).map_err(unsupported(&b.path))?
    };
    let probe = Bundle {
        skills: entries.clone(),
        ..b.clone()
    };
    let resolved = resolve_entries(&probe, catalog);
    let mut matched: Vec<String> = Vec::new();
    for r in refs {
        let canon = canonical_ref_text(r);
        let mut hits: Vec<&String> = entries
            .iter()
            .filter(|e| !e.trim().is_empty() && canonical_ref_text(e) == canon)
            .collect();
        if hits.is_empty() {
            let id = resolve_text(catalog, r)?;
            hits = resolved
                .iter()
                .zip(&entries)
                .filter(|(res, _)| res.outcome.as_ref().ok() == Some(&id))
                .map(|(_, e)| e)
                .collect();
        }
        if hits.is_empty() {
            return Err(Error::SkillNotInBundle {
                bundle: b.slug.clone(),
                reference: r.trim().to_string(),
            });
        }
        for h in hits {
            if !matched.contains(h) {
                matched.push(h.clone());
            }
        }
    }
    let expected = clean(entries.iter().filter(|e| !matched.contains(e)).cloned());
    if expected.is_empty() {
        return Err(Error::BundleWouldBeEmpty {
            bundle: b.slug.clone(),
        });
    }
    let (new_text, removed) = if rewrite_all {
        let removed: Vec<String> = entries
            .iter()
            .filter(|e| matched.contains(e))
            .cloned()
            .collect();
        (rewrite(b, &text, &expected)?, removed)
    } else {
        edit::remove_values(&text, &matched).map_err(unsupported(&b.path))?
    };
    verify_edit(b, &text, &new_text, &expected)?;
    write_atomic(&b.path, new_text.as_bytes())?;
    Ok(EntriesOutcome {
        slug: b.slug.clone(),
        path: b.path.clone(),
        added: Vec::new(),
        skipped: Vec::new(),
        removed,
        skill_count: expected.len(),
        rewrite: rewrite_all,
    })
}

/// Edit a bundle in `$EDITOR` (plan P13): the editor works on a temp copy
/// under `<cache dir>/edit/`; only a valid result with the same slug is
/// committed. `editor` is the `EDITOR` value read through `Dirs.env`.
pub fn edit_in_editor(
    dirs: &Dirs,
    set: &BundleSet,
    name: &str,
    editor: Option<&str>,
) -> Result<EditOutcome> {
    let b = find(set, name)?;
    refuse_symlink(&b.path)?;
    let parts: Vec<&str> = editor
        .map(|e| e.split_whitespace().collect())
        .unwrap_or_default();
    let Some((program, args)) = parts.split_first() else {
        return Err(Error::EditorNotSet);
    };
    let original = std::fs::read(&b.path)
        .map_err(|e| Error::io(format!("cannot read {}", b.path.display()), e))?;
    let edit_dir = dirs.edit_dir();
    std::fs::create_dir_all(&edit_dir)
        .map_err(|e| Error::io(format!("cannot create {}", edit_dir.display()), e))?;
    let tmp = tempfile::Builder::new()
        .prefix(&format!("{}-", b.slug))
        .suffix(".yaml")
        .tempfile_in(&edit_dir)
        .map_err(|e| {
            Error::io(
                format!("cannot create a temp file in {}", edit_dir.display()),
                e,
            )
        })?;
    std::fs::write(tmp.path(), &original)
        .map_err(|e| Error::io(format!("cannot write {}", tmp.path().display()), e))?;
    let command = parts.join(" ");
    let status = std::process::Command::new(program)
        .args(args)
        .arg(tmp.path())
        .status()
        .map_err(|e| Error::EditorFailed {
            command: command.clone(),
            detail: e.to_string(),
        })?;
    if !status.success() {
        return Err(Error::EditorFailed {
            command,
            detail: format!("exited with {status}"),
        });
    }
    let edited = std::fs::read(tmp.path())
        .map_err(|e| Error::io(format!("cannot read {}", tmp.path().display()), e))?;
    drop(tmp);
    if edited == original {
        return Ok(EditOutcome {
            slug: b.slug.clone(),
            path: b.path.clone(),
            changed: false,
        });
    }
    let invalid = |message: String| Error::BundleInvalid {
        path: b.path.clone(),
        message,
    };
    let text = String::from_utf8(edited)
        .map_err(|_| invalid("the edited text is not valid UTF-8".to_string()))?;
    let parsed = parse_bundle_text(&text, &file_stem(b)).map_err(|d| invalid(d.message))?;
    if parsed.slug != b.slug {
        return Err(invalid(format!(
            "the edit changes the slug from `{}` to `{}`; renaming a bundle is not supported",
            b.slug, parsed.slug
        )));
    }
    refuse_symlink(&b.path)?;
    write_atomic(&b.path, text.as_bytes())?;
    Ok(EditOutcome {
        slug: b.slug.clone(),
        path: b.path.clone(),
        changed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::load_dir;
    use crate::catalog::tests_support::catalog_of;
    use crate::project::index::tests_support::dirs_in;
    use std::fs;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn cat() -> Catalog {
        catalog_of(
            &[
                ("fx", "sdlc/code-review", "code-review"),
                ("fx", "testing/pytest-fixtures", "pytest-fixtures"),
                ("fx", "alpha-root-skill", "alpha-root-skill"),
                ("fx", "data/schema-migration", "schema-migration"),
                ("fx", "a/error-handling", "error-handling"),
                ("fx", "b/error-handling", "error-handling"),
            ],
            &[],
        )
    }

    struct Fx {
        _t: tempfile::TempDir,
        d: Dirs,
    }

    impl Fx {
        fn new() -> Fx {
            let t = tempfile::tempdir().unwrap();
            let d = dirs_in(t.path());
            fs::create_dir_all(&d.bundles).unwrap();
            Fx { _t: t, d }
        }
        fn write(&self, name: &str, text: &str) -> PathBuf {
            let p = self.d.bundles.join(name);
            fs::write(&p, text).unwrap();
            p
        }
        fn set(&self) -> BundleSet {
            load_dir(&self.d)
        }
    }

    const COMMENTED: &str = "# head\nname: commented\nskills:\n  - fx:sdlc/code-review  # main\n  - \"fx:testing/pytest-fixtures\"\n  - alpha-root-skill\nextra: {keep: 1}\n";

    #[test]
    fn render_manifest_shape_and_round_trip() {
        let text = render_manifest("My Set", &s(&["fx:a/b", "123", "yes", "a: b"])).unwrap();
        assert!(
            text.starts_with("name: My Set\nskills:\n  - fx:a/b\n"),
            "{text}"
        );
        let p = parse_bundle_text(&text, "x").unwrap();
        assert_eq!(p.skills, s(&["fx:a/b", "123", "yes", "a: b"]));
        assert_eq!(
            render_manifest("a\nb", &s(&["x"])).unwrap_err().code(),
            "invalid-bundle-name"
        );
        assert_eq!(
            render_manifest("a", &s(&["x\u{7}"])).unwrap_err().code(),
            "invalid-ref"
        );
    }

    #[test]
    fn save_rules() {
        let f = Fx::new();
        let out = save(&f.d, &f.set(), "My Set", &s(&["fx:a/b"]), false).unwrap();
        assert_eq!(out.slug, "my-set");
        assert_eq!(out.path, f.d.bundles.join("my-set.yaml"));
        assert!(!out.overwritten);
        // Exists without force.
        let e = save(&f.d, &f.set(), "my set", &s(&["x"]), false).unwrap_err();
        assert_eq!((e.code(), e.exit_code()), ("bundle-exists", 2));
        // Force overwrites at the existing file name.
        f.write("other name.yaml", "name: Other\nskills: [a]\n");
        let out = save(&f.d, &f.set(), "OTHER", &s(&["z"]), true).unwrap();
        assert!(out.overwritten);
        assert_eq!(out.path, f.d.bundles.join("other name.yaml"));
        assert!(!f.d.bundles.join("other.yaml").exists());
        assert_eq!(f.set().get("other").unwrap().skills, s(&["z"]));
        // Invalid names and empty skills.
        for bad in ["!!!", "", "a\u{0}b"] {
            let e = save(&f.d, &f.set(), bad, &s(&["x"]), false).unwrap_err();
            assert_eq!(e.code(), "invalid-bundle-name", "{bad:?}");
        }
        let long = "a".repeat(129);
        assert_eq!(
            save(&f.d, &f.set(), &long, &s(&["x"]), false)
                .unwrap_err()
                .code(),
            "invalid-bundle-name"
        );
        assert_eq!(
            save(&f.d, &f.set(), "empty", &[], false)
                .unwrap_err()
                .code(),
            "bundle-invalid"
        );
        // An invalid `<slug>.yaml` that is not loaded still blocks without force.
        f.write("broken.yaml", "skills: [\n");
        let e = save(&f.d, &f.set(), "broken", &s(&["x"]), false).unwrap_err();
        assert_eq!(e.code(), "bundle-exists");
        assert!(
            save(&f.d, &f.set(), "broken", &s(&["x"]), true)
                .unwrap()
                .overwritten
        );
    }

    #[test]
    fn duplicate_cases() {
        let f = Fx::new();
        f.write("src.yaml", COMMENTED);
        let out = duplicate(&f.d, &f.set(), "commented", "Copy One", false).unwrap();
        assert_eq!(out.slug, "copy-one");
        let text = fs::read_to_string(&out.path).unwrap();
        assert_eq!(text, COMMENTED.replace("name: commented", "name: Copy One"));
        assert_eq!(
            duplicate(&f.d, &f.set(), "commented", "copy one", false)
                .unwrap_err()
                .code(),
            "bundle-exists"
        );
        assert!(
            duplicate(&f.d, &f.set(), "commented", "copy one", true)
                .unwrap()
                .overwritten
        );
        assert_eq!(
            duplicate(&f.d, &f.set(), "nope", "x", false)
                .unwrap_err()
                .code(),
            "bundle-not-found"
        );
        // BOM source without a `name:` key: the inserted line follows the BOM.
        f.write("bom.yml", "\u{feff}skills:\r\n  - a\r\n");
        let out = duplicate(&f.d, &f.set(), "bom", "Bom Two", false).unwrap();
        let text = fs::read_to_string(&out.path).unwrap();
        assert_eq!(text, "\u{feff}name: Bom Two\r\nskills:\r\n  - a\r\n");
        assert_eq!(f.set().get("bom-two").unwrap().skills, s(&["a"]));
    }

    #[test]
    fn delete_removes_exactly_the_loaded_file() {
        let f = Fx::new();
        let target = f.write("Backend Dev.yaml", "name: Backend Dev\nskills: [a]\n");
        let sib1 = f.write("other.yaml", "skills: [a]\n");
        let sib2 = f.write("backend-dev-copy.yaml", "skills: [b]\n");
        let before = (fs::read(&sib1).unwrap(), fs::read(&sib2).unwrap());
        let out = delete(&f.d, &f.set(), "backend-dev").unwrap();
        assert_eq!(out.path, target);
        assert!(!target.exists());
        assert_eq!((fs::read(&sib1).unwrap(), fs::read(&sib2).unwrap()), before);
        assert_eq!(
            delete(&f.d, &f.set(), "backend-dev").unwrap_err().code(),
            "bundle-not-found"
        );
        // A bundle whose path parent is not the bundles dir is refused.
        let mut set = f.set();
        let i = set.bundles.iter().position(|b| b.slug == "other").unwrap();
        set.bundles[i].path = f.d.bundles.join("sub").join("other.yaml");
        assert_eq!(
            delete(&f.d, &set, "other").unwrap_err().code(),
            "bundle-edit-failed"
        );
        assert!(sib1.exists());
    }

    #[cfg(unix)]
    #[test]
    fn delete_unlinks_a_symlink_and_keeps_its_target() {
        let f = Fx::new();
        let outside = f._t.path().join("real.yaml");
        fs::write(&outside, "name: linked\nskills: [a]\n").unwrap();
        std::os::unix::fs::symlink(&outside, f.d.bundles.join("linked.yaml")).unwrap();
        delete(&f.d, &f.set(), "linked").unwrap();
        assert!(fs::symlink_metadata(f.d.bundles.join("linked.yaml")).is_err());
        assert_eq!(
            fs::read_to_string(&outside).unwrap(),
            "name: linked\nskills: [a]\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_manifest_refuses_in_place_writes() {
        let f = Fx::new();
        let outside = f._t.path().join("real.yaml");
        let body = "name: linked\nskills:\n  - code-review\n  - alpha-root-skill\n";
        fs::write(&outside, body).unwrap();
        let link = f.d.bundles.join("linked.yaml");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let c = cat();
        let marker = f._t.path().join("editor-ran");
        let editor = f._t.path().join("ed.sh");
        fs::write(
            &editor,
            format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&editor, fs::Permissions::from_mode(0o755)).unwrap();
        let ed = editor.to_string_lossy().into_owned();
        let errs = vec![
            add_refs(
                &f.d,
                &f.set(),
                &c,
                "linked",
                &s(&["fx:data/schema-migration"]),
                false,
            )
            .unwrap_err(),
            remove_refs(&f.d, &f.set(), &c, "linked", &s(&["code-review"]), false).unwrap_err(),
            remove_refs(&f.d, &f.set(), &c, "linked", &s(&["code-review"]), true).unwrap_err(),
            save(&f.d, &f.set(), "linked", &s(&["x"]), true).unwrap_err(),
            duplicate(&f.d, &f.set(), "linked", "linked", true).unwrap_err(),
            edit_in_editor(&f.d, &f.set(), "linked", Some(&ed)).unwrap_err(),
        ];
        for e in errs {
            assert_eq!((e.code(), e.exit_code()), ("bundle-edit-unsupported", 2));
        }
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&outside).unwrap(), body);
        assert!(!marker.exists(), "the editor must never start");
    }

    #[test]
    fn add_refs_cases() {
        let f = Fx::new();
        let p = f.write("c.yaml", COMMENTED);
        let c = cat();
        let out = add_refs(
            &f.d,
            &f.set(),
            &c,
            "commented",
            &s(&[
                "fx:data/schema-migration/",
                "code-review",
                "fx:alpha-root-skill",
            ]),
            false,
        )
        .unwrap();
        // `code-review` is new text (canonical differs from `fx:sdlc/code-review`).
        assert_eq!(
            out.added,
            s(&[
                "fx:data/schema-migration",
                "code-review",
                "fx:alpha-root-skill"
            ])
        );
        assert_eq!(out.skill_count, 6);
        let text = fs::read_to_string(&p).unwrap();
        assert!(text.contains("  - alpha-root-skill\n  - fx:data/schema-migration\n  - code-review\n  - fx:alpha-root-skill\nextra:"), "{text}");
        // Skipped duplicates; nothing written when nothing is new.
        let before = fs::read(&p).unwrap();
        let out = add_refs(
            &f.d,
            &f.set(),
            &c,
            "commented",
            &s(&["alpha-root-skill", " code-review "]),
            false,
        )
        .unwrap();
        assert!(out.added.is_empty());
        assert_eq!(out.skipped, s(&["alpha-root-skill", "code-review"]));
        assert_eq!(fs::read(&p).unwrap(), before);
        // Abort leaves the file untouched: ambiguous, not found, invalid.
        for (r, code) in [
            ("error-handling", "ambiguous"),
            ("nope", "not-found"),
            ("fx:", "invalid-ref"),
        ] {
            let e = add_refs(
                &f.d,
                &f.set(),
                &c,
                "commented",
                &s(&["fx:sdlc/code-review", r]),
                false,
            )
            .unwrap_err();
            assert_eq!(e.code(), code);
            assert_eq!(fs::read(&p).unwrap(), before);
        }
        assert_eq!(
            add_refs(&f.d, &f.set(), &c, "nope", &s(&["x"]), false)
                .unwrap_err()
                .code(),
            "bundle-not-found"
        );
    }

    #[test]
    fn remove_refs_matching_passes() {
        let f = Fx::new();
        let c = cat();
        let p = f.write("c.yaml", COMMENTED);
        // Pass 1: canonical text.
        let out = remove_refs(
            &f.d,
            &f.set(),
            &c,
            "commented",
            &s(&["fx:testing/pytest-fixtures/"]),
            false,
        )
        .unwrap();
        assert_eq!(out.removed, s(&["fx:testing/pytest-fixtures"]));
        assert_eq!(out.skill_count, 2);
        assert_eq!(
            fs::read_to_string(&p).unwrap(),
            COMMENTED.replace("  - \"fx:testing/pytest-fixtures\"\n", "")
        );
        // Pass 2: equivalent spelling resolving to the same id.
        let out =
            remove_refs(&f.d, &f.set(), &c, "commented", &s(&["code-review"]), false).unwrap();
        assert_eq!(out.removed, s(&["fx:sdlc/code-review"]));
        // Pass 3: not in bundle; would-be-empty; resolution errors.
        let e = remove_refs(
            &f.d,
            &f.set(),
            &c,
            "commented",
            &s(&["fx:data/schema-migration"]),
            false,
        )
        .unwrap_err();
        assert_eq!((e.code(), e.exit_code()), ("skill-not-in-bundle", 4));
        let e = remove_refs(
            &f.d,
            &f.set(),
            &c,
            "commented",
            &s(&["alpha-root-skill"]),
            false,
        )
        .unwrap_err();
        assert_eq!((e.code(), e.exit_code()), ("bundle-would-be-empty", 2));
        let e = remove_refs(
            &f.d,
            &f.set(),
            &c,
            "commented",
            &s(&["error-handling"]),
            false,
        )
        .unwrap_err();
        assert_eq!(e.exit_code(), 5);
        let e = remove_refs(&f.d, &f.set(), &c, "commented", &s(&["nope"]), false).unwrap_err();
        assert_eq!(e.exit_code(), 4);
    }

    #[test]
    fn unsupported_shape_and_rewrite_path() {
        let f = Fx::new();
        let c = cat();
        let flow = "\u{feff}name: flow\r\nskills: [code-review, alpha-root-skill, fx:data/schema-migration]\r\nunknown: {k: v}\r\n";
        let p = f.write("flow.yaml", flow);
        let e = remove_refs(&f.d, &f.set(), &c, "flow", &s(&["code-review"]), false).unwrap_err();
        assert_eq!((e.code(), e.exit_code()), ("bundle-edit-unsupported", 2));
        let e = add_refs(&f.d, &f.set(), &c, "flow", &s(&["pytest-fixtures"]), false).unwrap_err();
        assert_eq!(e.code(), "bundle-edit-unsupported");
        assert_eq!(fs::read_to_string(&p).unwrap(), flow);
        let out = remove_refs(&f.d, &f.set(), &c, "flow", &s(&["code-review"]), true).unwrap();
        assert!(out.rewrite);
        assert_eq!(out.removed, s(&["code-review"]));
        let text = fs::read_to_string(&p).unwrap();
        assert!(text.starts_with('\u{feff}'), "{text:?}");
        assert!(
            text.contains("\r\n") && !text.replace("\r\n", "").contains('\n'),
            "{text:?}"
        );
        let b = f.set();
        let b = b.get("flow").unwrap();
        assert_eq!(
            b.skills,
            s(&["alpha-root-skill", "fx:data/schema-migration"])
        );
        assert!(text.contains("unknown:"), "{text}");
        let out = add_refs(&f.d, &f.set(), &c, "flow", &s(&["pytest-fixtures"]), true).unwrap();
        assert_eq!(out.skill_count, 3);
    }

    #[test]
    fn verification_failure_writes_nothing() {
        let f = Fx::new();
        let b = Bundle {
            slug: "x".into(),
            name: "x".into(),
            description: String::new(),
            skills: s(&["a", "b"]),
            instruction: None,
            metadata: Default::default(),
            path: f.d.bundles.join("x.yaml"),
            file_name: "x.yaml".into(),
        };
        let old = "name: x\nskills: [a, b]\nk: 1\n";
        assert!(verify_edit(&b, old, "name: x\nskills: [a]\nk: 1\n", &s(&["a"])).is_ok());
        let e = verify_edit(&b, old, "name: x\nskills: [a]\nk: 2\n", &s(&["a"])).unwrap_err();
        assert_eq!((e.code(), e.exit_code()), ("bundle-edit-failed", 1));
        assert!(verify_edit(&b, old, "name: y\nskills: [a]\nk: 1\n", &s(&["a"])).is_err());
        assert!(verify_edit(&b, old, "name: x\nskills: [b]\nk: 1\n", &s(&["a"])).is_err());
        assert!(verify_edit(&b, old, "name: x\nskills: [\n", &s(&["a"])).is_err());
    }

    #[cfg(unix)]
    fn script(f: &Fx, name: &str, body: &str) -> String {
        use std::os::unix::fs::PermissionsExt;
        let p = f._t.path().join(name);
        fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        p.to_string_lossy().into_owned()
    }

    #[cfg(unix)]
    #[test]
    fn edit_in_editor_cases() {
        let f = Fx::new();
        let p = f.write("c.yaml", COMMENTED);
        let log = f._t.path().join("log");
        let edit_dir = f.d.edit_dir();
        let empty = |d: &Path| fs::read_dir(d).map(|r| r.count() == 0).unwrap_or(true);
        // Valid change commits; the temp name is logged.
        let ed = script(
            &f,
            "ok.sh",
            &format!(
                "echo \"$1\" >> '{}'\nprintf 'description: Edited.\\n' >> \"$1\"",
                log.display()
            ),
        );
        let out = edit_in_editor(&f.d, &f.set(), "commented", Some(&ed)).unwrap();
        assert!(out.changed);
        assert_eq!(
            fs::read_to_string(&p).unwrap(),
            format!("{COMMENTED}description: Edited.\n")
        );
        assert_eq!(f.set().get("commented").unwrap().description, "Edited.");
        assert!(empty(&edit_dir));
        // Unchanged bytes.
        let noop = script(
            &f,
            "noop.sh",
            &format!("echo \"$1\" >> '{}'", log.display()),
        );
        let before = fs::read(&p).unwrap();
        assert!(
            !edit_in_editor(&f.d, &f.set(), "commented", Some(&noop))
                .unwrap()
                .changed
        );
        let names: Vec<String> = fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        assert_eq!(names.len(), 2);
        assert_ne!(names[0], names[1], "temp names are never reused");
        assert!(
            names[0].contains("/edit/commented-") && names[0].ends_with(".yaml"),
            "{}",
            names[0]
        );
        // Invalid result, non-zero exit, slug change: file unchanged, temp gone.
        let bad = script(&f, "bad.sh", "printf 'skills: [\\n' > \"$1\"");
        let fail = script(
            &f,
            "fail.sh",
            "printf 'name: x\\nskills: [a]\\n' > \"$1\"\nexit 3",
        );
        let rename = script(
            &f,
            "rename.sh",
            "printf 'name: renamed\\nskills: [a]\\n' > \"$1\"",
        );
        for (ed, code) in [
            (&bad, "bundle-invalid"),
            (&fail, "editor-failed"),
            (&rename, "bundle-invalid"),
        ] {
            let e = edit_in_editor(&f.d, &f.set(), "commented", Some(ed)).unwrap_err();
            assert_eq!((e.code(), e.exit_code()), (code, 1));
            assert_eq!(fs::read(&p).unwrap(), before);
            assert!(empty(&edit_dir));
        }
        // Unset editor; missing program.
        assert_eq!(
            edit_in_editor(&f.d, &f.set(), "commented", None)
                .unwrap_err()
                .code(),
            "editor-not-set"
        );
        assert_eq!(
            edit_in_editor(&f.d, &f.set(), "commented", Some("  "))
                .unwrap_err()
                .code(),
            "editor-not-set"
        );
        let missing =
            f._t.path()
                .join("no-such-editor")
                .to_string_lossy()
                .into_owned();
        assert_eq!(
            edit_in_editor(&f.d, &f.set(), "commented", Some(&missing))
                .unwrap_err()
                .code(),
            "editor-failed"
        );
        assert!(empty(&edit_dir));
    }
}
