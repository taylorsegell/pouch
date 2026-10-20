//! Rendering: the `--json` envelope, human text, and the control-character
//! sanitizer applied to every untrusted string printed in human mode.

use pouch_core::adapter::RefreshNote;
use pouch_core::catalog::{Catalog, FlatDiagnostic, SOURCE_AND_CONFIG};
use pouch_core::error::Severity;
use pouch_core::plan::{Plan, Step, StepOutcome, StepResult};
use pouch_core::project::{RootDetection, TargetId};
use pouch_core::resolve::Provenance;
use pouch_core::state::strategy_text;
use pouch_core::verify::{EntryState, Reason, Record};
use serde_json::{Value, json};
use std::borrow::Cow;

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

/// Config-level plus source-level warnings of a catalog load; the CLI and
/// the picker both report exactly these.
pub fn catalog_warning_set(
    config_diags: &[FlatDiagnostic],
    catalog: &Catalog,
) -> Vec<FlatDiagnostic> {
    let mut all = config_diags.to_vec();
    all.extend(catalog.flat_diagnostics(None, SOURCE_AND_CONFIG));
    warnings_only(&all)
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
        let core = err
            .downcast_ref::<crate::cli::DataError>()
            .map(|d| &d.error)
            .or_else(|| err.downcast_ref::<pouch_core::error::Error>());
        match core {
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

// ---- activation output (milestone M3; CLI Contract) ----

/// The SES-2 sentence (spec 5), printed whenever a removal was applied.
pub const REMOVAL_CAVEAT: &str = "Removed from future discovery. A conversation that already loaded these instructions keeps them until that conversation ends.";

/// The honesty line printed when a plan writes a bundle brief (spec 11, PRD
/// G12): a brief is discoverable, not guaranteed to be read.
pub const BRIEF_NOTE: &str =
    "A bundle brief is a skill the agent can discover; pouch cannot make the agent read it.";

/// Most instruction lines shown under a brief in a plan.
const BRIEF_LINES_SHOWN: usize = 20;

/// The shared-destination sentence of an `agents-neutral` plan or result
/// (spec 4.7, AC-43).
pub fn visible_to_line(consumers: &[String]) -> String {
    format!("Visible to: {}.", consumers.join(", "))
}

/// The `.gitignore` offer heading (OWN-7).
pub const GITIGNORE_HEADING: &str = "Add these lines to .gitignore (pouch never edits it):";

/// `fx:sdlc/code-review` as (`sdlc/code-review`, `fx`).
fn split_id(id: &str) -> (String, String) {
    match id.split_once(':') {
        Some((alias, rel)) => (rel.to_string(), alias.to_string()),
        None => (id.to_string(), String::new()),
    }
}

fn root_how_text(how: RootDetection) -> &'static str {
    match how {
        RootDetection::Git => "git root",
        RootDetection::Explicit => "explicit",
    }
}

/// The plan as human text (spec 5). Every untrusted string is neutralized.
pub fn render_plan(plan: &Plan, refresh: Option<&RefreshNote>) -> String {
    let h = &plan.header;
    let mut s = String::new();
    s.push_str(&format!(
        "Project   {}  ({})\n",
        neutralize(&h.root.to_string_lossy()),
        root_how_text(h.root_how)
    ));
    s.push_str(&format!(
        "Target    {}  ->  {}/        visible to: {}\n",
        h.target.as_str(),
        neutralize(&h.dest_rel),
        neutralize(&h.consumers.join(", "))
    ));
    s.push_str(&format!("Strategy  {}\n", strategy_text(h.strategy)));
    if let Some(d) = &h.detection_text {
        s.push_str(&format!("Detected  {}\n", neutralize(d)));
    }
    if h.target == TargetId::AgentsNeutral {
        s.push_str(&format!(
            "Shared    {}\n",
            neutralize(&visible_to_line(&h.consumers))
        ));
    }
    s.push('\n');
    let mut body = String::new();
    for step in &plan.steps {
        let line = match step {
            Step::Add { id, name, .. } => {
                let (rel, alias) = split_id(id);
                format!(
                    "  + {:<28} {:<4} ->  {}/{}",
                    neutralize(&rel),
                    neutralize(&alias),
                    neutralize(&h.dest_rel),
                    neutralize(name)
                )
            }
            Step::Remove { id, .. } => {
                let (rel, alias) = split_id(id);
                format!(
                    "  - {:<28} {:<4} (remove, tool-owned)",
                    neutralize(&rel),
                    neutralize(&alias)
                )
            }
            Step::UpdateProvenance { id, .. } => {
                let (rel, alias) = split_id(id);
                format!(
                    "  ~ {:<28} {:<4} (provenance updated)",
                    neutralize(&rel),
                    neutralize(&alias)
                )
            }
            Step::DropOrphan { name, reason, .. } => format!(
                "  x {:<33} (released: file was {}; left untouched)",
                neutralize(name),
                reason.as_str()
            ),
            Step::RemoveDir { dir, .. } => format!(
                "  d {}/   (created by pouch; removed only when empty)",
                neutralize(dir)
            ),
            Step::WriteGenerated {
                bundle,
                name,
                content,
                ..
            } => {
                let body = pouch_core::generated::brief_body(content);
                let mut lines: Vec<String> = body
                    .lines()
                    .take(BRIEF_LINES_SHOWN)
                    .map(|l| format!("      {}", neutralize(l)))
                    .collect();
                let more = body.lines().count().saturating_sub(BRIEF_LINES_SHOWN);
                if more > 0 {
                    lines.push(format!("      ... ({more} more line(s))"));
                }
                format!(
                    "  generated  {}  (instruction of bundle {})\n{}",
                    neutralize(name),
                    neutralize(bundle),
                    lines.join("\n")
                )
            }
            Step::RemoveGenerated { bundle, name, .. } => format!(
                "  - generated  {}  (remove, instruction of bundle {})",
                neutralize(name),
                neutralize(bundle)
            ),
        };
        body.push_str(&line);
        body.push('\n');
    }
    for e in &plan.unchanged {
        let (rel, alias) = split_id(&e.id);
        body.push_str(&format!(
            "  = {:<28} {:<4} (unchanged)\n",
            neutralize(&rel),
            neutralize(&alias)
        ));
    }
    for e in &plan.carried {
        let (rel, alias) = split_id(&e.id);
        body.push_str(&format!(
            "  = {:<28} {:<4} (unchanged; source unreachable)\n",
            neutralize(&rel),
            neutralize(&alias)
        ));
    }
    if plan
        .steps
        .iter()
        .any(|st| matches!(st, Step::WriteGenerated { .. }))
    {
        body.push_str(&format!("  {BRIEF_NOTE}\n"));
    }
    if !body.is_empty() {
        s.push_str(&body);
        s.push('\n');
    }
    let mut notes = String::new();
    for c in &plan.conflicts {
        notes.push_str(&format!("  ! conflict  {}\n", neutralize(&c.message())));
    }
    for w in &plan.warnings {
        notes.push_str(&format!(
            "  ! {}  {}\n",
            neutralize(&w.code),
            neutralize(&w.message)
        ));
    }
    if !notes.is_empty() {
        s.push_str(&notes);
        s.push('\n');
    }
    s.push_str(&format!(
        "Catalog estimate   {} {}, ~{} tokens (estimate)\n",
        plan.estimate.skills,
        if plan.estimate.skills == 1 {
            "skill"
        } else {
            "skills"
        },
        plan.estimate.tokens
    ));
    if let Some(r) = refresh
        && !plan.steps.is_empty()
    {
        // The plan is shown before the apply, so the note reads in future tense.
        let text = r
            .text
            .replacen("this apply created", "this apply will create", 1);
        s.push_str(&format!("After apply        {}\n", neutralize(&text)));
    }
    s
}

/// What a finished change prints after the plan.
pub struct ResultView<'a> {
    pub results: &'a [StepResult],
    pub refresh: Option<&'a RefreshNote>,
    pub followups: &'a [String],
    pub removal_applied: bool,
    pub gitignore_lines: &'a [String],
    /// The consumers of a shared destination (empty: print nothing).
    pub visible_to: &'a [String],
}

