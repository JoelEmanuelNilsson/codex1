from __future__ import annotations

import argparse
import json
import sys
from typing import Any, Sequence

from .doctor import doctor_for
from .errors import Codex1Error
from .jsonio import write_json
from .status import status_for


class Codex1ArgumentParser(argparse.ArgumentParser):
    def error(self, message: str) -> None:
        raise Codex1Error("ARGUMENT_ERROR", message, exit_code=2)


def build_parser() -> Codex1ArgumentParser:
    parser = Codex1ArgumentParser(
        prog="codex1",
        description="Deterministic CLI substrate for Codex1 mission state.",
    )

    subparsers = parser.add_subparsers(dest="command", metavar="<command>")

    status_parser = subparsers.add_parser("status", help="Project current mission status.")
    _add_common_read_flags(status_parser)
    status_parser.set_defaults(handler=handle_status)

    doctor_parser = subparsers.add_parser("doctor", help="Run non-invasive installation and environment diagnostics.")
    _add_common_read_flags(doctor_parser)
    doctor_parser.add_argument("--e2e", action="store_true", help="Reserved for deeper integration checks; not implemented yet.")
    doctor_parser.set_defaults(handler=handle_doctor)

    init_parser = subparsers.add_parser("init", help="Create durable mission files. Not implemented in this skeleton.")
    init_parser.add_argument("--json", action="store_true", help="Emit stable JSON output.")
    init_parser.add_argument("--repo-root", help="Repository root to initialize. Defaults to the current working directory.")
    init_parser.set_defaults(handler=handle_not_implemented)

    _add_outcome_commands(subparsers)
    _add_plan_commands(subparsers)
    _add_task_commands(subparsers)
    _add_loop_commands(subparsers)
    _add_close_commands(subparsers)
    _add_ralph_commands(subparsers)

    return parser


