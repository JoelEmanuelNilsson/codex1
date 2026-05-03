# Codex1 Domain Docs

Use the repo's domain language before inventing vocabulary.

## Before Exploring

Read these when present:

- `CONTEXT.md` at repo root, or `CONTEXT-MAP.md` for multi-context repos.
- Repo ADRs in `docs/adr/`.
- Mission ADRs in `.codex1/missions/<id>/ADRS/`.
- Relevant specs and existing mission artifacts.

If the files do not exist, proceed silently. Do not suggest creating them upfront. Producer workflows create them lazily when terms or decisions actually crystallize.

## Glossary Rules

When `$clarify` resolves a domain term, update the relevant `CONTEXT.md` inline:

- Pick a canonical term and list aliases to avoid.
- Keep definitions tight.
- Show relationships between terms.
- Flag ambiguities explicitly.
- Include an example dialogue when it clarifies boundaries.
- Exclude generic programming terms.

If `CONTEXT-MAP.md` exists, infer the relevant context. Ask only if the context is unclear and the answer affects the mission.

## ADR Rules

Offer or write an ADR only when all three are true:

1. Hard to reverse: changing later would be meaningfully costly.
2. Surprising without context: a future reader would wonder why.
3. Real trade-off: plausible alternatives existed and one was chosen for a reason.

Repo-wide or long-lived architecture decisions belong in `docs/adr/`. Mission-specific execution decisions belong in `.codex1/missions/<id>/ADRS/`.

Keep ADRs lightweight by default: title plus one paragraph explaining context, decision, and why. Add status, options, tradeoffs, consequences, and artifact links only when they add real value.
