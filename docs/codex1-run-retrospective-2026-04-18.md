# Codex1 Run Retrospective - 2026-04-18

## Short Verdict

This run was genuinely useful and genuinely messy.

My honest opinion is that Codex1 got materially stronger, but the way it got
there was too chaotic for the product experience we ultimately want. Ralph
caught real problems that would otherwise have produced false completion, but
Ralph also exposed that the current control plane is not calm enough yet:
parent loop authority, reviewer lane lifecycle, user discussion pauses, stale
gate recovery, and subagent shutdown all need to feel much less brittle.

I trust the product direction more after this run. I trust the current UX less.
That is not a contradiction. The contracts are becoming right; the operator
experience around those contracts still needs work.

## What Went Well

Ralph prevented false completion multiple times. It blocked stale packages,
stale review bundles, stale fingerprints, missing review gates, missing proof
coverage, incomplete reviewer lane coverage, and mission-close overclaims. That
is exactly the kind of failure Codex1 is supposed to catch.

The run also forced several important concepts to become first-class:

- Parent/orchestrator authority is different from reviewer judgment.
- Reviewer output is evidence, not writeback authority.
- Child reviewers must not clear gates, compile packages, append closeouts, or
  terminalize missions.
- `$execute` may route into `$review-loop`, but execution must not claim
  mission completion by itself.
- Mission-close readiness is not the same as terminal mission-close approval.
- Advisor/CritiqueScout is useful as strategic critique, but it is not formal
  review evidence.
- Human discussion and `$close` need to be an interrupt boundary, not something
  Ralph fights.

The review-loop model is much better now than it was before. The parent should
orchestrate, aggregate, route repair/replan, and record durable outcomes. The
parent should not self-review. Reviewer agents should only persist bounded
findings or `NONE`. That is the right shape.

## What Went Badly

The loop was too noisy.

We had stale reviewer agents reporting after their boundary was superseded. We
had reviewer lanes trying, or appearing to try, parent-owned writeback. We had
late reviewer-output artifacts appear after mission-close had moved on. We had
authority tokens get lost across resumed parent turns. We had to move stale
lease files aside manually. We had bundles become stale because receipts changed
the package fingerprint, and then the package id changed the receipt, creating a
self-invalidating loop.

Those are not small UX papercuts. They are symptoms of a control-plane design
that is still too easy to perturb.

The worst part is that the user experience became confusing: the system could
say "complete," then stale lanes could still shout about old or newly persisted
findings. Even if durable repo truth is ultimately the source of truth, the
experience feels untrustworthy when old agents continue talking after their
authority window has closed.

My blunt take: the machine is contract-rich now, but the live orchestration
experience still feels like operating a prototype. The contracts are catching
real problems, but the ergonomics make the user feel like they are debugging the
machine instead of using it.

## Ralph Stop Loop Verdict

I agree with the Ralph stop loop as a product principle.

I do not fully agree with the current Ralph stop loop UX.

The principle is correct: Codex should not be allowed to casually stop, drift,
or declare success when mission truth says the next required action is known.
The stop hook should protect the user from false completion. It did that. Many
times.

But Ralph needs clearer mode boundaries:

- If the user is talking to Codex, that should be discussion mode, not forced
  execution mode.
- If a child agent is a reviewer, it should be Ralph-exempt and able to stop
  normally after persisting bounded output.
- If the parent loop is active, only the parent should receive stop pressure.
- If `$close` pauses the parent loop, writeback and integration should pause,
  but child lanes may finish bounded work.
- If a review bundle is superseded, old child lanes should be closed or their
  later outputs should be marked stale automatically.

Ralph currently has the right moral compass and a clumsy body.

It is catching real dishonesty, but sometimes it blocks the wrong actor or at
the wrong moment. The next big improvement should be making Ralph lease/mode
semantics feel obvious, recoverable, and boring.

## Before vs Now

Before this run, Codex1 had the general workflow shape:

1. Clarify the mission.
2. Plan it.
3. Execute slices.
4. Review.
5. Continue with Ralph until done.

But too much of that was enforced through skill prose, parent judgment, and
convention. The system was powerful, but ambiguity leaked through all the
places where it mattered most: review writeback, mission close, close/resume,
autopilot plan sealing, advisor use, and child lane authority.

Now the intended product model is much clearer:

1. `$clarify` locks the user's intended outcome.
2. `$plan` creates or updates the mission blueprint, specs, proof strategy, and
   execution route.
3. `$execute` advances a passed execution package and routes to `$review-loop`
   when proof-worthy review is owed.
4. `$review-loop` is parent-owned orchestration, not reviewer-local behavior.
5. Reviewer agents persist `reviewer-output` only.
6. Parent review writeback requires parent loop authority and a review truth
   snapshot.
7. `$close` pauses parent continuation so the user can talk or redirect.
8. Advisor/CritiqueScout can be invoked by the parent at strategic checkpoints,
   but advisor output is not formal review.
9. `$autopilot` should compose clarify, plan, execute, review, repair, and
   close loops without requiring the user to call each skill manually.
