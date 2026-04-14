# Codex1 Harness V1

## Master PRD and System Specification

## Context: What We Are Building

This product exists because normal Codex usage is still too easy to fool on large, ambiguous, high-risk engineering work.

The core failure is not "Codex writes imperfect code." The core failure is that Codex often starts with too little truth, plans too shallowly, collapses too early, delegates too vaguely, and then compensates for weak planning during execution.

That is exactly the wrong operating model for the kind of work this harness is meant to handle:

- giant refactors
- platform migrations
- auth, billing, and data-shape changes
- multi-surface product changes
- long-running efforts that must survive compaction, resume, restart, and multi-day continuation

We are not building a prompt pack.
We are not building a wrapper product that demotes Codex.
We are not building OMX again in a cleaner codebase.

We are building a **Codex-native harness for NASA's internal development team**.

In the simplest possible product terms, the only goal that matters is this:

- the user gives Codex1 the mission
- Codex1 clarifies until the mission is crystal clear
- Codex1 plans until the execution route is genuinely strong
- Codex1 then keeps going under Ralph discipline until the work is actually done or honestly waiting on the user
- this must happen inside native Codex CLI / Codex Desktop behavior, not through a wrapper runtime, sidecar daemon, or babysitter loop

Everything else in this PRD exists only to make that loop trustworthy on very large, risky engineering work.

That framing matters because it forces the right standard:

- ambiguity is dangerous
- hidden coupling is dangerous
- fake alternatives are dangerous
- vague specs are dangerous
- unsafe sequencing is dangerous
- unsafe same-workspace parallelism is dangerous
- review is mandatory
- "probably done" is not a valid state

The user experience we want is still simple:

- use Codex normally
- invoke a very small number of public skills
- let Codex become much more rigorous and much harder to fool

The public workflows are the user surface, not the entire internal machinery.

That distinction is important.

If the user invokes `$plan 5`, that should start the strongest planning workflow the system can run. The main logical orchestrating Codex thread should then decide how to carry out that planning program:

- what internal methods to run
- what internal skills to use
- what bounded subagents to spawn
- what evidence to gather
- what weak areas to deepen
- what contradictions to resolve

The user invokes the public skill.
The main logical orchestrating thread owns the mission.
It does not have to personally perform every planning or execution subtask.

Likewise, `replan` should not be a normal user-facing button. It is primarily an internal workflow used when execution, review, validators, or specialist subagents discover contradictions and need to bring structured replan signals back to the main orchestrating thread. The main thread remains the authority that decides what layer to reopen.

There should also be one high-trust public workflow above the manual sequence:

- `$autopilot`

`$autopilot` exists so the user can start the mission once and let the full Ralph-driven sequence happen:

1. clarify until the mission is locked enough
2. plan deeply
3. execute from the planning package
4. review and QA
5. continue until the default mission done bar is reached

The normal default done bar should be:

- production-grade code
- review-clean
- QAed enough for the mission class
- PR-ready

And when the repo/tooling context allows it, the harness should prefer to actually create the PR rather than merely describe one.

Another important truth is that the plan is not "a file."
The mission is a **folder-backed package**.

If the user wants to refactor a huge product or application, that entire effort is one mission root. Inside that mission root there may be:

- one canonical outcome lock
- one blueprint index
- optional blueprint subfolders
- many workstream spec folders
- receipts, ledgers, and supporting files
- subfolders for umbrella initiatives or child missions when the effort is truly massive

So the right question is not:

- should the whole plan be one markdown file?

The right question is:

- what mission-folder shape makes the work most understandable, durable, and executable?

This PRD therefore treats:

- `Outcome Lock` as a sacred mission artifact
- `Program Blueprint` as a logical planning package, not necessarily a single physical file
- `Workstream Spec` as a logical execution contract, often best represented as a small folder with a canonical `SPEC.md`

One more important behavior choice:

`$clarify` should usually ask **one main question at a time**. If multiple questions are tightly coupled and separating them would worsen clarity, it may ask more than one, but one main question is the default interaction pattern.

This document exists so that someone who reads only this context and the rest of the spec should understand:

- why we are doing this
- what we are trying to achieve
- what shape the system should have
- what mistakes it must be designed to prevent
- how the main thread, internal skills, subagents, artifacts, execution graph, and Ralph all fit together

This product is a harness, not a wrapper shell and not a runtime-first control plane.

The intended product is:

- normal interactive Codex CLI
- a skills-first workflow surface
- visible mission artifacts under `PLANS/`
- hidden machine state under `.ralph/`

The product is not:

- a helper CLI pretending to be the product
- a wrapper shell around Codex turns
- a TypeScript workflow engine with thin skill wrappers
- a repo-local planner hard-coded to one repository shape
- a large external orchestration runtime

This PRD is the canonical source of truth for a clean-slate build.

It must be strong enough that a fresh Codex session can rebuild the harness from scratch without inventing major behavior.

It must also be honest about what is not yet decided.

In this revision, the core Ralph continuity contract is frozen for supported macOS Codex CLI environments:

- Ralph continuity in V1 must work in Codex CLI alone
- non-terminal missions must not cleanly self-stop while actionable work remains
- `needs_user` is a non-terminal waiting verdict, not a terminal mission stop
- durable mission state plus Codex CLI resume are the recovery surface after interruption
- wrapper runtimes, tmux prompt injection, wording heuristics, and sidecar daemons are disallowed as correctness mechanisms

The remaining open details are narrower operator and environment questions, not permission to invent a different loop.

In particular, the exact operator-facing mechanics for:

- trusted-repo hook/profile installation and enablement
- mission-selection override on top of native Codex session/thread resume
- exact `multi_agent_v2` state-transition discipline between the main orchestrating thread and bounded native subagents across resumed sessions

are now frozen at the product-contract level in this revision, even though later implementation polish may refine operator wording and setup ergonomics.

Those areas remain product-critical, but they no longer block V1-core implementation after the current Codex research pass.

This PRD therefore does two things at once:

- fully specifies the settled product architecture, artifact model, `$clarify`, `$plan`, and the Ralph continuity contract
- explicitly marks remaining operator/env mechanics as open design items, not implementation permission slips

Only items explicitly listed in later "open design items" sections remain unresolved; all other Ralph continuity semantics in this PRD are normative.

## 1. Title and Positioning

### What this is

Codex1 Harness V1 is a **Codex-native, skills-first mission harness** for serious software engineering inside a repository.

Its job is to take a user from:

- a vague or partially formed desired outcome

to:

- a locked mission contract
- a deeply researched planning package
- bounded execution and review contracts
- durable mission state that can survive interruption and resume

The primary product win is making Codex think and operate better about:

- truth
- planning
- delegation
- proof
- review
- completion

not surrounding Codex with a thicker orchestration product.

### What this is not

Codex1 V1 must not be rebuilt as:

- a hidden workflow-code product with thin public skills
- a helper-CLI-first UX
- a wrapper-shell product
- a TypeScript workflow engine as the canonical product core
- a repo-local planner grounded in a hard-coded evidence catalog
- a runtime-first system where machinery matters more than Codex workflow quality

Codex1 must feel like:

- using Codex normally
- with much stronger clarification
- much stronger planning
- much safer execution
- much better artifact discipline
- much better review discipline
- much harder false completion
- much stronger mission continuity
- able to continue until actually done under Ralph discipline

## 2. Vision and Standard

### Customer standard

The quality bar for this product is:

**we are building a Codex harness for NASA's internal development team**

That is not aesthetic framing. It is an operating standard:

- ambiguity is dangerous
- shallow planning is failure
- fake alternatives are failure
- vague specs are failure
- unsafe sequencing is failure
- unsafe same-workspace parallelism is failure
- review is mandatory
- completion cannot be self-declared

### Product promise

The product promise is:

1. the user does not need a perfect first prompt
2. Codex deeply clarifies until the mission is safe to plan
3. Codex plans so well that execution becomes comparatively straightforward
4. execution must proceed from bounded contracts rather than invented architecture
5. replanning must not lose the locked mission
6. Codex returns to the user only when the mission contract or autonomy boundary truly requires it
7. once clarify is complete, Codex1 should be able to keep advancing the same mission under Ralph discipline until the work is actually done, honestly waiting on the user, or honestly blocked

Plain-language interpretation:

- the user should be able to start with `$clarify` or `$autopilot`, answer the necessary clarification questions, and then let Codex1 carry the mission forward
- the harness should do the hard work of planning, packetizing, executing, reviewing, resuming, and orchestrating bounded subagents
- the user should not have to manually babysit every continuation step on a huge refactor or feature once the mission is clarified enough to proceed
- the product succeeds only if this continue-till-done loop works natively in Codex rather than in a separate orchestration product

### Anti-goals

Codex1 V1 must not:

- make the helper CLI the real product
- hide core workflow semantics in private code while leaving skills thin
- depend on a hidden repo-local planning or orchestration scaffold masquerading as the product; visible mission artifacts under `PLANS/` are required, but hidden runtime control planes are not the product
- use prompt cleverness as a substitute for artifacts and gates
- optimize for throughput over mission quality
- treat unresolved design areas as free license to improvise

## 3. User Workflow Surface

### Public workflow surface

The public workflow surface for V1 is:

- `$clarify`
- `$autopilot`
- `$plan`
- `$execute`
- `$review`

These names are simple, obvious, and durable. V1 should not rename them.

### Public workflow rule

- `$clarify` is the only mandatory primitive the user must understand
- `$autopilot` is the preferred one-command workflow in the final product
- `$plan`, `$execute`, and `$review` remain public operator workflows
- `replan` remains internal

Public-surface rule:

- the user invokes the public skills
- the main logical orchestrating Codex thread may then use internal methods, internal skills, validators, and bounded subagents to carry out the work
- public skill invocation does not imply that the user-facing thread personally performs every subtask

### Skills are the implementation

The public skills are not decorative wrappers.

Rules:

- public skills are the primary user-facing product surface
- internal skills are real implementation methods
- the main Codex thread owns mission authority and synthesis
- bounded native subagent use is expected inside the skills-first workflow
- the PRD must not assume hidden workflow entrypoints as the canonical product path
- deterministic validators, compilers, inspectors, and repair helpers may exist underneath the skills, but they are subordinate tooling rather than the public product surface

### Workflow contract

| Workflow | Purpose | Typical input | Primary output |
| --- | --- | --- | --- |
| `$clarify` | turn a vague ask into a locked mission and create the mission package | user ask plus repo context | `MISSION-STATE.md`, then `OUTCOME-LOCK.md` when safe |
| `$autopilot` | run the end-to-end mission flow under Ralph discipline | user ask or existing mission | continuation until an honest terminal verdict (`complete` or `hard_blocked`) or a durable waiting non-terminal verdict (`needs_user`) |
| `$plan` | run the planning loop for a locked mission until the planning completion gate is reached, a durable waiting non-terminal verdict occurs, or an honest terminal verdict is reached | mission id or current locked mission | `PROGRAM-BLUEPRINT.md`, frontier `SPEC.md` files, and a passed execution package for the next target |
| `$execute` | run the execution loop for an already-packaged mission/spec/wave target until the next honest gate, durable waiting non-terminal verdict, or terminal verdict | mission/spec/wave target | receipts, state updates, review-ready output |
| `$review` | perform risk-first review against a bounded target and its proof | mission/spec/wave/diff target | findings plus continuation verdict |

### Happy path

1. The user has something in mind.
2. The user invokes `$clarify` or `$autopilot`.
3. `$clarify` creates the mission package, gathers truth, asks high-leverage questions, and writes the first sacred mission artifact.
4. Once the mission is locked enough, `$plan` produces the planning package and packages the next execution-safe target.
5. `$execute` advances the packaged target under Ralph discipline.
6. `$review` remains available directly and also participates inside the execution loop.
7. If reality breaks assumptions, structured replan signals are raised.
8. The mission continues until the default done bar is reached, an honest terminal verdict is reached, or the mission enters a durable non-terminal waiting state that is explicitly waiting on the user.

### `$autopilot` rule

`$autopilot` is a composition surface over the same contracts as the manual path.

Rules:

- `$autopilot` is a thin composition over the same clarify, plan, execute, and review contracts
- `$autopilot` must not become a second place where core semantics secretly live
- manual-path parity is mandatory
- `$autopilot` means advance the mission until the next honest terminal outcome or durable waiting yield
- only `complete` and `hard_blocked` are terminal mission outcomes
- `needs_user` is a non-terminal waiting verdict; it keeps the mission open and resumable rather than stopping it
- `continue_required`, `review_required`, `repair_required`, and `replan_required` are actionable non-terminal branch verdicts, not hidden implementation details
- CLI-native continuity is mandatory: while non-terminal actionable work remains, V1 uses native Codex Stop-hook continuation plus durable state, and after interruption V1 resumes from durable state plus Codex CLI resume
- `$autopilot` must not rely on tmux injection, wording heuristics, or a sidecar runtime as the real owner of continuation semantics
- the same mission and repo state must yield the same durable artifact classes, gate outcomes, and verdict family whether driven manually or through autopilot, even if intermediate turn count, artifact ordering, or resume boundaries differ

Plain-language interpretation:

- `$autopilot` is not "keep poking Codex until it feels done"
- `$autopilot` is "advance the same Ralph-governed mission cycle repeatedly until the mission is truly terminal or is durably waiting on the user"
- if the next branch is known and Codex can keep acting without new user input, `$autopilot` must keep going
- if only the user can answer the next question after bounded autonomous attempts have failed, `$autopilot` may yield the turn but must leave the mission open and resumable
- a builder must be able to remove the word `$autopilot` from the interaction, drive the same mission manually through `$clarify` / `$plan` / `$execute` / `$review`, and observe the same mission truth, verdict family, and closeout behavior

Remaining open detail:

- the exact operator-facing resume UX for `$autopilot` in multi-mission environments is intentionally left to implementation research, but the continuity contract above is fixed

## 4. Core Concepts

### Mission

A **mission** is the durable unit of clarified intent, planning, execution, review, and resumption.

One mission is one desired outcome with one stable identity and one evolving route.

### Outcome Lock

The **Outcome Lock** is the sacred destination contract.

It locks:

- what must be made true
- what must not break
- what tradeoffs are unacceptable
- what is out of scope
- what Codex may decide later without asking

It does not lock the first route.

### Mission State

**Mission State** is the non-sacred working artifact for clarification.

It records:

- current objective interpretation
- ambiguity state
- repo facts
- open assumptions
- next highest-value question
- feasibility notes when used

It is scratch space with discipline, not sacred truth.

### Program Blueprint

The **Program Blueprint** is the master planning package for the current mission revision.

It defines:

- truth
- system understanding
- touched surfaces
- invariants
- decision obligations
- selected route
- execution graph and waves when needed
- proof and review design
- migration and rollout posture
- frontier packetization

### Truth Register

The **Truth Register** is the planning-time record of what is:

- verified
- inferred
- assumed
- contradicted
- still unknown

Authority rule:

- the Truth Register classifies evidence, freshness, confidence, and contradiction state; it does not override the sacred destination contract or the selected route contract by itself
- `OUTCOME-LOCK.md` is authoritative for destination truth: objective, done-when, protected surfaces, unacceptable tradeoffs, non-goals, and autonomy boundary
- `PROGRAM-BLUEPRINT.md` is authoritative for route truth: selected architecture, proof design, review contract, packetization posture, graph shape, and replan policy
- `MISSION-STATE.md` is authoritative only for the live clarify worksheet while clarify is still in progress
- if the Truth Register exposes a contradiction against the current lock or blueprint, the answer is reopen or supersede at the correct layer, not silent precedence inversion

### Workstream Spec

A **Workstream Spec** is the bounded execution contract for one execution slice. It is the atomic write-capable planning unit that later feeds execution.

It identifies:

- purpose
- scope
- dependencies
- touched surfaces
- proof expectations
- review expectations
- replan boundary

It is a logical contract and may be physically represented as a single file for a tiny slice or, more often, as a workstream folder with a canonical `SPEC.md`.

A Workstream Spec is necessary for execution, but not sufficient by itself to start execution safely. Execution begins only from a current **Execution Package** that binds the selected target to current dependency, scope, proof, review, and wave truth.

### Execution Package

An **Execution Package** is the first-class hidden machine contract that proves a selected mission/spec/wave target is safe to execute now.

It binds:

- the selected target
- current lock and blueprint revisions
- included spec revisions
- dependency satisfaction at current revisions
- explicit read/write scope
- proof obligations
- review obligations
- replan boundary
- wave context when applicable

It is upstream of both the Writer Packet and the Review Bundle.

### Umbrella Mission

An **Umbrella Mission** is a large mission that contains major sub-areas or child missions. The parent mission remains the canonical top-level mission, while child missions or child work areas live in explicit subfolders and inherit lineage through explicit parent references.

V1 rule:

- umbrella or child missions are a reserved escape hatch for truly massive work
- they are not first-wave default behavior
- a normal mission should stay as flat as possible until scale clearly justifies nesting

Lineage rule:

- child missions do not inherit the parent `mission_id`; they inherit lineage
- every true child mission must get its own unique `mission_id`
- every child mission must also carry:
  - `root_mission_id`
  - `parent_mission_id`
- the root mission must use `root_mission_id = mission_id` and `parent_mission_id = null`
- only true child missions get a new `mission_id`; child work areas inside one mission remain part of the same mission and must not silently mint a second mission identity

Promotion rule:

- keep work as a child work area inside the current mission when it still shares the same Outcome Lock, the same blueprint revision, and the same mission-close completion bar
- mint a true child mission only when the work needs its own lock/blueprint cadence, its own mission-close decision, or an independently resumable long-lived identity that should survive beyond one bounded parent work area
- when uncertain, stay flat and keep the work inside the current mission rather than inventing a child mission too early

### Execution Graph and Waves

The **Execution Graph** is the source of truth for sequencing.

