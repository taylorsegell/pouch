//! Fuzzy search over the catalog (spec 4.3, plan D10).
//!
//! Composite string per skill: `name  category  alias  description` (two
//! spaces between fields), matched with `nucleo-matcher`. Filters apply
//! first; results are ordered by score descending, then ID ascending.
//! Match indices for highlighting arrive with the TUI (milestone M6).

use super::{Catalog, SkillId};
use crate::source::SourceAlias;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};

/// One search result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub id: SkillId,
    pub score: u32,
}

struct Item {
    idx: usize,
    composite: String,
}

impl AsRef<str> for Item {
    fn as_ref(&self) -> &str {
        &self.composite
    }
}

/// Category filter: normalized prefix matched at a path-component boundary.
fn category_matches(category: &str, filter: &str) -> bool {
    let filter = filter.trim_matches('/');
    if filter.is_empty() {
        return true;
    }
    category == filter
        || category
            .strip_prefix(filter)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Search the catalog. An empty query matches everything with score 0.
pub fn search(
    catalog: &Catalog,
    query: &str,
    category: Option<&str>,
    source: Option<&SourceAlias>,
) -> Vec<Hit> {
    let items: Vec<Item> = catalog
        .skills
        .iter()
        .enumerate()
        .filter(|(_, s)| source.is_none_or(|a| s.id.source == *a))
        .filter(|(_, s)| category.is_none_or(|c| category_matches(&s.category, c)))
        .map(|(idx, s)| Item {
            idx,
            composite: format!(
                "{}  {}  {}  {}",
                s.name, s.category, s.id.source, s.description
            ),
        })
        .collect();
    let mut hits: Vec<Hit> = if query.trim().is_empty() {
        items
            .into_iter()
            .map(|i| Hit {
                id: catalog.skills[i.idx].id.clone(),
                score: 0,
            })
            .collect()
    } else {
        let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
        let mut matcher = Matcher::new(Config::DEFAULT);
        pattern
            .match_list(items, &mut matcher)
            .into_iter()
            .map(|(i, score)| Hit {
                id: catalog.skills[i.idx].id.clone(),
                score,
            })
            .collect()
    };
    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
    hits
}

#[cfg(test)]
mod tests {
    use super::super::tests_support::catalog_of;
    use super::*;

    fn cat() -> Catalog {
        let mut c = catalog_of(
            &[
                ("fx", "sdlc/code-review", "code-review"),
                ("fx", "testing/pytest-fixtures", "pytest-fixtures"),
                ("fx", "data/schema-migration", "schema-migration"),
                ("fx", "sdlcx/other", "other"),
                ("fx", "sdlc/sub/deep", "deep"),
                ("two", "lint/code-review", "code-review"),
            ],
            &[],
        );
        for s in &mut c.skills {
            s.description = match s.name.as_str() {
                "pytest-fixtures" => "Fixtures; review scope.".into(),
                "schema-migration" => "Migrations; review rollback.".into(),
                "code-review" => "Reviewing code.".into(),
                _ => "Unrelated text.".into(),
            };
        }
        c
    }

    fn ids(h: &[Hit]) -> Vec<String> {
        h.iter().map(|h| h.id.to_string()).collect()
    }

    #[test]
    fn exact_name_ranks_first() {
        let c = cat();
        let h = search(&c, "code-review", None, None);
        let top: Vec<String> = ids(&h).into_iter().take(2).collect();
        assert_eq!(top, vec!["fx:sdlc/code-review", "two:lint/code-review"]);
    }

    #[test]
    fn ties_ordered_by_id() {
        let c = cat();
        let h = search(&c, "code-review", None, None);
        assert_eq!(h[0].score, h[1].score);
        assert!(h[0].id < h[1].id);
    }

    #[test]
    fn review_spans_categories() {
        let c = cat();
        let got = ids(&search(&c, "review", None, None));
        for want in [
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "fx:data/schema-migration",
        ] {
            assert!(got.contains(&want.to_string()), "{want} in {got:?}");
        }
    }

    #[test]
    fn source_and_category_filters() {
        let c = cat();
        let two = SourceAlias::parse("two").unwrap();
        assert_eq!(
            ids(&search(&c, "", None, Some(&two))),
            vec!["two:lint/code-review"]
        );
        assert_eq!(
            ids(&search(&c, "", Some("sdlc"), None)),
            vec!["fx:sdlc/code-review", "fx:sdlc/sub/deep"]
        );
        assert_eq!(
            ids(&search(&c, "", Some("/sdlc/sub/"), None)),
            vec!["fx:sdlc/sub/deep"]
        );
        assert_eq!(
            ids(&search(&c, "", Some("sdlcx"), None)),
            vec!["fx:sdlcx/other"]
        );
        assert!(search(&c, "", Some("sd"), None).is_empty());
    }

    #[test]
    fn empty_query_lists_all_by_id() {
        let c = cat();
        let h = search(&c, "  ", None, None);
        assert_eq!(h.len(), 6);
        assert!(h.iter().all(|h| h.score == 0));
        let mut sorted = ids(&h);
        sorted.sort();
        assert_eq!(ids(&h), sorted);
    }

    #[test]
    fn no_match_is_empty() {
        let c = cat();
        assert!(search(&c, "zzqqxx", None, None).is_empty());
    }

    #[test]
    fn smart_case() {
        let c = cat();
        // An uppercase letter makes the query case-sensitive.
        assert!(search(&c, "REVIEW", None, None).is_empty());
        assert!(search(&c, "review", None, None).len() >= 3);
        assert!(
            search(&c, "Review", None, None)
                .iter()
                .all(|h| h.id.to_string() != "fx:data/schema-migration")
        );
        assert!(!search(&c, "Reviewing", None, None).is_empty());
    }
}