10. Mission completion requires clean mission-close review, not just clean
    execution.

The conceptual upgrade is big. Codex1 now has a stronger answer to "who is
allowed to do what?" That is probably the most important product shift from the
run.

## What Changed At A High Level

The authority model became more explicit.

Before, parent threads, reviewer lanes, advisor lanes, and support subagents
could blur together. Now they are supposed to be distinct modes with distinct
capabilities. Parent owns mission truth. Reviewers own findings. Advisors own
advice. Writers own bounded implementation. The user owns interruption and
direction.

The review model became more honest.

Before, review could drift into parent-local judgment or self-clearing review
lanes. Now review-loop is explicitly parent orchestration over durable reviewer
outputs. Clean review requires required lane coverage and no P0/P1/P2 findings.
P3 can be recorded without blocking.

Mission close became a real boundary.

Before, final execution could too easily feel like completion. Now the system
distinguishes execution proof, mission-close readiness, and actual terminal
mission-close review.

Autopilot became more constrained.

Before, autopilot could conceptually move from planning to execution too
loosely. Now it needs stronger plan-seal evidence, autonomy checks, package
freshness, advisor checkpoint handling, and proof/review routing.

Close/resume became more product-relevant.

Before, pausing was mostly operational. Now `$close` is part of the UX contract:
the user must be able to interrupt and talk without Ralph forcing unrelated
execution.

## My Strongest Criticism

The system still has too many ways for "truth" and "workflow control" to drift
apart.

A few examples from the run:

- A receipt edit changed the spec fingerprint, which staled the package and
  review bundle.
- A hardcoded package id in a receipt created a self-invalidating package loop.
- Parent authority tokens were transient and easy to lose across resumed turns.
- A review truth snapshot could exist without the writeback token needed to use
  it.
- Stale child agents continued reporting after their bundle was superseded.
- Late reviewer outputs appeared after a gate had already been marked passed.
- The stop hook sometimes addressed the current parent as if it were still in
  the prior loop mode.

These are not just bugs. They suggest Codex1 needs a more explicit runtime
model for:

- active parent loop
- child lane registry
- live vs stale review wave
- writeback token lifecycle
- discussion/pause mode
- terminalization lock
- late artifact quarantine

I would treat that as a product architecture problem, not just test cleanup.

## My Strongest Praise

The system is now hard to fool.

That matters. A normal agentic workflow would likely have declared success much
earlier. Codex1 kept asking, in machine-readable ways:

- Is the package fresh?
- Is the review bundle fresh?
- Did required reviewer lanes actually write evidence?
- Did the child mutate mission truth?
- Did the parent try to self-review?
- Did the receipt prove the exact claim?
- Did mission close actually happen?

That is the right culture for a tool whose north star is "continue until done."
The work is not complete when the model feels satisfied. It is complete when
the mission contract, proof, review, and closeout all agree.

## What I Would Fix Next

First, I would harden subagent lifecycle.

When a bundle is superseded, all child lanes tied to that bundle should either
be closed automatically or their later outputs should be quarantined as stale.
They should not continue confusing the parent thread. A child lane should not
be able to write fresh-looking evidence for a dead review boundary.

Second, I would improve parent loop lease recovery.

The user and parent should not need to move `.ralph/loop-lease.json` around
manually. There should be a safe parent-owned recovery command or `$close` /
`$resume` pathway that handles orphaned verifier-backed leases.

Third, I would make discussion mode first-class.

When the user talks, Ralph should recognize that this is not necessarily a
request to continue the active loop. The parent should be able to answer,
clarify, explain, and decide whether to resume.

Fourth, I would separate readiness review from terminal mission-close review
more cleanly.

The system did eventually get there, but several findings came from proof rows
that sounded like "mission-close review is clean" before that final review had
actually happened. The vocabulary should make false completion harder:

- "ready for mission-close review"
- "mission-close review open"
- "mission-close review passed"
- "terminal complete"

Fifth, I would add late-output quarantine.

If reviewer-output is recorded after parent writeback or after bundle
supersession, it should be classified explicitly:

- accepted before writeback
- late but same active wave
- stale due to supersession
- contaminated after terminalization

That one feature would remove a lot of anxiety.

## What I Would Not Do

I would not remove Ralph.

I would not make review optional.

I would not let parent-local judgment replace reviewer outputs.

I would not simplify by going back to "the model just thinks it is done."

The hard parts of this run were painful because they were protecting the core
promise. The answer is not fewer contracts. The answer is better-centered
contracts with less noisy operation.

## Final Honest Opinion

This run felt like watching Codex1 grow a spine before it grew graceful
movement.

It now resists false success much better. It has a clearer product model. It
has better review authority boundaries. It has a more honest mission-close
story. It is closer to the kind of tool that can run for a long time without
lying to the user.

But it is not yet smooth. It is still too easy to get into situations where the
human has to understand leases, stale gates, reviewer outputs, and package
fingerprints. The product should eventually hide most of that behind clear skill
UX and deterministic recovery.

My verdict: successful architecture run, rough product run, excellent evidence
for what needs to be hardened next.

