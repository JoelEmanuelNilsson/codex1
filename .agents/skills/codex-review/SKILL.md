---
name: codex-review
description: Run Codex review as an advisory closeout check for local diffs, branches, or commits; verify findings before acting.
---

# Codex Review

Run Codex's built-in code review as a closeout check. This is advisory code review, not mission completion, proof sufficiency, PR approval, or native `/goal` state.

Use when:

- the user asks for Codex review, autoreview, second-model review, or a review pass
- after non-trivial code edits, before final report, commit, push, or PR update
- reviewing dirty local work, a branch diff, or a specific commit after fixes

## Codex1 Local Use

When this skill is used inside a Codex1 mission, treat review output as evidence only. Record useful review findings in `REVIEWS/` when the mission asks for review evidence. Main Codex triages accepted, rejected, stale, duplicate, or deferred findings in `TRIAGE/` when triage is useful. Do not treat a clean Codex review as PRD satisfaction, proof sufficiency, close safety, setup status, or native `/goal` completion.

`$codex-review` belongs naturally inside `proof-qa` work. Do not add a separate execution lane just to run review.

## Contract

- Treat review output as advisory. Never blindly apply it.
- Verify every finding by reading the real code path and adjacent files.
- Read dependency docs, source, or types when a finding depends on external behavior.
- Reject unrealistic edge cases, speculative risks, broad rewrites, and fixes that over-complicate the codebase.
- Prefer small fixes at the right ownership boundary; recommend a refactor only when it clearly improves the bug class or reviewability.
- If a review-triggered fix changes code, rerun focused tests and rerun Codex review.
- If rejecting a finding as intentional, add an inline code comment only when it explains a real invariant or ownership decision future reviewers need.
- Do not push just to review. Push only when the user requested push, ship, or PR update.
- Never switch or override the review model. If review hits model capacity, retry the same command a few times with the same model.

## Review Loop

1. Run the selected review target.
2. Read every finding against the real code path.
3. Classify each finding as accepted, rejected, duplicate, stale, or deferred.
4. Fix accepted findings only.
5. Rerun focused tests affected by those fixes.
6. Rerun Codex review after review-triggered edits.
7. Stop when the helper or review command exits 0 with no accepted/actionable findings.

Treat helper exit 0 plus absence of actionable findings as the clean review result. Do not run another full review just to get nicer wording. If a remaining finding is rejected or deferred, record why instead of looping forever.

## Pick Target

Dirty local work:

```bash
codex review --uncommitted
```

Use this only when the patch is actually unstaged, staged, or untracked in the current checkout. A clean `--uncommitted` review only proves there is no local patch.

Branch or PR work:

```bash
git fetch origin
codex review --base origin/main
```

If an open PR exists, prefer its actual base:

```bash
base=$(gh pr view --json baseRefName --jq .baseRefName)
codex review --base "origin/$base"
```

Committed single change:

```bash
codex review --commit HEAD
```

Use commit review for already-landed or already-pushed work on `main`. For a small stack, review each commit explicitly or review the branch before merging with `--base`.

## Helper

Bundled helper:

```bash
bash .agents/skills/codex-review/scripts/codex-review --help
```

The helper:

- chooses dirty local work first in `--mode auto`
- otherwise uses the current PR base if `gh pr view` works
- otherwise uses `origin/main` for non-main branches
- supports `--mode local`, `--mode branch`, and `--mode commit`
- supports `--parallel-tests "<command>"` when a known test command should run beside review
- supports `--output`, `--dry-run`, `--full-access`, and `--no-yolo`
- runs nested review with full access by default; use `--no-yolo` only when intentionally testing sandbox behavior
- writes only to stdout unless `--output` is set

Do not force local mode after committing. For committed, pushed, or PR work, point Codex at the commit or branch diff.

## Parallel Closeout

Format first if formatting can change line locations. Then it is OK to run tests and review in parallel when the test command is already known:

```bash
bash .agents/skills/codex-review/scripts/codex-review --parallel-tests "cargo test"
```

Tests may force code changes that stale the review. If tests or review lead to code edits, rerun the affected tests and rerun review until no accepted/actionable findings remain.

## Context Efficiency

Codex review can be noisy. For large changes, prefer a subagent filter when subagents are available. Ask it to run the review and return only:

- actionable findings it accepts
- findings it rejects, with one-line reasons
- exact files and tests to rerun

Run inline only for tiny changes or when subagents are unavailable.

## Final Report

Include:

- review command used
- tests/proof run
- findings accepted or rejected, briefly why
- the clean review result from the final helper/review run, or why a remaining finding was consciously rejected or deferred

Do not run another Codex review solely to improve final report wording.