In V1-core it is a DAG of runnable workstream spec nodes and dependency edges. Validation, proof, and review remain attached gate obligations on those runnable nodes rather than separate schedulable graph-node species.

**Waves** are schedule groups formed from validated frontier nodes proven safe to run together in one shared workspace.

The graph is truth. Waves are schedule.

### Review Ledger

The **Review Ledger** records blocking and non-blocking review findings, dispositions, evidence references, and mission-close review status. Review is a gate, not a note.

Ledger rule:

- once any independent review occurs, `REVIEW-LEDGER.md` becomes the required visible human review trail for that mission
- mission-close review status must always be written to `REVIEW-LEDGER.md`, even when the review result is clean

### Writer Packet

A **Writer Packet** is the execution brief handed to a bounded write-capable worker context.

It is not casual prose.

### Review Bundle

A **Review Bundle** is the deterministic package handed to a bounded read-only reviewer context.

It is the review-side analogue of the writer packet.

### Contradiction Record

A **Contradiction Record** is the structured machine record used when reality breaks assumptions or contracts strongly enough to require repair or replanning.

### Ralph

**Ralph** is the native control discipline that prevents false completion.

Ralph is not a second product.
Ralph is not a wrapper runtime.

Ralph exists to enforce:

- honest continuation
- explicit closeout
- phase and verdict discipline
- no prose-only completion

The Ralph product semantics and the supported Codex CLI continuity contract are frozen in this PRD revision.

Narrower operator ergonomics may still require later Codex research, but implementers must not treat Ralph mechanics themselves as still negotiable.

### Naming decisions

V1 naming decisions are:

- Product name: `Codex1 Harness`
- Public workflows: `$clarify`, `$autopilot`, `$plan`, `$execute`, `$review`
- Internal workflow: `replan`
- Clarification working artifact: `Mission State`
- Sacred artifact: `Outcome Lock`
- Master plan artifact: `Program Blueprint`
- Local execution artifact: `Workstream Spec`
- Pre-execution gate artifact: `Execution Package`
- Control closeout payload: `RALPH_CLOSEOUT`

## 5. Clarify / Outcome Lock

### Objective

`$clarify` exists to destroy planning-critical ambiguity until planning can safely begin. It is not casual Q&A. It is a mission-intake workflow that determines whether the destination is precise enough for deep planning.

`$clarify` is also responsible for mission bootstrap:

- create the mission package
- establish mission identity
- write `MISSION-STATE.md`
- ratify `OUTCOME-LOCK.md` only when lock gates pass

### Clarification principles

- ask one main high-leverage question per round by default
- ask 2-3 only when the questions are tightly coupled and separating them would worsen clarity
- rank questions by branch-reduction value
- read the repo whenever that reduces technical ambiguity
- do not drift into full planning
- do not lock early

### Ambiguity model

`$clarify` must track ambiguity across these dimensions:

| Dimension | High-impact if missing? | Meaning |
| --- | --- | --- |
| Objective clarity | yes | what the user wants made true |
| Success proof | yes | how success will be judged |
| Protected surfaces | yes | what must not regress |
| Tradeoff vetoes | yes | what is unacceptable even if convenient |
| Scope boundary | yes | what is out of scope |
| Autonomy boundary | yes | what Codex may decide later without asking |
| Baseline facts | medium/high | current-state facts needed to interpret the mission |
| Rollout / migration constraints | medium/high | constraints on compatibility, cutover, reversibility, or rollout |

Scoring rubric:

- every dimension must be scored `0 | 1 | 2 | 3`
- `0 = resolved`: current artifact truth is specific enough that the dimension does not materially constrain planning
- `1 = low`: a residual ambiguity remains, but it is explicitly recorded and would not materially change architecture, protected surfaces, or proof design
- `2 = medium`: the ambiguity could still change decomposition, sequencing, or proof/review design, but bounded repo reading or one more clarify round is likely sufficient
- `3 = high`: the ambiguity could materially change destination truth, protected surfaces, autonomy boundary, rollout posture, or the route-selection space and therefore blocks honest lock

Locking rule from scores:

- any dimension scored `3` blocks lock
- a dimension scored `2` may remain only if clarify has already exhausted the highest-value bounded repo reading for that uncertainty and the remaining uncertainty is explicitly carried as a recorded low-confidence input rather than hidden drift
- question priority should be chosen by the combination of impact, branch-reduction value, and current ambiguity score rather than by field order alone

### Question selection rule

Clarify must not choose the next question by missing-field order.

Clarify must choose the next question by:

- decision impact
- branch-reduction value
- how much planning risk the answer removes
- whether repo reading can eliminate the ambiguity without asking the user

### Clarify algorithm

1. Parse the user ask into provisional lock fields.
2. Read the repo if that can remove technical ambiguity.
3. Score ambiguity across the defined dimensions.
4. Ask the smallest set of questions that removes the most planning risk.
5. Reflect the current lock state back in `MISSION-STATE.md`.
6. Repeat until lock gates pass or the correct verdict is `needs_user`.

Stopping rule:

- clarification should stop once remaining ambiguity is low-impact, explicitly recorded, and not architecture-shaping
- if high-impact ambiguity remains and only the user can resolve it, return `needs_user`, write durable waiting state, and yield to the user instead of continuing a clarity treadmill

### Lock rule

The mission is locked only when:

- no high-impact ambiguity remains
- no hidden preference remains that would materially change planning
- success is observable
- protected surfaces are explicit
- unacceptable tradeoffs are explicit
- non-goals are explicit
- autonomy boundary is explicit
- at most three low-impact assumptions remain, all written down

### Clarify-owned vs plan-owned unknowns

Clarify owns the destination contract, not the route.

Clarify must resolve:

- destination
- success criteria
- protected surfaces
- unacceptable tradeoffs
- non-goals
- autonomy boundary
- enough baseline truth for safe planning

Clarify must not settle:

- architecture
- decomposition
- detailed sequencing
- execution-grade proof choreography

If repo reality creates feasibility uncertainty, clarify may run bounded feasibility work, but it must stop before option design, workstream invention, or graph construction.

### Repo-reading policy

`$clarify` may read the repo when that helps determine:

- existing boundaries
- likely protected surfaces
- likely interfaces
- whether the ask implies migration or compatibility pressure
- whether the ask obviously touches multiple surfaces

Clarify must not use repo reading as an excuse to start planning.

### High-risk feasibility probe

Clarify may run a small bounded feasibility probe before sacred lock when repo reality could prove the ask infeasible or materially constrained.

High-risk trigger examples:

- auth, billing, or payment-critical surfaces
- data-shape or schema migration
- public contract compatibility pressure
- multi-surface rollout constraints
- high rollback difficulty

Rules:

- feasibility work must stay narrow and evidence-oriented
- it must not choose architecture
- it must not decompose the mission into workstreams
- if feasibility remains uncertain after bounded probing, the correct outcome is constrained lock, `needs_user`, or `hard_blocked`, not fake certainty
- `needs_user` during clarify is allowed only when bounded autonomous probing has already failed and the remaining unblock is genuinely human-owned, such as a business decision, human approval, payment-tier change, sales/support contact, or credential/access handoff that Codex cannot complete itself
- the mere existence of auth, login, or external services is not enough to justify `needs_user`; Ralph should keep trying honest autonomous paths until a true human-only boundary is proven

### `MISSION-STATE.md`

`MISSION-STATE.md` is the live clarify working artifact.

Artifact discipline:

- `MISSION-STATE.md` is mutable working state, not sacred lock state
- it must preserve provenance for user-stated facts, repo-discovered facts, and Codex inferences instead of flattening them into one voice
- `mission_id` is immutable once created
- human-readable mission naming may evolve, but identity must not
- runtime closeout outcomes such as `continue_required`, `needs_user`, `hard_blocked`, and `complete` do not live in `MISSION-STATE.md.status`; those belong to Ralph closeouts and cached machine state

Required frontmatter:

```yaml
artifact: mission-state
mission_id: <id>
root_mission_id: <id>
parent_mission_id: <id | null>
version: 1
clarify_status: clarifying | waiting_user | ratified | superseded
slug: <human-readable-name>
current_lock_revision: <int | null>
reopened_from_lock_revision: <int | null>
```

It must capture at minimum:

- mission identity
- current interpreted objective
- ambiguity state by dimension
- current candidate success criteria
- current protected-surface hypotheses
- baseline repo facts gathered so far
- open assumptions
- highest-value next question
- feasibility notes when used

Mutable-field rule:

- `MISSION-STATE.md` may update interpretation, ambiguity scoring, repo facts, assumptions, and next-question logic as clarify progresses
- once a field is promoted into sacred lock, later drift must reopen or supersede the lock rather than silently mutating the historical working record
- once an `OUTCOME-LOCK.md` revision is ratified for the current mission cycle, `MISSION-STATE.md` should normally move to `clarify_status = ratified`
- `clarify_status = superseded` is for an older clarify worksheet that has been replaced by a newer clarification cycle or mission revision, not for a successfully ratified current worksheet
- constrained-lock details belong in the lock artifact and clarify notes, not as a second hidden runtime status inside `MISSION-STATE.md`

### `OUTCOME-LOCK.md`

`OUTCOME-LOCK.md` is the sacred mission contract.

Artifact discipline:

- `OUTCOME-LOCK.md` is sacred; it locks the destination, not the first route
- it must distinguish user-provided intent from repo-grounded feasibility facts and Codex clarifying synthesis
- `mission_id` is immutable once created
- the lock may be revised only by explicit reopen or superseding lock revision, never by silent mutation

Required frontmatter:

```yaml
artifact: outcome-lock
mission_id: <id>
root_mission_id: <id>
parent_mission_id: <id | null>
version: 1
lock_revision: <int>
status: draft | locked | reopened | superseded
lock_posture: unconstrained | constrained
slug: <human-readable-name>
```

It must capture at minimum:

- mission identity
- objective
- done-when criteria
- success measures
- protected surfaces
- unacceptable tradeoffs
- non-goals
- autonomy boundary
- baseline current facts
- rollout/migration constraints
- remaining low-impact assumptions
- explicit feasibility constraints when `lock_posture = constrained`
- reopen conditions

Locked-field rule:

- objective, done-when criteria, protected surfaces, unacceptable tradeoffs, non-goals, autonomy boundary, and reopen conditions are locked fields
- baseline facts and rollout constraints may be extended only by explicit lock revision when new truth materially changes the destination contract
- clarifying commentary belongs in `MISSION-STATE.md`, not as hidden mutable edits inside the sacred lock
- `lock_posture = constrained` means the destination is ratified but bounded by explicit feasibility or environment constraints discovered during clarify; it is not a second-class or half-locked mission

### Reopen rule

Outcome Lock must reopen if the mission core is invalidated.

Examples:

- desired outcome changed
- protected surfaces changed materially
- success criteria became impossible
- unacceptable tradeoffs changed

## 6. Plan / Replan

### Planning is the main product

The harness succeeds or fails on planning quality.

If later execution must invent architecture, sequencing, proof ownership, or review expectations, planning failed.

`$plan` is not a one-shot drafting action.

It is a Ralph-governed planning loop that continues until one of the following is true:

- the planning completion gate is honestly satisfied
- planning reaches durable `needs_user` waiting state
- planning reaches `hard_blocked`
- planning determines that a higher-layer reopen is required

### Planning stance

The harness must be repo-agnostic.

Planning must be:

- grounded in actual repo reality
- evidence-driven
- obligation-driven
- critique-driven
- honest about what is unresolved

Planning must not:

- rely on a hard-coded repo-local evidence catalog
- rely on predeclared planner surface clusters
- produce fake options when no real branching exists
- treat ceremony as rigor

### Plan levels

V1 uses five planning levels as rigor floors, not completion quotas.

| Level | Default use | Expected planning character |
| --- | --- | --- |
| 1 | bounded local change with low irreversible risk | compact plan, direct proof design |
| 2 | subsystem change with moderate coordination | clearer system map, explicit invariants, bounded option analysis |
| 3 | serious system change | full truth register, critique, proof matrix, and execution graph when needed |
| 4 | migration, public interface, or ops-coupled work | level 3 plus migration, rollout, rollback, and contingency design |
| 5 | mission-critical or rollback-limited work | adversarial, proof-heavy, contingency-aware planning |

### Risk-floor rule

Planning rigor must be at least the risk floor implied by the mission.

Signals include:

- public contract risk
- auth/billing/data migration risk
- rollback difficulty
- operational irreversibility
- blast radius
- protected-surface sensitivity
- cross-surface coupling

User-supplied level is a minimum floor, not the final classifier.

Rules:

- when the user does not specify a level, the harness should begin from the computed risk floor with a compact bias for bounded low-risk work
- bounded low-risk work should normally start on the fast lane rather than being pushed into quota-shaped ceremony
- migrations, public interfaces, or ops-coupled work should raise the floor materially
- rollback-limited or mission-critical work should raise the floor to the highest rigor
- final selected planning rigor must be `max(user_floor, risk_floor)`

### Decision obligations

The center of gravity for planning is the set of **decision obligations** that remain.

A decision obligation is any unresolved question that could materially change:

- architecture boundary placement
- migration or rollout posture
- rollback viability
- proof design
- execution sequencing
- blast radius
- protected-surface risk

Rules:

- only create options where a real decision obligation exists
- if only one viable route survives, record why that is true
- planning is not complete until all critical decision obligations are either resolved, converted into explicit proof-gated spikes, or escalated

Decision obligation minimum shape:

- `obligation_id`
- `question`
- `why_it_matters`
- `affects`
- `governing_contract_refs`
- `review_contract_refs`
- `mission_close_claim_refs`
- `blockingness`
- `candidate_route_count`
- `required_evidence`
- `status`
- `resolution_rationale`
- `evidence_refs`
- `proof_spike_scope`
- `proof_spike_success_criteria`
- `proof_spike_failure_criteria`
- `proof_spike_discharge_artifacts`
- `proof_spike_failure_route`

Required status enum:

- `open`
- `researched`
- `selected`
- `proof_gated_spike`
- `needs_user`
- `retired`

Schema rules:

- decision obligations are machine records, not prose-only notes
- blueprint serialization must include one record per obligation using the minimum-shape keys above
- `affects` must be a non-empty enum list from: `architecture_boundary | migration_rollout | rollback_viability | proof_design | review_contract | execution_sequencing | blast_radius | protected_surface_risk`
- `blockingness` must be one of: `critical | major | minor`
- `candidate_route_count` must be an integer `>= 1`
- `required_evidence`, `evidence_refs`, `governing_contract_refs`, `review_contract_refs`, and `mission_close_claim_refs` must be arrays of strings
- `resolution_rationale` is required whenever `status != open`
- `status = researched` means evidence gathering is complete enough to narrow options, but no route is yet selected
- `status = selected` means one route is chosen and planning is now carrying that route forward into the governing contract
- `status = proof_gated_spike` means the obligation stays open only through a bounded proof spike with explicit success/fail discharge criteria
- `status = needs_user` means bounded autonomous attempts failed and the remaining resolution depends on genuine user-only authority or input
- `status = retired` means the obligation is no longer live because the governing contract changed or the question was superseded by a higher-authority resolution
- `review_contract_refs` is required whenever `affects` includes `review_contract`
- `mission_close_claim_refs` is required whenever the obligation can change mission-close proof or integrated review conclusions
- `status = proof_gated_spike` is invalid unless `proof_spike_scope`, `proof_spike_success_criteria`, `proof_spike_failure_criteria`, `proof_spike_discharge_artifacts`, and `proof_spike_failure_route` are all populated
- `proof_spike_success_criteria` and `proof_spike_failure_criteria` must each be non-empty arrays of explicit discharge statements
- `proof_spike_failure_route` must be one of: `replan_required | needs_user | descoped`
- planning completion fails if any obligation with `blockingness = critical` remains in `open` or `researched`
- planning completion also fails if any obligation with `blockingness = major` remains in `open` or `researched` and still affects the selected route, the current frontier, the next execution package, or the current review contract

### Planning-owned vs execution-owned unknowns

Planning must resolve the questions that would otherwise force execution to invent architecture.

Planning must resolve questions that materially affect:

- architecture
- migration posture
- sequencing
- proof design
- review contract
- blast radius

Unknowns may be deferred to execution only when they genuinely depend on:

- code changes
- runtime behavior
- proof-gated discovery

### Planning stages

`$plan` runs as a gated program with these stages:

1. outcome lock validation
2. discovery
3. decision obligations
4. option program
5. research lanes
6. critique
7. deepening
8. proof and review design
9. blueprint selection
10. post-selection proof/review revalidation
11. packetization
12. execution package gate
13. completion gate

The public `$plan` skill is the user entrypoint into this program. The internal execution of that program may involve many hidden workflow components and specialized subagents coordinated by the main logical thread, but those semantics must remain visible in the skill method rather than being treated as a hidden product.

Loop rule:

- `$plan` must keep iterating while critical planning work remains and the verdict is still `continue_required`
- `$plan` must not stop merely because one document draft exists
- `$plan` must continue until the blueprint, frontier specs, and next selected execution package are strong enough that execution does not need to invent major missing decisions
- `$plan` must end every bounded planning cycle with a Ralph closeout or equivalent durable state write

### Internal planning methods

