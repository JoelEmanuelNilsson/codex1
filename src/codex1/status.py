from __future__ import annotations

from pathlib import Path
from typing import Any

from .repo import MissionSelection, load_state, resolve_mission, resolve_repo_root


STATUS_SCHEMA_VERSION = "codex1.status.v1"
ALLOWED_LOOP_MODES = {"none", "execute", "autopilot", "review_loop"}


def no_mission_status(repo_root: Path, warning: str | None = None) -> dict[str, Any]:
    payload: dict[str, Any] = {
        "ok": True,
        "schema_version": STATUS_SCHEMA_VERSION,
        "mission_id": None,
        "mission_root": None,
        "repo_root": str(repo_root),
        "state_revision": None,
        "outcome_digest": None,
        "plan_digest": None,
        "planning_mode": "unset",
        "phase": "inactive",
        "verdict": "inactive",
        "loop": {"active": False, "paused": False, "mode": "none"},
        "next_action": {"kind": "none", "owner": "codex", "required": False, "autonomous": False},
        "ready_steps": [],
        "ready_tasks": [],
        "ready_wave": None,
        "reviews": {"pending_boundaries": [], "accepted_blocking_count": 0},
        "replan": {"required": False, "reason": None},
        "close": {"ready": False, "required": False, "requires_mission_close_review": False},
        "stop": {
            "allow": True,
            "reason": "no_active_mission",
            "mode": "open",
            "message": "No active Codex1 mission is selected; stopping is allowed.",
        },
    }
    if warning:
        payload["warnings"] = [warning]
    return payload


def invalid_state_status(selection: MissionSelection, warning: str) -> dict[str, Any]:
    return {
        "ok": True,
        "schema_version": STATUS_SCHEMA_VERSION,
        "mission_id": selection.mission_id,
        "mission_root": str(selection.mission_root) if selection.mission_root else None,
        "repo_root": str(selection.repo_root),
        "state_revision": None,
        "outcome_digest": None,
        "plan_digest": None,
        "planning_mode": "unset",
        "phase": "invalid_state",
        "verdict": "invalid_state",
        "loop": {"active": False, "paused": False, "mode": "none"},
        "next_action": {
            "kind": "explain_and_stop",
            "owner": "codex",
            "required": False,
            "autonomous": False,
            "reason": "invalid_state",
            "message": warning,
        },
        "ready_steps": [],
        "ready_tasks": [],
        "ready_wave": None,
        "reviews": {"pending_boundaries": [], "accepted_blocking_count": 0},
        "replan": {"required": False, "reason": None},
        "close": {"ready": False, "required": False, "requires_mission_close_review": False},
        "stop": {
            "allow": True,
            "reason": "invalid_state_fail_open",
            "mode": "open",
            "message": f"Codex1 could not safely project mission state: {warning}",
        },
        "warnings": [warning],
    }


