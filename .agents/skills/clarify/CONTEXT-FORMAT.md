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

**Customer**:
A person or organization that places orders.
_Avoid_: Client, buyer, account

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
- Flag conflicts explicitly with the resolution, not just the conflict.
- Keep definitions tight: one sentence, defining what the term is.
- Show relationships and cardinality where obvious.
- Include only domain terms, not generic programming concepts. Before adding a term, ask whether a domain expert would need it to describe the product. If it is a code pattern, library, helper, error type, timeout, or utility concern, skip it.
- Group terms under headings when natural clusters emerge.
- Write example dialogue when it clarifies boundaries.

## Single vs Multi-context Repos

Single context: one root `CONTEXT.md`.

Multi-context: root `CONTEXT-MAP.md` points to per-context `CONTEXT.md` files and explains how the contexts relate.

```md
# Context Map

## Contexts

- [Ordering](./src/ordering/CONTEXT.md) - receives and tracks orders
- [Billing](./src/billing/CONTEXT.md) - generates invoices and processes payments

## Relationships

- **Ordering -> Billing**: Ordering emits order events; Billing consumes them to create invoices
```

If `CONTEXT-MAP.md` exists, read it before choosing where to write. If the relevant context is unclear and the answer changes the mission, ask. If no context files exist, create a root `CONTEXT.md` lazily when the first term is resolved.
