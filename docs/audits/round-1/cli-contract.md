# Round 1 — cli-contract audit

**Commit audited:** 73c128506ec712e88b6a1db0983892a1a831d1c8
**Lens:** cli-contract
**Reviewer:** Opus 4.7

## Summary

All 25 minimal-surface verbs plus the documented extras (`doctor`, `hook snippet`, `loop activate`, `close record-review`) are wired and honor the stable JSON envelope. Every handoff-suggested error code is present in `CliError`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass silently on HEAD, and the two prior blocking findings (cli-contract-audit.md P1-1 outcome-ratify gate on `plan choose-level`, iter4-cli-contract-audit.md F1 `plan.task_ids` upgrade-trap) are fixed at `crates/codex1/src/cli/plan/choose_level.rs:26-28` and `crates/codex1/src/cli/plan/check.rs:72`.

## P0

None.

## P1

None.

## P2

None.

## P3 (non-blocking)

### Handoff "suggested verdict value" `complete` vs code emits `terminal_complete`
- **Citation:** `crates/codex1/src/state/readiness.rs:31` emits `"terminal_complete"` for `Verdict::TerminalComplete`; `docs/codex1-rebuild-handoff/02-cli-contract.md:219` lists `complete` in the "Suggested verdict values" block.
- **Evidence:** `codex1 close complete --json` followed by `codex1 status --json` returns `"verdict": "terminal_complete"`, not `"complete"`. The handoff explicitly labels these as "Suggested" (line 210) and immediately cautions (line 223) "Do not call the mission complete when it is only ready for mission-close review." The emitted `terminal_complete` respects that guidance while still disambiguating from `mission_close_review_passed`; prior iter-4 audit accepted the same drift without comment.
- **Suggested fix:** Optional. Either (a) amend handoff line 219 from `complete` to `terminal_complete` to match code and keep the precise mission-close vocabulary the same section demands, or (b) leave as-is and treat the handoff list as illustrative — the "Suggested" label permits either.
