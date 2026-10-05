//! Reference parsing and the TEMPORARY `lookup` (plan D6).
//!
//! `lookup` handles the three spec 4.6 reference forms. It is superseded by
//! `resolve.rs` in milestone M2. Ambiguity is always an error with every
//! candidate listed; source precedence never picks.

use super::{Catalog, RelPath, Skill, SkillRef};
use crate::error::{Error, Result};
use crate::source::SourceAlias;
use std::fmt;

const MAX_CANDIDATES: usize = 5;

impl SkillRef {
    /// Parse `alias:rel/path`, `alias:name`, or a bare `name`.
    pub fn parse(text: &str) -> Result<Self> {
        let invalid = |reason: &str| Error::InvalidRef {
            reference: text.to_string(),
            reason: reason.to_string(),
        };
        let t = text.trim();
        if t.is_empty() {
            return Err(invalid("empty reference"));
        }
        match t.split_once(':') {
            Some((alias, rest)) => {
                let source =
                    SourceAlias::parse(alias).map_err(|_| invalid("invalid source alias"))?;
                if rest.is_empty() {
                    return Err(invalid("nothing after `:`"));
                }
                if rest.contains('/') {
                    let rest = rest.strip_suffix('/').unwrap_or(rest);
                    let rel_path = RelPath::parse(rest).map_err(|r| invalid(&r))?;
                    Ok(SkillRef::Qualified { source, rel_path })
                } else {
                    Ok(SkillRef::SourceName {
                        source,
                        name: rest.to_string(),
                    })
                }
            }
            None => Ok(SkillRef::Bare(t.to_string())),
        }
    }
}

impl fmt::Display for SkillRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkillRef::Qualified { source, rel_path } => write!(f, "{source}:{rel_path}"),
            SkillRef::SourceName { source, name } => write!(f, "{source}:{name}"),
            SkillRef::Bare(name) => f.write_str(name),
        }
    }
}

/// Outcome of `lookup`.
#[derive(Debug, Clone, PartialEq)]
pub enum Lookup<'a> {
    Found(&'a Skill),
    /// Candidates (closest first) and aliases of relevant unreachable sources.
    NotFound {
        candidates: Vec<String>,
        unreachable: Vec<String>,
    },
    /// Every matching ID, sorted.
    Ambiguous(Vec<String>),
}

/// Levenshtein distance over chars.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Skills within `max` of `target` (by `key`), closest first, ties by ID.
fn nearest<'a>(
    skills: impl Iterator<Item = &'a Skill>,
    target: &str,
    max: usize,
    key: impl Fn(&Skill) -> &str,
) -> Vec<String> {
    let mut scored: Vec<(usize, String)> = skills
        .map(|s| (edit_distance(key(s), target), s.id.to_string()))
        .filter(|(d, _)| *d <= max)
        .collect();
    scored.sort();
    scored.dedup_by(|a, b| a.1 == b.1);
    scored
        .into_iter()
        .take(MAX_CANDIDATES)
        .map(|(_, id)| id)
        .collect()
}

impl Catalog {
    fn unreachable_aliases(&self, only: Option<&SourceAlias>) -> Vec<String> {
        self.sources
            .iter()
            .filter(|s| !s.reachable && only.is_none_or(|a| *a == s.alias))
            .map(|s| s.alias.to_string())
            .collect()
    }