/// The result block (spec 5): counts, failed and skipped lines, SES-1,
/// followups, SES-2, and the `.gitignore` offer.
pub fn render_result(v: &ResultView<'_>) -> String {
    let done = |op: &str| {
        v.results
            .iter()
            .filter(|r| r.op == op && r.outcome == StepOutcome::Done)
            .count()
    };
    let adds = done("add") + done("write_generated");
    let removes = done("remove") + done("drop_orphan") + done("remove_generated");
    let updates = done("update_provenance");
    let total = adds + removes + updates;
    let mut s = String::new();
    if v.results.is_empty() {
        s.push_str("No changes: the destination already matches the selection.\n");
    } else {
        s.push_str(&format!(
            "Applied {total} change(s): +{adds} -{removes} ~{updates}\n"
        ));
    }
    for r in v.results {
        let name = neutralize(r.name.as_deref().unwrap_or(r.op)).into_owned();
        match &r.outcome {
            StepOutcome::Failed { message, .. } => {
                s.push_str(&format!("  failed  {name}: {}\n", neutralize(message)));
            }
            StepOutcome::Skipped(reason) => {
                s.push_str(&format!("  skipped {name}: {}\n", reason.code()));
            }
            StepOutcome::Done => {}
        }
    }
    if let Some(r) = v.refresh {
        s.push_str(&format!("After apply   {}\n", neutralize(&r.text)));
    }
    if !v.visible_to.is_empty() {
        s.push_str(&format!("{}\n", neutralize(&visible_to_line(v.visible_to))));
    }
    for f in v.followups {
        s.push_str(&format!("Then          {}\n", neutralize(f)));
    }
    if v.removal_applied {
        s.push_str(REMOVAL_CAVEAT);
        s.push('\n');
    }
    if !v.gitignore_lines.is_empty() {
        s.push_str(GITIGNORE_HEADING);
        s.push('\n');
        for l in v.gitignore_lines {
            s.push_str(&format!("  {l}\n"));
        }
    }
    s
}

