//! `slugify`: exact port of Hermes `slugify_skill_name` (M2 plan D2, P3).
//!
//! Hermes: `re.sub(r"[^\w-]", "", name.lower().replace(" ", "-").replace("_", "-"))`,
//! then runs of `-` collapse to one and `-` is trimmed from both ends. Python
//! `\w` is matched by `is_word_char`: `_`, or `char::is_alphanumeric` minus
//! the generated exclusion table `NOT_WORD` (see `word_table.rs` and
//! `crates/unam-core/tools/gen-word-table.py`).

use super::word_table::NOT_WORD;

/// True when `c` is in the generated exclusion table.
fn excluded(c: char) -> bool {
    let cp = c as u32;
    NOT_WORD
        .binary_search_by(|&(lo, hi)| {
            if hi < cp {
                std::cmp::Ordering::Less
            } else if lo > cp {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// Python `\w` for one character (Unicode `re`, as Hermes uses it).
pub fn is_word_char(c: char) -> bool {
    c == '_' || (c.is_alphanumeric() && !excluded(c))
}

/// Normalize a bundle name to its slug (`Backend Dev` gives `backend-dev`).
/// The result may be empty; callers treat an empty slug as an error.
pub fn slugify(name: &str) -> String {
    let lower = name.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    for c in lower.chars() {
        let c = if c == ' ' || c == '_' { '-' } else { c };
        if c != '-' && !is_word_char(c) {
            continue;
        }
        if c == '-' && out.ends_with('-') {
            continue;
        }
        out.push(c);
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Expected values computed from the Hermes algorithm with Python 3.14.5
    /// on 2026-10-05 (M2 plan, `bundle/slug.rs` spec).
    const TABLE: &[(&str, &str)] = &[
        ("Backend Dev", "backend-dev"),
        ("", ""),
        ("!!!", ""),
        ("foo.bar", "foobar"),
        ("a/b", "ab"),
        ("x@y", "xy"),
        ("a+b", "ab"),
        ("1.2.3", "123"),
        ("Caf\u{e9}", "caf\u{e9}"),
        ("Cafe\u{301}", "cafe"),
        (
            "\u{65e5}\u{672c}\u{8a9e} \u{30b9}\u{30ad}\u{30eb}",
            "\u{65e5}\u{672c}\u{8a9e}-\u{30b9}\u{30ad}\u{30eb}",
        ),
        ("a\tb", "ab"),
        ("  Hello   World  ", "hello-world"),
        ("My_Skill-Name", "my-skill-name"),
        ("---", ""),
        ("a--b", "a-b"),
        ("-lead", "lead"),
        ("trail-", "trail"),
        ("a b_c-d", "a-b-c-d"),
        ("Stra\u{df}e", "stra\u{df}e"),
        ("\u{130}stanbul", "istanbul"),
        ("\u{391}\u{3a3}", "\u{3b1}\u{3c2}"),
        ("\u{24b6} bc", "bc"),
        (
            "\u{939}\u{93f}\u{928}\u{94d}\u{926}\u{940}",
            "\u{939}\u{928}\u{926}",
        ),
        ("a\u{200d}b", "ab"),
        ("a\u{1f600}b", "ab"),
        ("\u{ff21}\u{ff42}\u{ff43}", "\u{ff41}\u{ff42}\u{ff43}"),
        ("\u{1c5} x", "\u{1c6}-x"),
        ("\u{216b} roman", "\u{217b}-roman"),
        ("a\u{b2}b", "a\u{b2}b"),
        ("e\u{301}\u{301}", "e"),
    ];

    #[test]
    fn fixture_table() {
        assert_eq!(TABLE.len(), 31);
        for (input, expected) in TABLE {
            assert_eq!(slugify(input), *expected, "input {input:?}");
        }
    }

    #[test]
    fn table_is_sorted_disjoint_and_alphanumeric() {
        assert!(!NOT_WORD.is_empty());
        let mut prev: Option<u32> = None;
        for &(lo, hi) in NOT_WORD {
            assert!(lo <= hi, "{lo:X}..{hi:X}");
            if let Some(p) = prev {
                assert!(lo > p + 1, "not sorted, disjoint, maximal at {lo:X}");
            }
            prev = Some(hi);
            for cp in lo..=hi {
                let c = char::from_u32(cp).unwrap();
                assert!(c.is_alphanumeric(), "U+{cp:04X} is not alphanumeric");
                assert!(!is_word_char(c), "U+{cp:04X} is a word char");
            }
        }
    }

    #[test]
    fn boundary_code_points() {
        for cp in [0x0345, 0x0363, 0x036F, 0x24B6, 0x24E9] {
            assert!(!is_word_char(char::from_u32(cp).unwrap()), "U+{cp:04X}");
        }
        for c in ['\u{370}', 'a', 'Z', '0', '_', '\u{e9}', '\u{65e5}'] {
            assert!(is_word_char(c), "{c:?}");
        }
        for c in ['-', ' ', '.', '/', '\u{200d}', '\u{1f600}'] {
            assert!(!is_word_char(c), "{c:?}");
        }
    }

    /// Maximal ranges of `c == '_' || c.is_alphanumeric()` over every scalar value.
    fn rust_ranges() -> Vec<(u32, u32)> {
        let mut out: Vec<(u32, u32)> = Vec::new();
        let mut start: Option<u32> = None;
        let mut prev = 0;
        for cp in 0..=0x10FFFFu32 {
            let word = char::from_u32(cp).is_some_and(|c| c == '_' || c.is_alphanumeric());
            if word {
                if start.is_none() {
                    start = Some(cp);
                }
                prev = cp;
            } else if let Some(s) = start.take() {
                out.push((s, prev));
            }
        }
        if let Some(s) = start {
            out.push((s, prev));
        }
        out
    }

    #[test]
    fn rust_word_ranges_file_matches_char_is_alphanumeric() {
        let text = include_str!("../../tools/rust-word-ranges.txt");
        let file_unicode = text
            .lines()
            .find_map(|l| l.strip_prefix("# rust-unicode: "))
            .unwrap();
        let (a, b, c) = char::UNICODE_VERSION;
        let running = format!("{a}.{b}.{c}");
        if running != file_unicode {
            println!(
                "SKIP: rust-word-ranges.txt has Unicode {file_unicode}, this compiler has {running}; run gen-word-table.py --check-rust"
            );
            return;
        }
        let file: Vec<(u32, u32)> = text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| {
                let (s, e) = l.split_once(' ').unwrap();
                (
                    u32::from_str_radix(s, 16).unwrap(),
                    u32::from_str_radix(e, 16).unwrap(),
                )
            })
            .collect();
        assert_eq!(file, rust_ranges());
    }

    fn check_shape(out: &str) -> Result<(), TestCaseError> {
        prop_assert!(out.chars().all(|c| c == '-' || is_word_char(c)), "{out:?}");
        prop_assert!(!out.starts_with('-') && !out.ends_with('-'), "{out:?}");
        prop_assert!(!out.contains("--"), "{out:?}");
        prop_assert_eq!(slugify(out), out);
        Ok(())
    }

    proptest! {
        #[test]
        fn slug_shape_and_idempotence(s in "\\PC{0,40}") {
            check_shape(&slugify(&s))?;
        }

        #[test]
        fn slug_shape_any_string(s in any::<String>()) {
            check_shape(&slugify(&s))?;
        }
    }
}