    /// TEMPORARY (plan D6): superseded by `resolve.rs` in milestone M2.
    pub fn lookup(&self, r: &SkillRef) -> Lookup<'_> {
        match r {
            SkillRef::Qualified { source, rel_path } => {
                let id = super::SkillId {
                    source: source.clone(),
                    rel_path: rel_path.clone(),
                };
                match self.by_id.get(&id) {
                    Some(&i) => Lookup::Found(&self.skills[i]),
                    None => Lookup::NotFound {
                        candidates: nearest(
                            self.skills.iter().filter(|s| s.id.source == *source),
                            rel_path.as_str(),
                            3,
                            |s| s.id.rel_path.as_str(),
                        ),
                        unreachable: self.unreachable_aliases(Some(source)),
                    },
                }
            }
            SkillRef::SourceName { source, name } => {
                let mut hits: Vec<&Skill> = self
                    .skills
                    .iter()
                    .filter(|s| {
                        s.id.source == *source
                            && (s.name == *name || s.id.rel_path.as_str() == name)
                    })
                    .collect();
                hits.sort_by(|a, b| a.id.cmp(&b.id));
                hits.dedup_by(|a, b| a.id == b.id);
                match hits.len() {
                    1 => Lookup::Found(hits[0]),
                    0 => Lookup::NotFound {
                        candidates: nearest(
                            self.skills.iter().filter(|s| s.id.source == *source),
                            name,
                            2,
                            |s| s.name.as_str(),
                        ),
                        unreachable: self.unreachable_aliases(Some(source)),
                    },
                    _ => Lookup::Ambiguous(hits.iter().map(|s| s.id.to_string()).collect()),
                }
            }
            SkillRef::Bare(name) => {
                let hits = self.by_name.get(name).map(Vec::as_slice).unwrap_or(&[]);
                match hits.len() {
                    1 => Lookup::Found(&self.skills[hits[0]]),
                    0 => Lookup::NotFound {
                        candidates: nearest(self.skills.iter(), name, 2, |s| s.name.as_str()),
                        unreachable: self.unreachable_aliases(None),
                    },
                    _ => {
                        let mut ids: Vec<String> = hits
                            .iter()
                            .map(|&i| self.skills[i].id.to_string())
                            .collect();
                        ids.sort();
                        Lookup::Ambiguous(ids)
                    }
                }
            }
        }
    }

    /// Parse and look up `text`, mapping misses to `Error` (exit 2, 4, or 5).
    pub fn resolve_text(&self, text: &str) -> Result<&Skill> {
        let r = SkillRef::parse(text)?;
        match self.lookup(&r) {
            Lookup::Found(s) => Ok(s),
            Lookup::NotFound {
                candidates,
                unreachable,
            } => {
                let mut message = format!("no skill matches `{}`", text.trim());
                if !unreachable.is_empty() {
                    message.push_str(&format!(
                        "; source {} unreachable",
                        unreachable
                            .iter()
                            .map(|a| format!("`{a}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                            + if unreachable.len() == 1 {
                                " is"
                            } else {
                                " are"
                            }
                    ));
                }
                Err(Error::NotFound {
                    reference: text.trim().to_string(),
                    message,
                    candidates,
                })
            }
            Lookup::Ambiguous(candidates) => Err(Error::Ambiguous {
                reference: text.trim().to_string(),
                candidates,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests_support::catalog_of;
    use super::*;
    use proptest::prelude::*;

    fn cat() -> Catalog {
        catalog_of(
            &[
                ("fx", "sdlc/code-review", "code-review"),
                (
                    "fx",
                    "engineering/backend/api/error-handling",
                    "error-handling",
                ),
                (
                    "fx",
                    "engineering/frontend/error-handling",
                    "error-handling",
                ),
                ("fx", "alpha-root-skill", "alpha-root-skill"),
                ("fx", "mismatch/dir-name-differs", "other-name"),
                ("two", "lint/code-review", "code-review"),
            ],
            &["gone"],
        )
    }

    fn found_id(l: Lookup<'_>) -> String {
        match l {
            Lookup::Found(s) => s.id.to_string(),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    fn r(text: &str) -> SkillRef {
        SkillRef::parse(text).unwrap()
    }

    #[test]
    fn parse_table() {
        assert!(matches!(
            r("fx:sdlc/code-review"),
            SkillRef::Qualified { .. }
        ));
        assert!(matches!(
            r("fx:sdlc/code-review/"),
            SkillRef::Qualified { .. }
        ));
        assert_eq!(r("fx:sdlc/code-review/").to_string(), "fx:sdlc/code-review");
        assert!(matches!(r("fx:code-review"), SkillRef::SourceName { .. }));
        assert_eq!(r("  code-review "), SkillRef::Bare("code-review".into()));
        for bad in [
            "",
            "   ",
            ":x",
            "Fx:x",
            "fx:",
            "fx:a//b",
            "fx:../x",
            "fx:/abs/x",
            "fx:a/./b",
        ] {
            let e = SkillRef::parse(bad).unwrap_err();
            assert_eq!(e.code(), "invalid-ref", "{bad}");
            assert_eq!(e.exit_code(), 2);
        }
    }

    #[test]
    fn qualified_forms() {
        let c = cat();
        assert_eq!(
            found_id(c.lookup(&r("fx:sdlc/code-review"))),
            "fx:sdlc/code-review"
        );
        match c.lookup(&r("fx:sdlc/code-reviw")) {
            Lookup::NotFound { candidates, .. } => {
                assert_eq!(candidates, vec!["fx:sdlc/code-review".to_string()]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn source_name_forms() {
        let c = cat();
        assert_eq!(
            found_id(c.lookup(&r("fx:code-review"))),
            "fx:sdlc/code-review"
        );
        assert_eq!(
            found_id(c.lookup(&r("two:code-review"))),
            "two:lint/code-review"
        );
        assert_eq!(
            found_id(c.lookup(&r("fx:alpha-root-skill"))),
            "fx:alpha-root-skill"
        );
        match c.lookup(&r("fx:error-handling")) {
            Lookup::Ambiguous(ids) => assert_eq!(ids.len(), 2),
            other => panic!("{other:?}"),
        }
        match c.lookup(&r("fx:code-reviwe")) {
            Lookup::NotFound { candidates, .. } => {
                assert_eq!(candidates, vec!["fx:sdlc/code-review".to_string()]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bare_forms() {
        let c = cat();
        assert_eq!(
            found_id(c.lookup(&r("other-name"))),
            "fx:mismatch/dir-name-differs"
        );
        match c.lookup(&r("code-review")) {
            Lookup::Ambiguous(ids) => assert_eq!(
                ids,
                vec![
                    "fx:sdlc/code-review".to_string(),
                    "two:lint/code-review".to_string()
                ]
            ),
            other => panic!("{other:?}"),
        }
        match c.lookup(&r("error-handling")) {
            Lookup::Ambiguous(ids) => assert_eq!(
                ids,
                vec![
                    "fx:engineering/backend/api/error-handling".to_string(),
                    "fx:engineering/frontend/error-handling".to_string()
                ]
            ),
            other => panic!("{other:?}"),
        }
        match c.lookup(&r("other-nam")) {
            Lookup::NotFound {
                candidates,
                unreachable,
            } => {
                assert_eq!(candidates, vec!["fx:mismatch/dir-name-differs".to_string()]);
                assert_eq!(unreachable, vec!["gone".to_string()]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unreachable_source_note() {
        let c = cat();
        let e = c.resolve_text("gone:sdlc/code-review").unwrap_err();
        assert_eq!(e.exit_code(), 4);
        assert!(e.to_string().contains("`gone` is unreachable"), "{e}");
        let e = c.resolve_text("code-review").unwrap_err();
        assert_eq!(e.exit_code(), 5);
        assert_eq!(e.candidates().unwrap().len(), 2);
    }

    #[test]
    fn edit_distance_cases() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("abc", "abc"), 0);
        assert_eq!(edit_distance("abc", "abd"), 1);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("\u{e9}a", "ea"), 1);
    }

    proptest! {
        #[test]
        fn parse_never_panics(s in "\\PC{0,40}") {
            let _ = SkillRef::parse(&s);
        }

        #[test]
        fn qualified_round_trips(alias in "[a-z][a-z0-9-]{0,10}", path in "[a-z]{1,5}(/[a-z0-9-]{1,5}){1,3}") {
            let text = format!("{alias}:{path}");
            let parsed = SkillRef::parse(&text).unwrap();
            let qualified = matches!(parsed, SkillRef::Qualified { .. });
            prop_assert!(qualified);
            prop_assert_eq!(parsed.to_string(), text);
        }
    }
}