/// The `plan` object of `--json` data.
pub fn plan_json(plan: &Plan) -> Value {
    plan.to_json()
}

/// One `results` row.
pub fn result_json(r: &StepResult) -> Value {
    let (result, code, message) = match &r.outcome {
        StepOutcome::Done => ("done", Value::Null, Value::Null),
        StepOutcome::Failed { code, message } => ("failed", json!(code), json!(message)),
        StepOutcome::Skipped(reason) => ("skipped", json!(reason.code()), Value::Null),
    };
    json!({
        "n": r.n, "op": r.op, "name": r.name, "result": result, "code": code, "message": message,
    })
}

/// The header fields shared by every activation command.
pub struct ChangeHeader<'a> {
    pub command: &'a str,
    pub project_root: &'a str,
    pub root_detection: RootDetection,
    pub target: &'a str,
    pub destination: &'a str,
    pub strategy: Option<&'a str>,
    pub detection: &'a str,
}

/// Everything after the header in the `--json` data.
pub struct ChangeBody<'a> {
    pub status: &'a str,
    pub op_id: Option<&'a str>,
    pub selection: Value,
    pub plan: Option<&'a Plan>,
    pub results: &'a [StepResult],
    pub receipt: Value,
    pub refresh: Option<&'a RefreshNote>,
    pub followups: &'a [String],
    pub removal_applied: bool,
    pub gitignore_lines: &'a [String],
    pub visible_to: &'a [String],
}

