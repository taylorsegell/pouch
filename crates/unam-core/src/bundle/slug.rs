//! `slugify`: a port of Hermes `slugify_skill_name` (M2 plan D2, P3).
//!
//! Hermes: `re.sub(r"[^\w-]", "", name.lower().replace(" ", "-").replace("_", "-"))`,
//! then runs of `-` collapse to one and `-` is trimmed from both ends. Python
//! `\w` is approximated by `is_word_char`: `_` or `char::is_alphanumeric`.
//! The two differ only on some combining marks and symbol letters (see
//! `docs/adapters.md`).

/// Approximation of Python `\w` for one character.
pub fn is_word_char(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
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

    /// Expected values: the Hermes algorithm (Python 3.14.5, 2026-10-05) on
    /// inputs where `\w` and `is_alphanumeric` agree.
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
        ("a\u{200d}b", "ab"),
        ("a\u{1f600}b", "ab"),
        ("\u{ff21}\u{ff42}\u{ff43}", "\u{ff41}\u{ff42}\u{ff43}"),
        ("a\u{b2}b", "a\u{b2}b"),
    ];

    #[test]
    fn fixture_table() {
        for (input, expected) in TABLE {
            assert_eq!(slugify(input), *expected, "input {input:?}");
        }
    }

    #[test]
    fn boundary_code_points() {
        for c in ['\u{370}', 'a', 'Z', '0', '_', '\u{e9}', '\u{65e5}'] {
            assert!(is_word_char(c), "{c:?}");
        }
        for c in ['-', ' ', '.', '/', '\u{200d}', '\u{1f600}'] {
            assert!(!is_word_char(c), "{c:?}");
        }
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
