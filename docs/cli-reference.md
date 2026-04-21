# CLI Reference

One section per `codex1` subcommand. The authoritative JSON envelope shapes and error codes live in [`cli-contract-schemas.md`](cli-contract-schemas.md); this document lifts the per-command surface directly from the clap definitions in `crates/codex1/src/cli/` so the `--help` text and the flags listed here match the installed binary.

Unless noted, every command:

- Prints a JSON envelope on stdout.
- Accepts the global flags `--mission <id>`, `--repo-root <path>`, `--json`, `--dry-run`, `--expect-revision <N>`.
- Implements the command behavior described here. `codex1 <cmd> --help` works for every command below.

Phase B ownership is tracked in the handoff package: see [`codex1-rebuild-handoff/02-cli-contract.md`](codex1-rebuild-handoff/02-cli-contract.md) for the minimal command surface and [`codex1-rebuild-handoff/03-planning-artifacts.md`](codex1-rebuild-handoff/03-planning-artifacts.md) for the file layout these commands read and write.

---

## codex1 init

**Purpose:** Create `PLANS/<mission>/` with a blank `OUTCOME.md` (fill markers), a minimal `PLAN.yaml` header, a fresh `STATE.json` at `revision: 0`, `phase: "clarify"`, an empty `EVENTS.jsonl`, and `specs/` + `reviews/` directories.
**Mutates state:** yes (creates a new mission directory).
**Arguments:** none beyond the global flags. Requires `--mission <id>`.
**Success:** `{"ok":true,"mission_id":"<id>","revision":0,"data":{"created":{…},"next_action":{"kind":"clarify","command":"$clarify","hint":"Fill in OUTCOME.md, then run `codex1 outcome ratify`."}}}`
**Errors:** `MISSION_NOT_FOUND` (if `PLANS/<id>/` already exists, or `--repo-root` is not a directory), `PARSE_ERROR` (IO failure).
**`--dry-run`:** supported. Reports what would be created without touching disk.
**Example:**
```bash
codex1 --json init --mission demo
```

---

## codex1 doctor

**Purpose:** Report CLI health. Must never crash on missing auth or config.
**Mutates state:** no.
**Arguments:** none.
**Success:** `{"ok":true,"data":{"version":"…","config":{…},"install":{…},"auth":{"required":false,…},"cwd":"…","warnings":[…]}}`
**Errors:** none under normal use; an IO failure on the probe file would surface as `PARSE_ERROR`.
**Example:**
```bash
codex1 --json doctor
```

---

## codex1 hook snippet

**Purpose:** Print the install one-liner and example `codex/hooks.json` stanza for the Ralph Stop hook.
**Mutates state:** no.
**Arguments:** none.
**Success:** `{"ok":true,"data":{"hook":{"event":"Stop","script_path_hint":"<repo-root>/scripts/ralph-stop-hook.sh",…},"install":{"codex_hooks_json_example":{…}},"note":"…"}}`
**Errors:** none.
**Example:**
```bash
codex1 --json hook snippet
```

The shell script itself lives at `scripts/ralph-stop-hook.sh` and is owned by Phase B Unit 12; see [`scripts/README-hook.md`](../scripts/README-hook.md) for wiring details once that unit lands.

---

## codex1 outcome check

**Purpose:** Validate `OUTCOME.md` mechanical completeness — required fields present, no `[codex1-fill:*]` markers, no empty-required sections, no obvious placeholder text (`TODO`, `TBD`, etc.). Does not judge semantic quality.
**Mutates state:** no.
**Arguments:** none beyond globals. Requires `--mission <id>`.
**Success:** `{"ok":true,"data":{"ratifiable":true,"missing_fields":[],"placeholders":[]}}`
**Errors:** `OUTCOME_INCOMPLETE` (with `context.missing_fields` and `context.placeholders`), `MISSION_NOT_FOUND`, `PARSE_ERROR`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json outcome check --mission demo
```

---

## codex1 outcome ratify

**Purpose:** Flip `STATE.json` `outcome.ratified = true` and advance `phase` from `clarify` to `plan`. Only succeeds if `outcome check` passes.
**Mutates state:** yes.
**Arguments:** none beyond globals. Requires `--mission <id>`.
**Success:** `{"ok":true,"mission_id":"demo","revision":1,"data":{"ratified_at":"2026-04-20T…Z"}}`
**Errors:** `OUTCOME_INCOMPLETE`, `MISSION_NOT_FOUND`, `REVISION_CONFLICT` (if `--expect-revision` mismatches).
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json outcome ratify --mission demo --expect-revision 0
```

