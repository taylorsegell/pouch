//! Line editor for the block `skills:` sequence (M2 plan D3, P12).
//!
//! Edits a hand-written manifest in place: only item lines are removed or
//! inserted, every other byte stays as it was (comments, quoting, CRLF, BOM).
//! Shapes the grammar cannot prove safe are refused with a reason; callers
//! wrap the reason as `BundleEditUnsupported` and offer `--rewrite`. Callers
//! also reparse the whole result before writing (see `bundle::verify_edit`).
//!
//! The module knows nothing about bundles or catalogs.

use std::fmt;

/// A refusal: the text has a shape this editor will not touch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditError(pub String);

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn refuse<T>(reason: impl Into<String>) -> Result<T, EditError> {
    Err(EditError(reason.into()))
}

/// The YAML text for one sequence entry, as `serde_norway` writes the string
/// (quoted when a plain scalar would not reparse as the same string).
pub(crate) fn yaml_scalar(value: &str) -> String {
    match serde_norway::to_string(value) {
        Ok(s) => s.strip_suffix('\n').unwrap_or(&s).to_string(),
        // Serializing a plain string cannot fail; fall back to a safe quote.
        Err(_) => format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"")),
    }
}

/// One line: its body and its terminator (`\n`, `\r\n`, or empty).
#[derive(Debug, Clone)]
struct Line {
    body: String,
    term: String,
}

fn split(text: &str) -> (bool, Vec<Line>) {
    let (bom, rest) = match text.strip_prefix('\u{feff}') {
        Some(r) => (true, r),
        None => (false, text),
    };
    let lines = rest
        .split_inclusive('\n')
        .map(|l| {
            if let Some(b) = l.strip_suffix("\r\n") {
                Line {
                    body: b.to_string(),
                    term: "\r\n".to_string(),
                }
            } else if let Some(b) = l.strip_suffix('\n') {
                Line {
                    body: b.to_string(),
                    term: "\n".to_string(),
                }
            } else {
                Line {
                    body: l.to_string(),
                    term: String::new(),
                }
            }
        })
        .collect();
    (bom, lines)
}

fn join(bom: bool, lines: &[Line]) -> String {
    let mut out = String::new();
    if bom {
        out.push('\u{feff}');
    }
    for l in lines {
        out.push_str(&l.body);
        out.push_str(&l.term);
    }
    out
}

/// One item of the block.
#[derive(Debug, Clone)]
struct Item {
    line: usize,
    value: String,
}

/// The located `skills:` block.
#[derive(Debug, Clone)]
struct Block {
    key_line: usize,
    indent: usize,
    items: Vec<Item>,
}

fn is_blank_or_comment(body: &str) -> bool {
    let t = body.trim_start_matches(' ');
    t.is_empty() || t.starts_with('#')
}

fn leading_ws(body: &str) -> &str {
    let end = body
        .find(|c: char| c != ' ' && c != '\t')
        .unwrap_or(body.len());
    &body[..end]
}

/// After a closing quote or a plain body: only spaces and an optional
/// `# comment` (preceded by a space) may follow.
fn tail_ok(rest: &str) -> bool {
    let t = rest.trim_start_matches(' ');
    t.is_empty() || (t.len() < rest.len() && t.starts_with('#'))
}

