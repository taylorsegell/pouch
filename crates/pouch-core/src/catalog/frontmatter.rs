//! `SKILL.md` frontmatter parsing (spec 4.3, plan steps 1 to 9).
//!
//! Lenient per DIS-3: a name/directory mismatch or an over-length name warns
//! and loads; a missing description or unparseable YAML skips the skill with
//! a diagnostic. Never panics on user data.

use crate::error::Diagnostic;
use serde_json::{Map, Value as Json};
use serde_norway::Value as Yaml;

/// Names longer than this warn (`name-too-long`, plan P2).
pub const MAX_NAME_CHARS: usize = 64;

/// A successfully parsed skill header.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub name: String,
    pub description: String,
    pub frontmatter: Map<String, Json>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse `SKILL.md` bytes. `Err` means the skill is skipped (severity error).
pub fn parse_skill_md(bytes: &[u8], dir_name: &str) -> Result<Parsed, Diagnostic> {
    // 1. UTF-8.
    let text = std::str::from_utf8(bytes)
        .map_err(|_| Diagnostic::error("not-utf8", "SKILL.md is not valid UTF-8"))?;
    // 2. BOM and CRLF.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let text = text.replace("\r\n", "\n");
    // 3. Opening fence.
    let Some(after_open) = text.strip_prefix("---\n") else {
        return Err(Diagnostic::error(
            "no-frontmatter",
            "SKILL.md does not begin with a `---` frontmatter fence",
        ));
    };
    // 4. Closing fence.
    let mut block_lines = Vec::new();
    let mut closed = false;
    for line in after_open.split('\n') {
        if line == "---" {
            closed = true;
            break;
        }
        block_lines.push(line);
    }
    if !closed {
        return Err(Diagnostic::error(
            "unterminated-frontmatter",
            "frontmatter has no closing `---` line",
        ));
    }
    let block = block_lines.join("\n");
    let mut diagnostics = Vec::new();
    // 5. Parse; 6. retry once with quoted colon values.
    let mapping = match parse_mapping(&block) {
        Ok(m) => m,
        Err(_) => {
            let retried = quote_colon_values(&block);
            match parse_mapping(&retried) {
                Ok(m) => {
                    diagnostics.push(Diagnostic::info(
                        "colon-retry-applied",
                        "unquoted values containing `: ` were quoted to parse the frontmatter",
                    ));
                    m
                }
                Err(msg) => {
                    return Err(Diagnostic::error(
                        "unparseable-yaml",
                        format!("frontmatter is not valid YAML: {msg}"),
                    ));
                }
            }
        }
    };
    // 7. Description and name.
    let description = match mapping.get("description") {
        Some(Yaml::String(s)) if !s.trim().is_empty() => s.clone(),
        _ => {
            return Err(Diagnostic::error(
                "missing-description",
                "`description` is missing, empty, or not a string",
            ));
        }
    };
    let name = match mapping.get("name") {
        Some(Yaml::String(s)) if !s.trim().is_empty() => s.clone(),
        _ => dir_name.to_string(),
    };
    // 8. Warnings that still load.
    if name != dir_name {
        diagnostics.push(Diagnostic::warning(
            "name-dir-mismatch",
            format!("name `{name}` differs from directory name `{dir_name}`"),
        ));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        diagnostics.push(Diagnostic::warning(
            "name-too-long",
            format!("name is longer than {MAX_NAME_CHARS} characters"),
        ));
    }
    // 9. Convert to JSON, preserving order.
    let frontmatter = mapping_to_json(&mapping);
    Ok(Parsed {
        name,
        description,
        frontmatter,
        diagnostics,
    })
}