def project_state_status(selection: MissionSelection, state: dict[str, Any]) -> dict[str, Any]:
    loop = _loop_projection(state.get("loop"))
    terminal = state.get("terminal") if isinstance(state.get("terminal"), dict) else {}
    plan = state.get("plan") if isinstance(state.get("plan"), dict) else {}
    outcome = state.get("outcome") if isinstance(state.get("outcome"), dict) else {}
    close = state.get("close") if isinstance(state.get("close"), dict) else {}
    replan = _replan_projection(state.get("replan"))
    reviews = state.get("reviews") if isinstance(state.get("reviews"), dict) else {}
    close_ready = _close_ready(state, close, outcome, plan, replan, reviews)
    requires_close_review = close.get("requires_mission_close_review", False)

    if terminal.get("complete") is True:
        verdict = "complete"
        next_action = {"kind": "none", "owner": "codex", "required": False, "autonomous": False}
        stop = _allow_stop("complete", "Mission is terminally complete; stopping is allowed.")
    elif loop["paused"]:
        verdict = "paused"
        next_action = {"kind": "none", "owner": "codex", "required": False, "autonomous": False}
        stop = _allow_stop("paused_loop", "Codex1 loop is paused; stopping is allowed.")
    elif not loop["active"]:
        verdict = "inactive"
        next_action = {"kind": "none", "owner": "codex", "required": False, "autonomous": False}
        stop = _allow_stop("inactive_loop", "Codex1 loop is inactive; stopping is allowed.")
    elif replan["required"]:
        verdict = "replan_required"
        next_action = {
            "kind": "replan",
            "owner": "codex",
            "required": True,
            "autonomous": True,
            "reason": replan.get("reason"),
        }
        stop = _stop_for_next_action(
            loop,
            next_action,
            "block_replan_required",
            f"Codex1 says required work remains: replan is required ({replan.get('reason') or 'unspecified'}).",
        )
    elif close_ready:
        verdict = "close_required"
        next_action = {"kind": "close_complete", "owner": "codex", "required": True, "autonomous": True}
        stop = _stop_for_next_action(
            loop,
            next_action,
            "block_close_complete_required",
            "Codex1 says required work remains: close complete is ready.",
        )
    else:
        verdict = "continue_required"
        next_action = {
            "kind": "explain_and_stop",
            "owner": "codex",
            "required": False,
            "autonomous": False,
            "reason": "foundation_status_projection_incomplete",
            "message": "This foundation slice can see an active mission, but detailed task projection is not implemented yet.",
        }
        stop = _allow_stop(
            "no_autonomous_next_action",
            "Codex1 has no known autonomous next action in this foundation slice; stopping is allowed.",
        )

    return {
        "ok": True,
        "schema_version": STATUS_SCHEMA_VERSION,
        "mission_id": state.get("mission_id") or selection.mission_id,
        "mission_root": str(selection.mission_root) if selection.mission_root else None,
        "repo_root": str(selection.repo_root),
        "state_revision": state.get("revision"),
        "outcome_digest": outcome.get("outcome_digest"),
        "plan_digest": plan.get("plan_digest"),
        "planning_mode": state.get("planning_mode", "unset"),
        "phase": state.get("phase", verdict),
        "verdict": verdict,
        "loop": loop,
        "next_action": next_action,
        "ready_steps": [],
        "ready_tasks": [],
        "ready_wave": None,
        "reviews": _reviews_projection(reviews),
        "replan": replan,
        "close": {
            "ready": close_ready,
            "required": verdict == "close_required",
            "requires_mission_close_review": requires_close_review,
        },
        "stop": stop,
    }


def status_for(repo_root_arg: str | None = None, mission_id: str | None = None) -> dict[str, Any]:
    repo_root = resolve_repo_root(repo_root_arg)
    selection = resolve_mission(repo_root, mission_id)
    if selection.mission_root is None:
        return no_mission_status(repo_root, selection.warning)
    if selection.warning:
        return no_mission_status(repo_root, selection.warning)
    state, warning = load_state(selection.mission_root, expected_mission_id=selection.mission_id)
    if warning or state is None:
        return invalid_state_status(selection, warning or "Unknown state loading error.")
    projection_warning = _state_projection_warning(state)
    if projection_warning:
        return invalid_state_status(selection, projection_warning)
    return project_state_status(selection, state)


def _loop_projection(raw_loop: Any) -> dict[str, Any]:
    if not isinstance(raw_loop, dict):
        return {"active": False, "paused": False, "mode": "none"}
    mode = raw_loop.get("mode")
    if mode not in ALLOWED_LOOP_MODES:
        mode = "none"
    return {
        "active": raw_loop.get("active", False),
        "paused": raw_loop.get("paused", False),
        "mode": mode,
    }


def _replan_projection(raw_replan: Any) -> dict[str, Any]:
    if not isinstance(raw_replan, dict):
        return {"required": False, "reason": None}
    return {"required": raw_replan.get("required", False), "reason": raw_replan.get("reason")}


def _reviews_projection(reviews: dict[str, Any]) -> dict[str, Any]:
    pending = reviews.get("pending_boundaries", [])
    accepted = reviews.get("accepted_blocking_count", 0)
    return {"pending_boundaries": pending, "accepted_blocking_count": accepted}