---

## codex1 plan choose-level

**Purpose:** Record the requested planning level and emit the `plan scaffold` next-action. Accepts interactive and non-interactive invocations.
**Mutates state:** yes (records `plan.requested_level` / `plan.effective_level`).
**Arguments:**
- `--level <LEVEL>` — accepts `light` / `medium` / `hard` or numeric aliases `1` / `2` / `3`.
- `--escalate <REASON>` — reason the effective level is higher than requested.
**Success:** `{"ok":true,"data":{"requested_level":"medium","effective_level":"hard","escalation_reason":"…","next_action":{"kind":"plan_scaffold","args":["codex1","plan","scaffold","--level","hard"]}}}`
**Errors:** `OUTCOME_NOT_RATIFIED`, `MISSION_NOT_FOUND`, `REVISION_CONFLICT`.
**Phase status:** Implemented. Rejects invocation when OUTCOME.md is not yet ratified.
**Example:**
```bash
codex1 --json plan choose-level --level medium
```

Use the product verbs `light` / `medium` / `hard` in docs and skill prompts. Numeric values are CLI input aliases; `low` and `high` are not product terms.

---

## codex1 plan scaffold

**Purpose:** Write a `PLAN.yaml` skeleton for the chosen level and create the matching `specs/T*/SPEC.md` stubs.
**Mutates state:** yes (writes `PLAN.yaml`, may set `plan.requested_level`).
**Arguments:**
- `--level <LEVEL>` (required) — `light` / `medium` / `hard` or `1` / `2` / `3`.
**Success:** `{"ok":true,"data":{"wrote":"PLANS/demo/PLAN.yaml","specs_created":[]}}`
**Errors:** `OUTCOME_NOT_RATIFIED`, `MISSION_NOT_FOUND`, `PARSE_ERROR`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json plan scaffold --level hard --mission demo
```

---

## codex1 plan check

**Purpose:** Validate `PLAN.yaml` structure and DAG — required sections present, every task has `id`/`kind`/`depends_on`/`spec`, root tasks use `depends_on: []`, all dependencies resolve, no cycles, unique task IDs, review tasks reference valid targets, hard planning evidence present when `effective: hard`. Locks the plan on success.
**Mutates state:** yes (sets `plan.locked = true`, computes `plan.hash`).
**Arguments:** none beyond globals.
**Success:** `{"ok":true,"data":{"tasks":4,"review_tasks":1,"hard_evidence":3}}`
**Errors:** `PLAN_INVALID`, `DAG_CYCLE`, `DAG_MISSING_DEP`, `OUTCOME_NOT_RATIFIED`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json plan check --mission demo
```

---

## codex1 plan graph

**Purpose:** Emit the DAG in a tool-friendly format.
**Mutates state:** no.
**Arguments:**
- `--format <mermaid|dot|json>` (default `mermaid`).
- `--out <FILE>` — optional; writes the rendering to this path in addition to echoing it in the envelope.
**Success:** `{"ok":true,"data":{"mermaid":"flowchart TD …"}}` (field name matches the chosen format).
**Errors:** `PLAN_INVALID`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json plan graph --format mermaid --mission demo
codex1 --json plan graph --format dot --out /tmp/plan.dot --mission demo
```

---

## codex1 plan waves

**Purpose:** Derive waves from `depends_on` + current task state. Waves are never stored — each call recomputes from the DAG and task status.
**Mutates state:** no.
**Arguments:** none beyond globals.
**Success:** `{"ok":true,"data":{"waves":[{"wave_id":"W1","tasks":["T1"],"parallel_safe":true,"blockers":[]}],"current_ready_wave":"W1","all_tasks_complete":false}}`
**Errors:** `PLAN_INVALID`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json plan waves --mission demo
```

---

## codex1 task next

