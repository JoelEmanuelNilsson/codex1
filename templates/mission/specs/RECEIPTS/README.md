# Receipts

- Mission id: `{{MISSION_ID}}`
- Spec id: `{{SPEC_ID}}`

Store proof artifacts for this spec here.

## Suggested Contents

- test output captures
- screenshots or logs that prove changed behavior
- interface verification notes
- check summaries referenced by review

## Naming Guidance

Use stable names that make the proof row obvious, for example:

- `proof-row-p1-tests.md`
- `proof-row-p2-api-compat.md`
- `review-bundle-{{SPEC_ID}}-{{REVIEW_BUNDLE_ID}}.md`

Include the governing bundle, package, revision, or fingerprint context inside
the receipt body whenever that context is required to prove freshness.