/// The `--json` data of `add`, `rm`, `use`, `deactivate`, `resume`,
/// `rollback` (CLI Contract; keys in contract order).
pub fn change_json(h: &ChangeHeader<'_>, b: &ChangeBody<'_>) -> Value {
    json!({
        "command": h.command,
        "project_root": h.project_root,
        "root_detection": match h.root_detection {
            RootDetection::Git => "git",
            RootDetection::Explicit => "explicit",
        },
        "target": h.target,
        "destination": h.destination,
        "strategy": h.strategy,
        "detection": h.detection,
        "status": b.status,
        "op_id": b.op_id,
        "selection": b.selection,
        "plan": b.plan.map(plan_json),
        "results": b.results.iter().map(result_json).collect::<Vec<_>>(),
        "receipt": b.receipt,
        "refresh": b.refresh,
        "followups": b.followups,
        "removal_caveat": if b.removal_applied { json!(REMOVAL_CAVEAT) } else { Value::Null },
        "gitignore_lines": b.gitignore_lines,
        "visible_to": b.visible_to,
    })
}

// ---- status output (milestone M4; spec 4.11, 5) ----

/// `bundle:slug` and `adhoc` joined with commas (`-` when empty).
fn provenance_text(p: &[Provenance]) -> String {
    if p.is_empty() {
        return "-".to_string();
    }
    p.iter()
        .map(|x| match x {
            Provenance::Bundle(slug) => format!("bundle:{slug}"),
            Provenance::Adhoc => "adhoc".to_string(),
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn reason_text(r: Reason) -> &'static str {
    match r {
        Reason::Missing => "the destination is missing",
        Reason::BrokenLink => "the link does not resolve",
        Reason::Modified => "the destination differs from what pouch placed",
        Reason::SourceSkillMissing => "the skill is no longer in its source",
    }
}

/// The project and target lines above a status or reconcile report.
pub fn render_target_header(
    root: &std::path::Path,
    how: RootDetection,
    target: &str,
    dest_rel: &str,
) -> String {
    format!(
        "Project   {}  ({})\nTarget    {}  ->  {}/\n\n",
        neutralize(&root.to_string_lossy()),
        root_how_text(how),
        target,
        neutralize(dest_rel)
    )
}

/// The six-state table, one detail line per orphaned or shadowed entry,
/// and the spec 4.11 footer.
pub fn render_records(records: &[Record]) -> String {
    let mut out = String::new();
    if records.is_empty() {
        out.push_str("No entries: nothing is selected, applied, or found in the destination.\n");
    } else {
        let rows: Vec<Vec<String>> = records
            .iter()
            .map(|r| {
                vec![
                    r.state.as_str().to_string(),
                    neutralize(r.id.as_deref().unwrap_or(&r.name)).into_owned(),
                    r.strategy.map_or("-", strategy_text).to_string(),
                    provenance_text(&r.provenance),
                    neutralize(&r.dest).into_owned(),
                    r.source_path
                        .as_deref()
                        .map_or("-".to_string(), |p| neutralize(p).into_owned()),
                ]
            })
            .collect();
        out.push_str(&table(
            &["STATE", "SKILL", "STRATEGY", "PROVENANCE", "DEST", "SOURCE"],
            &rows,
        ));
        let mut notes = String::new();
        for r in records {
            match r.state {
                EntryState::Orphaned => {
                    let why = r.reason.map_or("", reason_text);
                    notes.push_str(&format!(
                        "  ! {}: orphaned ({}): {why}\n",
                        neutralize(&r.dest),
                        r.reason.map_or("-", Reason::as_str),
                    ));
                    if !r.candidates.is_empty() {
                        let old = r.id.as_deref().unwrap_or("");
                        notes.push_str(&format!(
                            "      closest in the source: {}\n      to switch: `pouch rm {}` then `pouch add {}`\n",
                            neutralize(&r.candidates.join(", ")),
                            neutralize(old),
                            neutralize(&r.candidates[0]),
                        ));
                    }
                }
                EntryState::Shadowed => notes.push_str(&format!(
                    "  ! {}: shadowed by {}\n",
                    neutralize(&r.dest),
                    neutralize(
                        r.shadowed_by
                            .as_deref()
                            .unwrap_or("a higher-precedence scope")
                    ),
                )),
                _ => {}
            }
        }
        if !notes.is_empty() {
            out.push('\n');
            out.push_str(&notes);
        }
    }
    out.push('\n');
    out.push_str(pouch_core::verify::FOOTER);
    out.push('\n');
    out
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
    use pouch_core::error::Diagnostic;

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
        let err = anyhow::Error::new(pouch_core::error::Error::Ambiguous {
            reference: "x".into(),
            candidates: vec!["a:b".into(), "a:c".into()],
        });
        let info = ErrorInfo::from_anyhow(&err);
        assert_eq!(info.exit_code, 5);
        assert_eq!(info.to_json()["candidates"][1], "a:c");
    }

    mod activation {
        use super::super::*;
        use pouch_core::config::Strategy;
        use pouch_core::error::Diagnostic;
        use pouch_core::fsops::{OwnedExpectation, SkipReason};
        use pouch_core::hash::Hash;
        use pouch_core::plan::{Conflict, Estimate, OrphanReason, PlanEntry, PlanHeader};
        use pouch_core::project::TargetId;
        use std::collections::BTreeMap;
        use std::path::PathBuf;

        fn plan() -> Plan {
            Plan {
                header: PlanHeader {
                    root: PathBuf::from("/p"),
                    root_how: RootDetection::Git,
                    target: TargetId::ClaudeCode,
                    dest_rel: ".claude/skills".into(),
                    strategy: Strategy::Symlink,
                    detection_text: Some("not detected; .claude/skills will be created".into()),
                    consumers: vec!["claude-code".into(), "cursor (compat read)".into()],
                },
                op_id: "0000000000001-0000abcd".into(),
                steps: vec![
                    Step::Remove {
                        n: 1,
                        id: "fx:legacy/old-linter".into(),
                        name: "old-linter".into(),
                        expect: OwnedExpectation {
                            strategy: Strategy::Symlink,
                            source: "/s".into(),
                            tree_hash: None,
                        },
                    },
                    Step::DropOrphan {
                        n: 2,
                        id: "fx:a/gone".into(),
                        name: "gone".into(),
                        reason: OrphanReason::Modified,
                    },
                    Step::Add {
                        n: 3,
                        id: "fx:sdlc/code-review".into(),
                        name: "code-review\u{1b}[31m".into(),
                        strategy: Strategy::Symlink,
                        source: PathBuf::from("/lib/sdlc/code-review"),
                        tree_hash: Hash::of_bytes(b"x"),
                        provenance: vec![],
                    },
                    Step::UpdateProvenance {
                        n: 4,
                        id: "fx:x/y".into(),
                        name: "y".into(),
                        provenance: vec![],
                    },
                    Step::RemoveDir {
                        n: 5,
                        dir: ".claude/skills".into(),
                    },
                ],
                unchanged: vec![PlanEntry {
                    id: "fx:docs/api-reference".into(),
                    name: "api-reference".into(),
                    dest: ".claude/skills/api-reference".into(),
                }],
                carried: vec![],
                conflicts: vec![Conflict::DestNotDirectory {
                    dest_root: ".claude/skills".into(),
                }],
                warnings: vec![Diagnostic::warning(
                    "shadowed",
                    "code-review is shadowed by /h/.claude/skills/code-review (user scope wins)",
                )],
                estimate: Estimate {
                    skills: 4,
                    tokens: 310,
                },
                selection_hash: Hash::of_bytes(b"s"),
                dest_dirs_to_create: vec![],
            }
        }

        fn note() -> RefreshNote {
            RefreshNote {
                kind: "none",
                command: None,
                text: "no restart or reload required (evidence: https://code.claude.com/docs/en/skills)".into(),
                evidence: "https://code.claude.com/docs/en/skills".into(),
            }
        }

        #[test]
        fn render_plan_line_kinds() {
            let s = render_plan(&plan(), Some(&note()));
            for want in [
                "Project   /p  (git root)",
                "Target    claude-code  ->  .claude/skills/",
                "visible to: claude-code, cursor (compat read)",
                "Strategy  symlink",
                "Detected  not detected; .claude/skills will be created",
                "  + sdlc/code-review",
                "->  .claude/skills/code-review",
                "  - legacy/old-linter",
                "(remove, tool-owned)",
                "  x gone",
                "(released: file was modified; left untouched)",
                "  ~ x/y",
                "(provenance updated)",
                "  d .claude/skills/   (created by pouch; removed only when empty)",
                "  = docs/api-reference",
                "(unchanged)",
                "  ! conflict  .claude/skills exists and is not a directory",
                "  ! shadowed  code-review is shadowed by",
                "Catalog estimate   4 skills, ~310 tokens (estimate)",
                "After apply        no restart or reload required",
            ] {
                assert!(s.contains(want), "missing {want:?} in\n{s}");
            }
            assert!(!s.contains('\u{1b}'), "control characters are neutralized");
            assert!(s.contains("\\x1b"));
        }

        #[test]
        fn render_plan_brief_lines_honesty_note_and_shared_sentence() {
            let mut p = plan();
            p.header.target = TargetId::AgentsNeutral;
            p.header.consumers = ["codex", "cursor", "windsurf-devin", "hermes"]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let long: String = (1..=25).map(|i| format!("Line {i}\n")).collect();
            p.steps.push(Step::WriteGenerated {
                n: 6,
                bundle: "b".into(),
                name: "b-brief".into(),
                content: format!("---\nname: b-brief\n---\n\nFirst\u{1b}[31m\n{long}"),
                content_hash: Hash::of_bytes(b"c"),
                adopt: false,
            });
            p.steps.push(Step::RemoveGenerated {
                n: 7,
                bundle: "old".into(),
                name: "old-brief".into(),
                expect_hash: Hash::of_bytes(b"o"),
            });
            let s = render_plan(&p, None);
            for want in [
                "Shared    Visible to: codex, cursor, windsurf-devin, hermes.",
                "  generated  b-brief  (instruction of bundle b)",
                "      First\\x1b[31m",
                "      Line 18",
                "      ... (6 more line(s))",
                "  - generated  old-brief  (remove, instruction of bundle old)",
                BRIEF_NOTE,
            ] {
                assert!(s.contains(want), "missing {want:?} in\n{s}");
            }
            assert!(!s.contains("Line 20"), "{s}");
            assert!(!s.contains('\u{1b}'));
            // The claude-code plan has neither the shared sentence nor a note
            // unless a brief is written.
            let cc = render_plan(&plan(), None);
            assert!(!cc.contains("Shared") && !cc.contains(BRIEF_NOTE), "{cc}");
        }

        #[test]
        fn render_result_counts_briefs_and_prints_followups_and_consumers() {
            let results = vec![
                StepResult {
                    n: 1,
                    op: "write_generated",
                    id: None,
                    name: Some("b-brief".into()),
                    dest: None,
                    outcome: StepOutcome::Done,
                },
                StepResult {
                    n: 2,
                    op: "remove_generated",
                    id: None,
                    name: Some("old-brief".into()),
                    dest: None,
                    outcome: StepOutcome::Done,
                },
            ];
            let consumers = vec!["codex".to_string(), "hermes".to_string()];
            let followups = vec!["hermes skills trust /p  (why)".to_string()];
            let s = render_result(&ResultView {
                results: &results,
                refresh: None,
                followups: &followups,
                removal_applied: true,
                gitignore_lines: &[],
                visible_to: &consumers,
            });
            assert!(s.contains("Applied 2 change(s): +1 -1 ~0"), "{s}");
            assert!(s.contains("Visible to: codex, hermes."), "{s}");
            assert!(
                s.contains("Then          hermes skills trust /p  (why)"),
                "{s}"
            );
            assert!(s.contains("Removed from future discovery."), "{s}");
        }

        #[test]
        fn render_result_counts_ses2_and_gitignore() {
            let results = vec![
                StepResult {
                    n: 1,
                    op: "remove",
                    id: None,
                    name: Some("a".into()),
                    dest: None,
                    outcome: StepOutcome::Done,
                },
                StepResult {
                    n: 2,
                    op: "add",
                    id: None,
                    name: Some("b".into()),
                    dest: None,
                    outcome: StepOutcome::Failed {
                        code: "copy-refused".into(),
                        message: "link leaves".into(),
                    },
                },
                StepResult {
                    n: 3,
                    op: "add",
                    id: None,
                    name: Some("c".into()),
                    dest: None,
                    outcome: StepOutcome::Skipped(SkipReason::DestOccupied),
                },
            ];
            let lines = vec![".pouch/lock".to_string()];
            let s = render_result(&ResultView {
                results: &results,
                refresh: Some(&note()),
                followups: &[],
                removal_applied: true,
                gitignore_lines: &lines,
                visible_to: &[],
            });
            assert!(s.contains("Applied 1 change(s): +0 -1 ~0"), "{s}");
            assert!(s.contains("  failed  b: link leaves"));
            assert!(s.contains("  skipped c: dest-occupied"));
            assert!(s.contains("After apply   no restart or reload required"));
            assert!(s.contains("Removed from future discovery. A conversation that already loaded these instructions keeps them until that conversation ends."));
            assert!(s.contains(GITIGNORE_HEADING));
            assert!(s.contains("  .pouch/lock"));
            let s = render_result(&ResultView {
                results: &[],
                refresh: None,
                followups: &[],
                removal_applied: false,
                gitignore_lines: &[],
                visible_to: &[],
            });
            assert_eq!(
                s,
                "No changes: the destination already matches the selection.\n"
            );
        }

        #[test]
        fn change_json_keys_in_contract_order() {
            let p = plan();
            let v = change_json(
                &ChangeHeader {
                    command: "add",
                    project_root: "/p",
                    root_detection: RootDetection::Explicit,
                    target: "claude-code",
                    destination: ".claude/skills",
                    strategy: Some("symlink"),
                    detection: "absent",
                },
                &ChangeBody {
                    status: "applied",
                    op_id: Some("0000000000001-0000abcd"),
                    selection: json!(null),
                    plan: Some(&p),
                    results: &[],
                    receipt: json!(null),
                    refresh: Some(&note()),
                    followups: &[],
                    removal_applied: false,
                    gitignore_lines: &[],
                    visible_to: &[],
                },
            );
            let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
            assert_eq!(
                keys,
                vec![
                    "command",
                    "project_root",
                    "root_detection",
                    "target",
                    "destination",
                    "strategy",
                    "detection",
                    "status",
                    "op_id",
                    "selection",
                    "plan",
                    "results",
                    "receipt",
                    "refresh",
                    "followups",
                    "removal_caveat",
                    "gitignore_lines",
                    "visible_to"
                ]
            );
            assert_eq!(v["root_detection"], "explicit");
            assert!(v["removal_caveat"].is_null());
            assert_eq!(v["refresh"]["kind"], "none");
            let _ = BTreeMap::<u32, u32>::new();
        }
    }

    #[test]
    fn table_alignment() {
        let t = table(&["A", "BB"], &[vec!["long".into(), "x".into()]]);
        assert_eq!(t, "A     BB\nlong  x\n");
    }
}