**Purpose:** Report the next ready action — a single task, a parallel-safe wave, a review target, a replan prompt, or a mission-close handoff.
**Mutates state:** no.
**Arguments:** none beyond globals.
**Success:** `{"ok":true,"data":{"next":{"kind":"run_wave","wave_id":"W1","tasks":["T1"],"parallel_safe":true}}}` (alternate shapes for review / close / replan kinds).
**Errors:** `PLAN_INVALID`, `REPLAN_REQUIRED`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json task next --mission demo
```

---

## codex1 task start

**Purpose:** Transition a task to `InProgress`.
**Mutates state:** yes (updates `tasks[id].status`).
**Arguments:**
- `<TASK_ID>` (positional, required).
**Success:** `{"ok":true,"mission_id":"demo","revision":N,"data":{"task_id":"T2","status":"InProgress"}}`
**Errors:** `TASK_NOT_READY`, `REVISION_CONFLICT`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json task start T2 --mission demo --expect-revision 5
```

---

## codex1 task finish

**Purpose:** Mark a task complete after proof has been written.
**Mutates state:** yes.
**Arguments:**
- `<TASK_ID>` (positional, required).
- `--proof <PATH>` (required) — path to the proof file (usually `specs/T<id>/PROOF.md`).
**Success:** `{"ok":true,"mission_id":"demo","revision":N,"data":{"task_id":"T2","status":"Complete","proof_path":"specs/T2/PROOF.md"}}`
**Errors:** `PROOF_MISSING`, `TASK_NOT_READY`, `REVISION_CONFLICT`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json task finish T2 --proof specs/T2/PROOF.md --mission demo
```

---

## codex1 task status

**Purpose:** Show the record for a single task.
**Mutates state:** no.
**Arguments:**
- `<TASK_ID>` (positional, required).
**Success:** `{"ok":true,"data":{"task_id":"T2","status":"Complete","depends_on":["T1"],"proof_path":"…"}}`
**Errors:** `TASK_NOT_READY` (unknown task id), `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json task status T2 --mission demo
```

---

## codex1 task packet

**Purpose:** Emit a worker packet — the exact text a main-thread can paste into a worker subagent prompt. Contains task title, spec excerpt, allowed write paths, proof commands, and a mission summary.
**Mutates state:** no.
**Arguments:**
- `<TASK_ID>` (positional, required).
**Success:** `{"ok":true,"data":{"task_id":"T3","title":"…","spec_excerpt":"…","write_paths":["src/cli/outcome/**"],"proof_commands":["cargo test outcome"],"mission_summary":"…"}}`
**Errors:** `TASK_NOT_READY`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json task packet T3 --mission demo
```

---

## codex1 review start

**Purpose:** Begin a planned review task. Transitions the review record to `Open`.
**Mutates state:** yes.
**Arguments:**
- `<TASK_ID>` (positional, required) — the review task id.
**Success:** `{"ok":true,"mission_id":"demo","revision":N,"data":{"review_task_id":"T4","state":"Open"}}`
**Errors:** `TASK_NOT_READY`, `REVISION_CONFLICT`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json review start T4 --mission demo
```

---

## codex1 review packet

**Purpose:** Emit a reviewer packet — target files, diffs, proofs, review profile, and mission summary. Paste into reviewer subagent prompts.
**Mutates state:** no.
**Arguments:**
- `<TASK_ID>` (positional, required).
**Success:** `{"ok":true,"data":{"task_id":"T4","review_profile":"code_bug_correctness","targets":["T2"],"diffs":[{"path":"…","lines":[…]}],"proofs":["specs/T2/PROOF.md"],"mission_summary":"…"}}`
**Errors:** `TASK_NOT_READY`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json review packet T4 --mission demo
```

---

## codex1 review record

**Purpose:** Record the review outcome. Only the main thread should invoke this; reviewer subagents return findings text only. The CLI does not enforce caller identity — workflow prompts govern who calls it.
**Mutates state:** yes (updates `reviews[id]`, may bump `replan.consecutive_dirty_by_target`).
**Arguments:**
- `<TASK_ID>` (positional, required).
- `--clean` — marks the review clean. Mutually exclusive with `--findings-file`.
- `--findings-file <PATH>` — markdown file containing P0/P1/P2 findings.
- `--reviewers <LIST>` — comma-separated reviewer actor ids (e.g. `code-reviewer,intent-reviewer`).
**Success:** `{"ok":true,"mission_id":"demo","revision":N,"data":{"review_task_id":"T4","verdict":"clean","category":"accepted_current","reviewers":["code-reviewer","intent-reviewer"]}}`
**Errors:** `STALE_REVIEW_RECORD`, `REVIEW_FINDINGS_BLOCK` (if the call would push the consecutive-dirty counter over the threshold with no active repair path), `REPLAN_REQUIRED`, `REVISION_CONFLICT`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Examples:**
```bash
codex1 --json review record T4 --clean --reviewers code-reviewer,intent-reviewer --mission demo
codex1 --json review record T4 --findings-file /tmp/T4-findings.md --mission demo
```

