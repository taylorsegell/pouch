# v1 static activation: scheduled waves

Set by Taylor on 2026-10-09 to conserve 5-hour session usage. Each wave does only its listed work, ticks its box, reports, and STOPS. Wave 1 at 08:37 MDT, then every 5 hours. All agents run on sonnet, including the code-writing execute agent (Taylor, 2026-10-09). Never fable. Never push. Hard stops still apply.

Lighter process for M4 and M5 (Taylor-approved recommendation, 2026-10-09):
- Threat model: same user, trusted local filesystem; untrusted input is only skill file content and files from cloned repos. Validators must not add hardening beyond this.
- Plans capped at about 400 lines: decisions, AC-to-test map, short gate list; no restated function specs.
- PVL at most 2 cycles; only FAIL-class gaps (wrong requirement mapping, steps that cannot compile in order) trigger a cycle; nits go to execute time.
- Gates: fmt, clippy, cargo test --workspace --locked, perf (only if catalog code changes), source and foreign-file invariants, protected-docs diff.
- v1 is symlink-only going forward: keep existing copy code, add no new copy-specific branches in M4 or M5.
- M4 keeps a short evidence check (reconcile --repair deletes); M5 runs the light lane.

## Waves

- [x] Wave 1 (08:37 MDT Oct 9, ran partial): DONE: threat model added to process/context/all-context.md; open-items sheet written (open-items-decision-sheet_REF_09-10-26.md, 36 rows, 7 need a decision). CARRY-OVER DONE later on Oct 9 after Taylor approved M3 in chat: M3 closed and committed (d34bcb5, 7086c6d); slugify table machinery removed (about 1,640 lines); AC-31 fault matrix samples by default (UNAM_FULL_FAULT_MATRIX=1 for the full 400-run set); 510 tests green. Planned: M3 close ONLY IF Taylor has recorded harness/review-decision.json themselves or explicitly approved it in chat (never write it on his behalf): then run M3 UPDATE PROCESS (sonnet) and commit M3 (code commit, then process commit). If no decision exists, skip M3 close and the code changes below, and do only the threat model and open-items sheet. Then (only after M3 is committed) one sonnet execute agent: remove the slugify Unicode machinery (word_table.rs, tools/gen-word-table.py, tools/rust-word-ranges.txt, the related tests and gates) in favor of `c == '_' || c.is_alphanumeric()` plus a documented-differences note in docs/adapters.md; make the AC-31 fault matrix run a strided sample by default with the full matrix under an env var (UNAM_FULL_FAULT_MATRIX=1); run fmt, clippy, workspace tests; commit. Add the threat model paragraph to process/context/all-context.md standing rules (not to the protected brief). Write the open-items sheet (umbrella items 15 to 55 with a proposed answer each) to the task folder for Taylor. Stop.
- [x] Wave 2 (13:37 MDT Oct 9, done): M4 plan rewritten at 339 lines (light process: research and design folded into one sonnet plan agent); PVL cycle 1 gate PASS with 0 FAIL-class gaps and execute-time notes EX1 to EX8. Planned: M4 research, innovate (only if real design choices), plan supplement (cap about 400 lines), PVL (max 2 cycles). All sonnet. Stop at PVL verdict.
- [x] Wave 3 (ran 04:37 MDT Oct 10 via the wave-5 job, since waves 3 and 4 were not scheduled): M4 implemented (567 tests), EVL CLEAN (G1 to G6 green, full fault matrix passes). M4 code is NOT committed yet; wave 4 commits it with UPDATE PROCESS. Planned: M4 execute (sonnet) and EVL (sonnet tester). Stop.
- [x] Wave 4 (23:37 MDT Oct 9, ran 2026-10-12): M4 evidence check and UPDATE PROCESS, commit M4. M5 research and plan (light lane) and PVL. Stop. PROGRESS 2026-10-12: M4 evidence PASS; UPDATE PROCESS done (report `phase-05-m4-verify-status-reconcile-rollback_REPORT_12-10-26.md`, umbrella state, context); M4 committed (c3c9be7 code, 8d688e1 process); M5 plan supplemented (research folded into one sonnet plan agent) and PVL cycle 1 Gate: PASS with 0 FAIL-class gaps and execute-time notes EX1 to EX14 (EX1 first: keep hermes-only.yaml byte-identical, use a test-side opt-out copy). Wave 5 is next: M5 execute, EVL, UPDATE PROCESS, commit, then stop at the product Phase 1 gate.
- [ ] Wave 5 (04:37 MDT Oct 10): M5 execute, EVL, UPDATE PROCESS, commit. Stop at the product Phase 1 gate (hard stop for Taylor).

Gate for waves 2 to 5: if M3 is still uncommitted because no review decision exists, the wave does nothing except re-check for the decision, report, and stop. M4 must not start on an uncommitted M3.

If a wave cannot finish its work, it records where it stopped here and the next wave continues from that point before starting its own list.
