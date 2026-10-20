//! Directory resolution (PRD section 9, spec 2) and path expansion.
//!
//! Library code never reads the process environment directly: every lookup
//! goes through the `Env` carried by `Dirs`. Only `main.rs` calls
//! `Env::process()` (through `Dirs::from_process_env`).

use crate::error::{Error, Result};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

type Lookup = dyn Fn(&str) -> Option<OsString> + Send + Sync;

/// An environment-variable lookup. Cheap to clone.
#[derive(Clone)]
pub struct Env(Arc<Lookup>);

impl Env {
    /// The real process environment.
    pub fn process() -> Self {
        Env(Arc::new(|name: &str| std::env::var_os(name)))
    }

    /// A fixed map (tests). A name absent from the map is unset.
    pub fn from_map(map: HashMap<String, OsString>) -> Self {
        Env(Arc::new(move |name: &str| map.get(name).cloned()))
    }

    /// Look up a variable; an empty value counts as unset.
    pub fn get(&self, name: &str) -> Option<OsString> {
        (self.0)(name).filter(|v| !v.is_empty())
    }
}

impl fmt::Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Env(..)")
    }
}

/// The resolved pouch directories.
#[derive(Debug, Clone)]
pub struct Dirs {
    pub config: PathBuf,
    pub bundles: PathBuf,
    pub cache: PathBuf,
    pub data: PathBuf,
    /// Enterprise skill locations of the claude-code adapter (plan P14):
    /// `POUCH_ENTERPRISE_SKILLS_DIRS` (absolute paths separated like `PATH`)
    /// or the platform default.
    pub enterprise_skill_dirs: Vec<PathBuf>,
    pub env: Env,
}

impl Dirs {
    /// Resolve directories from the process environment.
    pub fn from_process_env() -> Result<Self> {
        Self::resolve(Env::process())
    }

