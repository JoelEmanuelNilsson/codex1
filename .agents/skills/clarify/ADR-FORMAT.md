# ADR Format

Use this when `$clarify` or `$plan` records an architecture decision.

## Location

- Repo-wide or long-lived decisions: `docs/adr/`
- Mission-specific execution decisions: `.codex1/missions/<id>/ADRS/`

Create the directory lazily only when the first ADR is needed.

## Template

```md
# {Short title of the decision}

{1-3 sentences: what context led to the decision, what was decided, and why.}
```

That can be the whole ADR. The value is recording that a decision was made and why, not filling out ceremony.

## Optional Sections

Only include these when they add genuine value:

- Status: `proposed`, `accepted`, `deprecated`, or `superseded by ADR-NNNN`
- Considered Options: rejected alternatives worth remembering
- Tradeoffs: real costs of the chosen direction
- Consequences: non-obvious effects future agents must know
- Links: related PRD, PLAN, SPECS, or subplans

## Numbering

For `docs/adr/`, use sequential names like `0001-slug.md`. Scan for the highest existing number and increment by one.

For mission `ADRS/`, use the mission's normal Codex1 artifact creation flow unless the repo has a stronger local convention.

## When To Offer Or Write An ADR

All three must be true:

1. Hard to reverse: changing later would be meaningfully costly.
2. Surprising without context: a future reader would wonder why.
3. Real trade-off: plausible alternatives existed and one was chosen for a reason.

Skip ADRs for easy-to-reverse choices, obvious implementation details, and decisions with no real alternative.

## What Qualifies

- Architectural shape, such as the main data-flow or module shape.
- Integration patterns between contexts, such as events instead of synchronous calls.
- Technology choices with lock-in, such as database, message bus, auth provider, or deployment target. Not every library deserves an ADR.
- Ownership and state boundaries. Explicit no's can be as useful as yes's.
- Deliberate deviations from the obvious path, so future agents do not "fix" something intentional.
- Durable constraints not visible in code, such as compliance, latency, or partner API constraints.
- Non-obvious rejected alternatives that future agents would otherwise suggest again.