def _close_ready(
    state: dict[str, Any],
    close: dict[str, Any],
    outcome: dict[str, Any],
    plan: dict[str, Any],
    replan: dict[str, Any],
    reviews: dict[str, Any],
) -> bool:
    if close.get("state") != "close_complete_ready":
        return False
    if outcome.get("ratified") is not True:
        return False
    if plan.get("locked") is not True:
        return False
    if replan.get("required") is True:
        return False
    if not _all_complete(state.get("steps")):
        return False
    if not _all_complete(state.get("tasks")):
        return False
    if reviews.get("accepted_blocking_count", 0) != 0:
        return False
    if reviews.get("pending_boundaries", []):
        return False
    return True


def _all_complete(raw_items: Any) -> bool:
    if raw_items is None:
        return True
    if isinstance(raw_items, dict):
        items = raw_items.values()
    elif isinstance(raw_items, list):
        items = raw_items
    else:
        return False
    for item in items:
        if not isinstance(item, dict):
            return False
        if item.get("status") != "complete":
            return False
    return True


def _state_projection_warning(state: dict[str, Any]) -> str | None:
    loop = state.get("loop")
    if loop is not None:
        if not isinstance(loop, dict):
            return "Unsupported STATE.json shape: loop must be a JSON object."
        for key in ("active", "paused"):
            if key in loop and not isinstance(loop[key], bool):
                return f"Unsupported STATE.json shape: loop.{key} must be a boolean."
        if "mode" in loop and loop["mode"] not in ALLOWED_LOOP_MODES:
            return f"Unsupported STATE.json shape: loop.mode has unsupported value {loop['mode']!r}."

    replan = state.get("replan")
    if replan is not None:
        if not isinstance(replan, dict):
            return "Unsupported STATE.json shape: replan must be a JSON object."
        if "required" in replan and not isinstance(replan["required"], bool):
            return "Unsupported STATE.json shape: replan.required must be a boolean."

    close = state.get("close")
    if close is not None:
        if not isinstance(close, dict):
            return "Unsupported STATE.json shape: close must be a JSON object."
        if "state" in close and not isinstance(close["state"], str):
            return "Unsupported STATE.json shape: close.state must be a string."
        if "requires_mission_close_review" in close and not isinstance(close["requires_mission_close_review"], bool):
            return "Unsupported STATE.json shape: close.requires_mission_close_review must be a boolean."

    terminal = state.get("terminal")
    if terminal is not None:
        if not isinstance(terminal, dict):
            return "Unsupported STATE.json shape: terminal must be a JSON object."
        if "complete" in terminal and not isinstance(terminal["complete"], bool):
            return "Unsupported STATE.json shape: terminal.complete must be a boolean."

    outcome = state.get("outcome")
    if outcome is not None:
        if not isinstance(outcome, dict):
            return "Unsupported STATE.json shape: outcome must be a JSON object."
        if "ratified" in outcome and not isinstance(outcome["ratified"], bool):
            return "Unsupported STATE.json shape: outcome.ratified must be a boolean."

    plan = state.get("plan")
    if plan is not None:
        if not isinstance(plan, dict):
            return "Unsupported STATE.json shape: plan must be a JSON object."
        if "locked" in plan and not isinstance(plan["locked"], bool):
            return "Unsupported STATE.json shape: plan.locked must be a boolean."

    reviews = state.get("reviews")
    if reviews is not None:
        if not isinstance(reviews, dict):
            return "Unsupported STATE.json shape: reviews must be a JSON object."
        if "pending_boundaries" in reviews and not isinstance(reviews["pending_boundaries"], list):
            return "Unsupported STATE.json shape: reviews.pending_boundaries must be a list."
        if "accepted_blocking_count" in reviews and not isinstance(reviews["accepted_blocking_count"], int):
            return "Unsupported STATE.json shape: reviews.accepted_blocking_count must be an integer."

    return None


def _allow_stop(reason: str, message: str) -> dict[str, Any]:
    return {"allow": True, "reason": reason, "mode": "open", "message": message}


def _stop_for_next_action(loop: dict[str, Any], next_action: dict[str, Any], reason: str, message: str) -> dict[str, Any]:
    if loop["active"] and not loop["paused"] and next_action.get("required") is True and next_action.get("autonomous") is True:
        return {"allow": False, "reason": reason, "mode": "strict", "message": message}
    return _allow_stop("inactive_loop", message)
