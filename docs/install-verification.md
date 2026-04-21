# Install and verification

This recipe installs `codex1` to `$HOME/.local/bin/` and verifies it from `/tmp`. The "verify from outside the source folder" step is the critical one — a CLI that only works via `cargo run` is not an installed CLI.

## Toolchain

- Rust `>= 1.89` (install via `rustup` if missing: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`).
- `git`, `make`, and a POSIX shell.

## Build

```bash
git clone https://github.com/JoelEmanuelNilsson/codex1 && cd codex1
make install-local
```

`make install-local` runs `cargo build --release` and copies `target/release/codex1` to `$HOME/.local/bin/codex1`. To install elsewhere, pass `INSTALL_DIR=<path> make install-local`; use the same `INSTALL_DIR=<path>` when you run `make verify-installed` or `make verify-contract`.

## PATH

```bash
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR" \
  || echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> ~/.zshrc
exec "$SHELL"
```

If you use bash, replace `~/.zshrc` with `~/.bashrc` (or `~/.bash_profile` on macOS).

## Verify from /tmp (the critical step)

```bash
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
cd /tmp
PATH="$INSTALL_DIR:$PATH"
command -v codex1                           # must print $INSTALL_DIR/codex1
codex1 --help                               # must show the full command tree
codex1 --json doctor                        # must print {"ok":true,...} without errors
codex1 --json init --mission demo           # creates PLANS/demo/
cat PLANS/demo/STATE.json                   # valid JSON, "revision":0, "phase":"clarify"
codex1 --json status --mission demo         # {"ok":true,"data":{"verdict":"needs_user", …}}
```

Every one of these must succeed before you move on. If `codex1 status` prints an error envelope instead of a JSON success, stop and investigate — the Ralph stop hook relies on this command returning stable JSON.

## Contract tests

```bash
make verify-contract      # fmt + clippy + cargo test + install-local + smoke (verify-installed)
# custom install location
INSTALL_DIR=/absolute/custom/bin make install-local verify-installed
```

The Makefile target chains `fmt clippy test install-local verify-installed`; `verify-installed` does the installed-binary smoke check.

## Skill auto-discovery

Codex1 ships six public skills under `.codex/skills/{clarify,plan,execute,review-loop,close,autopilot}`. They are auto-discovered by Codex when the repo is on the current working path. You can also symlink a shared copy from `~/.codex/skills/` if you maintain them outside this repo:

```bash
ln -s /path/to/codex1/.codex/skills/clarify ~/.codex/skills/clarify
# repeat per skill
```

See the handoff package at [`codex1-rebuild-handoff/01-product-flow.md`](codex1-rebuild-handoff/01-product-flow.md) for the workflow each skill drives.

## Ralph stop hook

The Ralph Stop hook is a short shell script that runs `codex1 status --json`, blocks when the resolved mission says `stop.allow == false`, and fails closed on mission-resolution/config errors such as ambiguous bare multi-mission discovery or explicit bad selectors. Wire it via your Codex `hooks.json` as described by:

```bash
codex1 --json hook snippet
```

The shell script itself and its install README live at [`../scripts/README-hook.md`](../scripts/README-hook.md).

## Troubleshooting

- `codex1: command not found` — `$HOME/.local/bin` is not on `PATH`; add it and `exec $SHELL`, or re-run `make install-local`.
- `MISSION_NOT_FOUND` — either pass `--mission <id>` explicitly or `cd` into a directory whose `PLANS/` subdirectory contains exactly one mission.
- `REVISION_CONFLICT` — another writer advanced `STATE.json` while you were staging a mutation. Re-read it (`cat PLANS/<id>/STATE.json`) and retry with the correct `--expect-revision`. The error envelope's `context.actual` field carries the current revision.
- `doctor` warns about a network filesystem — prefer local disk for mission state; `fs2` advisory locks behave unevenly over NFS/SMB.

Per-command shapes, envelopes, and error codes live in [`cli-reference.md`](cli-reference.md) and [`cli-contract-schemas.md`](cli-contract-schemas.md).
