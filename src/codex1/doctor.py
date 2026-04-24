from __future__ import annotations

import importlib.metadata
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from .repo import load_state, resolve_mission, resolve_repo_root


DOCTOR_SCHEMA_VERSION = "codex1.doctor.v1"
REQUIRED_HANDOFF_DOCS = [
    "README.md",
    "02-cli-contract.md",
    "03-planning-artifacts.md",
    "08-state-status-and-graph-contract.md",
    "09-implementation-errata.md",
    "10-first-slice-skill-contracts.md",
    "11-rebuild-spec-and-plan.md",
]


def doctor_for(repo_root_arg: str | None = None, mission_id: str | None = None) -> dict[str, Any]:
    repo_root = resolve_repo_root(repo_root_arg)
    checks = [
        _python_runtime_check(),
        _package_metadata_check(),
        _command_on_path_check(),
        _help_available_check(),
        _docs_presence_check(repo_root),
        _official_codex_source_check(),
        _config_parser_placeholder_check(),
        _model_policy_placeholder_check(),
        _state_event_drift_check(repo_root, mission_id),
    ]
    ok = all(check.get("ok") is not False or check.get("severity") in {"warning", "info"} for check in checks)
    return {
        "ok": ok,
        "schema_version": DOCTOR_SCHEMA_VERSION,
        "repo_root": str(repo_root),
        "checks": checks,
    }


def _check(check_id: str, ok: bool, severity: str = "error", **fields: Any) -> dict[str, Any]:
    status = "pass" if ok else ("warn" if severity == "warning" else "info" if severity == "info" else "fail")
    return {"id": check_id, "ok": ok, "status": status, "severity": severity, **fields}


def _python_runtime_check() -> dict[str, Any]:
    return _check(
        "python_runtime",
        sys.version_info >= (3, 11),
        version=".".join(str(part) for part in sys.version_info[:3]),
        required=">=3.11",
    )


def _package_metadata_check() -> dict[str, Any]:
    try:
        version = importlib.metadata.version("codex1")
    except importlib.metadata.PackageNotFoundError:
        return _check(
            "package_metadata",
            False,
            severity="warning",
            message="codex1 package metadata is not installed; run python3 -m pip install -e . for the console script.",
        )
    return _check("package_metadata", True, version=version)


def _command_on_path_check() -> dict[str, Any]:
    command = shutil.which("codex1")
    if command:
        try:
            with tempfile.TemporaryDirectory() as temp_dir:
                result = subprocess.run(
                    [command, "--help"],
                    cwd=temp_dir,
                    env=_outside_source_env(),
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=2,
                    check=False,
                )
        except (OSError, subprocess.TimeoutExpired) as exc:
            return _check(
                "command_available_on_path",
                False,
                severity="warning",
                path=command,
                message=f"codex1 was found on PATH, but --help could not run cleanly: {exc}.",
            )
        looks_current = result.returncode == 0 and "Deterministic CLI substrate" in result.stdout and "status" in result.stdout
        return _check(
            "command_available_on_path",
            looks_current,
            severity="warning" if not looks_current else "error",
            path=command,
            message=None if looks_current else "codex1 is on PATH, but its --help output does not match this foundation CLI.",
        )
    return _check(
        "command_available_on_path",
        False,
        severity="warning",
        message="codex1 is not on PATH in this process. Install with python3 -m pip install -e .",
    )


def _outside_source_env() -> dict[str, str]:
    env = os.environ.copy()
    source_path = Path(__file__).resolve().parents[1]
    pythonpath = env.get("PYTHONPATH")
    if pythonpath:
        kept = []
        for entry in pythonpath.split(os.pathsep):
            if not entry:
                continue
            try:
                if Path(entry).resolve() == source_path:
                    continue
            except OSError:
                pass
            kept.append(entry)
        if kept:
            env["PYTHONPATH"] = os.pathsep.join(kept)
        else:
            env.pop("PYTHONPATH", None)
    return env