---

## codex1 review status

**Purpose:** Show the review record for a task.
**Mutates state:** no.
**Arguments:**
- `<TASK_ID>` (positional, required).
**Success:** `{"ok":true,"data":{"review_task_id":"T4","state":"Passed","last_verdict":"clean","consecutive_dirty":0}}`
**Errors:** `TASK_NOT_READY`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json review status T4 --mission demo
```

---

## codex1 replan check

**Purpose:** Report whether replan is required. The canonical trigger is six consecutive dirty reviews against the same active target.
**Mutates state:** no.
**Arguments:** none beyond globals.
**Success:** `{"ok":true,"data":{"required":false,"reason":null,"consecutive_dirty_by_target":{"T4":2}}}`
**Errors:** `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json replan check --mission demo
```

---

## codex1 replan record

**Purpose:** Record a replan decision and reset the consecutive-dirty counter. New task rows are added by editing `PLAN.yaml` and running `plan check` — this command does not generate tasks.
**Mutates state:** yes (updates `replan`, resets `consecutive_dirty_by_target`).
**Arguments:**
- `--reason <CODE>` (required) — e.g. `six_dirty`, `architecture_shift`.
- `--supersedes <TASK_ID>` — optional; the task id whose boundary is superseded.
**Success:** `{"ok":true,"mission_id":"demo","revision":N,"data":{"reason":"six_dirty","supersedes":"T4","phase_after":"plan","plan_locked":false}}`
**Errors:** `REVISION_CONFLICT`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json replan record --reason six_dirty --supersedes T4 --mission demo
```

---

## codex1 loop activate

**Purpose:** Activate the loop in the requested mode.
**Mutates state:** yes (sets `loop.active = true`, `loop.paused = false`, and `loop.mode`).
**Arguments:**
- `--mode <MODE>` — one of `clarify`, `plan`, `execute`, `review_loop`, `mission_close`; defaults to `execute`.
**Success:** `{"ok":true,"mission_id":"demo","revision":N,"data":{"active":true,"paused":false,"mode":"execute"}}`
**Errors:** `PLAN_INVALID` (unknown mode), `REVISION_CONFLICT`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json loop activate --mode execute --mission demo
```

---

## codex1 loop pause

**Purpose:** Pause the active loop. Used by `$close` to let the user talk without Ralph forcing continuation.
**Mutates state:** yes (sets `loop.paused = true`).
**Arguments:** none beyond globals.
**Success:** `{"ok":true,"mission_id":"demo","revision":N,"data":{"active":true,"paused":true,"mode":"execute"}}`
**Errors:** `REVISION_CONFLICT`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json loop pause --mission demo
```

---

## codex1 loop resume

**Purpose:** Resume a paused loop. Clears `loop.paused`.
**Mutates state:** yes.
**Arguments:** none beyond globals.
**Success:** `{"ok":true,"mission_id":"demo","revision":N,"data":{"active":true,"paused":false,"mode":"execute"}}`
**Errors:** `REVISION_CONFLICT`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json loop resume --mission demo
```

---

## codex1 loop deactivate

**Purpose:** Deactivate the loop entirely. Used when abandoning a run or after `close complete`.
**Mutates state:** yes (sets `loop.active = false`).
**Arguments:** none beyond globals.
**Success:** `{"ok":true,"mission_id":"demo","revision":N,"data":{"active":false,"paused":false,"mode":"none"}}`
**Errors:** `REVISION_CONFLICT`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json loop deactivate --mission demo
```

---

## codex1 close check