| Method | Purpose | Mandatory when | Primary output |
| --- | --- | --- | --- |
| Truth Register | separate verified facts, inferred facts, assumptions, contradictions, and unknowns | always for `M+`; strongly recommended always | truth ledger |
| System Map | identify touched surfaces, boundaries, couplings | always | touched-surface map |
| Boundary and Coupling Map | find true interfaces and hidden dependencies | `L/XL`, migrations, multi-owner work | boundary register |
| Invariant Register | define what must not break | always | invariant set |
| Proof Matrix | define what must be proven | always | proof obligations |
| Decision Obligations | identify which unresolved questions actually matter | always | obligation set |
| Option Set Generation | create real alternative architecture/migration shapes only where decision obligations exist | `M+` or any non-trivial ambiguity | option dossiers |
| Option Research | fill evidence gaps per option | whenever options exist | research briefs |
| Adversarial Critique | challenge selected and competing options | `M+` always | disqualifiers and scorecards |
| Deepening Loop | strengthen weak areas only | whenever red or excessive amber findings remain | updated evidence/decisions |
| Proof and Review Design | define proof rows, evidence classes, and reviewer bundle design before execution | always | proof package design |
| Blueprint Assembly | freeze selected architecture and reject the rest | always | Program Blueprint |
| Workstream Packetization | turn blueprint into execution-grade specs | always for the runnable frontier and near-frontier; later backlog only when truth is stable enough | frontier `SPEC.md` files plus provisional backlog when needed |
| Execution Packaging | bind the next selected mission/spec/wave target into a current execution-safe package and verify the package gate | always for the next selected target before planning may complete | execution package plus package-gate verdict |
| Execution Graph & Waves | derive safe sequencing and parallelism | whenever more than one real runnable node exists or sequencing/parallel-safety is non-trivial | machine execution graph when needed |
| Replan Triage | choose local vs blueprint vs lock reopen | whenever assumptions break | replan verdict |

Mandatory-method rule:

- methods marked `always` are mandatory and may not be skipped
- methods whose trigger condition is met are mandatory for that mission
- `$plan` may decide conclusions, but it may not decide to bypass required planning method steps
- if a required method cannot be completed honestly, the correct result is `needs_user`, `hard_blocked`, or `replan_required`, not fake completion

### Planning subagent roster

The main planning thread owns synthesis and final decisions.

No-role-first implementation rule:

- Codex1 initial implementation must not begin by creating a fixed custom agent-role catalog
- the repo should first prove the end-to-end workflow using native bounded subagent orchestration, explicit briefs, durable artifacts, and the frozen `multi_agent_v2` contract
- in that initial phase, the harness may use native subagents without a formal named role set, as long as the orchestration method remains explicit and bounded
- the purpose of the initial phase is to learn which repeated subagent patterns are actually stable, necessary, and worth freezing into later named roles
- role creation is intentionally a late-stage product-hardening step, not an initial implementation prerequisite

V1-core max-depth rule:

- all internal subagents in V1-core are depth `1` only
- "fan out more broadly" means more sibling subagents when safe, not deeper recursive nesting
- read-only helper scans, critics, and reviewers may increase breadth within the allowed thread cap, but they do not get a deeper nesting budget in V1-core
- deeper nesting is out of V1-core unless explicitly introduced later as an experimental capability with its own safety rules

V1-core thread-cap rule:

- supported environments must set the native agent thread cap to `16`, for example through `.codex/config.toml` or the equivalent Codex-native config surface
- supported environments must keep `agents.max_depth = 1`
- V1-core planning and execution design may rely on up to `16` sibling native subagents when the graph and safety rules allow it
- the larger cap exists to support broad read-heavy scouting and bounded parallel support work, not to justify unsafe same-workspace write parallelism

Authorization rule:

- invocation of a public workflow such as `$plan`, `$execute`, `$review`, or `$autopilot` counts as user authorization for the main thread to use bounded internal subagents under the safety rules in this PRD
- this does not authorize unbounded delegation, hidden parallel write sprawl, or delegation that escapes the declared mission contracts

Platform-default clarification:

- Codex platform defaults are not the same thing as this harness contract
- Codex docs describe native subagents as spawning only when explicitly requested, with defaults of `agents.max_threads = 6` and `agents.max_depth = 1`
- this PRD therefore defines a supported V1 harness environment in which invoking a public workflow is the user's explicit request to run that workflow implementation, and that workflow implementation may itself explicitly request bounded internal native subagents
- workflow prompts/skills must explicitly request those native subagents when needed; the harness must not rely on Codex to infer hidden delegation from vague prose
- supported V1 environments must provide the native multi-agent surface and must set `agents.max_threads = 16` and `agents.max_depth = 1` before any workflow relies on that breadth
- this section is not a claim about bare Codex defaults; it is a contract for the configured and trusted V1 harness environment
- this PRD does not define an alternate workflow branch for native subagent runtime defects or broken approval surfacing; those are implementation/runtime bugs to fix in the supported environment, not product semantics

Native `multi_agent_v2` contract:

- V1 uses the native `multi_agent_v2` collaboration surface as the bounded internal subagent mechanism
- the authoritative V1 tool surface is `spawn_agent`, `send_message`, `assign_task`, `wait_agent`, `close_agent`, and `list_agents`
- V1 must not depend on legacy `send_input` or `resume_agent` semantics even if older docs, events, or compatibility surfaces still mention them
- `spawn_agent` child identity must be keyed by canonical `task_name` / task path, not by transient thread-id-facing UX
- `task_name` must satisfy the native lowercase/digits/underscores contract and must remain stable enough to serve as the harness child identity key
- `send_message` is queue-only and text-only in V1; it does not trigger a child turn
- `assign_task` is the turn-triggering child-delivery surface in V1 and may use `interrupt=true` only for explicit preemption
- `wait_agent` is a mailbox-edge waiting primitive only; it must not be treated as proof of child completion or as the child payload itself
- `close_agent` is required for intentional child shutdown and closure of no-longer-needed child lanes
- because `multi_agent_v2` does not expose a native `resume_agent` tool, resumed parent orchestration must reconcile child lanes from artifact truth plus live-agent discovery rather than from a child-resume primitive
- `multi_agent_v2` is not a license for hidden orchestration; it is the native bounded delegation substrate that the public skills may explicitly invoke under this PRD
- Codex1 implementation must later include a dedicated internal skill that teaches and standardizes correct native subagent orchestration for this harness
- that future orchestration skill must be grounded in `docs/MULTI-AGENT-V2-GUIDE.md`, not in ad hoc conversational folklore
- that future orchestration skill should encode at minimum:
  - artifact-first context handoff
  - bounded spawn briefs
  - canonical `task_name` usage
  - conservative `fork_turns` defaults
  - parent-owned reconciliation, completion judgment, and mission truth
- in the initial no-role-first implementation phase, that orchestration skill may target generic native subagents rather than a frozen custom role catalog
- only after Codex1 has been tested enough to reveal stable recurring orchestration patterns should that orchestration skill be refined into a more formalized subagent method if still useful

Compatibility table for native multi-agent naming:

| Concept | Official docs / public docs wording | Local trusted `codex-rs` source term | Current development environment wrapper | PRD canonical term | Notes |
| --- | --- | --- | --- | --- | --- |
| spawn a new child lane | `spawn_agent` | `spawn_agent` | `spawn_agent` | `spawn_agent` | aligned across docs, source, and current development surface |
| send text to an existing lane without triggering a turn | older public docs and config references still center `send_input` | `send_message` in `multi_agent_v2`; `send_input` in legacy surface | `send_message` | `send_message` | Codex1 treats queue-only delivery as a distinct V2 primitive |
| send text to an existing lane and trigger a turn | subagent docs describe routing follow-up instructions but do not freeze one public verb here | `assign_task` in `multi_agent_v2` | `followup_task` in the current development environment | `assign_task` | `followup_task` should be treated as a wrapper/alias for the `assign_task` concept, not as the Codex1 contract name |
| resume a previously closed child lane directly | older public docs and config references still mention `resume_agent` under stable multi-agent config | legacy `resume_agent`; no native V2 equivalent | none | none | not part of the Codex1 V1 contract; parent-led reconciliation replaces this |
| wait for agent activity / mailbox progress | `wait_agent` | `wait_agent` | `wait_agent` | `wait_agent` | Codex1 treats this as mailbox-edge waiting only, not payload truth |
| intentionally close a lane | `close_agent` | `close_agent` | `close_agent` | `close_agent` | aligned |
| inspect currently live lanes | not consistently foregrounded in older public docs | `list_agents` in `multi_agent_v2` | `list_agents` when surfaced | `list_agents` | used for reconciliation and inspection, especially on resume |

Naming-alignment rule:

- when public docs, local source, and the currently surfaced development environment use different names, the PRD follows the native `multi_agent_v2` conceptual split rather than older compatibility names
- `send_input` and `resume_agent` are legacy compatibility terminology only
- `followup_task` in the current development environment should be interpreted as the surfaced wrapper for the `assign_task` concept
- this naming drift is a documentation/alignment issue, not a reason to reopen the frozen Codex1 orchestration contract
- future implementation/docs work may add compatibility notes or aliases, but it must not collapse the queue-only `send_message` concept and the turn-triggering `assign_task` concept back into one ambiguous primitive

Companion implementation guide:

- `docs/MULTI-AGENT-V2-GUIDE.md` is the dedicated implementation-facing guide for how native `multi_agent_v2` actually works
- it exists to make the subagent orchestration method explicit and auditable for future Codex1 implementation work
- it is a companion technical guide, not a replacement for the frozen product contract in this PRD

Later scenario candidates for subagent use after implementation validation:

- repo and system mapping when the main thread needs fast bounded discovery of touched surfaces, boundaries, or hidden couplings
- invariant and non-breakage analysis when a mission needs explicit must-not-break checks before planning or execution can proceed safely
- option research and route comparison when real decision obligations exist and the main thread needs evidence rather than immediate synthesis
- adversarial critique when a provisional route needs pressure-testing for rollback holes, compatibility traps, or sequencing risk
- graph and wave auditing when more than one runnable node exists and parallel safety must be checked explicitly
- spec or contract drafting when the main thread has already selected the route and wants bounded artifact drafting from approved truth
- packet, bundle, or machine-artifact assembly when execution or review gates need deterministic payloads compiled from already-known truth
- bounded implementation or repair when a passed execution package exists and one scoped unit of work can be delegated safely
- independent read-only review when blocking review or mission-close review must judge a bounded bundle rather than a writer transcript

Scenario rule:

- these are scenario classes where subagents may later prove useful, not a required starting role catalog
- Codex1 should not freeze a named custom role catalog until the generic orchestration flow, artifact contracts, Ralph loops, and real subagent deployment scenarios have been exercised in implementation work
- once those behaviors are stable, the repeated successful patterns may be formalized more explicitly only if that adds real clarity or reliability

Rules:

- subagents gather evidence, draft, or challenge
- no subagent chooses the final architecture
- planning subagents are read-heavy by default
- unknown safety or ownership means serialize, not parallelize
- fast scan and surface-mapping subagents should prefer the spark lane when the task is text-only, read-heavy, latency-sensitive, and does not require frontier-grade synthesis
- deep synthesis, ambiguity resolution, and final planning judgments stay on the flagship orchestration lane

Planning delegation rule:

- bounded planning subagents must receive an explicit planning brief rather than a loose conversational handoff
- the planning brief must identify the concrete question to answer, the allowed repo surface, the expected output shape, what the subagent must not decide, and what evidence references it must return
- if the main thread cannot compile a clear planning brief, it is not ready to delegate that planning subtask

### Planning summary minimum shape

When the mission contains real architecture, migration, rollout, or proof-shaping choices, planning must produce a compact summary that includes:

- principles
- decision drivers
- viable options only when more than one route exists
- invalidation rationale when only one route survives

### Small-mission fast lane

For bounded low-risk work, planning may complete with:

- one compact canonical blueprint
- one ready spec unless real decomposition is necessary
- no standalone graph unless more than one real runnable node exists
- no option section unless a real decision obligation admits more than one route
- the same execution package gate before execution begins

Compact does not mean shallow.

### `PROGRAM-BLUEPRINT.md`

Logical artifact:

`Program Blueprint`

Physical representation rule:

- every mission must have one canonical blueprint index
- bounded missions may satisfy that with a single `PROGRAM-BLUEPRINT.md`
- larger missions may expand the blueprint into a foldered package with a canonical index file plus supporting blueprint documents
- bounded low-risk missions may stay in compact mode without expanding `blueprint/`
- compact mode must still preserve explicit proof design, review expectations, and bounded execution contracts

Visible default canonical index:

`PLANS/<mission-id>/PROGRAM-BLUEPRINT.md`

Required frontmatter:

```yaml
artifact: program-blueprint
mission_id: <id>
version: 1
lock_revision: <int>
blueprint_revision: <int>
plan_level: <effective-level 1..5>
problem_size: <S|M|L|XL when computed>
status: draft | approved | reopened
```

Canonical index sections:

1. Locked Mission Reference
2. Truth Register Summary
3. System Model
4. Invariants and Protected Behaviors
5. Proof Matrix
6. Decision Obligations
7. In-Scope Work Inventory
8. Option Set
9. Selected Architecture
10. Rejected Alternatives and Rationale
11. Migration / Rollout / Rollback Posture
12. Review Bundle Design
13. Workstream Overview
14. Execution Graph and Safe-Wave Rules
15. Risks and Unknowns
16. Decision Log
17. Replan Policy

Conditionality rules:

- `Option Set` is required only when actual decision obligations admit more than one viable route
- `Rejected Alternatives and Rationale` is required only when materially different alternatives were actually considered
- `Migration / Rollout / Rollback Posture` is required when the mission touches migration, operational rollout, compatibility, or rollback-sensitive work
- `Execution Graph and Safe-Wave Rules` is required when more than one real runnable node exists or when sequencing/parallel safety is non-trivial
- when only one viable route exists, the blueprint must say so explicitly and explain why no real alternative survived instead of inventing quota-shaped options

It must capture at minimum:

- mission identity
- lock revision
- selected planning level
- problem size when computed
- truth register
- system map
- invariants
- proof matrix
- decision obligations
- selected route
- invalidated alternatives or invalidation rationale
- review design
- work inventory
- frontier packetization summary
- graph summary when needed
- risks and unknowns
- replan policy at the contract level

Truth freshness rule:

- truth rows that influence architecture, sequencing, proof, or protected-surface safety must carry enough freshness metadata to be re-grounded later
- the harness must re-ground critical truth when rebases, new repo discoveries, local implementation changes, or higher-layer replans may have invalidated earlier facts
- stale critical truth is not a pass; it must be refreshed or explicitly downgraded in confidence

Minimum freshness metadata:

- evidence reference
- source type
- observation basis
- observed revision, commit, or repo state when knowable
- observed-at time or cycle reference when useful
- confidence or verification status

Work inventory rule:

- the blueprint must distinguish runnable frontier work, near-frontier work, proof-gated spikes, provisional backlog, and explicitly deferred work
- in-scope work inventory must name what the current mission intends to finish versus what is recognized but intentionally left outside the current completion bar
- no work item may appear runnable without an owning proof and review story

Decision Log minimum shape:

- decision id
- decision statement
- rationale
- evidence refs
- affected artifacts
- revision where adopted

Review Contract / Bundle Design minimum shape:

- what proof rows must be presented to review
- what receipts are required for blocking judgment
- what changed-file or interface context must be included
- which review lenses are mandatory for the mission or spec class
- what mission-close claims or cross-spec claims must be judged integrally rather than per-spec
- review bundle design is the planning-time contract for what review must judge, not merely a later execution convenience

Post-selection revalidation rule:

- once a route is selected into the current blueprint, the harness must revalidate that the proof design, review contract, and frontier packetization still match the chosen route rather than blindly carrying forward pre-selection drafts
- if route selection changes proof rows, review lenses, execution-package assumptions, or the next runnable frontier, those contracts must be rewritten before planning may claim completion

### `SPEC.md`

Logical artifact:

`Workstream Spec`

Physical representation rule:

- small workstreams may use one canonical spec file
- medium and large workstreams should use a workstream folder with a canonical `SPEC.md` and optional local support files

Visible default canonical path:

`PLANS/<mission-id>/specs/<workstream-id>/SPEC.md`

Required frontmatter:

```yaml
artifact: workstream-spec
mission_id: <id>
spec_id: <workstream-id>
version: 1
spec_revision: <int>
artifact_status: draft | active | superseded
packetization_status: runnable | near_frontier | proof_gated_spike | provisional_backlog | deferred_truth_motion | descoped
execution_status: not_started | packaged | executing | blocked | complete
owner_mode: solo | delegated | wave
blueprint_revision: <int>
blueprint_fingerprint: <sha256:... | null>
spec_fingerprint: <sha256:... | null>
```

It must capture at minimum:

- mission identity
- spec identity
- purpose
- in-scope
- out-of-scope
- dependencies
- touched surfaces
- read scope
- write scope
- interfaces and contracts touched
- implementation shape
- proof-of-completion expectations
- non-breakage expectations
- review lenses
- replan boundary
- truth basis refs
- freshness notes for critical assumptions when applicable

Optional workstream-local support files:

```text
PLANS/<mission-id>/specs/<workstream-id>/
  SPEC.md
  REVIEW.md
  RECEIPTS/
  NOTES.md
```

Frontier packetization statuses:

- `runnable`: the spec contract itself is execution-grade now; all required scope, proof, and review contracts exist, but execution still requires a current execution package for the selected target
- `near_frontier`: expected soon; packetized enough for dependency-aware preparation but may still await one bounded truth or sequencing resolution
- `proof_gated_spike`: intentionally narrow execution slice whose job is to resolve one decision obligation or risky unknown
- `provisional_backlog`: identified future slice that is not yet execution-grade and must not be treated as ready work
- `deferred_truth_motion`: future slice intentionally left unpacketized because current truth is still moving too much for honest execution-grade scoping

State-axis rule:

- `artifact_status` tracks whether the canonical spec artifact itself is current or superseded
- `packetization_status` tracks planning/frontier readiness
- `execution_status` tracks runtime progress against the current packaged target
- implementations must not collapse these three axes back into one overloaded `status` field
- planning completion and mission-close rules key off `packetization_status`
- runtime resume and execution-loop rules key off `execution_status`
- reopen/supersede behavior keys off `artifact_status`

Legal-combination rule:

- `execution_status in {packaged, executing}` requires `artifact_status = active` and `packetization_status in {runnable, proof_gated_spike}`
- `packetization_status in {near_frontier, provisional_backlog, deferred_truth_motion, descoped}` forbids `execution_status in {packaged, executing}`
- `artifact_status = superseded` forbids `execution_status in {packaged, executing}`
- implementations must treat any other combination as invalid current-state serialization rather than interpreting it loosely

Promotion and mission-close rule:

- `near_frontier` must be promoted to `runnable`, explicitly descoped, or moved into a follow-on mission before the parent mission may close
- a `proof_gated_spike` must end only through one of its predeclared discharge routes: produce or update a `runnable` spec, justify descoping, or trigger the declared failure route
- `provisional_backlog` and `deferred_truth_motion` must not be smuggled into mission closure as silently abandoned work; they must be explicitly moved, descoped, or retained in a visible follow-on plan

### Planning completion bar

Planning is complete only when:

- critical truth is explicit
- critical decision obligations are resolved, escalated, or converted into proof-gated spikes
- the chosen route has survived critique strongly enough that execution need not invent architecture
- proof and review design are explicit enough that review does not improvise
- the frontier is packetized into bounded specs
- at least the next selected execution target has passed the execution package gate

Planning loop stop rule:

- if the planning completion bar is not met, `$plan` must keep going
- if the remaining blockers are resolvable inside the current planning layer, `$plan` continues
- if only the user can resolve the blocker, `$plan` returns `needs_user`
- if a prior layer must reopen, `$plan` returns `replan_required`
- if safe continuation is impossible under current tooling or policy, `$plan` returns `hard_blocked`

### Replan decision tree

1. If desired outcome, protected surfaces, success measures, unacceptable tradeoffs, or autonomy boundary changed or were proven impossible: reopen Outcome Lock.
2. Else if architecture, contract surfaces, migration shape, proof matrix, review contract, or critical path changed: blueprint replan.
3. Else if the selected route still stands but the current dependency snapshot, wave composition, packaged scope, or package-level proof/review context changed materially: reopen at `execution_package`.
4. Else: local replan only.

Public vs internal replan rule:

- `replan` is not a normal user-facing workflow in V1
- in normal operation, replanning is an internal workflow used by execution, review, validators, and bounded subagents to surface contradictions back to the main orchestrating thread
- the main thread remains the authority that decides which layer to reopen

### Replan salvage and reuse policy

Replan must preserve correct work whenever that work is still valid under the reopened layer.

Rules:

- local spec repair should preserve valid code, receipts, and proof rows from unaffected specs
- blueprint replan should preserve completed specs whose contracts still match the new blueprint contract
- lock reopen is the strongest invalidation and may force broader artifact review, but it must still prefer explicit salvage over blanket discard
- no replan may silently throw away valid evidence; discarded work must be named and justified
- `REPLAN-LOG.md` must state what was preserved, what was invalidated, and why

## 7. Execute / Review / Replan Contracts

This section defines product contracts and artifact/state effects.

It does **not** freeze the exact live interactive Codex CLI mechanics.

### Execution unit

`$execute` is the Ralph-governed execution surface for advancing the validated planning package one bounded target at a time.

`$execute` only runs a target that already has a passed execution package.

The atomic execution unit is a **runnable workstream spec node**: one bounded spec with explicit write scope, dependencies, completion proof, and review requirements.

V1 rule:

- one runnable workstream spec equals one execution graph node
- one graph node equals one independently reviewable execution slice

Specs may have local support files, but the graph must schedule only the canonical runnable spec nodes, not hidden sub-nodes buried inside one spec.

Execution loop rule:

- `$execute` is not a one-step patch action
- `$execute` runs as a Ralph loop over the selected mission/spec/wave target
- `$execute` must not start from blueprint approval alone; it starts only from a passed execution package for the selected target
- `$execute` continues until the target honestly reaches a terminal outcome for that looped target: `hard_blocked` or `complete`
- `needs_user` is not terminal for `$execute`; it writes durable waiting state, yields control to the user, and leaves the target open for later resume
- if the current execution cycle reaches a blocking review gate, the main orchestrating thread invokes `$review`, records durable state, and then continues the execution loop from the post-review truth
- if the current execution cycle needs bounded repair inside the current execution contract, the main orchestrating thread invokes bounded repair subagents, records durable state, and then continues the execution loop
- if the current execution cycle crosses the declared replan boundary or breaks a broader assumption/contract, the main orchestrating thread invokes the internal `replan` skill, updates the reopened layer honestly, and then continues from the reopened truth
- if the selected next frontier is not currently packaged, the mission must return to `execution_package` rather than letting `$execute` improvise
- every bounded execution cycle must end in explicit durable state, not conversational implication

### Target resolution

`$execute` accepts:

- `mission:<id>`
- `spec:<id>`
- `wave:<id>`
- a clean path that resolves unambiguously to one of the above

If target resolution is ambiguous, execution must not start.

Resolution rule:

- `spec:<id>` resolves to exactly one canonical workstream spec
- `wave:<id>` resolves to exactly one validated wave manifest and must name the exact included spec set
- `mission:<id>` does not mean "run the whole mission at once"; it resolves to the current next selected executable target for that mission, and the resulting execution package must still name the exact included spec set it authorizes
- no target form may rely on unstated spec expansion; the execution package is the final source of truth for the exact included specs being executed now

### Execution package gate

Before execution begins, the selected target must have a passed execution package.

The execution package gate must prove:

- target resolution already passed unambiguously
- a real target in the current mission revision
- all referenced runnable specs are current and structurally complete
- satisfied dependencies
- dependency satisfaction at current revisions
- explicit read scope
- explicit write scope
- explicit proof obligations
- explicit review obligations
- an explicit replan boundary
- if the target is a wave, wave validation already passed

If one is missing, execution must not improvise.

If the package gate fails, control returns to `$plan` and internal packetization / frontier preparation rather than starting execution anyway.

Gate-status rule:

- execution is authorized only when the selected execution package is in `status = passed`
- `draft`, `ready_for_gate`, `failed`, `superseded`, and `consumed` are all non-executable states
- no prose-only statement that "the package looks good" may substitute for a machine-checkable `passed` package

### Execution Package

The Execution Package is the hidden contract that binds a selected target to current execution-safe truth.

Minimum shape:

- `package_id`
- `mission_id`
- `target_type`
- `target_id`
- `lock_revision`
- `lock_fingerprint`
- `blueprint_revision`
- `blueprint_fingerprint`
- `dependency_snapshot_fingerprint`
- `wave_fingerprint`
- `included_specs`
- dependency satisfaction state
- explicit read scope
- explicit write scope
- proof obligations
- review obligations
- `replan_boundary`
- wave context when applicable
- `gate_checks`
- `validation_failures`
- `validated_at`
- `status`

Execution package status enum:

- `draft`: package is still being assembled and is not executable
- `ready_for_gate`: package is structurally complete enough to evaluate, but the gate has not yet passed
- `passed`: the package gate has passed and this is the only state that authorizes execution
- `failed`: the gate was evaluated and failed; execution is prohibited until a new or repaired package passes
- `superseded`: later truth, replan, or revision motion invalidated this package; execution is prohibited
- `consumed`: this package has already been used for an execution attempt or bounded execution cycle and must not be silently reused without revalidation

Gate-check rule:

- `gate_checks` must record the pass/fail status of each required gate obligation named in the execution-package gate above
- `validation_failures` must name the exact failed gate obligations when `status = failed`
- a package may move to `passed` only when every required gate check is explicitly marked passed
- any change to governing revisions, fingerprints, dependency truth, wave truth, or declared scope must move a previously `passed` package to `superseded` or force a fresh package

Fingerprint-set rule:

- `included_specs` must be a structured array of:
  - `spec_id`
  - `spec_revision`
  - `spec_fingerprint`
- `lock_fingerprint`, `blueprint_fingerprint`, `dependency_snapshot_fingerprint`, and `wave_fingerprint` together form the minimum governing fingerprint set for package invalidation
- `wave_fingerprint` may be `null` only when the selected target is not a wave

### `replan_boundary`

`replan_boundary` is the machine contract that says which changes stay inside local repair and which changes force a broader reopen.

Minimum shape:

- `local_repair_allowed`
- `trigger_matrix`

`trigger_matrix` minimum shape:

- `trigger_code`
- `reopen_layer`

Required enums:

- `trigger_code`: `write_scope_expansion | interface_contract_change | dependency_truth_change | proof_obligation_change | review_contract_change | protected_surface_change | migration_rollout_change | outcome_lock_change`
- `reopen_layer`: `execution_local | execution_package | blueprint | mission_lock`

Rules:

- every `replan_boundary` must declare whether local repair is allowed at all
- every `trigger_code` listed above that could occur for the target must map to exactly one `reopen_layer`
- crossing any declared trigger invalidates the current execution package and forbids pretending local execution is still inside contract
- `review_contract_change` is a first-class trigger; review obligations changing is not a silent local-repair event
- the same `replan_boundary` contract must be copied consistently into the Workstream Spec, Execution Package, and Writer Packet for the governed target

### Frontier and wave selection

The execution graph is the source of truth for sequencing. Waves are schedule.

Mission-level execution selects in this order:

1. finish already in-flight review/rework nodes
2. use the currently packaged target if one already exists
3. otherwise return to `execution_package` to bind the next selected frontier target honestly
4. form a wave only from nodes that satisfy safe-wave rules and package-gate requirements

### Safe same-workspace parallelism

V1-core write execution should default to:

- one writer at a time in the current workspace
- or isolated write execution when worktree-style isolation is conveniently available

Same-workspace parallel write execution is not the normal V1 path.

If enabled at all, it is an advanced mode that must be explicitly proven safe.

Advanced same-workspace parallel writes are allowed only when:

- dependency-free at the graph level
- `write_paths` are pairwise disjoint
- no `write_paths` overlap any same-wave `read_paths`
- `exclusive_resources` are pairwise disjoint
- no shared schema/deploy/lockfile/global-config side effects exist
- unknown side effects default to singleton-wave

Read-only subagents may fan out more broadly in sibling count, not in recursive depth.

### Writer packet

Before delegated write-capable execution begins, the harness must compile a bounded writer packet.

The Writer Packet is derived from a passed Execution Package. It does not replace the package gate.

Writer packet minimum shape:

- `packet_id`
- `mission_id`
- `source_package_id`
- `target_spec_id`
- `blueprint_revision`
- `spec_revision`
- `allowed_read_paths`
- `allowed_write_paths`
- `proof_rows`
- `required_checks`
- `review_lenses`
- `replan_boundary`
- `explicitly_disallowed_decisions`

Writer-packet derivation rule:

- every writer packet must point back to exactly one passed execution package through `source_package_id`
- if the source execution package becomes `failed`, `superseded`, or stale relative to governing revisions/fingerprints, the writer packet is invalid immediately
- writer packets are bounded child-execution briefs, not durable authority to outrun package invalidation

### Review bundle

Before blocking review begins, the harness must compile a deterministic review bundle.

The Review Bundle is derived from execution results plus the packaged contract it is judging.

Review bundle minimum shape:

- `bundle_id`
- `mission_id`
- `bundle_kind`
- `source_package_id`
- `lock_revision`
- `lock_fingerprint`
- `blueprint_revision`
- `blueprint_fingerprint`
- `governing_revision`
- `mandatory_review_lenses`

When `bundle_kind = spec_review`, the bundle must also include:

- `target_spec_id`
- `spec_revision`
- `spec_fingerprint`
- `proof_rows_under_review`
- `receipts`
- `changed_files_or_diff`
- `touched_interface_contracts`

When `bundle_kind = mission_close`, the bundle must also include:

- `mission_level_proof_rows`
- `cross_spec_claim_refs`
- `included_spec_refs`
- `visible_artifact_refs`
- `deferred_descoped_follow_on_refs`
- `open_finding_summary`

Governing-context rule:

- the review bundle must bind the reviewer to the exact lock, blueprint, and spec contract revisions/fingerprints that the reviewed work claims to satisfy
- for spec review, the bundle must also bind the reviewer to the exact source execution package context that authorized the work being judged
- independent review must not clear a diff against stale governing contract context after replan, reopen, or resume
- when governing contract context changes materially, the prior review bundle is superseded and a fresh bundle is required
- `bundle_kind = mission_close` is mandatory before a mission may close as `complete`

### Review model

Review is mandatory.

Review lenses include:

- spec conformance
- correctness and regression
- interface compatibility
- safety/security/policy
- evidence adequacy
- operability / rollback / observability

Finding classes:

- `B-Arch`
- `B-Spec`
- `B-Proof`
- `NB-Hardening`
- `NB-Note`

Blocking findings must block completion until repaired, explicitly descoped, or reopened at the correct layer.

Execution-to-review rule:

- review is not a parallel product outside execution discipline; it is one of the required gates inside the mission loop
- when a selected target reaches a blocking review gate, `$execute` must route into `$review` rather than pretending the target is finished
- after `$review`, the main orchestrating thread must either continue execution, invoke bounded repair, invoke replan, yield in durable `needs_user` waiting state, or end only with `hard_blocked` or `complete`

Independence rule:

- blocking review must be performed by a fresh read-only reviewer thread or reviewer role
- that reviewer must consume a bounded review bundle, not the writer's full working transcript
- the minimum `spec_review` bundle is: target spec contract, proof rows, receipts, changed files/diff, and any touched interface contracts
- the same execution context that made the change may self-check and collect proof, but it must not be the authority that clears blocking review
- mission-close review must also be read-only and independent from the last writing execution context

Mission-close review rule:

- passing per-spec review is necessary but not sufficient for mission completion
- before the mission may close, an independent mission-close review must check the integrated outcome against the Outcome Lock, blueprint-level invariants, cross-spec claims, and any mission-level proof rows
- mission-close review must verify that deferred, descoped, follow-on, and salvaged work are all represented honestly in visible artifacts
- a mission may not close while unresolved blocking findings remain anywhere in the mission review surface

### `gates.json`

`gates.json` is the machine-readable index of which required gates currently exist, whether they are fresh, and what evidence they depend on.

Minimum shape:

- `mission_id`
- `current_phase`
- `updated_at`
- `gates`

Each gate record must include:

- `gate_id`
- `gate_kind`
- `target_ref`
- `governing_refs`
- `status`
- `blocking`
- `opened_at`
- `evaluated_at`
- `evaluated_against_ref`
- `evidence_refs`
- `failure_refs`
- `superseded_by`

Required enums:

- `gate_kind`: `outcome_lock | planning_completion | execution_package | blocking_review | mission_close_review`
- `status`: `open | passed | failed | stale | superseded`
- `blocking`: `true | false`

Lifecycle rules:

- a gate record must be created when that gate becomes required for the mission or selected target
- any governing revision/fingerprint/scope change that invalidates a previously passed gate must move that gate to `stale` or `superseded`
- `evaluated_against_ref` must name the exact artifact instance, package, bundle, or review surface the gate most recently judged
- `gates.json` is an index, not a substitute for execution packages, review bundles, or the review ledger
- `complete` is illegal while any required gate is `open`, `failed`, or `stale`

### Completion proof

A spec is complete only when:

- all declared proof rows are satisfied
- required tests/checks passed
- touched interfaces were verified
- review found no blocking issues
- artifact and receipt writeback is complete
- downstream state is honestly updated

If a proof row is blank, the spec is not complete.

### Structured contradictions

Execution and review must not surface replans as vague prose.

Any non-local contradiction must be recorded in structured form.

Minimum contradiction record:

- `contradiction_id`
- `discovered_in_phase`
- `discovered_by`
- `target_type`
- `target_id`
- `evidence_refs`
- `violated_assumption_or_contract`
- `suggested_reopen_layer`
- `reason_code`
- `status`

Required enums:

- `suggested_reopen_layer`: `execution_local | execution_package | blueprint | mission_lock`
- `status`: `open | triaged | accepted_for_repair | accepted_for_replan | resolved | dismissed`

Required conditional fields:

- `triage_decision`, `triaged_at`, and `triaged_by` are required whenever `status != open`
- `resolution_ref` and `resolved_at` are required whenever `status = resolved` or `status = dismissed`
- `machine_action` and `next_required_branch` are required whenever `status = accepted_for_repair` or `status = accepted_for_replan`
- `governing_revision` is required for every contradiction record so reopen decisions bind to the exact revision/fingerprint that was violated

Required enums:

- `triage_decision`: `contain_locally | repair_in_layer | reopen_execution_package | reopen_blueprint | reopen_mission_lock | dismiss`
- `machine_action`: `continue_local_execution | force_review | force_repair | force_replan | yield_needs_user | halt_hard_blocked`
- `next_required_branch`: `execution | review | repair | replan | needs_user | mission_close`

Binding rules:

- contradictions must be written to `.ralph/missions/<mission-id>/contradictions.ndjson`; prose summaries are secondary
- when `suggested_reopen_layer` is `blueprint` or `mission_lock`, execution must not continue past local containment until a `replan_required` closeout references that contradiction
- a contradiction may be marked `resolved` only when `resolution_ref` points to the closeout or `REPLAN-LOG.md` entry that discharged it
- contradiction handling is not complete unless the machine-action fields above tell Ralph what branch must happen next

### Replan contract

Replanning must preserve valid work whenever that work is still valid under the reopened layer.

Rules:

- local repair preserves unaffected local work
- broader replan preserves still-valid completed work
- blanket discard is disallowed unless explicitly justified
- non-local replan must leave visible evidence in `REPLAN-LOG.md`

### Open design items in this section

The following operator and edge mechanics are intentionally deferred and must be resolved through Codex research before implementation:

- exact operator UX for running the `execution_package` phase and gate in Codex CLI
- exact operator UX for invoking `$review`, bounded repair, and `replan` from inside `$execute`
- exact review-loop ergonomics in Codex CLI once the frozen Ralph continuity contract is already in place
- exact operator-facing UX for observing and steering already-frozen native-subagent ownership/session reconciliation during execute/review/repair across resumes
- mission-close operator continuation ergonomics