def _add_common_read_flags(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--json", action="store_true", help="Emit stable JSON output.")
    parser.add_argument("--repo-root", help="Repository root to inspect. Defaults to the current working directory.")
    parser.add_argument("--mission", help="Durable mission id under PLANS/<mission-id>.")


def handle_status(args: argparse.Namespace) -> int:
    payload = status_for(repo_root_arg=args.repo_root, mission_id=args.mission)
    if args.json:
        write_json(payload)
    else:
        print(_status_text(payload))
    return 0


def handle_doctor(args: argparse.Namespace) -> int:
    if args.e2e:
        raise Codex1Error("NOT_IMPLEMENTED", "doctor --e2e is reserved for later integration proof.", exit_code=1)
    payload = doctor_for(repo_root_arg=args.repo_root, mission_id=args.mission)
    if args.json:
        write_json(payload)
    else:
        print(_doctor_text(payload))
    return 0 if payload["ok"] else 1


def handle_ralph_stop_hook(args: argparse.Namespace) -> int:
    hook_input = _read_hook_input()
    if hook_input.get("stop_hook_active") is True:
        payload: dict[str, Any] = {}
    else:
        try:
            status = status_for(repo_root_arg=args.repo_root, mission_id=args.mission)
        except Exception:
            status = {"stop": {"allow": True}}
        stop = status.get("stop", {})
        if stop.get("allow") is False:
            payload = {
                "decision": "block",
                "reason": stop.get("message") or stop.get("reason") or "Codex1 says required autonomous work remains.",
            }
        else:
            payload = {}
    write_json(payload)
    return 0


def handle_not_implemented(args: argparse.Namespace) -> int:
    raise Codex1Error("NOT_IMPLEMENTED", "This command is part of the Codex1 contract but is not implemented in this foundation skeleton.")


def main(argv: Sequence[str] | None = None) -> int:
    argv = list(argv if argv is not None else sys.argv[1:])
    parser = build_parser()
    wants_json = "--json" in argv
    try:
        args = parser.parse_args(argv)
        if not hasattr(args, "handler"):
            parser.print_help()
            return 0
        return int(args.handler(args))
    except Codex1Error as exc:
        if wants_json:
            write_json(exc.to_json(), stream=sys.stderr)
        else:
            print(f"codex1: {exc.code}: {exc.message}", file=sys.stderr)
        return exc.exit_code


def _status_text(payload: dict[str, Any]) -> str:
    mission = payload.get("mission_id") or "none"
    verdict = payload.get("verdict")
    stop = payload.get("stop", {})
    return f"mission={mission} verdict={verdict} stop_allow={str(stop.get('allow')).lower()}"


def _doctor_text(payload: dict[str, Any]) -> str:
    failed = [check for check in payload["checks"] if check.get("ok") is False and check.get("severity") == "error"]
    warnings = [check for check in payload["checks"] if check.get("ok") is False and check.get("severity") == "warning"]
    info = [check for check in payload["checks"] if check.get("severity") == "info"]
    return f"doctor ok={str(payload['ok']).lower()} errors={len(failed)} warnings={len(warnings)} info={len(info)}"


def _read_hook_input() -> dict[str, Any]:
    if sys.stdin.isatty():
        return {}
    try:
        raw = sys.stdin.read()
    except OSError:
        return {}
    if not raw.strip():
        return {}
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError:
        return {}
    return payload if isinstance(payload, dict) else {}


def _add_outcome_commands(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    parser = subparsers.add_parser("outcome", help="Check or ratify OUTCOME.md. Foundation stubs.")
    children = parser.add_subparsers(dest="outcome_command", metavar="<outcome-command>", required=True)
    for name in ("check", "ratify"):
        child = children.add_parser(name, help=f"Outcome {name}. Not implemented in this skeleton.")
        _add_common_read_flags(child)
        child.set_defaults(handler=handle_not_implemented)


def _add_plan_commands(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    parser = subparsers.add_parser("plan", help="Scaffold, check, or lock PLAN.yaml. Foundation stubs.")
    children = parser.add_subparsers(dest="plan_command", metavar="<plan-command>", required=True)

    choose_mode = children.add_parser("choose-mode", help="Choose planning mode. Not implemented in this skeleton.")
    _add_common_read_flags(choose_mode)
    choose_mode.set_defaults(handler=handle_not_implemented)

    choose_level = children.add_parser("choose-level", help="Choose planning level. Not implemented in this skeleton.")
    _add_common_read_flags(choose_level)
    choose_level.set_defaults(handler=handle_not_implemented)

    scaffold = children.add_parser("scaffold", help="Scaffold a plan. Not implemented in this skeleton.")
    _add_common_read_flags(scaffold)
    scaffold.add_argument("--mode", choices=["normal", "graph"], help="Planning mode to scaffold.")
    scaffold.add_argument("--level", help="Planning level to scaffold.")
    scaffold.set_defaults(handler=handle_not_implemented)

    for name in ("check", "lock"):
        child = children.add_parser(name, help=f"Plan {name}. Not implemented in this skeleton.")
        _add_common_read_flags(child)
        child.add_argument("--expect-revision", type=int, help="Expected STATE.json revision for the transition.")
        child.set_defaults(handler=handle_not_implemented)


def _add_task_commands(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    parser = subparsers.add_parser("task", help="Inspect or record task lifecycle transitions. Foundation stubs.")
    children = parser.add_subparsers(dest="task_command", metavar="<task-command>", required=True)
    for name in ("next", "status", "packet"):
        child = children.add_parser(name, help=f"Task {name}. Not implemented in this skeleton.")
        _add_common_read_flags(child)
        child.add_argument("task_id", nargs="?", help="Task or step id.")
        child.set_defaults(handler=handle_not_implemented)
    for name in ("start", "finish"):
        child = children.add_parser(name, help=f"Task {name}. Not implemented in this skeleton.")
        _add_common_read_flags(child)
        child.add_argument("task_id", help="Task or step id.")
        child.add_argument("--proof", help="Proof artifact path for finish.")
        child.add_argument("--expect-revision", type=int, help="Expected STATE.json revision for the transition.")
        child.set_defaults(handler=handle_not_implemented)


def _add_loop_commands(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    parser = subparsers.add_parser("loop", help="Activate, pause, resume, or deactivate a mission loop. Foundation stubs.")
    children = parser.add_subparsers(dest="loop_command", metavar="<loop-command>", required=True)
    for name in ("activate", "pause", "resume", "deactivate"):
        child = children.add_parser(name, help=f"Loop {name}. Not implemented in this skeleton.")
        _add_common_read_flags(child)
        child.add_argument("--mode", choices=["execute", "autopilot", "review_loop"], help="Loop mode for activation.")
        child.add_argument("--expect-revision", type=int, help="Expected STATE.json revision for the transition.")
        child.set_defaults(handler=handle_not_implemented)


def _add_close_commands(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    parser = subparsers.add_parser("close", help="Check or complete mission closeout. Foundation stubs.")
    children = parser.add_subparsers(dest="close_command", metavar="<close-command>", required=True)
    for name in ("check", "complete"):
        child = children.add_parser(name, help=f"Close {name}. Not implemented in this skeleton.")
        _add_common_read_flags(child)
        child.add_argument("--expect-revision", type=int, help="Expected STATE.json revision for the transition.")
        child.set_defaults(handler=handle_not_implemented)


def _add_ralph_commands(subparsers: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    parser = subparsers.add_parser("ralph", help="Ralph Stop-hook adapter.")
    children = parser.add_subparsers(dest="ralph_command", metavar="<ralph-command>", required=True)
    stop_hook = children.add_parser("stop-hook", help="Emit a Codex Stop-hook allow/block decision.")
    _add_common_read_flags(stop_hook)
    stop_hook.set_defaults(handler=handle_ralph_stop_hook)


if __name__ == "__main__":
    raise SystemExit(main())
