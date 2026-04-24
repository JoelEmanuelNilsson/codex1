# Codex1

This repository is being rebuilt from the canonical handoff in
`docs/codex1-rebuild-handoff/`.

The current implementation slice is intentionally small:

- `codex1 --help`
- `codex1 status --json`
- `codex1 doctor --json`
- `codex1 ralph stop-hook --json`
- foundation command-surface stubs for outcome/plan/task/loop/close
- stable JSON error output

Install the command locally with:

```bash
python3 -m pip install -e .
```

Run tests with:

```bash
python3 -m unittest discover -s tests
```
