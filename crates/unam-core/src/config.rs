//! `config.toml` (spec 2.1, 4.1).
//!
//! Rewriting the file does not preserve comments (a `toml` crate limitation,
//! plan P10).

use crate::error::{Diagnostic, Error, Result};
use crate::fsops::{ToolPath, write_atomic};
use crate::paths::Dirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Activation strategy default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    #[default]
    Symlink,
    Copy,
}

/// Destination policy default (applies to agents-neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DestinationPolicy {
    #[default]
    Shared,
    Isolated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Defaults {
    pub strategy: Strategy,
    pub destination_policy: DestinationPolicy,
    pub catalog_budget_tokens: u32,
    pub allow_missing: bool,
}

impl Default for Defaults {
    fn default() -> Self {
        Defaults {
            strategy: Strategy::Symlink,
            destination_policy: DestinationPolicy::Shared,
            catalog_budget_tokens: 1500,
            allow_missing: false,
        }
    }
}

/// One `[[sources]]` entry. `path` is stored as the user spelled it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEntry {
    pub alias: String,
    pub path: String,
    pub precedence: u32,
}

impl SourceEntry {
    /// The path with `~` and environment variables expanded.
    pub fn expanded_path(&self, dirs: &Dirs) -> Result<PathBuf> {
        dirs.expand(&self.path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub sources: Vec<SourceEntry>,
    #[serde(default)]
    pub defaults: Defaults,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            version: 1,
            sources: Vec::new(),
            defaults: Defaults::default(),
        }
    }
}

#[derive(Deserialize)]
struct VersionProbe {
    version: Option<u32>,
}