def _help_available_check() -> dict[str, Any]:
    from .cli import build_parser

    root_help = build_parser().format_help()
    status_help = build_parser()._subparsers._group_actions[0].choices["status"].format_help()  # type: ignore[attr-defined]
    doctor_help = build_parser()._subparsers._group_actions[0].choices["doctor"].format_help()  # type: ignore[attr-defined]
    useful = all(term in root_help for term in ["status", "doctor"]) and "--json" in status_help and "--json" in doctor_help
    return _check("help_available", useful)


def _docs_presence_check(repo_root: Path) -> dict[str, Any]:
    docs_root = repo_root / "docs" / "codex1-rebuild-handoff"
    missing = [name for name in REQUIRED_HANDOFF_DOCS if not (docs_root / name).is_file()]
    return _check("handoff_docs_present", not missing, docs_root=str(docs_root), missing=missing)


def _official_codex_source_check() -> dict[str, Any]:
    source = Path("/Users/joel/.codex/.codex-official-repo")
    return _check(
        "official_codex_source_available",
        source.is_dir(),
        severity="warning",
        path=str(source),
        message=None if source.is_dir() else "Local official Codex source of truth is not present.",
    )


def _config_parser_placeholder_check() -> dict[str, Any]:
    return _check(
        "codex_hook_config_parser_integration",
        False,
        severity="info",
        message="Not implemented in this foundation skeleton; no Codex TOML parser was invoked.",
    )


def _model_policy_placeholder_check() -> dict[str, Any]:
    return _check(
        "model_policy_available",
        False,
        severity="info",
        models=["gpt-5.5", "gpt-5.4-mini"],
        message="Not checked in this foundation skeleton; no deployment model catalog API was invoked.",
    )


def _state_event_drift_check(repo_root: Path, mission_id: str | None) -> dict[str, Any]:
    selection = resolve_mission(repo_root, mission_id)
    if selection.mission_root is None:
        return _check(
            "state_event_revision_drift",
            True,
            severity="info",
            status="skipped",
            message="No durable mission is selected; no STATE.json/EVENTS.jsonl drift to inspect.",
        )
    state_path = selection.mission_root / "STATE.json"
    events_path = selection.mission_root / "EVENTS.jsonl"
    if not state_path.exists():
        return _check(
            "state_event_revision_drift",
            False,
            severity="warning",
            message=f"Mission exists but STATE.json is missing: {state_path}.",
        )

    import json

    state, state_warning = load_state(selection.mission_root, expected_mission_id=selection.mission_id)
    if state_warning or state is None:
        return _check("state_event_revision_drift", False, severity="warning", message=state_warning)
    state_revision = state.get("revision")
    if not isinstance(state_revision, int):
        return _check(
            "state_event_revision_drift",
            False,
            severity="warning",
            message=f"Unsupported STATE.json revision: {state_revision!r}.",
            state_revision=state_revision,
            latest_event_revision=None,
        )
    latest_event_revision = None
    malformed_events = 0
    if events_path.exists():
        try:
            event_lines = events_path.read_text(encoding="utf-8").splitlines()
        except OSError as exc:
            return _check(
                "state_event_revision_drift",
                False,
                severity="warning",
                state_revision=state_revision,
                latest_event_revision=None,
                message=f"Cannot read EVENTS.jsonl: {exc}.",
            )
        for line in event_lines:
            if not line.strip():
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                malformed_events += 1
                continue
            if not isinstance(event, dict):
                malformed_events += 1
                continue
            revision_after = event.get("revision_after")
            if not isinstance(revision_after, int):
                malformed_events += 1
                continue
            latest_event_revision = revision_after
    ok = latest_event_revision == state_revision
    if state_revision == 0 and latest_event_revision is None and malformed_events == 0:
        ok = True
    if malformed_events:
        ok = False
    messages = []
    if malformed_events:
        messages.append(f"EVENTS.jsonl contains {malformed_events} malformed event row(s).")
    if not ok:
        if latest_event_revision != state_revision:
            messages.append(f"STATE.json is authoritative, but EVENTS.jsonl is missing audit for revision {state_revision}.")
    message = " ".join(messages) if messages else None
    return _check(
        "state_event_revision_drift",
        ok,
        severity="warning" if not ok else "info",
        state_revision=state_revision,
        latest_event_revision=latest_event_revision,
        malformed_event_rows=malformed_events,
        message=message,
    )