/// The scalar token of an item body (quotes included), or `None` for a null
/// entry.
fn item_token(body: &str) -> Result<Option<String>, EditError> {
    let b = body.trim_start_matches(' ');
    if b.is_empty() || b.starts_with('#') {
        return Ok(None);
    }
    if let Some(after) = b.strip_prefix('"') {
        let mut escaped = false;
        for (i, c) in after.char_indices() {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                let end = 1 + i + 1;
                if !tail_ok(&b[end..]) {
                    return refuse("unexpected text after a quoted entry");
                }
                return Ok(Some(b[..end].to_string()));
            }
        }
        return refuse("multi-line quoted entry (unterminated double quote)");
    }
    if let Some(after) = b.strip_prefix('\'') {
        let chars: Vec<(usize, char)> = after.char_indices().collect();
        let mut k = 0;
        while k < chars.len() {
            let (i, c) = chars[k];
            if c == '\'' {
                if chars.get(k + 1).map(|x| x.1) == Some('\'') {
                    k += 2;
                    continue;
                }
                let end = 1 + i + 1;
                if !tail_ok(&b[end..]) {
                    return refuse("unexpected text after a quoted entry");
                }
                return Ok(Some(b[..end].to_string()));
            }
            k += 1;
        }
        return refuse("multi-line quoted entry (unterminated single quote)");
    }
    // Plain scalar: ends at the first ` #` or the end of the line.
    let plain = match b.find(" #") {
        Some(i) => &b[..i],
        None => b,
    };
    let plain = plain.trim_end_matches(' ');
    if let Some(c) = plain.chars().next()
        && "[{&*!|>%@`".contains(c)
    {
        return refuse(format!(
            "entry starting with `{c}` (flow collection, anchor, alias, tag, block scalar, or reserved)"
        ));
    }
    if plain.starts_with("- ") || plain == "-" {
        return refuse("nested sequence entry");
    }
    if plain.contains(": ") || plain.ends_with(':') {
        return refuse("mapping entry (`key: value`)");
    }
    Ok(Some(plain.to_string()))
}

/// The entry value of a token: `serde_norway` parsing of the token alone,
/// stringified (string as is, number or boolean as text, null as empty).
fn token_value(token: Option<&str>) -> Result<String, EditError> {
    let Some(token) = token else {
        return Ok(String::new());
    };
    match serde_norway::from_str::<serde_norway::Value>(token) {
        Ok(serde_norway::Value::String(s)) => Ok(s),
        Ok(serde_norway::Value::Number(n)) => Ok(n.to_string()),
        Ok(serde_norway::Value::Bool(b)) => Ok(b.to_string()),
        Ok(serde_norway::Value::Null) => Ok(String::new()),
        Ok(_) => refuse("entry is not a scalar"),
        Err(e) => refuse(format!("entry does not parse: {e}")),
    }
}

/// Parse an item line: `Some((indent, body))` when it is `-` at indent 0 or
/// 2 followed by a space or the end of the line.
fn item_line(body: &str) -> Option<(usize, &str)> {
    for indent in [0usize, 2] {
        let Some(rest) = body.get(indent..) else {
            continue;
        };
        if !body[..indent].chars().all(|c| c == ' ') {
            continue;
        }
        if rest == "-" {
            return Some((indent, ""));
        }
        if let Some(b) = rest.strip_prefix("- ") {
            return Some((indent, b));
        }
    }
    None
}

fn locate(lines: &[Line]) -> Result<Block, EditError> {
    let mut keys = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let b = &l.body;
        if b.starts_with("\"skills\":") || b.starts_with("'skills':") {
            return refuse("quoted `skills` key");
        }
        if b.starts_with("skills:") {
            keys.push(i);
        }
    }
    let key_line = match keys.as_slice() {
        [] => {
            if lines
                .iter()
                .any(|l| l.body.trim_start().starts_with("skills:"))
            {
                return refuse(
                    "`skills:` appears only nested (for example under `metadata`), not at the top level",
                );
            }
            return refuse("no `skills:` block was found");
        }
        [one] => *one,
        _ => return refuse("duplicate `skills:` key"),
    };
    let rest = &lines[key_line].body["skills:".len()..];
    if !tail_ok(rest) {
        let first = rest.trim_start_matches(' ').chars().next().unwrap_or(' ');
        let what = match first {
            '[' => "a flow sequence",
            '&' | '*' => "an anchor or alias",
            '!' => "a tagged value",
            '|' | '>' => "a block scalar",
            _ => "a scalar value",
        };
        return refuse(format!("`skills:` holds {what}, not a block sequence"));
    }
    let mut indent: Option<usize> = None;
    let mut items = Vec::new();
    for (i, l) in lines.iter().enumerate().skip(key_line + 1) {
        let b = &l.body;
        if leading_ws(b).contains('\t') {
            return refuse("tab character in indentation");
        }
        if is_blank_or_comment(b) {
            continue;
        }
        if let Some((ind, body)) = item_line(b) {
            match indent {
                None => indent = Some(ind),
                Some(fixed) if fixed != ind => {
                    return refuse("inconsistent indentation of the `skills` entries");
                }
                Some(_) => {}
            }
            let token = item_token(body)?;
            let value = token_value(token.as_deref())?;
            items.push(Item { line: i, value });
            continue;
        }
        if !b.starts_with(' ') {
            break;
        }
        return refuse("multi-line or nested entry in the `skills` block");
    }
    let Some(indent) = indent else {
        return refuse("`skills:` has no entries");
    };
    Ok(Block {
        key_line,
        indent,
        items,
    })
}