Rule:

- product contracts in this section are binding
- the Ralph continuity contract is not reopened by these items
- only the narrower operator and edge mechanics in this section remain intentionally open
- open mechanics are not permission to improvise

## 8. Ralph Loop / Gates / State

### Ralph role

Ralph is the anti-false-completion discipline for the mission.

Ralph exists to ensure:

- honest continuation
- explicit closeout
- phase awareness
- verdict discipline
- no illegitimate stop
- the ability to continue until actually done under Ralph discipline, even across many turns or long-running mission windows

### Ralph product semantics

Ralph must preserve at least these concepts:

- `phase`
- `activity`
- `verdict`
- explicit closeout
- durable resume intent
- mission terminality vs non-terminal waiting state

Ralph must govern at least these workflow classes:

- planning loops
- execution loops
- mission-close loops
- autopilot continuation

### V1 CLI continuity model

V1 Ralph continuity must have a complete, correct Codex CLI implementation.

For the Codex CLI surface, Ralph in V1 is a CLI-native loop over durable artifacts, not a wrapper runtime.

For the Codex CLI surface, native Codex Stop hooks are the canonical clean-stop veto mechanism. This hook-based rule is the Ralph continuity adapter for CLI; it must not be misread as a claim that every other native Codex surface exposes the same continuation primitive.

Why this is explicit:

- the core product promise is "do not falsely stop early"
- the most dangerous historical failure modes are false stop and false continue
- false stop happens when coarse mode state or conversation tone is mistaken for honest completion
- false continue happens when runtime glue keeps pushing after the mission should honestly yield or stop
- V1 therefore defines the control authority and stop/yield semantics directly in the PRD instead of leaving them to implementation folklore

Required continuity surfaces:

- for the Codex CLI surface, native Codex Stop-hook continuation for clean-stop veto while a mission is non-terminal and still actionable
- durable mission state under `PLANS/` and `.ralph/`
- Codex CLI resume surfaces after interruption, such as `codex resume` or `codex exec resume`

Supported-behavior basis:

- the public Codex docs explicitly describe hooks as under development and off by default
- the public Codex docs explicitly describe `Stop` as a turn-scoped lifecycle event that can return `decision: "block"` with a `reason`, causing Codex to continue by creating a new continuation prompt from that reason
- the public Codex docs explicitly describe matching hooks from multiple files as all running, and state that if any matching `Stop` hook returns `continue: false`, that takes precedence over continuation decisions from other matching `Stop` hooks
- the public Codex docs explicitly describe native CLI resume surfaces such as `codex resume` and `codex exec resume`
- this PRD freezes only those documented Codex hook and resume behaviors as product assumptions
- Ralph uses native Stop-hook behavior only as the live CLI clean-stop veto adapter; interrupted-session recovery is artifact-driven from `PLANS/`, `.ralph/`, valid closeouts, and native CLI resume surfaces rather than from undocumented hook-memory folklore alone
- any stronger build-specific observations beyond the public docs belong to supported-build qualification evidence, not to the timeless normative Ralph product contract by themselves

Truth-source rule:

- this PRD distinguishes three different truth layers and they must not be conflated
- **documented Codex guarantees** are the public behaviors explicitly described in official Codex documentation
- **supported-build qualification evidence** is the stronger behavior demonstrated by the current trusted macOS Codex CLI build and its source/tests before V1 is shipped or upgraded
- **Ralph product truth** is the mission truth carried by `PLANS/`, `.ralph/`, valid closeouts, and the continuity rules in this PRD
- the harness may require stronger supported-build behavior than the docs explicitly promise, but when it does so that stronger claim must be phrased as a supported-environment qualification requirement, not as a false statement about what the docs already guarantee
- the harness must never let transient hook-memory behavior outrank artifact truth; hook behavior is an adapter layer, while mission truth remains artifact-driven

Interpretation rule:

- when the docs are explicit, the docs define the public Codex guarantee
- when the trusted macOS open-source implementation and tests demonstrate stronger stable behavior than the docs spell out, that behavior may be used as supported-build qualification evidence for V1
- when docs and implementation are silent, the PRD must not invent guarantees
- when docs and implementation appear to differ, supported-environment qualification must be resolved against the actual trusted macOS build intended for V1 deployment before the behavior is frozen for use

Current trusted-build observations used as qualification evidence:

- in the current trusted macOS open-source Codex build, hook configs are discovered across config layers in precedence order and matching `Stop` handlers are all selected
- in the current trusted macOS open-source Codex build, matching `Stop` handlers execute concurrently, but their parsed results are reassembled in handler-vector order before aggregation
- in the current trusted macOS open-source Codex build, any `continue:false` result wins over blocking continuation
- in the current trusted macOS open-source Codex build, multiple blocking `Stop` reasons and continuation-prompt fragments are aggregated in handler order
- in the current trusted macOS open-source Codex build, blocked `Stop` continuation prompts are written into turn history / rollout state and current tests show that those persisted hook-prompt messages survive native resume
- those observations are important because they show the current macOS build can support a real Ralph loop cleanly, but Ralph still must not rely on undocumented multi-hook merge behavior when one authoritative Ralph Stop pipeline can avoid that ambiguity entirely

Disallowed as correctness mechanisms:

- tmux prompt injection
- wording-based or stall-pattern auto-nudge as the real continuation authority
- external watcher/daemon or wrapper shell that becomes the real owner of Ralph semantics
- a second hidden runtime where `$autopilot` or `$execute` semantics actually live

Environment rule:

- supported Codex CLI environments for V1 Ralph continuity must enable `features.codex_hooks = true`
- supported Codex CLI environments for V1 Ralph continuity are defined for trusted macOS builds only in this PRD revision
- supported Codex CLI environments for V1 Ralph continuity must run a current supported macOS Codex build that provides native Stop-hook continuation behavior
- that supported macOS Codex build must be able to block clean stop with a continuation prompt during a live CLI turn
- that supported macOS Codex build must be able to record the blocked continuation prompt into turn history strongly enough that the next CLI cycle continues from the same mission truth rather than from free-form assistant recall
- that supported macOS Codex build must be able to demonstrate artifact-correct native resume after interruption
- if the trusted macOS build also demonstrates persisted hook-prompt continuity across native resume, that behavior may be used and documented as supported-build qualification evidence for V1
- supported Codex CLI environments must preflight-verify the hook and resume behaviors named in this section before Ralph-governed mission work starts
- if a candidate macOS Codex build cannot demonstrate those behaviors, it is not a supported Ralph CLI environment for this PRD
- the product contract is defined only for that fully supported environment
- this PRD does not define any partial-continuity mode
- supported-build qualification should always start from the latest current trusted macOS Codex CLI build; at the time of this PRD revision, that is Codex `0.120.0` released on April 11, 2026
- later upgrades must repeat the same qualification rather than inheriting trust automatically

Platform-default clarification:

- Codex docs currently describe `features.codex_hooks` as under development and off by default
- this PRD is therefore not claiming that unset Codex defaults already satisfy Ralph continuity
- instead, V1 intentionally treats native Stop hooks as a supported-environment prerequisite that must be enabled and preflight-qualified
- this is still less brittle than OMX because correctness depends on one native lifecycle seam plus durable artifacts, not on tmux anchoring, watcher nudges, wording heuristics, or sidecar runtime glue

Experimental-surface clarification:

- in this PRD, "experimental" does not mean "bad", "fake", or "unusable"
- it means the Codex team has not yet frozen that surface as a default, fully-settled platform guarantee across every environment
- that status does not disqualify the surface for V1 when the surface is:
  - natively provided by Codex
  - documented enough to establish the public guarantee we need
  - stronger in the current trusted macOS source/tests than the docs alone
  - preflight-qualified and release-qualified on the exact trusted build we ship against
- the actual anti-goal is not "experimental"; the actual anti-goal is "hacky, hidden, or folklore-driven"
- native Codex hooks plus qualification are acceptable for V1
- tmux babysitting, wrapper daemons, wording heuristics, and sidecar control planes are not
- reviewers must therefore not collapse "experimental native surface" into "invalid product seam" without checking documented guarantees, current trusted-build evidence, and qualification requirements together

### Control authority rule

When different signals disagree, Ralph must use this authority order:

1. current visible mission artifacts under `PLANS/` plus the latest valid closeout
2. current machine state under `.ralph/`
3. `ACTIVE-CYCLE` as in-flight evidence
4. transient runtime hints, shell context, tmux state, or conversational wording

Interpretation rule:

- lower-authority hints may help recovery or ergonomics
- lower-authority hints must not overrule explicit visible artifacts or valid closeouts
- if visible and hidden state disagree after interruption, the implementation must reconcile conservatively and prefer the safest non-complete interpretation

### Definitions

For V1, the following terms are used precisely:

- **actionable non-terminal**: the mission is not terminal, the next required branch is known, and Codex can continue without asking the user for new information
- **waiting non-terminal**: the mission is not terminal, but Codex cannot honestly continue until the user supplies a decision, credential, clarification, or other external input
- **terminal**: the mission has honestly reached `complete` or `hard_blocked`
- **interrupted**: the current turn/process/session ended before a valid closeout established terminality or waiting yield
- **clean stop**: Codex is about to conclude the turn normally rather than being externally interrupted

Non-term rule:

- `blocking verdict` is not a normative V1 taxonomy term
- implementations must use the `terminal`, `actionable non-terminal`, and `waiting non-terminal` distinctions above rather than inventing a separate stop/yield verdict family

Actionable examples:

- `continue_required` with a known next target
- `review_required` where the review bundle is ready
- `repair_required` where the repair stays inside the current valid contract
- `replan_required` where the reopen layer is known

Waiting examples:

- a product decision only the user can make
- credential, access, billing, approval, or support escalation that remains unresolved after bounded autonomous attempts and now truly requires the user
- ambiguity whose resolution would materially change architecture, scope, or protected-surface handling
- external approval or business input that Codex cannot honestly infer

### Phase model

V1 phase model:

- `clarify`
- `planning`
- `execution_package`
- `execution`
- `review`
- `repair`
- `replan`
- `mission_close`
- `complete`

Phase rule:

- `clarify` is the intake/ambiguity-destruction phase that produces mission truth strong enough for lock
- `planning` is the route-selection and packetization phase that produces the current blueprint and frontier contracts
- `execution_package` is the internal Ralph phase where an approved blueprint and current frontier are turned into a selected execution-safe target
- `execution_package` is owned by planning / frontier preparation, not by `$execute`
- a mission may move from `execution` back to `execution_package` whenever the next frontier is not yet safely packaged, even if the blueprint remains valid
- `execution_package` may end in `continue_required`, `replan_required`, `needs_user`, `hard_blocked`, or honest transition to `execution`
- `execution_package` is not satisfied by blueprint approval alone
- `review` is the blocking review phase for bounded execution targets
- `mission_close` is the final integrated mission-close review / closure phase, not a synonym for generic blocking review

### Activity model

The activity model should at least express:

- idle
- active work
- waiting on bounded subagents
- waiting on user input
- ready for gate

### Verdict model

The verdict model should at least express:

- `continue_required`
- `review_required`
- `repair_required`
- `replan_required`
- `needs_user`
- `hard_blocked`
- `complete`

Verdict interpretation rule:

- `continue_required`, `review_required`, `repair_required`, `replan_required`, and `needs_user` are continuation verdicts, not honest final completion claims
- `review_required` means the orchestrating thread must invoke `$review`
- `repair_required` means the orchestrating thread must invoke bounded repair work inside the current execution layer when still valid
- `replan_required` means the orchestrating thread must invoke the internal `replan` skill and reopen the correct layer before continuing
- `needs_user` means bounded autonomous attempts have already failed and the remaining blocker is a genuine human-only authority or intervention gap; the mission remains open in a durable waiting state and may yield control to the user, but Ralph must not mark the mission terminal or complete
- `needs_user` must not be used as a synonym for uncertainty, fatigue, weak planning, or "login exists somewhere"; it is reserved for proven human-only intervention

### Terminality and yield rule

- `complete` and `hard_blocked` are the only terminal mission outcomes in V1
- `needs_user` is a non-terminal yield state, not a mission stop
- when `needs_user` is reached, the mission must durably record the outstanding question or required user decision, why Codex cannot resolve it itself, and the resume condition that clears the waiting state
- a current turn may yield back to the user once the waiting state and canonical user-facing request are durably written, but that yield must not mutate the mission into a terminal state
- yielding is permission for the current turn to hand control back while keeping the mission open; it is not a terminal stop and it must stay yieldable on later clean-stop attempts until the waiting condition is cleared or the verdict changes

Interpretation guidance:

- terminal means the mission may truthfully be reported as ended
- yield means the current turn may stop talking, but the mission remains open
- "waiting on the user" is therefore closer to a paused open file than to a closed ticket
- a builder must not implement `needs_user` by flipping the mission into a terminal state with a helpful note attached
- a builder must not implement `needs_user` as an infinite self-loop that repeatedly asks the same question without durable waiting state
- a builder must not emit `needs_user` until bounded autonomous attempts have been tried and recorded

### Waiting-state handshake

`needs_user` must use a resumable two-phase handshake so interruption cannot orphan the waiting request and re-emission stays idempotent by `waiting_request_id`.

Required rule:

1. first write durable waiting state with a stable `waiting_request_id`, the exact canonical request text, `waiting_for`, `resume_condition`, and why the remaining unblock is human-only
2. then emit that exact canonical request to the user
3. then mark the request as emitted for that waiting cycle with `request_emitted_at` or equivalent durable marker

Resume rule:

- if waiting state exists and `request_emitted_at` is present, resume must not invent a new request; it may re-surface the same canonical request
- if waiting state exists and `request_emitted_at` is absent, resume must treat the request as not yet delivered and emit the exact same canonical request once
- the same `waiting_request_id` must survive across resumes until the waiting condition is cleared or superseded
- duplicate surfacing of that same canonical request is acceptable; changing request identity or inventing a new request is not

### Stop-hook discipline

For the Codex CLI surface, when a clean stop is attempted, Ralph must decide from mission truth rather than conversational tone.

Required rule:

1. if the latest closed cycle is terminal (`complete` or `hard_blocked`) and has valid closeout, allow stop
2. if the mission is non-terminal and still actionable, block stop and emit a deterministic continuation prompt derived from current mission state
3. if the mission is in `needs_user` waiting state and the canonical waiting request has been durably written but not yet emitted in the current waiting cycle, emit that exact canonical request before yielding
4. if the mission is in durable `needs_user` waiting state and the current turn has already emitted the canonical waiting request for that cycle, allow the turn to yield to the user without marking the mission terminal
5. if the turn ends without valid closeout, safest non-complete interpretation wins

Yield rule:

- allow stop means the mission is terminal and closed
- allow yield means the current turn may end while the mission remains open in `needs_user`
- later clean-stop attempts while the same waiting state remains active must continue to allow yield rather than silently converting the mission into a terminal stop

Repeatability rule:

- stop blocking must be keyed to a mission-state signature, not merely "Ralph is active", so later fresh stop attempts can be re-blocked while the same mission remains non-terminal
- duplicate replays for the same already-blocked stop reply may be suppressed, but that suppression must not turn a still-open mission into an allowed terminal stop

### Native Stop-hook authority (Codex CLI)

To keep CLI continuation deterministic without over-claiming undocumented Codex internals, V1 freezes only the following documented-hook constraints and harness-side authority rules:

- public Codex hook docs say matching hooks from multiple files all run
- public Codex hook docs say `Stop` ignores `matcher`, so matcher scoping alone cannot isolate one Stop authority
- public Codex hook docs say multiple matching command hooks for the same event can be launched concurrently
- public Codex hook docs say any matching `Stop` hook returning `continue:false` takes precedence over continuation decisions from other matching `Stop` hooks

Current trusted macOS implementation observations:

- the current trusted macOS open-source Codex implementation discovers hook handlers across config layers in precedence order, assigns each handler a stable discovery/display order, and preserves duplicate matching `Stop` handlers
- the current trusted macOS open-source Codex implementation executes matching `Stop` handlers concurrently but aggregates their parsed results in handler order
- the current trusted macOS open-source Codex implementation currently concatenates multiple blocking reasons and continuation fragments in handler order
- the current trusted macOS open-source Codex implementation currently records blocked hook-prompt messages into conversation / rollout history and current tests show those prompts remain visible after native resume
- these current implementation observations are strong evidence that a real Ralph loop is feasible and supportable on trusted macOS builds
- however, V1 still chooses one authoritative Ralph Stop decision pipeline so Ralph correctness does not depend on richer undocumented merge semantics than necessary

Harness-level authority rule:

- supported V1 environments must expose exactly one authoritative Ralph Stop decision pipeline
- that authority may be implemented either as one Stop handler or as one aggregator hook script, but not as multiple independent Ralph decision sources
- supported V1 environments must not register more than one Stop decision source capable of affecting Ralph terminality, continuation, or waiting-yield behavior
- any additional matched Stop hooks must either be absent or be preflight-verified observational hooks that cannot emit allow/block/continue decisions
- because the public docs do not freeze richer merge semantics, this PRD must not rely on undocumented multi-hook ordering, prompt-fragment merge order, or hook-race resolution
- if bootstrap detects more than one Stop decision source capable of changing Ralph terminality/yield outcomes, the environment is not valid for V1 until configuration is repaired

Why this is the right contract:

- the current trusted macOS Codex build already appears capable of carrying blocked continuation prompts across live turns and native resume
- that is good and should be used
- but the harness becomes more robust, not less, when it still requires one authoritative Ralph Stop pipeline instead of depending on coincidental multi-hook merge behavior staying unchanged forever
- this rule therefore takes the best of both worlds: it uses the real native Codex behavior that exists today, while deliberately minimizing the amount of undocumented aggregation behavior the harness needs to trust