fn parse_mapping(block: &str) -> Result<serde_norway::Mapping, String> {
    match serde_norway::from_str::<Yaml>(block) {
        Ok(Yaml::Mapping(m)) => Ok(m),
        Ok(Yaml::Tagged(t)) => match t.value {
            Yaml::Mapping(m) => Ok(m),
            _ => Err("frontmatter is not a mapping".to_string()),
        },
        Ok(_) => Err("frontmatter is not a mapping".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// Quote top-level `key: value` lines whose plain value contains `: `.
fn quote_colon_values(block: &str) -> String {
    let mut out = Vec::new();
    for line in block.split('\n') {
        out.push(quote_line(line).unwrap_or_else(|| line.to_string()));
    }
    out.join("\n")
}

fn quote_line(line: &str) -> Option<String> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let idx = line.find(": ")?;
    let key = &line[..idx];
    if key.is_empty() || key.starts_with('#') || key.starts_with('-') {
        return None;
    }
    let value = line[idx + 2..].trim();
    let starts_special = value.chars().next().is_none_or(|c| {
        matches!(
            c,
            '"' | '\'' | '|' | '>' | '[' | '{' | '&' | '*' | '!' | '#'
        )
    });
    if starts_special || !value.contains(": ") {
        return None;
    }
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    Some(format!("{key}: \"{escaped}\""))
}

fn key_string(key: &Yaml) -> String {
    match key {
        Yaml::String(s) => s.clone(),
        Yaml::Null => "null".to_string(),
        Yaml::Bool(b) => b.to_string(),
        Yaml::Number(n) => n.to_string(),
        Yaml::Tagged(t) => key_string(&t.value),
        other => serde_norway::to_string(other)
            .map(|s| s.trim_end().to_string())
            .unwrap_or_default(),
    }
}

fn mapping_to_json(m: &serde_norway::Mapping) -> Map<String, Json> {
    let mut out = Map::new();
    for (k, v) in m {
        out.insert(key_string(k), yaml_to_json(v));
    }
    out
}

/// Convert YAML to JSON: tags unwrapped, non-finite floats as strings.
pub fn yaml_to_json(v: &Yaml) -> Json {
    match v {
        Yaml::Null => Json::Null,
        Yaml::Bool(b) => Json::Bool(*b),
        Yaml::Number(n) => {
            if let Some(i) = n.as_i64() {
                Json::from(i)
            } else if let Some(u) = n.as_u64() {
                Json::from(u)
            } else {
                let f = n.as_f64().unwrap_or(f64::NAN);
                serde_json::Number::from_f64(f)
                    .map(Json::Number)
                    .unwrap_or_else(|| Json::String(n.to_string()))
            }
        }
        Yaml::String(s) => Json::String(s.clone()),
        Yaml::Sequence(seq) => Json::Array(seq.iter().map(yaml_to_json).collect()),
        Yaml::Mapping(m) => Json::Object(mapping_to_json(m)),
        Yaml::Tagged(t) => yaml_to_json(&t.value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn ok(text: &str, dir: &str) -> Parsed {
        parse_skill_md(text.as_bytes(), dir).unwrap()
    }

    fn skipped(text: &str) -> String {
        parse_skill_md(text.as_bytes(), "d").unwrap_err().code
    }

    fn codes(p: &Parsed) -> Vec<&str> {
        p.diagnostics.iter().map(|d| d.code.as_str()).collect()
    }

    #[test]
    fn valid() {
        let p = ok("---\nname: d\ndescription: Does things.\n---\nbody\n", "d");
        assert_eq!(p.name, "d");
        assert_eq!(p.description, "Does things.");
        assert!(p.diagnostics.is_empty());
    }

    #[test]
    fn name_falls_back_to_dir() {
        let p = ok("---\ndescription: x\n---\n", "dir");
        assert_eq!(p.name, "dir");
        let p = ok("---\nname: 42\ndescription: x\n---\n", "dir");
        assert_eq!(p.name, "dir");
        let p = ok("---\nname: \"  \"\ndescription: x\n---\n", "dir");
        assert_eq!(p.name, "dir");
    }

    #[test]
    fn missing_empty_and_non_string_description() {
        assert_eq!(skipped("---\nname: d\n---\n"), "missing-description");
        assert_eq!(
            skipped("---\nname: d\ndescription: \"  \"\n---\n"),
            "missing-description"
        );
        assert_eq!(
            skipped("---\nname: d\ndescription: 42\n---\n"),
            "missing-description"
        );
        assert_eq!(
            skipped("---\ndescription: [a, b]\n---\n"),
            "missing-description"
        );
    }

    #[test]
    fn unparseable() {
        assert_eq!(
            skipped("---\nname: d\ndescription: [unclosed\n---\n"),
            "unparseable-yaml"
        );
    }

    #[test]
    fn colon_retry_succeeds() {
        let p = ok(
            "---\nname: d\ndescription: Use when: reviewing code\n---\n",
            "d",
        );
        assert_eq!(p.description, "Use when: reviewing code");
        assert_eq!(codes(&p), vec!["colon-retry-applied"]);
    }

    #[test]
    fn colon_retry_quotes_and_backslashes_escaped() {
        let p = ok(
            "---\nname: d\ndescription: Say \"hi\": c:\\x now\n---\n",
            "d",
        );
        assert_eq!(p.description, "Say \"hi\": c:\\x now");
    }

    #[test]
    fn retry_still_failing() {
        assert_eq!(
            skipped("---\nname: d\ndescription: Use when: x\nbad: [oops\n---\n"),
            "unparseable-yaml"
        );
    }

    #[test]
    fn retry_leaves_block_scalars_and_quoted_values_alone() {
        let text = "---\nname: d\ndescription: |\n  Use when: indented\nother: \"quoted: value\"\nx: a: b\n---\n";
        let p = ok(text, "d");
        assert_eq!(p.description, "Use when: indented\n");
        assert_eq!(p.frontmatter["other"], "quoted: value");
        assert_eq!(p.frontmatter["x"], "a: b");
    }

    #[test]
    fn name_dir_mismatch_warns() {
        let p = ok("---\nname: other\ndescription: x\n---\n", "dir");
        assert_eq!(p.name, "other");
        assert_eq!(codes(&p), vec!["name-dir-mismatch"]);
        assert!(p.diagnostics[0].message.contains("other"));
        assert!(p.diagnostics[0].message.contains("dir"));
    }

    #[test]
    fn name_too_long_warns() {
        let long = "a".repeat(65);
        let p = ok(&format!("---\nname: {long}\ndescription: x\n---\n"), &long);
        assert_eq!(codes(&p), vec!["name-too-long"]);
        let ok64 = "a".repeat(64);
        let p = ok(&format!("---\nname: {ok64}\ndescription: x\n---\n"), &ok64);
        assert!(p.diagnostics.is_empty());
    }

    #[test]
    fn bom_only() {
        let p = ok("\u{feff}---\nname: d\ndescription: x\n---\n", "d");
        assert_eq!(p.description, "x");
    }

    #[test]
    fn crlf_only() {
        let p = ok("---\r\nname: d\r\ndescription: x\r\n---\r\n", "d");
        assert_eq!(p.description, "x");
    }

    #[test]
    fn bom_and_crlf() {
        let p = ok(
            "\u{feff}---\r\nname: d\r\ndescription: x y\r\n---\r\nbody\r\n",
            "d",
        );
        assert_eq!(p.description, "x y");
    }

    #[test]
    fn no_fence() {
        assert_eq!(skipped("name: d\ndescription: x\n"), "no-frontmatter");
        assert_eq!(skipped(" ---\nname: d\n---\n"), "no-frontmatter");
    }

    #[test]
    fn unterminated() {
        assert_eq!(
            skipped("---\nname: d\ndescription: x\n"),
            "unterminated-frontmatter"
        );
    }

    #[test]
    fn closing_fence_at_eof() {
        let p = ok("---\nname: d\ndescription: x\n---", "d");
        assert_eq!(p.description, "x");
    }

    #[test]
    fn invalid_utf8() {
        let e = parse_skill_md(b"---\nname: \xff\n---\n", "d").unwrap_err();
        assert_eq!(e.code, "not-utf8");
    }

    #[test]
    fn list_frontmatter() {
        assert_eq!(skipped("---\n- a\n- b\n---\n"), "unparseable-yaml");
    }

    #[test]
    fn unknown_keys_and_order_preserved() {
        let p = ok(
            "---\nzeta: 1\nname: d\nalpha: [1, 2]\ndescription: x\nmid: {k: v}\n---\n",
            "d",
        );
        let keys: Vec<&String> = p.frontmatter.keys().collect();
        assert_eq!(keys, vec!["zeta", "name", "alpha", "description", "mid"]);
        assert_eq!(p.frontmatter["mid"]["k"], "v");
    }

    #[test]
    fn non_string_key_stringified() {
        let p = ok(
            "---\n1: one\ntrue: yes\nname: d\ndescription: x\n---\n",
            "d",
        );
        assert_eq!(p.frontmatter["1"], "one");
        assert!(p.frontmatter.contains_key("true"));
    }

    #[test]
    fn non_finite_float_as_string() {
        let p = ok("---\ndescription: x\nv: .inf\n---\n", "d");
        assert!(p.frontmatter["v"].is_string());
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
            let _ = parse_skill_md(&bytes, "d");
        }

        #[test]
        fn never_panics_on_fenced_text(body in "[ -~\n]{0,200}") {
            let _ = parse_skill_md(format!("---\n{body}\n---\n").as_bytes(), "d");
        }
    }
}
