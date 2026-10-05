//! Rendering: the `--json` envelope, human text, and the control-character
//! sanitizer applied to every untrusted string printed in human mode.

use serde_json::{Value, json};
use std::borrow::Cow;
use unam_core::catalog::FlatDiagnostic;
use unam_core::error::Severity;

/// Maximum characters of a description head in `search` output.
pub const HEAD_CHARS: usize = 80;

/// Replace C0 controls (except LF and TAB), DEL, and C1 controls with the
/// literal text `\xNN` so untrusted text cannot drive the terminal.
pub fn neutralize(s: &str) -> Cow<'_, str> {
    let bad = |c: char| {
        let n = c as u32;
        (n < 0x20 && c != '\n' && c != '\t') || n == 0x7f || (0x80..=0x9f).contains(&n)
    };
    if !s.chars().any(bad) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if bad(c) {
            out.push_str(&format!("\\x{:02x}", c as u32));
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
}

/// First line of `text`, neutralized, at most `max` characters (cut with `...`).
pub fn head(text: &str, max: usize) -> String {
    let first = text.lines().next().unwrap_or("");
    let clean = neutralize(first);
    if clean.chars().count() <= max {
        clean.into_owned()
    } else {
        let mut cut: String = clean.chars().take(max.saturating_sub(3)).collect();
        cut.push_str("...");
        cut
    }
}

/// Left-aligned table with two spaces between columns (widths in chars).
pub fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    let cols = header.len();
    let mut widths: Vec<usize> = header.iter().map(|h| h.chars().count()).collect();
    for r in rows {
        for (i, cell) in r.iter().enumerate().take(cols) {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let line = |cells: Vec<&str>| {
        let mut s = String::new();
        for (i, c) in cells.iter().enumerate() {
            if i + 1 == cells.len() {
                s.push_str(c);
            } else {
                s.push_str(c);
                let pad = widths[i] - c.chars().count() + 2;
                s.push_str(&" ".repeat(pad));
            }
        }
        s.trim_end().to_string()
    };
    let mut out = line(header.to_vec());
    out.push('\n');
    for r in rows {
        out.push_str(&line(r.iter().map(String::as_str).collect()));
        out.push('\n');
    }
    out
}

/// One human diagnostic line: `warning[code] where: message`.
pub fn diag_line(d: &FlatDiagnostic) -> String {
    let label = match d.diagnostic.severity {
        Severity::Info => "note",
        Severity::Warning => "warning",
        Severity::Error => "skipped",
    };
    let place = d
        .diagnostic
        .path
        .clone()
        .or_else(|| d.skill.clone())
        .or_else(|| d.source.clone())
        .unwrap_or_default();
    let mut s = format!(
        "{label}[{}] {}: {}",
        d.diagnostic.code,
        neutralize(&place),
        neutralize(&d.diagnostic.message)
    );
    if let Some(r) = &d.diagnostic.remedy {
        s.push_str(&format!("\n  hint: {}", neutralize(r)));
    }
    s
}

/// Diagnostics of severity warning or error (the envelope `warnings` array).
pub fn warnings_only(diags: &[FlatDiagnostic]) -> Vec<FlatDiagnostic> {
    diags
        .iter()
        .filter(|d| d.diagnostic.severity != Severity::Info)
        .cloned()
        .collect()
}

/// A structured error for the envelope and the human renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    pub remedy: Option<String>,
    pub candidates: Option<Vec<String>>,
    pub exit_code: i32,
}

impl ErrorInfo {
    pub fn from_anyhow(err: &anyhow::Error) -> Self {
        match err.downcast_ref::<unam_core::error::Error>() {
            Some(e) => ErrorInfo {
                code: e.code().to_string(),
                message: e.to_string(),
                remedy: e.remedy().map(str::to_string),
                candidates: e.candidates().map(<[String]>::to_vec),
                exit_code: e.exit_code(),
            },
            None => ErrorInfo {
                code: "error".to_string(),
                message: format!("{err:#}"),
                remedy: None,
                candidates: None,
                exit_code: 1,
            },
        }
    }