/// True when `alias` matches `^[a-z0-9][a-z0-9-]{0,31}$`.
pub fn is_valid_alias(alias: &str) -> bool {
    let bytes = alias.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 32
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

impl Config {
    /// Parse and validate config text. Returns diagnostics (precedence ties).
    pub fn parse(text: &str, path: &std::path::Path) -> Result<(Config, Vec<Diagnostic>)> {
        let parse_err = |message: String| Error::ConfigParse {
            path: path.to_path_buf(),
            message,
        };
        let probe: VersionProbe =
            toml::from_str(text).map_err(|e| parse_err(e.message().to_string()))?;
        match probe.version {
            None => return Err(parse_err("missing `version`".to_string())),
            Some(1) => {}
            Some(v) => {
                return Err(Error::ConfigUnsupportedVersion {
                    path: path.to_path_buf(),
                    version: v,
                });
            }
        }
        let config: Config =
            toml::from_str(text).map_err(|e| parse_err(e.message().to_string()))?;
        let mut seen = std::collections::BTreeSet::new();
        for s in &config.sources {
            if !is_valid_alias(&s.alias) {
                return Err(parse_err(format!("invalid alias {:?}", s.alias)));
            }
            if !seen.insert(s.alias.clone()) {
                return Err(parse_err(format!("duplicate alias {:?}", s.alias)));
            }
        }
        let diags = config.tie_diagnostics();
        Ok((config, diags))
    }

    fn tie_diagnostics(&self) -> Vec<Diagnostic> {
        let mut by_prec: std::collections::BTreeMap<u32, Vec<&str>> = Default::default();
        for s in &self.sources {
            by_prec.entry(s.precedence).or_default().push(&s.alias);
        }
        by_prec
            .into_iter()
            .filter(|(_, aliases)| aliases.len() > 1)
            .map(|(prec, mut aliases)| {
                aliases.sort();
                Diagnostic::warning(
                    "precedence-tie",
                    format!(
                        "sources {} share precedence {prec}; ordered alphabetically",
                        aliases.join(", ")
                    ),
                )
            })
            .collect()
    }

    /// Sources ordered by `(precedence, alias)`.
    pub fn ordered_sources(&self) -> Vec<&SourceEntry> {
        let mut v: Vec<&SourceEntry> = self.sources.iter().collect();
        v.sort_by(|a, b| (a.precedence, &a.alias).cmp(&(b.precedence, &b.alias)));
        v
    }

    /// Load `config.toml`, creating the default file on first run.
    pub fn load_or_create(dirs: &Dirs) -> Result<(Config, Vec<Diagnostic>)> {
        let path = dirs.config_file();
        match std::fs::read_to_string(&path) {
            Ok(text) => Config::parse(&text, &path),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let config = Config::default();
                config.save(dirs)?;
                Ok((config, Vec::new()))
            }
            Err(e) => Err(Error::ConfigRead { path, source: e }),
        }
    }

    /// Serialize to TOML text.
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string(self).map_err(|e| Error::ConfigParse {
            path: PathBuf::from("config.toml"),
            message: e.to_string(),
        })
    }

    /// Write the config atomically (comments are not preserved).
    pub fn save(&self, dirs: &Dirs) -> Result<()> {
        write_atomic(
            &ToolPath::new(dirs.config_file()),
            self.to_toml()?.as_bytes(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Env;
    use std::collections::HashMap;
    use std::ffi::OsString;

    fn dirs_in(root: &std::path::Path, extra: &[(&str, &str)]) -> Dirs {
        let mut m: HashMap<String, OsString> = HashMap::new();
        m.insert("HOME".into(), root.join("home").into());
        m.insert("UNAM_CONFIG_DIR".into(), root.join("config").into());
        m.insert("UNAM_CACHE_DIR".into(), root.join("cache").into());
        m.insert("XDG_DATA_HOME".into(), root.join("data").into());
        for (k, v) in extra {
            m.insert(k.to_string(), OsString::from(v));
        }
        Dirs::resolve(Env::from_map(m)).unwrap()
    }

    fn p() -> PathBuf {
        PathBuf::from("/c/config.toml")
    }

    #[test]
    fn first_run_creates_default_file() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path(), &[]);
        let (c, diags) = Config::load_or_create(&d).unwrap();
        assert_eq!(c, Config::default());
        assert!(diags.is_empty());
        let text = std::fs::read_to_string(d.config_file()).unwrap();
        assert!(text.contains("version = 1"), "{text}");
        assert!(text.contains("strategy = \"symlink\""), "{text}");
        assert!(text.contains("destination_policy = \"shared\""), "{text}");
        assert!(text.contains("catalog_budget_tokens = 1500"), "{text}");
        assert!(text.contains("allow_missing = false"), "{text}");
        let (again, _) = Config::load_or_create(&d).unwrap();
        assert_eq!(again, c);
    }

    #[test]
    fn round_trip_with_sources() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path(), &[]);
        let mut c = Config::default();
        c.sources.push(SourceEntry {
            alias: "fx".into(),
            path: "~/lib".into(),
            precedence: 10,
        });
        c.save(&d).unwrap();
        let (loaded, _) = Config::load_or_create(&d).unwrap();
        assert_eq!(loaded, c);
    }

    #[test]
    fn version_two_errors() {
        let e = Config::parse("version = 2\n", &p()).unwrap_err();
        assert_eq!(e.code(), "config-version-unsupported");
        assert_eq!(
            Config::parse("x = 1\n", &p()).unwrap_err().code(),
            "config-invalid"
        );
        assert_eq!(
            Config::parse("version = [\n", &p()).unwrap_err().code(),
            "config-invalid"
        );
    }

    #[test]
    fn invalid_and_duplicate_alias() {
        let bad = "version = 1\n[[sources]]\nalias = \"Bad\"\npath = \"/x\"\nprecedence = 10\n";
        assert_eq!(
            Config::parse(bad, &p()).unwrap_err().code(),
            "config-invalid"
        );
        let dup = "version = 1\n[[sources]]\nalias = \"a\"\npath = \"/x\"\nprecedence = 10\n[[sources]]\nalias = \"a\"\npath = \"/y\"\nprecedence = 20\n";
        assert_eq!(
            Config::parse(dup, &p()).unwrap_err().code(),
            "config-invalid"
        );
    }

    #[test]
    fn alias_rule() {
        for ok in ["a", "0", "fx", "my-lib-2", &"a".repeat(32)] {
            assert!(is_valid_alias(ok), "{ok}");
        }
        for bad in [
            "",
            "-a",
            "A",
            "a_b",
            "a.b",
            "a b",
            "a:b",
            &"a".repeat(33),
            "\u{e9}",
        ] {
            assert!(!is_valid_alias(bad), "{bad}");
        }
    }

    #[test]
    fn source_path_expansion() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path(), &[("LIBS", "/libs")]);
        let e = |path: &str| SourceEntry {
            alias: "a".into(),
            path: path.into(),
            precedence: 10,
        };
        assert_eq!(
            e("~/x").expanded_path(&d).unwrap(),
            t.path().join("home").join("x")
        );
        assert_eq!(
            e("$LIBS/x").expanded_path(&d).unwrap(),
            PathBuf::from("/libs/x")
        );
        assert_eq!(
            e("${NOPE}/x").expanded_path(&d).unwrap_err().code(),
            "config-unset-variable"
        );
    }

    #[test]
    fn tie_warning_and_deterministic_order() {
        let text = "version = 1\n[[sources]]\nalias = \"zz\"\npath = \"/z\"\nprecedence = 10\n[[sources]]\nalias = \"aa\"\npath = \"/a\"\nprecedence = 10\n[[sources]]\nalias = \"mm\"\npath = \"/m\"\nprecedence = 5\n";
        let (c, diags) = Config::parse(text, &p()).unwrap();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "precedence-tie");
        let order: Vec<&str> = c
            .ordered_sources()
            .iter()
            .map(|s| s.alias.as_str())
            .collect();
        assert_eq!(order, vec!["mm", "aa", "zz"]);
    }

    #[test]
    fn unknown_key_ignored() {
        let text = "version = 1\nfuture = true\n[defaults]\nstrategy = \"copy\"\nnew_thing = 3\n";
        let (c, _) = Config::parse(text, &p()).unwrap();
        assert_eq!(c.defaults.strategy, Strategy::Copy);
        assert_eq!(c.defaults.catalog_budget_tokens, 1500);
    }
}
