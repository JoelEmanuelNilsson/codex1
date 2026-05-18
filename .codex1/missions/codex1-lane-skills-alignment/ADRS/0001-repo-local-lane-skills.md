# Repo-local lane skills

Codex1 plans will assign execution lanes such as TDD, diagnosis, architecture, prototype, proof/QA, and standard work. Codex1 setup will install full repo-local lane skill files rather than references to global skills, because a repo should remain understandable and executable even when global skills are removed, renamed, or changed.

Status: accepted

Considered Options:

- Keep lane skills global and only mention them from Codex1 plans.
- Install tiny proxy skills that point to global skill paths.
- Install full repo-local lane skills as managed setup files.

Tradeoffs:

- Full local copies make setup larger and require bundle-version tests.
- Local copies are more reliable for agentic execution and match Codex1's local-first model.

Consequences:

- `codex1 setup install` must update the managed bundle marker when lane skills are added.
- Lane skills must preserve original behavior closely, with only small Codex1 wrappers where needed.
- Deletion remains explicit through uninstall or future prune behavior.

Links:

- `PRD.md`
- `PLAN.md`
- `SPECS/0001-codex1-lane-setup-contract.md`
