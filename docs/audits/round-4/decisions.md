# Round 4 decisions

Baseline audited: 703a171 (round-3 fixes). Round-4 audits committed at d7c708e. Fix commit: this commit.

Reviewer reports: `docs/audits/round-4/{cli-contract,e2e-walkthrough,handoff-cross-check,skills-audit,correctness-invariants,test-adequacy}.md`.

Cross-reviewer dedupe:

- **cli-contract P2-1 ≡ e2e-walkthrough P2-1** — same `review packet` YAML block-scalar leak, same root cause, same fix. Resolved once below.

Correctness-invariants and handoff-cross-check each reported 0 findings at every priority.

## FIX

- **cli-contract/e2e-walkthrough P2-1 · `review packet` leaks YAML block-scalar indicator `|` into `mission_summary`** — replaced the substring-based `read_interpreted_destination` in `crates/codex1/src/cli/review/packet.rs` with a `serde_yaml::from_str` parse of the OUTCOME.md frontmatter, mirroring the sibling implementations at `crates/codex1/src/cli/task/worker_packet.rs:60-86` and `crates/codex1/src/cli/close/closeout.rs:136-150`. The old parser trimmed only trailing whitespace from each line, so the leading space between `interpreted_destination:` and `|` defeated the `trimmed == "|"` guard, letting the `|` token into the output. The fix also adds a local `extract_frontmatter` helper (CRLF-tolerant, matches the task/worker_packet copy). Parse failures remain silent (return `None`) to match the sibling tolerance model — the packet is an informational artifact, not a gate. Test: `tests/review.rs::review_packet_mission_summary_strips_yaml_block_scalar` — seeds OUTCOME.md with `interpreted_destination: |\n  Body line 1\n  Body line 2`, runs `review packet T5 --mission demo`, asserts `mission_summary` does not contain `|`, starts with neither `|` nor whitespace, and preserves the `"Body line 1\nBody line 2"` joined body.
- **test-adequacy P2-1 · `CliError::ReviewFindingsBlock` envelope has no triggering integration test** — added `tests/review.rs::review_record_findings_then_retry_returns_review_findings_block_envelope`. Starts a review, then records with `--findings-file does/not/exist.md`, asserts `ok == false`, `code == "REVIEW_FINDINGS_BLOCK"`, `retryable == false`, and that `message` contains both `"findings file not found"` and the offending path. The prior single match for the code string in the test suite (`tests/close.rs:304`) came from a `Blocker` struct in `close check`, structurally independent of the `CliError` variant at `src/cli/review/record.rs:57`. No production code change.

## REJECT

- **skills-audit P3-1 · `execute/SKILL.md:32` prose says `task next` "cannot surface repair or replan"** — descriptive prose drift, not a dispatch-contract bug. The skill's Step 1 dispatches on `data.next_action.kind` from `codex1 --json status` (round-1 skills P1-1 fix); no agent inspects `task next` kinds to drive dispatch. Round-1/2/3 precedent (e.g. round-1 "execute SKILL 'Use after' wording", round-2/3 "non-loop-scope prose drift") rejects this class as P3. The reviewer themselves classified as P3 and recommended REJECT.

## Totals

Counts below are per unique finding after cross-reviewer dedupe.

| Category | FIX | REJECT |
|----------|-----|--------|
| P0       |  0  |   0    |
| P1       |  0  |   0    |
| P2       |  2  |   0    |
| P3       |  0  |   1    |