Deterministic continuation-prompt rule:

- the continuation prompt must be derived from mission truth, not from whatever the assistant happened to say last
- the continuation prompt should identify the next required branch, target, and proof/checkpoint expectation when known
- two identical mission states should yield materially equivalent continuation prompts
- the continuation prompt must not invent new planning or execution goals that are absent from current mission truth
- the hook layer may transport and persist the continuation prompt across CLI cycles, and the current trusted macOS implementation/tests indicate that it does so across native resume as well, but it must not become the authority that invents that prompt from scratch without current Ralph truth

Stop decision table:

| Mission situation | Stop decision | Why |
| --- | --- | --- |
| valid terminal closeout with `complete` | allow stop | mission is honestly done |
| valid terminal closeout with `hard_blocked` | allow stop | mission is honestly blocked beyond Codex's authority |
| valid non-terminal actionable closeout | block stop | next branch is known and Codex can continue |
| valid `needs_user` waiting closeout with canonical request not yet emitted | emit canonical request, then allow yield, keep mission open | waiting state is durable but delivery for this waiting cycle has not yet completed |
| valid `needs_user` waiting closeout with canonical user request already emitted | allow yield, keep mission open | turn may hand back control without falsely closing mission |
| missing closeout, stale state, or ambiguous interruption | conservative non-complete path | false completion is worse than one extra cycle |

### Ownership and authority rule

Ralph continuity is mission-first, not session-first.

Rules:

- each mission has one canonical `mission_id` and one canonical durable state lineage
- the main orchestrating thread is the only authority allowed to mutate canonical Ralph mission state and closeouts
- bounded native subagents may gather evidence, draft artifacts, package bundles, or review targets, but they must report results back to the main orchestrating thread rather than mutating canonical mission truth directly
- resume must attach to the mission selected by canonical artifacts and latest valid closeout, not merely whichever session or subagent touched the repo most recently
- when session or subagent evidence conflicts with canonical mission artifacts, canonical mission artifacts win until an explicit Ralph-governed repair or reopen updates them

### Closeout discipline

Every bounded orchestration cycle must end with a durable closeout artifact.

No prose-only "done" may advance the mission.

### `RALPH_CLOSEOUT`

Every orchestration cycle must end with a machine-checkable closeout.

Required minimum fields:

- `closeout_id`
- `closeout_seq`
- `cycle_id`
- `mission_id`
- `phase`
- `activity`
- `verdict`
- `target`
- `cycle_kind`
- `lock_revision`
- `lock_fingerprint`
- `blueprint_revision`
- `blueprint_fingerprint`
- `governing_revision`
- `resume_mode`
- `terminality`
- `next_phase`
- `next_action`
- `reason_code`
- `summary`

Sequence rules:

- `closeout_seq` must be strictly monotonic per `mission_id` and increment by exactly `1` from the previous valid closeout
- `cycle_id` must match the in-flight `ACTIVE-CYCLE` being closed
- `closeout_id` must be unique per mission and stable across retry/re-emission of the same closeout event

When `phase = execution_package`, the closeout must identify which target is now packaged or why packaging could not yet complete honestly.

Additional rules:

- when `verdict` is `continue_required`, `review_required`, `repair_required`, or `replan_required`, the closeout must carry a deterministic continuation prompt or equivalent machine directive for the next CLI cycle
- when `verdict = needs_user`, the closeout must carry the exact outstanding user question or request, why user input is required, and what condition clears the waiting state

Mandatory conditional rule:

- when `verdict = needs_user`, `waiting_request_id`, `waiting_for`, `canonical_waiting_request`, `resume_condition`, and `terminality = waiting_non_terminal` are required, and `request_emitted_at` is required once the canonical request has been emitted for that waiting cycle

Required enums:

- `cycle_kind`: `bounded_progress | gate_evaluation | waiting_handshake | recovery_reentry | mission_close | contradiction_handling`
- `terminality`: `terminal | actionable_non_terminal | waiting_non_terminal`
- `resume_mode`: `allow_stop | continue | yield_to_user`
- `next_phase`: `clarify | planning | execution_package | execution | review | repair | replan | mission_close | complete | null`

Required format rule:

- `reason_code` must be a stable lowercase snake_case machine token chosen from the implementation's documented closeout reason vocabulary; free-form prose is not a valid `reason_code`

Required mapping rules:

- `verdict in {complete, hard_blocked}` requires `terminality = terminal` and `resume_mode = allow_stop`
- `verdict in {continue_required, review_required, repair_required, replan_required}` requires `terminality = actionable_non_terminal` and `resume_mode = continue`
- `verdict = needs_user` requires `terminality = waiting_non_terminal` and `resume_mode = yield_to_user`
- terminal closeouts may use `next_phase = complete | null`; non-terminal closeouts must name a non-terminal `next_phase`
- if `phase = execution_package` and the selected target is now executable, `next_phase` must be `execution`
- if `phase = execution_package` and no passed package yet exists, `next_phase` must remain `execution_package`

Recommended conditional fields for implementation clarity:

- `continuation_prompt`
- `waiting_request_id`
- `waiting_for`
- `canonical_waiting_request`
- `request_emitted_at`
- `resume_condition`

Field intent:

- `continuation_prompt` keeps native Stop continuation deterministic
- `waiting_request_id` keeps waiting-state resume idempotent across interruptions
- `waiting_for` tells the human exactly what is missing
- `canonical_waiting_request` is the exact user-facing request text that can be re-emitted without inventing a new waiting identity
- `request_emitted_at` distinguishes "waiting request was durably emitted" from "waiting state was written but delivery was interrupted"
- `resume_condition` tells the machine/human what event clears waiting state
- `terminality` removes ambiguity between terminal and yielding closeouts
- `next_phase` removes ambiguity about which branch resume should enter next
- `governing_revision` plus the fingerprint fields make sure resume logic is bound to the same artifact revision/fingerprint the cycle used

### `ACTIVE-CYCLE`

The mission should maintain a minimal record of in-flight work so interrupted work can be interpreted conservatively on resume.

The active cycle should identify at minimum:

- cycle id
- mission id
- phase
- cycle kind
- target
- current bounded action
- opened-after-closeout sequence
- `lock_revision`
- `lock_fingerprint`
- `blueprint_revision`
- `blueprint_fingerprint`
- `governing_revision`
- preconditions already checked when known
- expected outputs
- attempt index
- active packet or bundle references when applicable
- expected child task paths and required child deliverables when bounded native subagents are part of the current cycle

### `state.json`

`state.json` is the cached machine snapshot of the mission's current Ralph interpretation.

It exists so the next CLI cycle can load one explicit machine-readable summary instead of reconstructing current Ralph state from scattered artifacts on every step.

It is still subordinate to:

1. visible mission artifacts under `PLANS/`
2. the latest valid `RALPH_CLOSEOUT`
3. conservative interruption recovery rules

`state.json` must not become a second sacred truth surface.

Required minimum fields:

- `mission_id`
- `phase`
- `activity`
- `verdict`
- `current_target`
- `next_phase`
- `next_action`
- `resume_mode`
- `lock_revision`
- `lock_fingerprint`
- `blueprint_revision`
- `blueprint_fingerprint`
- `governing_revision`
- `terminality`
- `last_valid_closeout_ref`
- `last_applied_closeout_seq`
- `active_cycle_id`
- `waiting_request_id`
- `waiting_for`
- `canonical_waiting_request`
- `request_emitted_at`
- `resume_condition`
- `active_child_task_paths`

Field rules:

- `phase`, `activity`, and `verdict` must reflect the latest valid Ralph interpretation of the mission rather than a stale in-flight guess
- `current_target` must identify the mission/spec/wave target currently being governed when one exists, and may be `null` only when the mission is between bounded targets
- `next_phase` must identify the next governed branch instead of leaving resume to infer branch transitions from prose alone
- `next_action` must name the next required branch or machine action, not a vague prose summary
- `resume_mode` must be one of `allow_stop | continue | yield_to_user`
- `terminality` must be one of `terminal | actionable_non_terminal | waiting_non_terminal`
- `lock_revision`, `lock_fingerprint`, `blueprint_revision`, `blueprint_fingerprint`, and `governing_revision` must bind the snapshot to the artifact revision context it depends on
- `last_valid_closeout_ref` must identify the closeout entry from which the current machine snapshot was derived
- `last_applied_closeout_seq` must match the sequence of `last_valid_closeout_ref`
- `active_cycle_id` must either match the current `ACTIVE-CYCLE` record or be `null` when no bounded cycle is in flight
- `active_child_task_paths` must list the canonical `multi_agent_v2` task paths currently expected to produce deliverables for the active mission state, and must be an empty list when no child lanes are presently expected
- `waiting_request_id`, `waiting_for`, `canonical_waiting_request`, and `resume_condition` may be `null` only when the mission is not currently in `needs_user`
- `request_emitted_at` may be `null` only when the mission is not currently in `needs_user` or when the waiting request has been durably written but not yet emitted in the current waiting cycle
- when `verdict = needs_user`, `waiting_request_id`, `waiting_for`, `canonical_waiting_request`, and `resume_condition` must be populated and must match the current waiting-state handshake exactly
- `terminality` and `resume_mode` in `state.json` must match the latest valid closeout rather than a stale guessed transition

Update rule:

- after every bounded orchestration cycle, the implementation must update `state.json` to match the latest valid closeout and current mission interpretation
- on interruption recovery, resume logic must load `state.json` first, then reconcile it against `ACTIVE-CYCLE`, visible artifacts, and the latest valid closeout
- if `state.json` disagrees with visible artifacts or a newer valid closeout, `state.json` must be repaired rather than trusted blindly

### Crash-consistent closeout commit protocol

`closeouts.ndjson`, `state.json`, and `active-cycle.json` are one logical commit surface.

Latest-valid-closeout rule:

- "latest valid closeout" means the last parseable, schema-valid, fully written closeout record in `closeouts.ndjson`
- a truncated, malformed, or half-written trailing NDJSON line must be ignored as invalid append debris rather than treated as the newest closeout
- recovery must prefer the last valid complete record over optimistic parsing of a damaged tail

Durable write rules:

- rewrites of `state.json` and `active-cycle.json` must use: write temp file -> fsync temp file -> atomic rename -> fsync parent directory
- `closeouts.ndjson` writes must append exactly one complete NDJSON line per closeout and fsync the file before any dependent write

Cycle-close commit order (must not be reordered):

1. read current `ACTIVE-CYCLE` and latest valid closeout; compute `next_closeout_seq = previous + 1`
2. append closeout with `closeout_seq = next_closeout_seq`, matching `cycle_id`, and stable `closeout_id`
3. fsync `closeouts.ndjson`
4. rewrite `state.json` from that exact closeout, including matching `last_valid_closeout_ref` and `last_applied_closeout_seq`
5. clear `active-cycle.json` (or set no in-flight cycle) via atomic rewrite
6. only then may stop/yield logic consume the new state

Recovery rules:

- if latest valid closeout sequence is newer than `state.last_applied_closeout_seq`, rebuild `state.json` from latest valid closeout
- if `state.last_applied_closeout_seq` is newer than latest valid closeout sequence, treat `state.json` as uncommitted or corrupt and repair from latest valid closeout
- if `ACTIVE-CYCLE` exists and a valid closeout already exists for the same `cycle_id`, treat `ACTIVE-CYCLE` as stale and clear it
- if `ACTIVE-CYCLE` exists without matching valid closeout, treat it as interrupted non-complete work
- terminality or yield decisions must derive from the latest valid closeout authority, with `state.json` as cache

### Visible vs hidden truth

Visible mission artifacts under `PLANS/` are the human-facing truth surface.

Hidden machine state under `.ralph/` exists to support:

- continuity
- interruption recovery
- contradiction recording
- execution-package, packet, and bundle storage
- closeout tracking

Hidden machine state must reconcile back to visible artifacts.

`state.json` is therefore a cached machine snapshot of current Ralph interpretation, not a higher-authority truth source than visible artifacts plus the latest valid closeout.

### Artifact fingerprints

Mission identity alone is not enough for precise invalidation.

V1 must also track:

- `lock_fingerprint`: hash of the approved Outcome Lock contract
- `blueprint_fingerprint`: hash of the approved blueprint contract, including selected architecture, proof matrix, execution graph, and replan policy
- `spec_fingerprint`: hash of the current canonical execution contract for one spec
- `dependency_snapshot_fingerprint`: hash of the dependency satisfaction context used by the current execution package
- `wave_fingerprint`: hash of the current wave composition and validation context when the selected target is a wave

Rules:

1. local spec repair should be able to invalidate only the affected spec contract when the blueprint contract did not materially change
2. blueprint replan must invalidate downstream spec contracts that reference the old blueprint contract
3. closeouts, machine contracts, and receipts must carry the relevant current revisions and fingerprints for the layer they depend on

### Dependency graph validation

V1-core execution graph schema is spec-node based:

- `spec_id`
- `depends_on`
- `produces`
- `read_paths`
- `write_paths`
- `exclusive_resources`
- `coupling_tags`
- `ownership_domains`
- `risk_class`
- `acceptance_checks`
- `evidence_type`

Validation and review obligations attach to runnable spec nodes rather than becoming standalone schedulable graph nodes in V1-core.

Attached obligation minimum shape:

- `obligation_id`
- `kind`
- `target_spec_id`
- `target_spec_revision`
- `discharges_claim_ref`
- `proof_rows`
- `acceptance_checks`
- `required_evidence`
- `review_lenses`
- `blocking`
- `status`
- `satisfied_by`
- `evidence_refs`

Required enums:

- `kind`: `validation | review`
- `status`: `open | satisfied | failed | descoped`

Contract rule:

- each runnable spec contract must carry its attached validation and review obligations
- attached obligations must bind to the exact spec revision/fingerprint they judge
- every attached validation or review obligation must point to the exact claim it discharges
- valid `discharges_claim_ref` targets include a proof-matrix row, a protected-surface rule, a non-breakage rule, or a mission-close integrated claim
- attached obligations that are not linked to a concrete claim do not satisfy graph or gate completeness
- graph/gate validity is not satisfied unless those attached obligations are structurally complete for the target spec class
- cross-spec or mission-level claims must not be forced into one spec-local obligation when they are inherently integrated; those remain first-class proof obligations in the blueprint proof matrix and mission-close review contract

Validation rules:

- only hard dependencies are allowed
- graph must be acyclic
- every dependency must resolve to a known runnable spec node
- every acceptance criterion must terminate in an attached validation or review obligation
- hidden prerequisites fail
- fake dependencies fail

### Wave validation

V1 same-workspace wave rules:

- read-only tasks may parallelize freely
- write-capable tasks may parallelize only if `write_paths` are pairwise disjoint
- a same-wave task's `write_paths` may not overlap another task's `read_paths`
- directory ownership overlaps with descendants
- blocking validation or review for a spec must not be treated as a concurrent same-wave write peer of the spec it is judging
- shared schema/deploy/global config/lockfile surfaces are singleton-wave by default
- `risk_class = meta` or `risk_class = unknown` is singleton-wave by default
- overlapping `ownership_domains` or incompatible `coupling_tags` serialize unless explicitly proven safe
- unknown ownership or side effects serialize

### Continuation / resume behavior

- the main logical thread may span multiple actual Codex turns
- if a turn ends with `continue_required`, the mission remains open and the next cycle is known; Ralph must not mark it done
- if a turn ends with `review_required`, `repair_required`, `replan_required`, or `needs_user`, the mission also remains open; those verdicts identify the next required orchestration branch or waiting condition rather than a terminal stop
- if a turn ends with actionable non-terminal work remaining, the Codex CLI implementation must use native Stop-hook continuation rather than allowing a clean terminal stop
- if a turn ends in durable `needs_user` waiting state, the turn may yield to the user once the canonical waiting request is durably written, and later clean-stop attempts while that same waiting state remains active must continue to yield rather than terminate the mission
- if a turn ends without valid closeout, safest non-complete interpretation wins
- only `complete` and `hard_blocked` are legitimate terminal cycle outcomes from an already-closed cycle
- after process/app/CLI interruption, recovery must come from durable state plus Codex CLI resume rather than a wrapper runtime or tmux babysitter

Resume algorithm guidance:

Multi-mission selection precondition:

- resume always operates on one selected `mission_id`
- when `mission_id` is provided explicitly by the operator, that explicit target is used
- when no explicit `mission_id` is provided, the implementation must use the deterministic selection algorithm below before any branch-resume logic runs

Deterministic selection algorithm when `mission_id` is not explicit:

1. build candidate missions from `.ralph/missions/*` whose latest valid closeout is non-terminal
2. if no candidates exist, do not resume orchestration branches
3. if exactly one candidate exists, bind resume to that `mission_id`
4. if more than one candidate exists, do not auto-select; emit one canonical mission-selection request and enter durable `needs_user`
5. if ranking or artifact inputs are missing or contradictory, do not auto-select; emit one canonical mission-selection request and enter durable `needs_user`

Selection-state durability rule:

- when resume cannot bind one mission deterministically because no explicit `mission_id` was provided and multiple candidates exist, the waiting state must be written to `.ralph/selection-state.json` rather than to an arbitrary mission-scoped file
- `.ralph/selection-state.json` must carry at minimum:
  - `selection_request_id`
  - `candidate_mission_ids`
  - `canonical_selection_request`
  - `selected_mission_id`
  - `request_emitted_at`
  - `created_at`
  - `resolved_at`
  - `cleared_at`
- while `selected_mission_id = null`, the selection state represents an open mission-choice wait and must preserve the same canonical selection request across resumes
- once the user chooses a mission, `selected_mission_id` and `resolved_at` must be written before the main thread binds that mission for resume
- once a mission is selected, `.ralph/selection-state.json` must be cleared or superseded so future resumes do not surface stale mission-selection prompts

Native subagent reconciliation rule on resume:

- in V1, mission selection is a harness-level `mission_id` decision; native Codex CLI resume selects a session/thread, not a mission
- when an operator provides an explicit `mission_id`, that target must win before any branch-resume logic and before any child-lane reconciliation
- after the selected mission is loaded, the main orchestrating thread must reconcile any expected `multi_agent_v2` child task paths for the current in-flight cycle against the live child set returned by `list_agents`
- canonical child identity is the `task_name` / task path returned by native `spawn_agent`, not a transient thread id and not a nickname
- if an expected child task path is live and still non-final, the parent may continue waiting, messaging, or assigning work to that child lane
- if an expected child task path is missing, or is in a final non-success state before its required deliverable was integrated, the parent must treat that lane as interrupted-or-failed work rather than as completion evidence
- because `multi_agent_v2` has no native `resume_agent`, the parent must recover such lanes by bounded respawn or by serializing the remaining work back into the main thread according to current mission truth
- child/session hints must never override canonical mission artifacts, valid closeouts, or the selected `mission_id`

1. load `state.json`, `active-cycle.json`, and the latest valid closeout
2. if explicit `mission_id` is absent and `.ralph/selection-state.json` exists with `selected_mission_id = null`, preserve or re-emit the same canonical selection request instead of inventing a new chooser state
3. if explicit `mission_id` is absent and `.ralph/selection-state.json` exists with `selected_mission_id` populated and `cleared_at = null`, bind that mission first and then clear or supersede the selection state before branch resume
4. verify the relevant current revisions/fingerprints for the layer being resumed
5. if `ACTIVE-CYCLE` exists without a matching valid closeout, treat the cycle as interrupted rather than complete
6. if `ACTIVE-CYCLE` or current cycle state names expected child task paths, reconcile them against the live `multi_agent_v2` child set before treating any child lane as complete
7. if the latest valid closeout is terminal, do not resume execution branches
8. if the latest valid closeout is actionable non-terminal, resume the next required branch from that closeout
9. if the latest valid closeout is `needs_user`, keep the mission open in waiting state and:
   - re-surface the same canonical request when `request_emitted_at` is present
   - emit the same canonical request once before yielding when `request_emitted_at` is absent
10. if state is contradictory or incomplete, choose the safest non-complete interpretation and repair the machine state before proceeding

Reference scenarios:

- **Normal continue**: `$execute` ends a cycle with `review_required`; closeout names the review bundle; Stop blocks and the next turn routes into `$review`
- **Normal waiting yield**: `$plan` reaches a product decision only the user can make; closeout records `needs_user` plus the exact question; the turn may yield, but the mission remains open
- **Interrupted mid-cycle**: process dies after writing `ACTIVE-CYCLE` but before closeout; on resume, the system treats the cycle as interrupted and re-enters from the safest non-complete state
- **True completion**: mission-close review passes, final closeout is `complete`, and later clean stop is allowed
- **True hard block**: external dependency or authority limit makes honest continuation impossible; closeout is `hard_blocked`, and later clean stop is allowed

### Open design items in this section

The following are intentionally unresolved in this PRD revision:

- exact hook/profile installation and enablement ergonomics in trusted repos

Rule:

- Ralph product semantics are required
- the Ralph continuity contract above is frozen
- research may refine ergonomics and edge recovery, but implementers must not casually replace the contract with tmux/runtime heuristics or other wrapper mechanics

## 9. Repo-Native Surface Model

### Open-source distribution model

Codex1 is intended to ship as an open-source implementation, not merely as an internal prompt pack.

Distribution rule:

- the open-source repo name and primary install surface should be `codex1`
- active implementation work for this product should happen in a repository named `codex1`
- docs, skills, templates, setup logic, doctor logic, qualification logic, restore logic, and release workflow should live in `codex1` as the canonical source repo
- active implementation may proceed in that `codex1` repo before the public open-source launch is declared ready
- the runtime must remain native Codex CLI
- any `codex1` CLI surface is an installer / doctor / qualification / restore helper, not a second execution runtime
- the product must not require a wrapper shell, tmux runtime, daemon, or alternate orchestration engine in order to function

Source / package / target repo contract:

- the `codex1` source repo is the canonical implementation and release repo for Codex1 itself
- the installed `codex1` package/binary is a distribution artifact that provides setup, doctor, qualification, restore, and uninstall helpers
- the target repo is any user repository where Codex1-managed trusted-repo surfaces are applied
- `codex1 setup`, `codex1 doctor`, `codex1 qualify-codex`, `codex1 restore`, and `codex1 uninstall` operate on the target repo root, not on the `codex1` source repo root, unless the source repo is itself intentionally being used as the target repo

Open-source launch sequencing rule:

- Codex1 should not publicly launch as a finished open-source harness until the generic no-role-first orchestration flow has been implemented and tested end to end
- only after the real workflow has revealed stable subagent deployment scenarios, skill boundaries, and Ralph-loop behavior should the final custom agent-role catalog be created
- final named-role creation is therefore a last-mile hardening step before open-source launch, not an initial implementation dependency

Open-source adoption rule:

- a new user should be able to try Codex1 safely in an existing repo without fear of losing prior Codex setup
- Codex1 must therefore default to reversible installation with explicit backup, explicit diff summary, and explicit restore path
- the first-run experience should be: install `codex1`, run `codex1 setup`, run `codex1 doctor`, optionally run `codex1 qualify-codex` for deeper proof, then use native Codex CLI with the public skills
- `codex1 qualify-codex` becomes mandatory before release, upgrade sign-off, or any claim that a target repo is on a fully supported trusted build

### OSS command contract

The recommended open-source command surface is:

- `codex1 setup`
- `codex1 doctor`
- `codex1 qualify-codex`
- `codex1 restore`
- `codex1 uninstall`

Command role rule:

- `codex1 setup` prepares the trusted repo and supported Codex environment
- `codex1 doctor` verifies the supported environment and reports actionable fixes
- `codex1 qualify-codex` runs the conformance/qualification checks required by this PRD
- `codex1 restore` restores a previous backup set
- `codex1 uninstall` removes Codex1-managed setup safely, preferably by using backup metadata
- none of these commands may become the primary runtime owner of mission logic, Ralph semantics, planning rules, or review rules
- `codex1 setup` is mutating and idempotent
- `codex1 doctor` is read-only and diagnostic only
- `codex1 qualify-codex` executes gates and writes inspectable evidence artifacts
- `codex1 doctor` may report last-known qualification freshness/status, but it must not pretend that a stale qualification report is a fresh gate run
- the named OSS command surface above is the minimum public support surface, not a ban on narrower deterministic helper commands or subcommands that remain inside the tool-layer contract below

Exact setup contract:

- `codex1 setup` must resolve the target repo root (current directory or explicit `--repo-root`) before writing any project-scoped files
- `codex1 setup` must never mutate the `codex1` source repo just because the binary was installed from it; the source repo is changed only when it is explicitly the target repo
- `codex1 setup` must show which Codex surfaces it will modify before applying changes
- `codex1 setup` must install or update only the minimum Codex1-managed trusted-repo surfaces needed for V1:
  - project `.codex/config.toml` overrides
  - hook registrations in `.codex/hooks.json`
  - skill discovery bridge or installed skill paths
  - repo `AGENTS.md` scaffolding only when applicable
- `codex1 setup` must choose and report an explicit skill-install mode for the target repo: `skills_config_bridge | linked_skills | copied_skills`
- `codex1 setup` must preserve non-Codex1-managed config and hook entries whenever safe
- `codex1 setup` must detect whether the target repo is already trusted by Codex before claiming supported status
- if the target repo is not trusted and setup cannot complete that trust step through an explicit native Codex flow safely, setup must fail with clear remediation rather than pretending the repo is ready
- `codex1 setup` must print an explicit changed-path summary after completion

Exact doctor contract:

- `codex1 doctor` must classify findings at minimum as pass, warn, or fail
- `codex1 doctor` must check:
  - trusted repo state
  - effective Codex config
  - `features.codex_hooks = true`
  - effective agent caps
  - skill discovery / skill bridge validity
  - hook registration validity
  - latest supported-build qualification status when known
- `codex1 doctor` must report concrete remediation guidance for every fail or warn state
- `codex1 doctor` must emit an inspectable effective-config report for all harness-required keys
- each effective-config entry must report:
  - `key`
  - `required_value`
  - `effective_value`
  - `source_layer`
  - `status(pass|warn|fail)`

Exact qualification contract:

- `codex1 qualify-codex` must execute the conformance gates required by this PRD, or a scoped subset with explicit status when a gate cannot yet run
- `codex1 qualify-codex` must produce an inspectable qualification result for the exact current trusted macOS Codex CLI build
- qualification output must identify:
  - the Codex build/version tested
  - which gates passed
  - which gates failed
  - which gates were skipped and why
  - where the qualification evidence was written
- qualification evidence must be versioned and later readable by `codex1 doctor`

### Backup-first installation contract

Before `codex1 setup` modifies any user or repo Codex surfaces, it must create a reversible backup set.

At minimum, setup must backup any file or directory it is about to modify among:

- `~/.codex/config.toml` if touched
- project `.codex/config.toml`
- repo `AGENTS.md`
- `.codex/hooks.json`
- Codex1-managed skill links, copied skill directories, or skill-discovery bridge config
- any other Codex1-managed file that setup will rewrite

Backup rule:

- each setup run that changes managed surfaces must create a timestamped backup set and manifest
- the manifest must identify every touched path, whether it was created or modified, and the restore source for that path
- backup metadata must be stored outside the managed runtime state and must survive uninstall
- restore must be able to return the repo and user Codex surfaces to the pre-setup state represented by that backup set

Backup manifest minimum shape:

- `backup_id`
- `created_at`
- `repo_root`
- `codex1_version` when known
- `paths[]`, where each entry records:
  - `path`
  - `scope` (`user | project`)
  - `change_kind` (`created | modified | removed | linked`)
  - `managed_by`
  - `component`
  - `install_mode`
  - `ownership_mode`
  - `managed_selector`
  - `origin`
  - `backup_path` when prior contents existed
  - `before_hash`
  - `after_hash`
  - `restore_action`

Ownership rule:

- `ownership_mode` must distinguish at minimum `full_file | managed_block | managed_entry | symlink`
- `managed_selector` must identify the exact managed block, key, hook entry, or other sub-file region when `ownership_mode != full_file`
- path-level backup alone is not sufficient for shared files such as `.codex/hooks.json` or `AGENTS.md`; restore/uninstall must know which exact Codex1-managed entry or block it owns

Suggested shape:

- `~/.codex1/backups/<timestamp>/manifest.json`
- backed-up file copies under the same backup root

Safety rule:

- Codex1-managed setup must never silently overwrite user-owned content without backup and explicit reporting
- Codex1-managed setup should preserve non-Codex1-managed hook/config entries whenever safe
- restore capability is part of the product trust model for open-source adoption, not optional polish
- if a copied or linked skill path has user drift relative to its manifest hashes, restore/uninstall must fail safe with explicit remediation instead of destructive overwrite or deletion
- shared files must use entry-level/block-level ownership metadata or dedicated Codex1-owned companion files; "rewrite the whole file and hope the backup is enough" is not a valid V1 safety story

Restore and uninstall contract:

- `codex1 restore` must accept either the latest backup set or an explicit backup identifier
- `codex1 restore` must use the backup manifest as the source of truth for which files to restore, recreate, delete, or unlink
- `codex1 restore` must report any path it could not restore exactly
- `codex1 uninstall` must remove Codex1-managed install surfaces without disturbing unrelated user-managed Codex setup
- when a matching backup exists, `codex1 uninstall` should prefer restore-driven removal over ad hoc deletion
- uninstall without recoverable backup metadata must fail safe rather than guessing destructively
- restore/uninstall may remove or unlink only paths explicitly marked as Codex1-managed in backup metadata

### `AGENTS.md`

`AGENTS.md` owns:

- workflow stance
- quality bar
- repo-specific commands
- artifact conventions

It must stay thin and durable.

It must not own:

- dynamic mission state
- current gates
- mission-specific planning details
- the real workflow method

### Skills

Public repo-local skills are the main user surface:

- `clarify`
- `autopilot`
- `plan`
- `execute`
- `review`

Internal skills or methods should exist for:

- replan
- system mapping
- invariants
- truth register
- option research
- critique
- deepening
- packetization
- graph audit
- review audit

Rules:

- public skills are user-facing
- internal skills carry operational method
- the user should not need to manually chain internal skills during normal use
- the rebuild must not hide core workflow semantics in private code while leaving skills thin
- internal skills should call deterministic tool-layer helpers for precise compilation, validation, inspection, and repair support when that is more reliable than re-deriving those transforms in prompt logic

### Config

`.codex/config.toml` may own:

- model defaults
- reasoning defaults
- sandbox defaults
- approval defaults
- agent thread caps
- workflow profiles when helpful

It must not own:

- mission logic
- planning rules
- hidden workflow semantics

Trusted-repo enablement contract:

- user-level `~/.codex/config.toml` may carry personal defaults, but trusted project-scoped `.codex/config.toml` is the authoritative place for harness-required project overrides
- project-scoped Codex config is part of the supported-environment contract only after the repo is explicitly trusted by Codex
- trusted project-scoped config must enable `features.codex_hooks = true`
- trusted project-scoped config must provide the harness-required multi-agent settings, including `agents.max_threads = 16` and `agents.max_depth = 1`
- trusted project-scoped config may define named workflow profiles when helpful, but the PRD does not require the product to invent a separate helper CLI or wrapper bootstrap surface
- trusted project-scoped config may bridge non-default skill locations through `skills.config` when needed, but that config bridge is a repository setup concern rather than hidden workflow semantics
- bootstrap/preflight must verify the effective config result in the live trusted repo before Ralph-governed work starts
- `codex1 setup` is the preferred open-source way to install or update these trusted-repo overrides safely

Config precedence contract (highest to lowest):

1. explicit runtime flags for the current invocation
2. trusted project `.codex/config.toml`
3. user `~/.codex/config.toml`
4. Codex defaults

Effective-config rule:

- supported status requires all harness-required keys to resolve correctly in the effective config for the current trusted target repo
- when user and project config disagree, the effective trusted project result wins for harness-required keys unless the current invocation explicitly overrides it

Required V1 default model contract:

- the main orchestration model default must be `gpt-5.4`
- the standard helper model default must be `gpt-5.4-mini`
- the spark / fast-parallel model default must be `gpt-5.3-codex-spark`
- the hard-coding specialist lane must use `gpt-5.3-codex`

Required V1 reasoning contract:

- `gpt-5.4` must default to `high` reasoning for orchestration, mission locking, deep planning, hard bug investigation, replan, and mission-close synthesis
- `gpt-5.4-mini` must default to `high` reasoning for test authoring, make-it-green loops, QA, correctness review, spec/PRD mismatch checking, and other supportive correctness-heavy lanes
- `gpt-5.3-codex` must default to `xhigh` reasoning for the hardest implementation, architecture-rescue, deep refactor, AI-slop cleanup, and hardest coding/brainstorming lanes
- `gpt-5.3-codex-spark` must default to `high` reasoning for fast text-only parallel scouting, exploration, hotspot finding, repo-surface mapping, and other low-latency support lanes

Required V1 routing guidance:

- use `gpt-5.4` for all public-skill orchestration and heavy planning by default
- use `gpt-5.4-mini` for test-writing, green-loop, QA, and correctness-checking lanes by default
- use `gpt-5.3-codex` when the task calls for the strongest specialized coding model rather than the general orchestration model
- use `gpt-5.3-codex-spark` for broad parallel exploration and read-heavy support work where speed matters, especially when many sibling scouts are useful
- spark workers should be kept narrow and numerous rather than overloaded into one giant scout context

Agent-cap rule:

- supported V1 environments must set `agents.max_threads = 16` or the equivalent Codex-native agent thread-cap setting
- planning and execution policies should assume that up to `16` sibling native subagents are available when safe
- raising the cap does not loosen graph safety, ownership, review independence, or same-workspace write rules

### Hooks

For supported Codex CLI Ralph continuity, native Codex hooks are required.

They must remain tiny and must not become the orchestration brain.

Explicit contract:

- hooks are required for the supported CLI continuity adapter because the CLI Stop-hook rule is part of the frozen Ralph contract
- hooks may enforce stop, block, or yield decisions from existing mission truth and may emit the continuation prompt fragments that drive the next CLI continuation cycle
- the supported environment must provide exactly one authoritative Ralph Stop decision pipeline (single Stop handler or one aggregator handler) and must fail preflight if conflicting independent Stop pipelines are detected
- hooks must not invent or reinterpret mission truth, planning truth, review truth, or completion truth
- hooks must not become a hidden planner, hidden reviewer, hidden workflow engine, or second runtime
- hooks are authoritative for the CLI stop-veto adapter only; visible artifacts, canonical Ralph state, and valid closeouts remain the real mission authority

### Helper CLI

The helper CLI is not the product.

Rules:

- V1-core must not depend on a helper CLI for normal mission work
- user mission creation must happen through public skills, not helper commands
- helper CLI, if introduced later, is operator support only

### Deterministic tool layer

Deterministic CLI/tools are encouraged when they make Codex1 more reliable, inspectable, and testable.

Implementation references:

- the OpenAI guide `agent-friendly CLIs` should be treated as the primary external reference for how to shape Codex1 deterministic commands so Codex can inspect `--help`, request stable JSON, narrow output, retry safely, and work through large sources without dragging raw noise into the thread: <https://developers.openai.com/codex/use-cases/agent-friendly-clis>
- the OpenAI `cli-creator` skill should be treated as a practical reference for creating the actual `codex1` helper CLI and its command ergonomics: <https://github.com/openai/skills/tree/main/skills/.curated/cli-creator>

Deterministic tool rule:

- deterministic CLI/tools should own compilation, validation, inspection, diffing, repair, and reporting work that can be expressed as precise inputs and precise outputs
- those tools may be exposed as `codex1` subcommands, internal helper commands, or library-backed command surfaces
- those tools are subordinate to skills, artifacts, Ralph closeouts, and the main orchestrating thread
- deterministic tools must not become the primary user-facing mission workflow surface
- deterministic tools must not become the hidden authority for mission semantics, route choice, review judgment, or completion truth

Codex1 should have deterministic CLI/tools for:

- setup
- doctor
- qualify-codex
- restore
- uninstall
- effective-config inspection
- trust/setup state inspection
- artifact schema validation
- artifact fingerprint generation
- artifact diffing and version comparison
- execution-package compilation
- execution-package validation
- writer-packet compilation
- writer-packet validation
- review-bundle compilation
- review-bundle validation
- `gates.json` validation
- `closeouts.ndjson` validation
- latest-valid-closeout resolution
- `state.json` rebuild and repair from closeouts
- active-cycle and state consistency checks
- safe-wave validation
- dependency graph validation
- qualification report writing and reading
- backup manifest writing and reading
- restore-plan preview and changed-path preview

Codex1 must not make deterministic CLI/tools the primary product surface for:

- `clarify`
- `autopilot`
- `plan`
- `execute`
- `review`
- Ralph mission control or orchestration
- deciding user intent
- deciding clarify sufficiency
- choosing architecture
- selecting the winning route
- deciding replan layer from raw contradiction context without the governing workflow
- deciding mission completion
- making independent review judgments
- orchestrating subagents as the core product behavior
- owning canonical mission truth instead of artifacts and closeouts
- replacing public skills with helper commands

Boundary rule:

- skills own judgment and workflow
- artifacts and closeouts own durable truth
- deterministic CLI/tools own precise transforms, compilation, validation, inspection, and repair support
- if a command starts choosing what the mission means or whether the mission is done, it has crossed out of the deterministic tool layer and into forbidden hidden-product behavior

## 10. Concrete V1 File and Artifact Model

### Repository tree

```text
AGENTS.md
.codex/
  config.toml
  agents/
  skills/
    clarify/
    autopilot/
    plan/
    execute/
    review/
    internal-*
PLANS/
  <mission-id>/
    README.md
    MISSION-STATE.md
    OUTCOME-LOCK.md
    PROGRAM-BLUEPRINT.md
    specs/
      <workstream-id>/
        SPEC.md
.ralph/
  missions/
    <mission-id>/
      active-cycle.json
      state.json
      closeouts.ndjson
      contradictions.ndjson
      execution-packages/
      receipts/
      packets/
      bundles/
docs/
  codex1-prd.md
  MULTI-AGENT-V2-GUIDE.md
```

Conditionally required mission additions when triggered:

```text
PLANS/
  <mission-id>/
    blueprint/
    REVIEW-LEDGER.md
    REPLAN-LOG.md
    missions/
      <child-mission-id>/
.ralph/
  missions/
    <mission-id>/
      execution-graph.json
      gates.json
  selection-state.json
```

### File purposes

| Path | Purpose | Human-facing? |
| --- | --- | --- |
| `PLANS/<mission-id>/README.md` | canonical human resume surface | yes |
| `PLANS/<mission-id>/MISSION-STATE.md` | clarify working state | yes |
| `PLANS/<mission-id>/OUTCOME-LOCK.md` | sacred destination contract | yes |
| `PLANS/<mission-id>/PROGRAM-BLUEPRINT.md` | canonical planning package | yes |
| `PLANS/<mission-id>/blueprint/*` | optional expanded planning detail | yes |
| `PLANS/<mission-id>/specs/<workstream-id>/SPEC.md` | bounded execution contract | yes |
| `PLANS/<mission-id>/REVIEW-LEDGER.md` | readable review summary and dispositions | yes |
| `PLANS/<mission-id>/REPLAN-LOG.md` | readable replan summary | yes |
| `.ralph/missions/<mission-id>/active-cycle.json` | minimal record of in-flight work | no |
| `.ralph/missions/<mission-id>/state.json` | cached machine snapshot of current Ralph interpretation, including waiting/resume linkage | no |
| `.ralph/missions/<mission-id>/closeouts.ndjson` | cycle closeout log | no |
| `.ralph/missions/<mission-id>/contradictions.ndjson` | structured contradiction log | no |
| `.ralph/missions/<mission-id>/execution-packages/*` | execution-safe target contracts for the next runnable mission/spec/wave target | no |
| `.ralph/missions/<mission-id>/receipts/*` | proof and task receipts | no |
| `.ralph/missions/<mission-id>/packets/*` | writer packet payloads | no |
| `.ralph/missions/<mission-id>/bundles/*` | review bundle payloads | no |
| `.ralph/missions/<mission-id>/execution-graph.json` | machine graph when non-trivial sequencing is needed | no |
| `.ralph/missions/<mission-id>/gates.json` | machine-readable summary of required gate states, freshness, and evidence refs | no |
| `.ralph/selection-state.json` | durable mission-selection waiting state when resume cannot yet bind one mission deterministically | no |
| `docs/MULTI-AGENT-V2-GUIDE.md` | implementation-facing guide for correct native `multi_agent_v2` orchestration and context handoff | yes |

### Mission README

`README.md` is the human resume surface.

It must answer:

- what this mission is
- current phase
- current verdict
- next recommended action
- current blockers
- where a fresh reader should start

Visible-artifact precedence rule:

- `README.md` is summary-only and must never override canonical artifact truth if it drifts
- `MISSION-STATE.md` is canonical only for live clarify worksheet state
- `OUTCOME-LOCK.md` is canonical for destination truth
- `PROGRAM-BLUEPRINT.md` is canonical for route truth, proof/review design, packetization posture, and graph summary
- `SPEC.md` is canonical for one bounded execution slice
- `REVIEW-LEDGER.md` is canonical for readable review history and mission-close review disposition
- `REPLAN-LOG.md` is canonical for readable non-local replan history

### Artifact expectations

- visible artifacts should be markdown
- hidden machine state should be JSON or NDJSON
- every artifact and machine payload should carry mission identity
- hidden state should reconcile back to visible mission artifacts
- execution packages, writer packets, and review bundles may remain hidden unless retained visibly for audit

### Adaptive artifact scaling

For bounded low-risk missions:

- keep the mission package compact
- prefer one canonical blueprint
- prefer one ready spec unless real decomposition is necessary
- omit standalone graph state unless more than one real runnable node exists
- create `REPLAN-LOG.md` only when a real non-local replan occurs
- `REVIEW-LEDGER.md` may be absent only before any independent review has occurred and while no findings or mission-close review state exist
- once the first blocking review, non-blocking finding with disposition, or mission-close review occurs, `REVIEW-LEDGER.md` becomes required for the mission and remains part of the visible package until closure

For larger missions:

- expand blueprint detail when necessary
- use one folder-backed spec per bounded execution slice
- keep artifact scaling justified by mission size and coupling, not ceremony

## 11. V1 Boundaries

### V1-core required

V1-core requires:

- mission-centered package model
- open-source repo + install surface built around `codex1`
- `README.md` as human resume surface
- `MISSION-STATE.md`
- `OUTCOME-LOCK.md`
- `PROGRAM-BLUEPRINT.md`
- truth register inside planning
- frontier-first `SPEC.md` files
- first-class execution package gate for the next selected target
- public skill surface
- internal skills for the real method
- visible truth under `PLANS/`
- hidden machine state under `.ralph/`
- execution package, writer packet, and review bundle concepts
- contradiction records
- mandatory review as a product contract
- manual-path / autopilot parity as a product contract
- backup-first setup/restore trust model for external adopters
- OSS helper command surface: `codex1 setup`, `codex1 doctor`, `codex1 qualify-codex`, `codex1 restore`, and `codex1 uninstall`
- inspectable qualification evidence for the trusted Codex build
- doctor-to-qualification linkage that surfaces missing, stale, or failed qualification honestly

### Explicitly out of V1-core

V1-core excludes:

- helper-CLI-first UX
- wrapper-shell product behavior
- TypeScript workflow modules as the product core
- thin public skills over hidden orchestration
- hidden repo-local orchestration scaffold masquerading as the real product
- casual invention of Ralph/execute/review/autopilot mechanics
- large external orchestration runtimes

### Unresolved by design in this PRD revision

The previously open blocking items in this area are now frozen enough for V1-core implementation:

- trusted-repo hook/config enablement is part of the supported-environment contract
- mission-selection override is a harness-level `mission_id` contract layered on top of native Codex session/thread resume
- native child ownership, handoff, and resumed-session reconciliation are frozen around `multi_agent_v2`

Future research may still refine operator wording and polish, but these items no longer reopen the frozen Ralph continuity contract or block V1-core implementation.

## 12. Open Design Items Requiring Codex Research

There are no remaining known V1-core architectural blockers in this section after the current research pass.

Any future research in this area is allowed only to refine ergonomics, operator mechanics, upgrade qualification procedures, or adapt to later Codex platform changes without weakening the frozen product contract.

Rule:

- future research is not an implementation permission slip
- it may refine polish or upgrade handling, but it must not casually reopen the frozen continuity, mission-selection, or `multi_agent_v2` ownership contracts

Operational maintenance note:

- outside the harness implementation itself, the team should maintain a separate Codex automation that checks for Codex updates daily
- when an update is detected, that automation should review official changelogs, relevant merged PRs, and trusted source changes to determine whether the supported-environment assumptions or setup guidance for this PRD need revision
- that daily Codex-change audit is an operational safeguard against platform drift
- it is intentionally not part of the harness product contract and does not replace the qualification gates in this PRD

## 13. Build Guidance for a Fresh Session

Phase A — first working slice:

1. canonical docs and artifact templates
2. open-source install surface (`codex1` setup/doctor/qualify-codex/restore/uninstall) plus backup/restore contract
3. the `docs/MULTI-AGENT-V2-GUIDE.md` companion guide as the explicit technical reference for native subagent behavior
4. the deterministic tool layer for precise compilation, validation, inspection, diffing, and repair support
   Use the OpenAI `agent-friendly CLIs` guide and `cli-creator` skill as concrete implementation references for the `codex1` helper CLI/tool surface.
5. public and internal skills as the primary behavior layer
6. create the later internal subagent-orchestration skill from the Multi-Agent V2 guide so generic native subagent handoff, spawn briefs, and reconciliation are standardized rather than improvised
7. implement the real workflow first with no fixed custom agent-role catalog, using bounded native subagents plus explicit orchestration method
8. clarify artifact flow
9. planning artifact flow
10. artifact and machine-state write rules
11. implement the frozen Ralph continuity contract for supported Codex CLI environments

Phase B — launch hardening:

12. supported-build qualification on the latest current trusted macOS Codex CLI build for the hook/resume and `multi_agent_v2` behaviors frozen in this PRD
13. test enough real missions to learn stable subagent deployment scenarios, skill boundaries, and Ralph-loop behavior
14. only then distill the repeated successful orchestration patterns into a final named custom agent-role catalog
15. public open-source launch only after the role catalog and harness behavior are both stable
16. implementation polish and later upgrade-adaptation work only after the frozen V1-core contract is working end to end

The build must not begin by creating:

- hidden workflow-code authority
- thin public skills
- hidden repo-local orchestration scaffolds in place of visible mission artifacts
- helper-CLI product surfaces
- a speculative custom agent-role catalog before the real workflow has been proven

## 14. Acceptance Criteria

This PRD is acceptable only if a fresh Codex session can answer, without invention:

- what the product is
- what the public skill UX is
- why skills are the primary implementation surface
- what `$clarify` must do
- what `$plan` must do
- what artifacts exist and why
- what the visible truth surface is
- what hidden machine state exists conceptually
- what execute/review/replan contracts must guarantee
- which Ralph continuity mechanics are frozen
- which narrower operator/env mechanics remain unresolved
- what must still be researched before implementation

This PRD is acceptable only if a supported implementation can also pass the following conformance gates:

- **Open-source setup safety gate**: `codex1 setup` backs up every managed path before modification, reports what it changed, and leaves a usable restore manifest behind
- **Restore honesty gate**: `codex1 restore` can return managed Codex surfaces to the selected backup state without leaving partial Codex1-managed modifications behind
- **Doctor honesty gate**: `codex1 doctor` reports real supported-environment failures such as missing hooks, wrong agent caps, wrong trust/setup state, or invalid managed config rather than silently tolerating them
- **Supported-build qualification gate**: the exact trusted macOS Codex CLI build intended for V1 must be qualified against this PRD before release or upgrade; qualification must verify hooks enabled, one authoritative Ralph Stop decision pipeline, live Stop blocking with continuation prompt, and artifact-correct native resume
- **Hook-prompt evidence gate**: if the implementation claims persisted blocked continuation prompts across native resume as part of its supported-environment story, that exact behavior must be demonstrated on the exact trusted macOS build used for qualification rather than merely assumed from docs, release notes, or a different source snapshot
- **Stop-hook continuity gate**: when the latest valid closeout is actionable non-terminal, a clean stop in Codex CLI is blocked and continued from Ralph truth; when the latest valid closeout is terminal (`complete` or `hard_blocked`), clean stop is allowed
- **Resume correctness gate**: after interruption following durable `ACTIVE-CYCLE` write but before terminal closeout, `codex resume` or `codex exec resume` returns to the same `mission_id` and follows the safest non-complete path rather than falsely concluding completion
- **`needs_user` idempotency gate**: after waiting state is written, repeated resumes preserve the same `waiting_request_id`, canonical request identity, and non-terminal mission state; they may re-surface the same canonical request, but they must not invent a new request identity or mark the mission complete
- **Mission-selection determinism gate**: with exactly one non-terminal candidate mission, resume binds to that mission; with multiple non-terminal candidates, resume does not auto-select and instead enters canonical mission-selection `needs_user`
- **Mission-selection override gate**: when an operator provides an explicit `mission_id`, that explicit target wins over session heuristics before branch-resume logic begins
- **Manual/autopilot parity gate**: for the same mission truth and the same completion bar, manual public-skill progression and `$autopilot` must converge to the same durable artifact state and gate outcomes even if their intermediate turn shapes differ
- **Execution-package honesty gate**: execution cannot begin unless target resolution is unambiguous and the selected execution package is in a passed state
- **Review-gate honesty gate**: a mission cannot close as `complete` while required blocking review or mission-close review remains missing, stale, or failed
- **Contradiction/reopen honesty gate**: when a contradiction record requires reopen or replan, the mission must not silently continue as if the governing contract were still satisfied
- **No-false-terminal gate**: non-terminal verdicts such as `continue_required`, `review_required`, `repair_required`, `replan_required`, and `needs_user` must never be surfaced as terminal completion
- **Deterministic-tool honesty gate**: deterministic `codex1` tool outputs must be stable, machine-readable, and subordinate to skills/artifacts/closeouts; those tools may compile, validate, inspect, diff, or repair support state, but they must not silently own mission judgment, route choice, review judgment, or completion truth
- **`multi_agent_v2` ownership gate**: canonical mission truth is mutated only by the main orchestrating thread; child lanes may supply evidence or drafts, but they must not silently mutate canonical Ralph truth
- **`multi_agent_v2` reconciliation gate**: on resume, expected child task paths are reconciled against live native child lanes; missing or failed child lanes are treated as interrupted-or-failed work, not as completion evidence
- **`wait_agent` honesty gate**: `wait_agent` is treated only as mailbox-edge waiting and never as proof that a child is complete or that its deliverable has been integrated
- **OSS helper-surface gate**: `codex1 setup`, `codex1 doctor`, `codex1 qualify-codex`, `codex1 restore`, and `codex1 uninstall` must operate on the resolved target repo honestly and must not blur source-repo, installed-package, and target-repo responsibilities
- **Effective-config honesty gate**: `codex1 doctor` must report the effective required config baseline, including source layer, and must not claim support when runtime flags, project config, and user config resolve to the wrong effective values

This PRD is not acceptable if it allows a builder to plausibly conclude that they should:

- rebuild the product around hidden workflow code
- keep skills thin
- hard-code planner grounding to one repo
- replace Ralph with tmux/runtime heuristics, wrapper shells, or external babysitter loops
- treat `needs_user` as a terminal mission stop

## 15. Final Recommendation

The final product test is simple:

- a user should be able to enter native Codex, invoke Codex1, clarify the mission until it is truly clear, and then let Codex1 continue under Ralph until the mission is actually done or honestly waiting on the user
- if that continue-till-done loop does not work reliably for large real-world work inside native Codex behavior, then the product has missed the point no matter how many helper commands, templates, or internal mechanisms exist
- all helper surfaces, hidden state, validators, graphs, specs, packets, review bundles, and qualification gates are justified only insofar as they make that loop rigorous, resumable, and trustworthy

Build Codex1 Harness V1 as a **skills-first, mission-centered, planning-first Codex-native harness** with:

- one sacred Outcome Lock
- one canonical Program Blueprint per mission revision
- bounded Workstream Specs as the execution contract
- one first-class Execution Package gate between blueprint and execution
- visible truth under `PLANS/`
- hidden machine state under `.ralph/`
- execution packages, writer packets, review bundles, contradiction records, and closeouts as first-class concepts
- no helper-CLI-first UX
- no hidden workflow-engine-first architecture

The primary product win must come from:

- stronger clarification
- stronger planning
- stronger execution packetization and subagent discipline
- stronger proof and review discipline
- better mission continuity
- better native Codex workflow
- a real continue-till-done Ralph loop that keeps advancing the mission without wrapper-runtime hacks

Not from:

- wrapper-shell cleverness
- orchestration sprawl
- hidden workflow machinery
- repo-local planning hacks
- a second runtime that calls Codex from the outside and pretends to be the product

That is the clean-slate V1 to build, and that continue-till-done native Codex loop is the main product outcome by which the whole harness should be judged.