    /// Resolve directories through the given lookup.
    pub fn resolve(env: Env) -> Result<Self> {
        let home = || -> Result<PathBuf> {
            env.get("HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .ok_or(Error::NoHomeDir)
        };
        let config = match pouch_var(&env, "POUCH_CONFIG_DIR")? {
            Some(p) => p,
            None => match xdg_var(&env, "XDG_CONFIG_HOME") {
                Some(p) => p.join("pouch"),
                None => home()?.join(".config").join("pouch"),
            },
        };
        let bundles = match pouch_var(&env, "POUCH_BUNDLES_DIR")? {
            Some(p) => p,
            None => config.join("bundles"),
        };
        let cache = match pouch_var(&env, "POUCH_CACHE_DIR")? {
            Some(p) => p,
            None => match xdg_var(&env, "XDG_CACHE_HOME") {
                Some(p) => p.join("pouch"),
                None => home()?.join(".cache").join("pouch"),
            },
        };
        let data = match xdg_var(&env, "XDG_DATA_HOME") {
            Some(p) => p.join("pouch"),
            None => home()?.join(".local").join("share").join("pouch"),
        };
        let enterprise_skill_dirs = enterprise_dirs(&env)?;
        Ok(Dirs {
            config,
            bundles,
            cache,
            data,
            enterprise_skill_dirs,
            env,
        })
    }

    /// The absolute `HOME` through the carried environment.
    pub fn home(&self) -> Result<PathBuf> {
        self.env
            .get("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .ok_or(Error::NoHomeDir)
    }

    /// `<config dir>/config.toml`.
    pub fn config_file(&self) -> PathBuf {
        self.config.join("config.toml")
    }

    /// `<cache dir>/catalog/<alias>.json`.
    pub fn catalog_cache_file(&self, alias: &str) -> PathBuf {
        self.cache.join("catalog").join(format!("{alias}.json"))
    }

    /// `<data dir>/projects.json`: the project index owned by
    /// `project::index` (`{"version": 1, "projects": [...]}`).
    pub fn project_index_file(&self) -> PathBuf {
        self.data.join("projects.json")
    }

    /// `<bundles dir>/<slug>.yaml`, after validating that `slug` is a single
    /// normal path component (plan P14): non-empty, at most 128 bytes, no
    /// `/`, `\`, NUL, or control character, no leading `.`.
    pub fn bundle_file(&self, slug: &str) -> Result<PathBuf> {
        validate_slug(slug)?;
        Ok(self.bundles.join(format!("{slug}.yaml")))
    }

    /// `<cache dir>/edit`: where `bundle edit` keeps its temp copies.
    pub fn edit_dir(&self) -> PathBuf {
        self.cache.join("edit")
    }

    /// Expand `~`, `$NAME`, and `${NAME}` through the carried environment.
    pub fn expand(&self, raw: &str) -> Result<PathBuf> {
        expand_path(raw, &self.env)
    }
}

/// Maximum slug length in UTF-8 bytes (plan P14).
pub const MAX_SLUG_BYTES: usize = 128;

fn validate_slug(slug: &str) -> Result<()> {
    let invalid = |reason: &str| Error::InvalidBundleName {
        name: slug.to_string(),
        reason: reason.to_string(),
    };
    if slug.is_empty() {
        return Err(invalid("empty name"));
    }
    if slug.len() > MAX_SLUG_BYTES {
        return Err(invalid("longer than 128 bytes"));
    }
    if slug.chars().any(|c| c.is_control()) {
        return Err(invalid("control character"));
    }
    if slug.contains('/') || slug.contains('\\') {
        return Err(invalid("path separator"));
    }
    if slug.starts_with('.') {
        return Err(invalid("leading `.`"));
    }
    let mut parts = std::path::Path::new(slug).components();
    match (parts.next(), parts.next()) {
        (Some(std::path::Component::Normal(_)), None) => Ok(()),
        _ => Err(invalid("not a single path component")),
    }
}

/// The environment variable overriding the enterprise skill locations.
pub const ENTERPRISE_SKILLS_VAR: &str = "POUCH_ENTERPRISE_SKILLS_DIRS";

/// Platform default enterprise skill locations (an Assumption, plan P14).
pub fn default_enterprise_skill_dirs() -> Vec<PathBuf> {
    if cfg!(target_os = "macos") {
        vec![PathBuf::from(
            "/Library/Application Support/ClaudeCode/.claude/skills",
        )]
    } else if cfg!(target_os = "linux") {
        vec![PathBuf::from("/etc/claude-code/.claude/skills")]
    } else {
        Vec::new()
    }
}

fn enterprise_dirs(env: &Env) -> Result<Vec<PathBuf>> {
    let Some(value) = env.get(ENTERPRISE_SKILLS_VAR) else {
        return Ok(default_enterprise_skill_dirs());
    };
    let mut out = Vec::new();
    for p in std::env::split_paths(&value) {
        if p.as_os_str().is_empty() {
            continue;
        }
        if !p.is_absolute() {
            return Err(Error::InvalidEnvPath {
                name: ENTERPRISE_SKILLS_VAR.to_string(),
                value: p.to_string_lossy().into_owned(),
            });
        }
        out.push(p);
    }
    Ok(out)
}

fn pouch_var(env: &Env, name: &str) -> Result<Option<PathBuf>> {
    match env.get(name) {
        None => Ok(None),
        Some(v) => {
            let p = PathBuf::from(&v);
            if p.is_absolute() {
                Ok(Some(p))
            } else {
                Err(Error::InvalidEnvPath {
                    name: name.to_string(),
                    value: v.to_string_lossy().into_owned(),
                })
            }
        }
    }
}

fn xdg_var(env: &Env, name: &str) -> Option<PathBuf> {
    env.get(name).map(PathBuf::from).filter(|p| p.is_absolute())
}

fn lookup_var(env: &Env, name: &str, raw: &str) -> Result<String> {
    env.get(name)
        .map(|v| v.to_string_lossy().into_owned())
        .ok_or_else(|| Error::ConfigUnsetVariable {
            name: name.to_string(),
            raw: raw.to_string(),
        })
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Expand a leading `~` or `~/`, then `$NAME` and `${NAME}` anywhere.
/// An unset variable is `ConfigUnsetVariable`.
pub fn expand_path(raw: &str, env: &Env) -> Result<PathBuf> {
    let mut out = String::new();
    let rest = if raw == "~" {
        out.push_str(&lookup_var(env, "HOME", raw)?);
        ""
    } else if let Some(tail) = raw.strip_prefix("~/") {
        out.push_str(&lookup_var(env, "HOME", raw)?);
        out.push('/');
        tail
    } else {
        raw
    };
    let chars: Vec<char> = rest.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '$' && i + 1 < chars.len() {
            if chars[i + 1] == '{' {
                if let Some(end) = chars[i + 2..].iter().position(|&ch| ch == '}') {
                    let name: String = chars[i + 2..i + 2 + end].iter().collect();
                    if !name.is_empty() && name.chars().all(is_name_char) {
                        out.push_str(&lookup_var(env, &name, raw)?);
                        i += end + 3;
                        continue;
                    }
                }
            } else if is_name_start(chars[i + 1]) {
                let mut j = i + 1;
                while j < chars.len() && is_name_char(chars[j]) {
                    j += 1;
                }
                let name: String = chars[i + 1..j].iter().collect();
                out.push_str(&lookup_var(env, &name, raw)?);
                i = j;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    Ok(PathBuf::from(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> Env {
        Env::from_map(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), OsString::from(v)))
                .collect(),
        )
    }

    fn dirs(pairs: &[(&str, &str)]) -> Result<Dirs> {
        Dirs::resolve(env(pairs))
    }

    #[test]
    fn home_defaults() {
        let d = dirs(&[("HOME", "/h")]).unwrap();
        assert_eq!(d.config, PathBuf::from("/h/.config/pouch"));
        assert_eq!(d.bundles, PathBuf::from("/h/.config/pouch/bundles"));
        assert_eq!(d.cache, PathBuf::from("/h/.cache/pouch"));
        assert_eq!(d.data, PathBuf::from("/h/.local/share/pouch"));
        assert_eq!(
            d.config_file(),
            PathBuf::from("/h/.config/pouch/config.toml")
        );
        assert_eq!(
            d.catalog_cache_file("fx"),
            PathBuf::from("/h/.cache/pouch/catalog/fx.json")
        );
        assert_eq!(
            d.project_index_file(),
            PathBuf::from("/h/.local/share/pouch/projects.json")
        );
    }

    #[test]
    fn xdg_beats_home() {
        let d = dirs(&[
            ("HOME", "/h"),
            ("XDG_CONFIG_HOME", "/xc"),
            ("XDG_CACHE_HOME", "/xk"),
            ("XDG_DATA_HOME", "/xd"),
        ])
        .unwrap();
        assert_eq!(d.config, PathBuf::from("/xc/pouch"));
        assert_eq!(d.cache, PathBuf::from("/xk/pouch"));
        assert_eq!(d.data, PathBuf::from("/xd/pouch"));
        assert_eq!(d.bundles, PathBuf::from("/xc/pouch/bundles"));
    }

    #[test]
    fn pouch_beats_xdg() {
        let d = dirs(&[
            ("HOME", "/h"),
            ("XDG_CONFIG_HOME", "/xc"),
            ("XDG_CACHE_HOME", "/xk"),
            ("POUCH_CONFIG_DIR", "/uc"),
            ("POUCH_CACHE_DIR", "/uk"),
        ])
        .unwrap();
        assert_eq!(d.config, PathBuf::from("/uc"));
        assert_eq!(d.config_file(), PathBuf::from("/uc/config.toml"));
        assert_eq!(d.cache, PathBuf::from("/uk"));
        assert_eq!(d.bundles, PathBuf::from("/uc/bundles"));
    }

    #[test]
    fn bundles_override() {
        let d = dirs(&[("HOME", "/h"), ("POUCH_BUNDLES_DIR", "/b")]).unwrap();
        assert_eq!(d.bundles, PathBuf::from("/b"));
    }

    #[test]
    fn empty_and_relative_xdg_ignored() {
        let d = dirs(&[
            ("HOME", "/h"),
            ("XDG_CONFIG_HOME", ""),
            ("XDG_CACHE_HOME", "rel/path"),
            ("XDG_DATA_HOME", ""),
        ])
        .unwrap();
        assert_eq!(d.config, PathBuf::from("/h/.config/pouch"));
        assert_eq!(d.cache, PathBuf::from("/h/.cache/pouch"));
        assert_eq!(d.data, PathBuf::from("/h/.local/share/pouch"));
    }

    #[test]
    fn empty_pouch_var_is_unset() {
        let d = dirs(&[("HOME", "/h"), ("POUCH_CONFIG_DIR", "")]).unwrap();
        assert_eq!(d.config, PathBuf::from("/h/.config/pouch"));
    }

    #[test]
    fn relative_pouch_config_dir_errors() {
        let e = dirs(&[("HOME", "/h"), ("POUCH_CONFIG_DIR", "rel")]).unwrap_err();
        assert_eq!(e.code(), "invalid-env-path");
    }

    #[test]
    fn no_home_no_overrides_errors() {
        let e = dirs(&[]).unwrap_err();
        assert_eq!(e.code(), "no-home-dir");
    }

    #[test]
    fn no_home_with_all_overrides_succeeds() {
        let d = dirs(&[
            ("POUCH_CONFIG_DIR", "/uc"),
            ("POUCH_CACHE_DIR", "/uk"),
            ("XDG_DATA_HOME", "/xd"),
        ])
        .unwrap();
        assert_eq!(d.data, PathBuf::from("/xd/pouch"));
    }

    #[test]
    fn expansion_cases() {
        let d = dirs(&[("HOME", "/h"), ("VAR", "/v"), ("SUB", "s")]).unwrap();
        assert_eq!(d.expand("~").unwrap(), PathBuf::from("/h"));
        assert_eq!(d.expand("~/x").unwrap(), PathBuf::from("/h/x"));
        assert_eq!(d.expand("$VAR/a").unwrap(), PathBuf::from("/v/a"));
        assert_eq!(d.expand("${VAR}/a").unwrap(), PathBuf::from("/v/a"));
        assert_eq!(d.expand("/p/${SUB}x").unwrap(), PathBuf::from("/p/sx"));
        assert_eq!(d.expand("/plain").unwrap(), PathBuf::from("/plain"));
        assert_eq!(d.expand("/a~b").unwrap(), PathBuf::from("/a~b"));
        assert_eq!(d.expand("/cost$").unwrap(), PathBuf::from("/cost$"));
        assert_eq!(d.expand("/x/$1").unwrap(), PathBuf::from("/x/$1"));
    }

    #[test]
    fn bundle_file_validates_slug() {
        let d = dirs(&[("HOME", "/h"), ("POUCH_BUNDLES_DIR", "/b")]).unwrap();
        assert_eq!(
            d.bundle_file("backend-dev").unwrap(),
            PathBuf::from("/b/backend-dev.yaml")
        );
        assert_eq!(
            d.bundle_file("caf\u{e9}").unwrap(),
            PathBuf::from("/b/caf\u{e9}.yaml")
        );
        let long = "a".repeat(129);
        for bad in [
            "a/b", "..", ".", ".hidden", "", "a\u{7}b", "a\0b", "a\\b", &long,
        ] {
            let e = d.bundle_file(bad).unwrap_err();
            assert_eq!(e.code(), "invalid-bundle-name", "{bad:?}");
            assert_eq!(e.exit_code(), 2);
        }
        assert!(d.bundle_file(&"a".repeat(128)).is_ok());
    }

    #[test]
    fn enterprise_default_per_platform() {
        let d = dirs(&[("HOME", "/h")]).unwrap();
        assert_eq!(d.enterprise_skill_dirs, default_enterprise_skill_dirs());
        if cfg!(target_os = "macos") {
            assert_eq!(
                d.enterprise_skill_dirs,
                vec![PathBuf::from(
                    "/Library/Application Support/ClaudeCode/.claude/skills"
                )]
            );
        } else if cfg!(target_os = "linux") {
            assert_eq!(
                d.enterprise_skill_dirs,
                vec![PathBuf::from("/etc/claude-code/.claude/skills")]
            );
        }
    }

    #[test]
    fn enterprise_override_with_two_entries_and_relative_rejected() {
        let d = dirs(&[("HOME", "/h"), (ENTERPRISE_SKILLS_VAR, "/e1:/e2")]).unwrap();
        assert_eq!(
            d.enterprise_skill_dirs,
            vec![PathBuf::from("/e1"), PathBuf::from("/e2")]
        );
        let e = dirs(&[("HOME", "/h"), (ENTERPRISE_SKILLS_VAR, "/e1:rel")]).unwrap_err();
        assert_eq!(e.code(), "invalid-env-path");
    }

    #[test]
    fn home_set_and_unset() {
        let d = dirs(&[("HOME", "/h")]).unwrap();
        assert_eq!(d.home().unwrap(), PathBuf::from("/h"));
        let d = dirs(&[
            ("POUCH_CONFIG_DIR", "/uc"),
            ("POUCH_CACHE_DIR", "/uk"),
            ("XDG_DATA_HOME", "/xd"),
        ])
        .unwrap();
        assert_eq!(d.home().unwrap_err().code(), "no-home-dir");
    }

    #[test]
    fn edit_dir_under_cache() {
        let d = dirs(&[("HOME", "/h"), ("POUCH_CACHE_DIR", "/k")]).unwrap();
        assert_eq!(d.edit_dir(), PathBuf::from("/k/edit"));
    }

    #[test]
    fn unset_variable_errors() {
        let d = dirs(&[("HOME", "/h")]).unwrap();
        assert_eq!(
            d.expand("$NOPE/x").unwrap_err().code(),
            "config-unset-variable"
        );
        assert_eq!(
            d.expand("${NOPE}").unwrap_err().code(),
            "config-unset-variable"
        );
        let nohome = Env::from_map(HashMap::new());
        assert_eq!(
            expand_path("~/x", &nohome).unwrap_err().code(),
            "config-unset-variable"
        );
    }
}
