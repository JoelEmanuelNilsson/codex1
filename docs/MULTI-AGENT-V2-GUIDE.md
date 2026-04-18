# How Multi-Agent V2 Works

This guide explains Codex native Multi-Agent V2 in plain language.

It is written for a reader who wants to understand:

- what Multi-Agent V2 is
- what tools it exposes
- how a child agent gets context
- how to message, wait for, and close child agents
- how resume works
- what is documented behavior versus what is inferred from the local open-source Codex implementation
- how Codex1 should use Multi-Agent V2 safely

This guide is intentionally practical.
It is not just a list of tool names.

## Short version

Multi-Agent V2 is Codex's native child-agent system.

The main thread can:

1. spawn a child agent
2. optionally fork some recent conversation history into it
3. give it a bounded task
4. message it later
5. wait for mailbox updates
6. inspect live agents
7. close agents when done

The most important design rule is:

- the main thread keeps mission truth and final authority
- child agents do bounded work
- durable artifacts on disk should carry the real truth
- full transcript forking should be used sparingly

## 1. What Multi-Agent V2 is

At a high level, Codex can run subagent workflows by spawning specialized agents in parallel and then collecting their results.

Official docs:

- [Subagents](https://developers.openai.com/codex/subagents)
- [Subagent concepts](https://developers.openai.com/codex/concepts/subagents)

The reason this exists is simple:

- too much noisy intermediate work in the main thread causes context pollution
- long noisy conversations can become less reliable over time
- subagents let the main thread stay focused on requirements, decisions, and final outputs

Official docs explicitly recommend subagents for bounded parallel work such as:

- exploration
- tests
- triage
- summarization

And they explicitly warn that write-heavy parallel work needs more care.

## 2. What makes V2 different from the older surface

There was an older collaboration surface.
In practice, you may still see legacy names in docs, older code, or compatibility layers.

The important distinction is:

- older surface: `send_input`, `resume_agent`
- Multi-Agent V2: `send_message`, `assign_task`

When `multi_agent_v2` is enabled, the local `codex-rs` source on this Mac registers this V2 tool surface:

- `spawn_agent`
- `send_message`
- `assign_task`
- `wait_agent`
- `close_agent`
- `list_agents`

Source:

- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/spec.rs:776](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/spec.rs#L776)

The guide below is about that V2 surface.

Important nuance:

- this is not the same thing as saying V2 is always on in every build
- in the local source snapshot inspected for this guide, V2 registration is feature-gated
- so you should distinguish:
  - official docs
  - the local source snapshot
  - the exact qualified build you actually run

## 3. The basic mental model

Think of Multi-Agent V2 as a small native collaboration protocol:

- the parent thread is the leader
- each child is a separate agent thread
- each child has its own context and status
- the child is identified by a canonical task path
- the parent can talk to the child, wait, inspect, and close it

The parent should not treat the child as a second orchestrator.

The clean mental model is:

- parent owns truth
- child owns a bounded assignment

## 4. The actual V2 tools

### `spawn_agent`

Purpose:

- create a new child agent thread and give it an initial task

In V2:

- `task_name` is required
- `items` is required
- optional `agent_type`
- optional `model`
- optional `reasoning_effort`
- optional `fork_turns`

Important source-backed details:

- `fork_context` is not supported in V2
- use `fork_turns` instead
- the V2 output returns `task_name` plus optional `nickname`
- `agent_id` still exists in the output schema, but it is legacy and null in the current V2 implementation
- `task_name` must not be empty
- `task_name` must use only lowercase letters, digits, and underscores
- `task_name` must not be `root`, `.`, or `..`
- `task_name` must not contain `/`
- spawn can fail if the current child depth would exceed the configured depth limit

One more practical note:

- the native source-level tool shape is item-based
- some higher-level wrappers may present this as a plain text message
- for V2, the safe beginner mental model is: the child receives text input items, and the protocol is currently text-first

Sources:

- [/Users/joel/.codex-documentation-repo/codex-rs/tools/src/agent_tool.rs:45](/Users/joel/.codex-documentation-repo/codex-rs/tools/src/agent_tool.rs#L45)
- [/Users/joel/.codex-documentation-repo/codex-rs/tools/src/agent_tool.rs:596](/Users/joel/.codex-documentation-repo/codex-rs/tools/src/agent_tool.rs#L596)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs:203](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs#L203)
- [/Users/joel/.codex-documentation-repo/codex-rs/tools/src/agent_tool.rs:355](/Users/joel/.codex-documentation-repo/codex-rs/tools/src/agent_tool.rs#L355)
- [/Users/joel/.codex-documentation-repo/codex-rs/protocol/src/agent_path.rs:120](/Users/joel/.codex-documentation-repo/codex-rs/protocol/src/agent_path.rs#L120)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs:47](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs#L47)

### `send_message`

Purpose:

- add a message to an existing live agent without triggering a new turn immediately

This is queue-only.

In practice:

- use it for updates, extra notes, or context you want the child to see later
- do not use it when you want the child to wake up and work right now
- the target can be either:
  - a thread id
  - a canonical task path
- in V2, the message surface currently supports text content only

Source:

- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/message_tool.rs:25](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/message_tool.rs#L25)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/agent_resolver.rs:7](/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/agent_resolver.rs#L7)

### `assign_task`

Purpose:

- send a message to an existing live agent and trigger a turn immediately

This is the "wake up and do this now" tool.

It also supports:

- `interrupt = true`

Use interrupt only when you genuinely want to preempt what the child is doing.

Just like `send_message`:

- the target can be either a thread id or a canonical task path
- the V2 surface is currently text-only

Source:

- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/message_tool.rs:31](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/message_tool.rs#L31)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/message_tool.rs:126](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/message_tool.rs#L126)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/agent_resolver.rs:7](/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/agent_resolver.rs#L7)

### `wait_agent`

Purpose:

- wait for mailbox activity

This is the most misunderstood tool.

What it does:

- it waits for mailbox sequence change
- it does not return the child's actual payload
- it does not prove the child is complete

Its return shape is extremely small:

- `"Wait completed."`
- or `"Wait timed out."`

plus `timed_out`

It also clamps timeout to the configured allowed window rather than accepting any arbitrary value.

So the correct use is:

1. call `wait_agent`
2. then inspect live state or new messages
3. do not treat `wait_agent` itself as completion proof

Sources:

- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs:31](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs#L31)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs:83](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs#L83)

### `list_agents`

Purpose:

- show the live agents in the current thread tree

This is what the parent should use when it needs to reconcile:

- which child lanes still exist
- which task paths are still live
- which statuses are final or non-final

It also supports optional `path_prefix`.

Important details:

- `path_prefix` is resolved using the same root-relative or local-relative agent-path rules as other V2 agent targets
- the returned `agent_name` is usually the canonical task path
- if a live agent has no canonical path available, `agent_name` falls back to the thread id
- the root thread can appear in the list as `/root`

Source:

- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/list_agents.rs:26](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/list_agents.rs#L26)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/control.rs:746](/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/control.rs#L746)
- [/Users/joel/.codex-documentation-repo/codex-rs/tools/src/agent_tool.rs:391](/Users/joel/.codex-documentation-repo/codex-rs/tools/src/agent_tool.rs#L391)

### `close_agent`

Purpose:

- explicitly close a child lane you no longer need

Important details:

- root cannot be closed as a child
- close marks the spawn edge as `closed`
- close also shuts down the target child and live descendants

Sources:

- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/close_agent.rs:33](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/close_agent.rs#L33)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/control.rs:616](/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/control.rs#L616)

## 5. How child identity works

In V2, the important identity is not a friendly nickname.
It is the canonical task path.

Examples:

- `/root/specdrafter1`
- `/root/reviewer1`
- `/root/planning/specdrafter1`

This path comes from the `task_name` you provide at spawn time.

Why this matters:

- the path is what the parent should use as the stable child identity
- nicknames are just display labels
- thread ids are real, but not the best long-term conceptual handle for the harness

Source:

- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs:188](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs#L188)

## 6. How child status works

The native status model is:

- `pending_init`
- `running`
- `interrupted`
- `completed(...)`
- `errored(...)`
- `shutdown`
- `not_found`

Important distinction:

- non-final: `pending_init`, `running`, `interrupted`
- final: `completed`, `errored`, `shutdown`, `not_found`

That means `interrupted` is not the same as failed.
It may still be resumable work.

Sources:

- [/Users/joel/.codex-documentation-repo/codex-rs/protocol/src/protocol.rs:1600](/Users/joel/.codex-documentation-repo/codex-rs/protocol/src/protocol.rs#L1600)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/status.rs:22](/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/status.rs#L22)

## 7. How context gets into a child

This is the most important part of the system.

A child does not have to be born with the entire parent conversation copied into it.

In practice, child context comes from four places:

1. inherited session/base instructions
2. selected role config
3. the spawn brief
4. optional forked parent history

### Inherited base instructions

The child inherits the parent session's base instruction/config foundation and then applies role-specific overrides.

Source:

- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs:67](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs#L67)

### Role config

The child then applies the selected role config.

This is where a role like `specdrafter1` should learn its persistent method.
That is much better than rewriting a giant prompt every spawn.

Source:

- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs:77](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs#L77)

### Spawn brief

The spawn brief is the bounded task you send with `spawn_agent`.

This should usually be short:

- what to do
- which files/artifacts are authoritative
- what not to decide
- what to return

### Optional forked history

V2 uses `fork_turns`.

Allowed values:

- `none`
- `all`
- a positive integer string like `3`

Meaning:

- `none`: do not fork parent transcript
- `all`: child gets the full forked history
- `3`: child gets only the most recent three turns

Sources:

- [/Users/joel/.codex-documentation-repo/codex-rs/tools/src/agent_tool.rs:596](/Users/joel/.codex-documentation-repo/codex-rs/tools/src/agent_tool.rs#L596)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs:231](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs#L231)

## 8. When to fork history and when not to

Default recommendation:

- use `fork_turns = none` most of the time

Use small forking like `1`, `2`, or `3` when:

- the child needs very recent conversational nuance
- the user just clarified something not yet written into artifacts
- the child is continuing a local conversational branch

Use `all` only when:

- the child truly needs almost the same working context as the parent

Why not always fork everything?

- more tokens
- more noise
- more context pollution
- less clear source of truth

Codex docs explicitly frame subagents as a way to reduce context pollution and keep the main thread focused.

Source:

- [Subagent concepts](https://developers.openai.com/codex/concepts/subagents#why-subagent-workflows-help)

## 9. The most important Codex1 rule about context

For Codex1, the correct order of truth should be:

1. role instructions
2. durable artifacts on disk
3. bounded spawn brief
4. optional forked recent turns

Not:

1. giant transcript copy

This matters because if a child needs the whole conversation to do routine work, the real problem is usually that the main thread has not yet written enough durable truth into artifacts.

So the real handoff layer should be:

- `OUTCOME-LOCK.md`
- `MISSION-STATE.md`
- `PROGRAM-BLUEPRINT.md`
- `SPEC.md`
- execution package
- review bundle
- contradiction record

not the raw transcript.

## 10. How approvals and sandboxing work

Official docs say:

- subagents inherit the current sandbox policy
- parent turn live overrides are reapplied to the child
- if a non-interactive flow cannot surface a new approval, the action fails and the error is surfaced back to the parent workflow

Source:

- [Subagents: approvals and sandbox controls](https://developers.openai.com/codex/subagents#approvals-and-sandbox-controls)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_common.rs:196](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_common.rs#L196)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs:67](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs#L67)

This is important because it means child execution is not a magical escape hatch.
Children still live under the same security/approval world.

In the local source, this runtime inheritance is broader than just approvals:

- model/provider and reasoning settings are refreshed from the live turn
- approval policy is refreshed from the live turn
- sandbox policy is refreshed from the live turn
- cwd is refreshed from the live turn

So a child inherits both prompt context and current runtime policy.

## 11. Limits: max threads and max depth

Official config docs say:

- `agents.max_threads` defaults to `6`
- `agents.max_depth` defaults to `1`

Source:

- [Config reference](https://developers.openai.com/codex/config-reference#configtoml)

What that means:

- depth `1` allows direct children of the main thread
- it does not encourage deep recursive trees by default
- thread cap limits how many agent threads can be open concurrently

In the local source on this Mac:

- generic multi-agent collaboration is stable and on by default
- `multi_agent_v2` itself is marked under development and default-disabled in this source snapshot

Source:

- [/Users/joel/.codex-documentation-repo/codex-rs/features/src/lib.rs:703](/Users/joel/.codex-documentation-repo/codex-rs/features/src/lib.rs#L703)
- [/Users/joel/.codex-documentation-repo/codex-rs/features/src/lib.rs:709](/Users/joel/.codex-documentation-repo/codex-rs/features/src/lib.rs#L709)

That means you should distinguish:

- public product docs
- current local source snapshot
- actual qualified build you intend to run

## 12. How resume works

The most important thing to understand is:

- V2 does not expose a native `resume_agent` tool

So child recovery is not:

- "resume this child by native child tool"

It is:

- resume the parent/session
- inspect live children
- reconcile expected child task paths
- respawn or reassign when needed

The local source also shows that persisted spawn-edge state tracks `open` versus `closed`.
Closed descendants stay closed on resume.
Open descendants are reconciled by the parent from artifact truth plus live-lane
inspection. If a lane is missing or stale, the safe Codex1 move is parent-led
respawn or serialization back into the parent thread, not child-local resume.

Sources:

- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/spec.rs:185](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/spec.rs#L185)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/control.rs:360](/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/control.rs#L360)

The safe Codex1 rule is:

- the parent owns reconciliation
- the child is never the owner of mission truth

## 13. How a good Codex1 handoff should work

Example: `specdrafter1`

Bad pattern:

- parent writes a giant custom prompt explaining the whole mission

Good pattern:

1. parent writes durable artifacts
2. parent spawns `specdrafter1`
3. parent says which artifact paths are authoritative
4. child reads those files
5. child drafts the spec
6. parent integrates or rejects the result

That is cheaper, more reliable, and easier to resume later.

## 14. Recommended best practices

For novices, these are the safest habits:

1. keep the main thread focused on decisions and truth
2. use subagents for bounded work
3. prefer read-heavy child roles first
4. keep write-capable child work tightly bounded
5. use artifacts as the main truth-transfer mechanism
6. use `fork_turns` only when needed
7. do not treat `wait_agent` as proof of completion
8. use canonical task paths as child identity
9. close child lanes intentionally when done
10. do not let children decide mission truth, terminality, or reopen layers

## 15. Common mistakes

These are the most common conceptual errors:

- treating `wait_agent` as if it returns the child's final result
- treating nicknames as stable identity
- forking the entire transcript by default
- using child agents as hidden orchestrators
- letting a child mutate canonical mission truth
- assuming `resume_agent` exists in V2
- using subagents to compensate for weak artifacts

## 16. What Codex1 should do with Multi-Agent V2

For Codex1 specifically, the best use of Multi-Agent V2 is:

- main thread owns mission truth and Ralph state
- children do bounded evidence, drafting, execution, or review work
- context transfer happens mainly through durable artifacts
- `fork_turns` is a secondary aid, not the main truth layer
- resume is parent-led and artifact-led
- execute and autopilot should treat `list_agents` as reconciliation input and
  `wait_agent` only as mailbox-edge waiting, never as completion proof
- `$review-loop` should treat child reviewer lanes as findings-only workers:
  they return `NONE` or structured findings, never gate updates, closeouts, or
  completion decisions
- review judgment itself belongs to those reviewer lanes; the parent
  orchestrates, aggregates, and writes back, but does not perform code/spec,
  intent, integration, or mission-close review locally

That keeps Codex1:

- Codex-native
- less hacky
- cheaper to run
- easier to qualify
- easier to reason about

## 16.1 Review-Lane Profiles

Codex1 review children should be routed by review profile, not by one generic
review prompt:

- `local_spec_intent`: `gpt-5.4`, for spec and intent judgment after a local
  spec or phase boundary
- `integration_intent`: `gpt-5.4`, for cross-slice coherence and end-goal fit
- `mission_close`: two `gpt-5.4` reviewers, for PRD/intent/mission-close
  judgment before terminal completion
- `code_bug_correctness`: `gpt-5.3-codex`, for code defects and implementation
  correctness after code-producing work

`gpt-5.4-mini` is not a default blocking-review model. Use it only for a
non-blocking support role if a later plan explicitly permits that.

Child reviewer output must be either `NONE` or structured findings with
severity, evidence refs, rationale, and suggested next action. P0, P1, and P2
findings block clean review; P3 findings are non-blocking by default.

When a native hook invocation can identify a child as a findings-only reviewer
lane, it should pass lane metadata such as `laneRole = findings_only_reviewer`
or a review `childLaneKind` value. Codex1's Stop-hook handling treats that lane
as allowed to return its bounded payload even while parent mission gates remain
blocked. Parent/controller lanes still enforce the full Ralph mission gate
contract.

Before spawning findings-only review lanes, the parent should capture Codex1
review truth, then capture a frozen review evidence snapshot for the child
brief. The full review truth snapshot is parent-held writeback capability; do
not embed it in child-visible review evidence or hand it to reviewer lanes. The
parent later includes the parent-held truth snapshot when recording the
parent-owned review outcome. If a child lane changes gates, closeouts, state,
ledgers, specs, receipts, bundles, or mission-close artifacts, the snapshot
guard makes the review wave contaminated instead of letting the child clear
mission truth. Reviewer children should consume the frozen evidence snapshot
first and use live mutable repo paths only when the parent keeps the guard
active.

For blocking reviews, reviewer children should be spawned as read-only
explorer-style findings-only lanes with `fork_turns` set to `none`. Do not use
mutation-capable worker/default reviewer lanes as the blocking review role.

When the parent records a review outcome, it must cite reviewer-agent output
evidence such as `reviewer-output:<lane-or-artifact>`. That evidence is the
authority for `NONE` or findings. Parent-local judgment is allowed only for
orchestration and aggregation; it is not a substitute review result.

## 17. One-sentence summary

Multi-Agent V2 is a native child-thread collaboration system where the parent spawns bounded child agents, communicates with them through a small tool surface, keeps mission truth on the main thread, and should use durable artifacts rather than giant transcript forks as the main context handoff mechanism.

## Appendix: primary sources

Official docs:

- [Subagents](https://developers.openai.com/codex/subagents)
- [Subagent concepts](https://developers.openai.com/codex/concepts/subagents)
- [Configuration reference](https://developers.openai.com/codex/config-reference#configtoml)

Local open-source Codex implementation inspected on this Mac:

- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/spec.rs](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/spec.rs)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/message_tool.rs](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/message_tool.rs)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/close_agent.rs](/Users/joel/.codex-documentation-repo/codex-rs/core/src/tools/handlers/multi_agents_v2/close_agent.rs)
- [/Users/joel/.codex-documentation-repo/codex-rs/tools/src/agent_tool.rs](/Users/joel/.codex-documentation-repo/codex-rs/tools/src/agent_tool.rs)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/control.rs](/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/control.rs)
- [/Users/joel/.codex-documentation-repo/codex-rs/protocol/src/protocol.rs](/Users/joel/.codex-documentation-repo/codex-rs/protocol/src/protocol.rs)
- [/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/status.rs](/Users/joel/.codex-documentation-repo/codex-rs/core/src/agent/status.rs)
- [/Users/joel/.codex-documentation-repo/codex-rs/features/src/lib.rs](/Users/joel/.codex-documentation-repo/codex-rs/features/src/lib.rs)
