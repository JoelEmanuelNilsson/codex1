# CONTEXT.md Format

Use this when `$clarify` resolves project language.

## Structure

```md
# {Context Name}

{One or two sentence description of what this context is and why it exists.}

## Language

**Order**:
{A concise description of the term}
_Avoid_: Purchase, transaction

**Invoice**:
A request for payment sent to a customer after delivery.
_Avoid_: Bill, payment request

## Relationships

- An **Order** produces one or more **Invoices**
- An **Invoice** belongs to exactly one **Customer**

## Example dialogue

> **Dev:** "When a **Customer** places an **Order**, do we create the **Invoice** immediately?"
> **Domain expert:** "No. An **Invoice** is generated once **Fulfillment** is confirmed."

## Flagged ambiguities

- "account" was used to mean both **Customer** and **User**. Resolved: these are distinct concepts.
```

## Rules

- Be opinionated. Pick one canonical term and list aliases to avoid.
- Flag conflicts explicitly.
- Keep definitions tight: one sentence, defining what the term is.
- Show relationships and cardinality where obvious.
- Include only domain terms, not generic programming concepts.
- Group terms under headings when natural clusters emerge.
- Write example dialogue when it clarifies boundaries.

## Single vs Multi-context Repos

Single context: one root `CONTEXT.md`.

Multi-context: root `CONTEXT-MAP.md` points to per-context `CONTEXT.md` files. If `CONTEXT-MAP.md` exists, read it and update the relevant context. If unclear, ask only when the context changes the mission.

Create context files lazily. If no `CONTEXT.md` exists, create one only when the first term is resolved.