**Purpose:** Check terminal readiness. Shares readiness logic with `codex1 status` so the two cannot disagree. Required: outcome ratified, plan locked, all non-superseded tasks complete or review-clean, planned review tasks clean, mission-close review clean, no active blockers.
**Mutates state:** no.
**Arguments:** none beyond globals.
**Success:** `{"ok":true,"data":{"ready":false,"verdict":"continue_required","blockers":[{"code":"TASK_NOT_READY","detail":"T7 is pending"}]}}`
**Errors:** `MISSION_NOT_FOUND`. Blockers are reported inside `data.blockers`, not as top-level errors.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json close check --mission demo
```

---

## codex1 close record-review

**Purpose:** Record the mission-close review verdict. Use after `close check` reports `ready_for_mission_close_review`.
**Mutates state:** yes.
**Arguments:**
- exactly one of `--clean` or `--findings-file <PATH>`.
- `--reviewers <CSV>` optional reviewer names.
**Success:** `{"ok":true,"mission_id":"demo","revision":N,"data":{"target":"__mission_close__","verdict":"clean","review_state":"passed"}}`
**Errors:** `CLOSE_NOT_READY`, `REVIEW_FINDINGS_BLOCK`, `REVISION_CONFLICT`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json close record-review --clean --mission demo
```

---

## codex1 close complete

**Purpose:** Write `CLOSEOUT.md` and mark the mission terminal. Only succeeds if `close check` would pass.
**Mutates state:** yes (sets `close.terminal_at`, advances `phase` to `terminal`).
**Arguments:** none beyond globals.
**Success:** `{"ok":true,"mission_id":"demo","revision":N,"data":{"closeout_path":"PLANS/demo/CLOSEOUT.md","terminal_at":"2026-04-20T…Z","mission_id":"demo"}}`
**Errors:** `CLOSE_NOT_READY`, `TERMINAL_ALREADY_COMPLETE` (idempotent re-invocation), `REVISION_CONFLICT`, `MISSION_NOT_FOUND`.
**Phase status:** Implemented.
**Example:**
```bash
codex1 --json close complete --mission demo
```

---

## codex1 status

**Purpose:** Unified mission status. Consumed by skills, the main thread, Ralph, and humans debugging mission state. Shares `verdict` / `close_ready` derivation with `codex1 close check` via `state::readiness`.
**Mutates state:** no.
**Arguments:** none beyond globals. Omitting `--mission` causes the command to walk up from CWD; if no single mission resolves, it emits a graceful `stop.allow: true` envelope so Ralph never blocks the shell on a stray CWD.
**Success (Phase B target shape):**
```json
{
  "ok": true,
  "mission_id": "demo",
  "revision": 7,
  "data": {
    "phase": "execute",
    "verdict": "continue_required",
    "loop": { "active": true, "paused": false, "mode": "execute" },
    "next_action": { "kind": "run_wave", "wave_id": "W2", "tasks": ["T2","T3"] },
    "ready_tasks": ["T2","T3"],
    "parallel_safe": true,
    "parallel_blockers": [],
    "review_required": [],
    "replan_required": false,
    "close_ready": false,
    "stop": {
      "allow": false,
      "reason": "active_loop",
      "message": "Run wave W2 or use $close to pause."
    }
  }
}
```
**Errors:** `MISSION_NOT_FOUND` (only when `--mission` is given explicitly and cannot be resolved), `STATE_CORRUPT`, `PARSE_ERROR`.
**Phase status:** Implemented. The projection emits `phase`, `verdict`, `loop`, `next_action`, `ready_tasks`, `parallel_safe`, `parallel_blockers`, `review_required`, `replan_required`, `close_ready`, and `stop`. The `stop.allow` and `verdict` fields come from the shared readiness helper, so the Ralph contract is honored.
**Example:**
```bash
codex1 --json status --mission demo
```

---

## Global flags

| Flag | Scope | Purpose |
| --- | --- | --- |
| `--mission <ID>` | all | Directory name under `PLANS/`. Optional for `doctor` and `hook snippet`. |
| `--repo-root <PATH>` | all | Overrides CWD discovery. Resolves to `<PATH>/PLANS/<id>/`. |
| `--json` | all | Reserved; JSON is the default on every command. Present for cli-creator parity. |
| `--dry-run` | mutating | Validate and report without writing. |
| `--expect-revision <N>` | mutating | Strict equality against `STATE.json` revision. Returns `REVISION_CONFLICT` on mismatch. |
| `--help` | all | Clap-generated help. Always works, even for unimplemented commands. |

Mission resolution precedence and the full STATE.json / envelope / error-code schemas are spelled out in [`cli-contract-schemas.md`](cli-contract-schemas.md).