/// Entry values of the block in order (a null entry is the empty string).
pub fn list_entries(text: &str) -> Result<Vec<String>, EditError> {
    let (_, lines) = split(text);
    Ok(locate(&lines)?.items.into_iter().map(|i| i.value).collect())
}

/// Delete every item line whose value is in `values` (only that line, its
/// terminator included). Returns the new text and the removed values, one
/// per removed line.
pub fn remove_values(text: &str, values: &[String]) -> Result<(String, Vec<String>), EditError> {
    let (bom, lines) = split(text);
    let block = locate(&lines)?;
    let mut drop = vec![false; lines.len()];
    let mut removed = Vec::new();
    for item in &block.items {
        if values.contains(&item.value) {
            drop[item.line] = true;
            removed.push(item.value.clone());
        }
    }
    let kept: Vec<Line> = lines
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !drop[*i])
        .map(|(_, l)| l)
        .collect();
    Ok((join(bom, &kept), removed))
}

/// Insert one item line per value not already present (by exact value),
/// directly after the last item line. Returns the new text, the added
/// values, and the skipped values.
pub fn add_values(
    text: &str,
    values: &[String],
) -> Result<(String, Vec<String>, Vec<String>), EditError> {
    let (bom, mut lines) = split(text);
    let block = locate(&lines)?;
    let existing: Vec<&str> = block.items.iter().map(|i| i.value.as_str()).collect();
    let mut added: Vec<String> = Vec::new();
    let mut skipped = Vec::new();
    for v in values {
        if v.chars().any(char::is_control) {
            return refuse("entry contains a control character");
        }
        if existing.contains(&v.as_str()) || added.contains(v) {
            skipped.push(v.clone());
        } else {
            added.push(v.clone());
        }
    }
    if added.is_empty() {
        return Ok((join(bom, &lines), added, skipped));
    }
    let term = lines[block.key_line].term.clone();
    let term = if term.is_empty() {
        "\n".to_string()
    } else {
        term
    };
    let last = block.items.last().map_or(block.key_line, |i| i.line);
    let at_eof_without_term = lines[last].term.is_empty();
    if at_eof_without_term {
        lines[last].term = term.clone();
    }
    let prefix = " ".repeat(block.indent);
    let mut new_lines: Vec<Line> = added
        .iter()
        .map(|v| Line {
            body: format!("{prefix}- {}", yaml_scalar(v)),
            term: term.clone(),
        })
        .collect();
    if at_eof_without_term && let Some(l) = new_lines.last_mut() {
        l.term = String::new();
    }
    lines.splice(last + 1..last + 1, new_lines);
    Ok((join(bom, &lines), added, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    const BASE: &str = "name: b\nskills:\n  - fx:a/one\n  - \"fx:b/two\"\n  - 'three'\n  - four  # trailing\ndescription: d\n";

    #[test]
    fn list_quoting_and_comments() {
        assert_eq!(
            list_entries(BASE).unwrap(),
            s(&["fx:a/one", "fx:b/two", "three", "four"])
        );
    }

    #[test]
    fn dash_at_indent_zero_and_two() {
        let zero = "skills:\n- a\n- b\nname: x\n";
        assert_eq!(list_entries(zero).unwrap(), s(&["a", "b"]));
        let (out, removed) = remove_values(zero, &s(&["a"])).unwrap();
        assert_eq!(out, "skills:\n- b\nname: x\n");
        assert_eq!(removed, s(&["a"]));
        let (out, added, _) = add_values(zero, &s(&["c"])).unwrap();
        assert_eq!(out, "skills:\n- a\n- b\n- c\nname: x\n");
        assert_eq!(added, s(&["c"]));
        let (out, _, _) = add_values(BASE, &s(&["c"])).unwrap();
        assert!(
            out.contains("  - four  # trailing\n  - c\ndescription: d\n"),
            "{out}"
        );
    }

    #[test]
    fn comment_and_blank_lines_between_items_stay() {
        let t = "skills:\n  - a\n\n  # helpers\n  - b\n  - c\n";
        let (out, _) = remove_values(t, &s(&["b"])).unwrap();
        assert_eq!(out, "skills:\n  - a\n\n  # helpers\n  - c\n");
        let (out, _) = remove_values(t, &s(&["a", "c"])).unwrap();
        assert_eq!(out, "skills:\n\n  # helpers\n  - b\n");
    }

    #[test]
    fn key_with_trailing_comment() {
        let t = "skills:   # the list\n  - a\n  - b\n";
        assert_eq!(list_entries(t).unwrap(), s(&["a", "b"]));
        let (out, _) = remove_values(t, &s(&["a"])).unwrap();
        assert_eq!(out, "skills:   # the list\n  - b\n");
    }

    #[test]
    fn crlf_bom_and_missing_final_newline() {
        let t = "\u{feff}name: b\r\nskills:\r\n  - a\r\n  - b";
        let (out, _) = remove_values(t, &s(&["a"])).unwrap();
        assert_eq!(out, "\u{feff}name: b\r\nskills:\r\n  - b");
        let (out, _) = remove_values(t, &s(&["b"])).unwrap();
        assert_eq!(out, "\u{feff}name: b\r\nskills:\r\n  - a\r\n");
        let (out, _, _) = add_values(t, &s(&["c"])).unwrap();
        assert_eq!(out, "\u{feff}name: b\r\nskills:\r\n  - a\r\n  - b\r\n  - c");
        let crlf = "skills:\r\n  - a\r\nname: x\r\n";
        let (out, _, _) = add_values(crlf, &s(&["z"])).unwrap();
        assert_eq!(out, "skills:\r\n  - a\r\n  - z\r\nname: x\r\n");
        let lf_eof = "skills:\n  - a";
        let (out, _, _) = add_values(lf_eof, &s(&["b", "c"])).unwrap();
        assert_eq!(out, "skills:\n  - a\n  - b\n  - c");
    }

    #[test]
    fn remove_first_middle_last_several_and_duplicates() {
        let t = "skills:\n  - a\n  - b\n  - c\n  - b\n  - d\n";
        assert_eq!(
            remove_values(t, &s(&["a"])).unwrap().0,
            "skills:\n  - b\n  - c\n  - b\n  - d\n"
        );
        assert_eq!(
            remove_values(t, &s(&["c"])).unwrap().0,
            "skills:\n  - a\n  - b\n  - b\n  - d\n"
        );
        assert_eq!(
            remove_values(t, &s(&["d"])).unwrap().0,
            "skills:\n  - a\n  - b\n  - c\n  - b\n"
        );
        let (out, removed) = remove_values(t, &s(&["b", "d"])).unwrap();
        assert_eq!(out, "skills:\n  - a\n  - c\n");
        assert_eq!(removed, s(&["b", "b", "d"]));
    }

    #[test]
    fn untouched_lines_are_byte_identical() {
        let t = "# head\nname: n   # c\nskills:\n  - 'x'  # one\n  - y\n  - \"z\"\ninstruction: |\n  body\n";
        let (out, _) = remove_values(t, &s(&["y"])).unwrap();
        let before: Vec<&str> = t
            .split_inclusive('\n')
            .filter(|l| *l != "  - y\n")
            .collect();
        let after: Vec<&str> = out.split_inclusive('\n').collect();
        assert_eq!(before, after);
    }

    #[test]
    fn add_after_trailing_blank_and_comment_lines() {
        let t = "skills:\n  - a\n\n# about\nname: x\n";
        let (out, _, _) = add_values(t, &s(&["b"])).unwrap();
        assert_eq!(out, "skills:\n  - a\n  - b\n\n# about\nname: x\n");
    }

    #[test]
    fn add_skips_existing_and_repeated_values() {
        let (out, added, skipped) = add_values(BASE, &s(&["three", "new", "new"])).unwrap();
        assert_eq!(added, s(&["new"]));
        assert_eq!(skipped, s(&["three", "new"]));
        assert_eq!(list_entries(&out).unwrap().len(), 5);
        let (out, added, skipped) = add_values(BASE, &s(&["three"])).unwrap();
        assert_eq!(out, BASE);
        assert!(added.is_empty());
        assert_eq!(skipped, s(&["three"]));
    }

    #[test]
    fn refusals() {
        let cases: &[(&str, &str)] = &[
            ("skills: [a, b]\n", "flow sequence"),
            ("skills: &x\n  - a\n", "anchor or alias"),
            ("skills:\n  - *x\n", "starting with `*`"),
            ("skills: !!seq\n  - a\n", "tagged value"),
            ("skills: |\n  a\n", "block scalar"),
            ("skills: a\n", "scalar value"),
            ("skills:\n  - !!str a\n", "starting with `!`"),
            ("metadata:\n  skills:\n    - a\n", "only nested"),
            ("name: x\n", "no `skills:` block"),
            ("skills:\n  - a\nskills:\n  - b\n", "duplicate"),
            ("\"skills\":\n  - a\n", "quoted"),
            ("'skills':\n  - a\n", "quoted"),
            ("skills:\n  - a\n- b\n", "inconsistent indentation"),
            ("skills:\n\t- a\n", "tab"),
            ("skills:\n  - a\n    continued\n", "multi-line or nested"),
            ("skills:\n  - \"a\n    b\"\n", "multi-line quoted"),
            ("skills:\n  - 'a\n", "multi-line quoted"),
            ("skills:\n  - a: b\n", "mapping entry"),
            ("skills:\n  - a:\n", "mapping entry"),
            ("skills:\n  - - a\n", "nested sequence"),
            ("skills:\nname: x\n", "no entries"),
            ("skills:\n  - \"a\" b\n", "after a quoted entry"),
            ("skills:\n  - [a]\n", "starting with `[`"),
            ("skills:\n  - {a: b}\n", "starting with `{`"),
        ];
        for (text, needle) in cases {
            let e = list_entries(text).unwrap_err();
            assert!(e.0.contains(needle), "{text:?}: {e}");
            assert!(remove_values(text, &s(&["a"])).is_err(), "{text:?}");
            assert!(add_values(text, &s(&["z"])).is_err(), "{text:?}");
        }
    }

    #[test]
    fn null_entry_round_trips_untouched() {
        let t = "skills:\n  -\n  - # nothing\n  - a\n  - ~\n";
        assert_eq!(list_entries(t).unwrap(), s(&["", "", "a", ""]));
        let (out, _) = remove_values(t, &s(&["a"])).unwrap();
        assert_eq!(out, "skills:\n  -\n  - # nothing\n  - ~\n");
    }

    #[test]
    fn numeric_looking_value_is_inserted_quoted() {
        let (out, _, _) = add_values("skills:\n  - a\n", &s(&["123"])).unwrap();
        assert_eq!(out, "skills:\n  - a\n  - '123'\n");
        let v: serde_norway::Value = serde_norway::from_str(&out).unwrap();
        assert_eq!(v["skills"][1], serde_norway::Value::String("123".into()));
    }

    #[test]
    fn family_a_quoted_entries_with_special_content() {
        let entries = [
            (r#""a: b""#, "a: b"),
            (r#""a #b""#, "a #b"),
            (r#""a\"b""#, "a\"b"),
            (r#""a\\""#, "a\\"),
            ("'a''b'", "a'b"),
            ("'a: b'", "a: b"),
            ("'a #b'", "a #b"),
        ];
        for (token, value) in entries {
            let t = format!("skills:\n  - keep\n  - {token}\n  - last\n");
            assert_eq!(
                list_entries(&t).unwrap(),
                s(&["keep", value, "last"]),
                "{token}"
            );
            let (out, removed) = remove_values(&t, &s(&[value])).unwrap();
            assert_eq!(out, "skills:\n  - keep\n  - last\n", "{token}");
            assert_eq!(removed, s(&[value]));
            let (out, added, _) = add_values("skills:\n  - keep\n", &s(&[value])).unwrap();
            assert_eq!(added, s(&[value]));
            assert_eq!(list_entries(&out).unwrap(), s(&["keep", value]), "{token}");
            let v: serde_norway::Value = serde_norway::from_str(&out).unwrap();
            assert_eq!(v["skills"][1], serde_norway::Value::String(value.into()));
        }
        let t = "skills:\n  - \"x\"  # say \"hi\"\n  - y\n";
        assert_eq!(list_entries(t).unwrap(), s(&["x", "y"]));
        assert_eq!(remove_values(t, &s(&["x"])).unwrap().0, "skills:\n  - y\n");
    }

    #[test]
    fn family_b_scalar_looking_entries() {
        let t = "skills:\n  - true\n  - 123\n  - null\n  - ~\n  - 1.50\n  - 0x10\n  - yes\n";
        assert_eq!(
            list_entries(t).unwrap(),
            s(&["true", "123", "", "", "1.5", "16", "yes"])
        );
        for v in ["true", "null", "~", "123", "1e3", "0x10"] {
            let (out, _, _) = add_values("skills:\n  - a\n", &s(&[v])).unwrap();
            let line = out.lines().last().unwrap();
            assert!(line.contains('\'') || line.contains('"'), "{v}: {line}");
            let parsed: serde_norway::Value = serde_norway::from_str(&out).unwrap();
            assert_eq!(parsed["skills"][1], serde_norway::Value::String(v.into()));
        }
        for v in ["yes", "no"] {
            let (out, _, _) = add_values("skills:\n  - a\n", &s(&[v])).unwrap();
            assert_eq!(out, format!("skills:\n  - a\n  - {v}\n"));
            let parsed: serde_norway::Value = serde_norway::from_str(&out).unwrap();
            assert_eq!(parsed["skills"][1], serde_norway::Value::String(v.into()));
        }
    }

    #[test]
    fn family_c_entry_equal_to_the_key_name() {
        let t = "skills:\n  - a\n  - skills\n  - \"skills:\"\n";
        assert_eq!(list_entries(t).unwrap(), s(&["a", "skills", "skills:"]));
        let (out, _) = remove_values(t, &s(&["skills"])).unwrap();
        assert_eq!(out, "skills:\n  - a\n  - \"skills:\"\n");
        let (out, _, _) = add_values("skills:\n  - a\n", &s(&["skills"])).unwrap();
        assert_eq!(out, "skills:\n  - a\n  - skills\n");
        let (out, _, _) = add_values("skills:\n  - a\n", &s(&["skills:"])).unwrap();
        let line = out.lines().last().unwrap();
        assert!(line.contains('\'') || line.contains('"'), "{line}");
        assert_eq!(list_entries(&out).unwrap(), s(&["a", "skills:"]));
    }

    #[test]
    fn yaml_scalar_round_trip_table() {
        for v in [
            "plain",
            "a: b",
            "-lead",
            "#x",
            "trail ",
            "123",
            "true",
            "it's",
            "say \"hi\"",
            "caf\u{e9}",
            "skills:",
            "yes",
        ] {
            let tok = yaml_scalar(v);
            let parsed: serde_norway::Value = serde_norway::from_str(&tok).unwrap();
            assert_eq!(
                parsed,
                serde_norway::Value::String(v.into()),
                "{v:?} -> {tok}"
            );
        }
    }

    // ---- proptest over generated block manifests ----

    const LITERALS: &[&str] = &[
        "true", "false", "null", "~", "yes", "no", "123", "1e3", "0x10", "skills", "skills:",
    ];

    fn value_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            "[A-Za-z0-9:/._-]{1,24}".prop_filter("class 1 rules", |t| {
                !t.starts_with('-') && !t.contains(": ")
            }),
            "[A-Za-z0-9:/._ #'\"\\\\-]{1,24}".prop_filter("not blank", |t| !t.trim().is_empty()),
            proptest::sample::select(LITERALS).prop_map(str::to_string),
        ]
    }

    fn plain_ok(v: &str) -> bool {
        v.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && v.chars()
                .all(|c| c.is_ascii_alphanumeric() || ":/._-".contains(c))
            && !v.ends_with(':')
            && matches!(
                serde_norway::from_str::<serde_norway::Value>(v),
                Ok(serde_norway::Value::String(ref s)) if s == v
            )
    }

    fn render(v: &str, double: bool) -> String {
        if plain_ok(v) {
            v.to_string()
        } else if double {
            format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
        } else {
            format!("'{}'", v.replace('\'', "''"))
        }
    }

    #[derive(Debug, Clone)]
    struct Manifest {
        text: String,
        values: Vec<String>,
        item_lines: Vec<String>,
    }

    fn manifest_strategy() -> impl Strategy<Value = Manifest> {
        (
            proptest::collection::vec((value_strategy(), any::<bool>(), 0u8..4, 0u8..4), 1..8),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
        )
            .prop_map(|(items, indent2, crlf, bom, final_nl)| {
                let eol = if crlf { "\r\n" } else { "\n" };
                let ind = if indent2 { "  " } else { "" };
                let mut lines: Vec<String> = vec![
                    "# generated".to_string(),
                    "name: prop".to_string(),
                    "skills:  # list".to_string(),
                ];
                let mut item_lines = Vec::new();
                let mut values = Vec::new();
                for (v, double, comment, extra) in items {
                    let mut line = format!("{ind}- {}", render(&v, double));
                    if comment == 1 {
                        line.push_str("  # note");
                    } else if comment == 2 {
                        line.push_str(" # say \"hi\" 'x'");
                    }
                    lines.push(line.clone());
                    item_lines.push(line);
                    values.push(v);
                    if extra == 1 {
                        lines.push(String::new());
                    } else if extra == 2 {
                        lines.push(format!("{ind}# between"));
                    }
                }
                lines.push("description: after".to_string());
                let mut text = String::new();
                if bom {
                    text.push('\u{feff}');
                }
                text.push_str(&lines.join(eol));
                if final_nl {
                    text.push_str(eol);
                }
                Manifest {
                    text,
                    values,
                    item_lines,
                }
            })
    }

    fn reparsed_skills(text: &str) -> Vec<String> {
        let t = text.strip_prefix('\u{feff}').unwrap_or(text);
        let v: serde_norway::Value = serde_norway::from_str(t).unwrap();
        v["skills"]
            .as_sequence()
            .map(|seq| {
                seq.iter()
                    .map(|x| match x {
                        serde_norway::Value::String(s) => s.clone(),
                        serde_norway::Value::Null => String::new(),
                        other => serde_norway::to_string(other)
                            .unwrap()
                            .trim_end()
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    proptest! {
        #[test]
        fn remove_then_reparse_and_untouched_lines(m in manifest_strategy(), pick in any::<proptest::sample::Index>()) {
            prop_assert_eq!(list_entries(&m.text).unwrap(), m.values.clone());
            let target = m.values[pick.index(m.values.len())].clone();
            let (out, removed) = remove_values(&m.text, std::slice::from_ref(&target)).unwrap();
            let expected: Vec<String> = m.values.iter().filter(|v| **v != target).cloned().collect();
            prop_assert_eq!(removed.len(), m.values.len() - expected.len());
            if expected.is_empty() {
                prop_assert!(list_entries(&out).is_err());
            } else {
                prop_assert_eq!(reparsed_skills(&out), expected.clone());
                prop_assert_eq!(list_entries(&out).unwrap(), expected);
            }
            // Every line that was not removed is byte-identical, in order.
            let removed_lines: Vec<String> = m
                .item_lines
                .iter()
                .zip(&m.values)
                .filter(|(_, v)| **v == target)
                .map(|(l, _)| l.clone())
                .collect();
            let (_, old) = split(&m.text);
            let (_, new) = split(&out);
            let old_kept: Vec<String> = old
                .iter()
                .filter(|l| !removed_lines.contains(&l.body))
                .map(|l| l.body.clone())
                .collect();
            let new_bodies: Vec<String> = new.iter().map(|l| l.body.clone()).collect();
            prop_assert_eq!(old_kept, new_bodies);
        }

        #[test]
        fn add_then_list_appends_in_order(m in manifest_strategy(), extra in proptest::collection::vec(value_strategy(), 1..4)) {
            let mut new_values: Vec<String> = Vec::new();
            for v in extra {
                if !m.values.contains(&v) && !new_values.contains(&v) {
                    new_values.push(v);
                }
            }
            prop_assume!(!new_values.is_empty());
            let (out, added, skipped) = add_values(&m.text, &new_values).unwrap();
            prop_assert_eq!(&added, &new_values);
            prop_assert!(skipped.is_empty());
            let mut expected = m.values.clone();
            expected.extend(new_values);
            prop_assert_eq!(list_entries(&out).unwrap(), expected.clone());
            prop_assert_eq!(reparsed_skills(&out), expected);
        }
    }
}