    pub fn to_json(&self) -> Value {
        let mut v = json!({
            "code": self.code,
            "message": self.message,
            "remedy": self.remedy,
        });
        if let Some(c) = &self.candidates {
            v["candidates"] = json!(c);
        }
        v
    }

    /// Human lines for stderr: `error: ...`, the remedy, then candidates.
    pub fn human(&self) -> String {
        let mut s = format!("error: {}\n", neutralize(&self.message));
        if let Some(r) = &self.remedy {
            s.push_str(&format!("hint: {}\n", neutralize(r)));
        }
        if let Some(c) = &self.candidates {
            if !c.is_empty() {
                s.push_str("candidates:\n");
            }
            for id in c {
                s.push_str(&format!("  {}\n", neutralize(id)));
            }
        }
        s
    }
}

/// The `--json` envelope (spec 5).
pub fn envelope(
    ok: bool,
    command: &str,
    data: Value,
    warnings: &[FlatDiagnostic],
    errors: &[ErrorInfo],
) -> Value {
    json!({
        "ok": ok,
        "command": command,
        "data": data,
        "warnings": warnings,
        "errors": errors.iter().map(ErrorInfo::to_json).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use unam_core::error::Diagnostic;

    #[test]
    fn neutralize_controls() {
        assert_eq!(neutralize("a\u{1b}[31mb"), "a\\x1b[31mb");
        assert_eq!(neutralize("\0"), "\\x00");
        assert_eq!(neutralize("\u{7f}"), "\\x7f");
        assert_eq!(neutralize("\u{9b}"), "\\x9b");
        assert_eq!(neutralize("line\nnext\ttab"), "line\nnext\ttab");
        assert_eq!(
            neutralize("caf\u{e9} \u{65e5}\u{672c}"),
            "caf\u{e9} \u{65e5}\u{672c}"
        );
        assert!(matches!(neutralize("plain"), Cow::Borrowed(_)));
    }

    #[test]
    fn head_truncation() {
        assert_eq!(head("short\nsecond line", 80), "short");
        let long = "x".repeat(100);
        let h = head(&long, 80);
        assert_eq!(h.chars().count(), 80);
        assert!(h.ends_with("..."));
        let multi = "\u{e9}".repeat(100);
        let h = head(&multi, 80);
        assert_eq!(h.chars().count(), 80);
        // Neutralized text truncates on a char boundary.
        let esc = format!("{}{}", "\u{1b}".repeat(30), "\u{65e5}".repeat(10));
        let h = head(&esc, 80);
        assert_eq!(h.chars().count(), 80);
        assert!(!h.contains('\u{1b}'));
    }

    #[test]
    fn envelope_shape_has_empty_arrays() {
        let v = envelope(true, "search", json!({"x": 1}), &[], &[]);
        assert_eq!(v["ok"], true);
        assert_eq!(v["command"], "search");
        assert!(v["warnings"].as_array().unwrap().is_empty());
        assert!(v["errors"].as_array().unwrap().is_empty());
        let d = FlatDiagnostic {
            source: Some("fx".into()),
            skill: None,
            diagnostic: Diagnostic::warning("walk-capped", "m"),
        };
        let v = envelope(true, "search", json!({}), &[d], &[]);
        let w = &v["warnings"][0];
        for k in [
            "source", "skill", "code", "severity", "message", "path", "remedy",
        ] {
            assert!(w.get(k).is_some(), "missing {k}");
        }
    }

    #[test]
    fn error_json_candidates_only_when_present() {
        let e = ErrorInfo {
            code: "io-error".into(),
            message: "m".into(),
            remedy: None,
            candidates: None,
            exit_code: 1,
        };
        assert!(e.to_json().get("candidates").is_none());
        let err = anyhow::Error::new(unam_core::error::Error::Ambiguous {
            reference: "x".into(),
            candidates: vec!["a:b".into(), "a:c".into()],
        });
        let info = ErrorInfo::from_anyhow(&err);
        assert_eq!(info.exit_code, 5);
        assert_eq!(info.to_json()["candidates"][1], "a:c");
    }

    #[test]
    fn table_alignment() {
        let t = table(&["A", "BB"], &[vec!["long".into(), "x".into()]]);
        assert_eq!(t, "A     BB\nlong  x\n");
    }
}
