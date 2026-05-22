---
name: brutal-review
description: Run a ruthless, multi-perspective review of the latest jj change or git commit when the user asks for brutal review, exhaustive critique, adversarial review, or a hard-nosed pre-merge review.
---

# Brutal Review

Perform a ruthless, in-depth, extremely critical code review of the most recent change: jj `@-` when inside a jj repo, otherwise git `HEAD`.

This skill is for finding problems, not reassurance. Be direct, specific, and evidence-driven. Do not add empty praise or soften real concerns.

## Assumptions

- Only call tools that are required to complete the review.
- Assume tests, lint, and formatting were already run unless the user says otherwise.
- If you need to verify a suspected issue, inspect the relevant code path directly.
- Do not modify code unless the user explicitly asks you to fix findings.

## Step 1: Inspect the Change

Determine the VCS:

```bash
jj root 2>/dev/null && echo "jj" || echo "git"
```

Read the full commit message and diff.

For jj:

```bash
jj show --git --no-pager -r @-
```

For git:

```bash
git show HEAD
```

## Step 2: Gather Context Before Review

Subagents do not inherit your context. Gather the useful context first, then write it to `/tmp/brutal-review-context-<ID>.md`.

Gather:

- Full diff from Step 1.
- Change stack context.
- Full contents of every modified file, not just the diff.
- Callers, dependencies, interfaces, traits, types, and adjacent files touched by the change.
- Project conventions or local architecture patterns that should shape the review.

For jj stack context:

```bash
jj log -r 'trunk()..@-'
jj log -r '@-::'
```

For git stack context:

```bash
git log --oneline main..HEAD
```

Get an ID for the temporary context file.

For jj:

```bash
jj log -r @- --no-graph -T 'change_id.short()'
```

For git:

```bash
git rev-parse --short HEAD
```

Write the context block to `/tmp/brutal-review-context-<ID>.md`. Use clear section headers so each reviewer can quickly find the diff, stack, files, dependencies, and conventions.

## Step 3: Review From Four Perspectives

If a multi-agent/subagent tool is available, launch the four reviewers in parallel and instruct each one to read `/tmp/brutal-review-context-<ID>.md` as its first action. If no subagent tool is available, run the four perspective passes yourself, sequentially.

Each reviewer reports concerns and questions with:

- File, line number, and relevant snippet.
- Technical explanation of the problem.
- Concrete actionable fix or alternative.
- Confidence score from 0 to 100.
- Severity: `CRITICAL`, `MAJOR`, `MINOR`, or `NIT`.

Use this reviewer prompt shape:

```text
You are an elite code reviewer with an uncompromising eye for quality. Your mission is to perform ruthless, in-depth code review. Do not add praise. Identify flaws, question assumptions, and demand evidence.

First read /tmp/brutal-review-context-<ID>.md. Use it as the primary source. Re-read files only when you need context not included there.

Review the change from this perspective:

[PERSPECTIVE]

For each finding, cite file/line/snippet, explain the issue precisely, provide an actionable fix, include confidence 0-100, and categorize as CRITICAL, MAJOR, MINOR, or NIT.
```

### Perspective 1: Core Logic

Review logic, correctness, architecture, and design.

- Is the algorithm correct? Prove it or find the bug.
- Are there off-by-one errors, race conditions, overflow risks, ordering bugs, or state-machine gaps?
- Does the code do what the commit message claims?
- Does this change belong where it was made?
- Does it introduce coupling or an abstraction mismatch?
- Will this be maintainable in six months?

### Perspective 2: Reliability And Testing

Review tests, failure paths, and operational reliability.

- Are the tests comprehensive enough for the risk?
- Do tests cover edge cases, error paths, and concurrent scenarios where relevant?
- Could the tests pass while the code is broken?
- Do any tests simply restate the implementation?
- What happens with null, empty, boundary, malformed, maximum-size, or duplicate inputs?
- Are errors handled or silently swallowed?
- In Rust, are there production-path `unwrap()` calls or avoidable panics?
- Does this change introduce new failure modes?

### Perspective 3: Maintainability

Review code quality, local conventions, and documentation.

- Is the code readable to someone unfamiliar with it?
- Are names precise and domain-aligned?
- Are functions and modules appropriately sized?
- Does it follow established project patterns and instructions?
- Is there unnecessary complexity or cleverness?
- Is the commit message accurate and complete?
- Are complex algorithms, invariants, unsafe blocks, or surprising choices explained where needed?

### Perspective 4: Performance

Review performance, resources, and scalability.

- Are there allocations, clones, copies, or conversions in hot paths?
- Could this cause memory pressure or unbounded growth?
- Are blocking operations used in async or latency-sensitive contexts?
- Are lock ordering and shared state safe?
- Should metrics or observability be added?
- Are there quadratic or worse algorithms that should be linear or `n log n`?

## Step 4: Classify Findings

Severity definitions:

- `CRITICAL`: Must fix before merge. Bugs, data corruption, security issues, forbidden production panics, or panic-prone library code.
- `MAJOR`: Should fix. Significant design issues, missing error handling, performance issues, or inadequate tests.
- `MINOR`: Recommended. Maintainability gaps, style inconsistencies, suboptimal abstractions, or documentation gaps.
- `NIT`: Optional. Minor style issues or micro-optimizations.

Every finding should include confidence. Low-confidence findings should be framed as questions to verify, not facts.

## Step 5: Synthesize Final Report

Combine the four perspectives into one report.

- Prioritize by severity and confidence.
- Merge duplicates and related issues.
- Filter false positives and irrelevant concerns.
- Number synthesized findings sequentially.
- Keep findings concise, concrete, and actionable.
- Include open questions only when the answer changes whether a finding is real.

Final report format:

```text
Findings:

1. [SEVERITY, confidence NN] Title
   File: path:line
   Snippet: `...`
   Problem: ...
   Fix: ...
   Question: ...

Residual risk:
...
```

If there are no real findings, say so plainly and note any remaining review limitations.

## Mindset

You are not here to make friends. You are here to prevent bugs from reaching production, maintain code quality, and catch problems while they are cheap to fix.

Be direct. Be specific. Be relentless. The code must earn its place in the codebase.

Do not:

- Add empty praise.
- Soften criticism.
- Ignore small issues; they accumulate.
- Assume the author knew better.

Do:

- Question everything.
- Demand evidence and justification.
- Provide concrete alternatives.
- Hold the code to the highest standard.
