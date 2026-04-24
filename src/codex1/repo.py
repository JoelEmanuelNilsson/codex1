from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


STATE_SCHEMA_VERSION = "codex1.state.v1"


@dataclass(frozen=True, slots=True)
class MissionSelection:
    repo_root: Path
    mission_id: str | None
    mission_root: Path | None
    source: str
    warning: str | None = None


def resolve_repo_root(repo_root_arg: str | None, cwd: Path | None = None) -> Path:
    if repo_root_arg:
        return Path(repo_root_arg).expanduser().resolve()
    return _discover_repo_root(cwd or Path.cwd())


def resolve_mission(repo_root: Path, mission_id: str | None, cwd: Path | None = None) -> MissionSelection:
    plans_root = repo_root / "PLANS"
    if mission_id:
        invalid_reason = _invalid_mission_id_reason(mission_id)
        if invalid_reason:
            return MissionSelection(repo_root, mission_id, None, "argument", invalid_reason)
        mission_root = (plans_root / mission_id).resolve()
        if not _is_relative_to(mission_root, plans_root.resolve()):
            return MissionSelection(
                repo_root,
                mission_id,
                None,
                "argument",
                f"Mission id '{mission_id}' resolves outside {plans_root}.",
            )
        if mission_root.is_dir():
            return MissionSelection(repo_root, mission_id, mission_root, "argument")
        return MissionSelection(
            repo_root,
            mission_id,
            mission_root,
            "argument",
            warning=f"Mission '{mission_id}' does not exist under {plans_root}.",
        )

    cwd = (cwd or Path.cwd()).resolve()
    try:
        relative = cwd.relative_to(plans_root.resolve())
    except ValueError:
        relative = None
    if relative and relative.parts:
        candidate_id = relative.parts[0]
        candidate_root = plans_root / candidate_id
        if candidate_root.is_dir():
            return MissionSelection(repo_root, candidate_id, candidate_root.resolve(), "cwd")

    active_path = plans_root / "ACTIVE.json"
    if active_path.exists():
        try:
            active = json.loads(active_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            return MissionSelection(repo_root, None, None, "none", f"Ignoring invalid ACTIVE.json: {exc}.")
        if not isinstance(active, dict):
            return MissionSelection(repo_root, None, None, "none", "Ignoring ACTIVE.json with unsupported shape.")
        active_mission = active.get("mission_id")
        if active.get("schema_version") != "codex1.active.v1" or not isinstance(active_mission, str):
            return MissionSelection(repo_root, None, None, "none", "Ignoring ACTIVE.json with unsupported shape.")
        invalid_reason = _invalid_mission_id_reason(active_mission)
        if invalid_reason:
            return MissionSelection(repo_root, None, None, "none", f"Ignoring ACTIVE.json: {invalid_reason}")
        active_root = (plans_root / active_mission).resolve()
        if not _is_relative_to(active_root, plans_root.resolve()):
            return MissionSelection(
                repo_root,
                None,
                None,
                "none",
                f"Ignoring ACTIVE.json: mission id '{active_mission}' resolves outside {plans_root}.",
            )
        if active_root.is_dir():
            return MissionSelection(repo_root, active_mission, active_root, "active_pointer")
        return MissionSelection(repo_root, None, None, "none", f"Ignoring stale ACTIVE.json for '{active_mission}'.")

    return MissionSelection(repo_root, None, None, "none")


def _invalid_mission_id_reason(mission_id: str) -> str | None:
    mission_path = Path(mission_id)
    if mission_path.is_absolute():
        return f"Mission id '{mission_id}' must be a relative id under PLANS/."
    if not mission_id or any(part in {"", ".", ".."} for part in mission_path.parts):
        return f"Mission id '{mission_id}' must not contain empty, '.', or '..' path segments."
    return None


def _is_relative_to(path: Path, parent: Path) -> bool:
    try:
        path.relative_to(parent)
    except ValueError:
        return False
    return True


def load_state(mission_root: Path, expected_mission_id: str | None = None) -> tuple[dict[str, Any] | None, str | None]:
    state_path = mission_root / "STATE.json"
    if not state_path.exists():
        return None, f"Missing state file: {state_path}."
    try:
        state = json.loads(state_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return None, f"Could not parse STATE.json: {exc}."
    if not isinstance(state, dict):
        return None, "Unsupported STATE.json shape: expected a JSON object."
    if state.get("schema_version") != STATE_SCHEMA_VERSION:
        return None, f"Unsupported STATE.json schema_version: {state.get('schema_version')!r}."
    if expected_mission_id is not None and state.get("mission_id") != expected_mission_id:
        return (
            None,
            f"STATE.json mission_id {state.get('mission_id')!r} does not match selected mission {expected_mission_id!r}.",
        )
    return state, None


def _discover_repo_root(cwd: Path) -> Path:
    current = cwd.resolve()
    for candidate in (current, *current.parents):
        if candidate.name == "PLANS":
            return candidate.parent.resolve()
        if (candidate / "docs" / "codex1-rebuild-handoff").is_dir() or (candidate / ".git").is_dir():
            return candidate.resolve()
    return current
