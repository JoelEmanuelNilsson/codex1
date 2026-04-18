# Spec Notes

- Mission id: `contract-centered-architecture`
- Spec id: `qualification_evidence_pipeline`

Use this file for bounded local notes, spike observations, or drafting scratch
that supports the spec but does not override it.

## Active Notes

- Qualification reports now persist per-gate evidence payloads and surface them through `gate.evidence_path`.
- The qualification support-surface proof row is now exercised by a real `qualification_cli` test instead of filtering out all tests.
- The native child-lane gate now derives the decisive `spawn_agent`/`list_agents`/`wait`/`close_agent` evidence from raw JSONL events instead of trusting the final model-authored summary.
- The live native gate now stores full raw stdout and stderr in persisted gate evidence so `evidence_path` remains inspectable even when the JSONL output is large.

## Caution

If a note changes the actual contract, move that change into `SPEC.md` or the
appropriate higher-layer artifact instead of letting this file become hidden
truth.
