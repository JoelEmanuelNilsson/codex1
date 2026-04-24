from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[1]
PYTHONPATH = str(REPO_ROOT / "src")
if PYTHONPATH not in sys.path:
    sys.path.insert(0, PYTHONPATH)


def run_codex1(*args: str, cwd: Path | None = None, input_text: str | None = None) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["PYTHONPATH"] = PYTHONPATH + os.pathsep + env.get("PYTHONPATH", "")
    return subprocess.run(
        [sys.executable, "-m", "codex1", *args],
        cwd=cwd or REPO_ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        input=input_text,
        check=False,
    )


def write_state(mission_root: Path, **overrides: object) -> None:
    state = {
        "schema_version": "codex1.state.v1",
        "mission_id": mission_root.name,
        "revision": 0,
        "planning_mode": "normal",
        "loop": {"active": False, "paused": False, "mode": "none"},
        "replan": {"required": False, "reason": None},
        "close": {"state": "not_ready", "requires_mission_close_review": False},
        "terminal": {"complete": False, "completed_revision": None},
    }
    state.update(overrides)
    mission_root.mkdir(parents=True, exist_ok=True)
    (mission_root / "STATE.json").write_text(json.dumps(state), encoding="utf-8")


class Codex1CliTests(unittest.TestCase):
    def test_root_help_lists_foundation_commands(self) -> None:
        result = run_codex1("--help")
        self.assertEqual(result.stderr, "")
        for command in ("status", "doctor", "outcome", "plan", "task", "loop", "close", "ralph"):
            with self.subTest(command=command):
                self.assertIn(command, result.stdout)

    def test_subcommand_help_is_available(self) -> None:
        for command in ("status", "doctor", "init"):
            with self.subTest(command=command):
                result = run_codex1(command, "--help")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("--json", result.stdout)

    def test_foundation_nested_subcommand_help_is_available(self) -> None:
        commands = [
            ("outcome", "check"),
            ("plan", "choose-mode"),
            ("plan", "choose-level"),
            ("plan", "scaffold"),
            ("plan", "lock"),
            ("task", "next"),
            ("task", "start"),
            ("task", "finish"),
            ("loop", "activate"),
            ("loop", "pause"),
            ("close", "check"),
            ("close", "complete"),
            ("ralph", "stop-hook"),
        ]
        for command in commands:
            with self.subTest(command=command):
                result = run_codex1(*command, "--help")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("--json", result.stdout)

    def test_status_json_without_mission_is_inactive_and_allows_stop(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            result = run_codex1("status", "--json", "--repo-root", temp_dir, cwd=Path(temp_dir))
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["schema_version"], "codex1.status.v1")
        self.assertIsNone(payload["mission_id"])
        self.assertEqual(payload["verdict"], "inactive")
        self.assertEqual(payload["next_action"]["kind"], "none")
        self.assertTrue(payload["stop"]["allow"])
        self.assertEqual(payload["stop"]["reason"], "no_active_mission")

    def test_status_rejects_absolute_mission_id(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir, tempfile.TemporaryDirectory() as outside_dir:
            outside = Path(outside_dir)
            (outside / "STATE.json").write_text(
                json.dumps({"schema_version": "codex1.state.v1", "mission_id": "outside", "revision": 99}),
                encoding="utf-8",
            )
            result = run_codex1("status", "--json", "--repo-root", repo_dir, "--mission", outside_dir)
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        self.assertTrue(payload["ok"])
        self.assertIsNone(payload["mission_id"])
        self.assertIsNone(payload["mission_root"])
        self.assertEqual(payload["verdict"], "inactive")
        self.assertIn("warnings", payload)
        self.assertIn("relative id under PLANS", payload["warnings"][0])

    def test_status_rejects_parent_traversal_mission_id(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            outside = repo / "escape"
            outside.mkdir()
            (outside / "STATE.json").write_text(
                json.dumps({"schema_version": "codex1.state.v1", "mission_id": "escape", "revision": 99}),
                encoding="utf-8",
            )
            result = run_codex1("status", "--json", "--repo-root", repo_dir, "--mission", "../escape")
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        self.assertTrue(payload["ok"])
        self.assertIsNone(payload["mission_id"])
        self.assertIsNone(payload["mission_root"])
        self.assertEqual(payload["verdict"], "inactive")
        self.assertIn("warnings", payload)
        self.assertIn("must not contain", payload["warnings"][0])

    def test_status_ignores_active_pointer_with_non_object_shape(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            plans = repo / "PLANS"
            plans.mkdir()
            (plans / "ACTIVE.json").write_text("[]", encoding="utf-8")
            result = run_codex1("status", "--json", "--repo-root", repo_dir)
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        self.assertTrue(payload["ok"])
        self.assertIsNone(payload["mission_id"])
        self.assertEqual(payload["verdict"], "inactive")
        self.assertIn("warnings", payload)
        self.assertIn("unsupported shape", payload["warnings"][0])

    def test_status_ignores_active_pointer_with_parent_traversal_mission_id(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            plans = repo / "PLANS"
            plans.mkdir()
            outside = repo / "escape"
            outside.mkdir()
            (outside / "STATE.json").write_text(
                json.dumps({"schema_version": "codex1.state.v1", "mission_id": "escape", "revision": 99}),
                encoding="utf-8",
            )
            (plans / "ACTIVE.json").write_text(
                json.dumps({"schema_version": "codex1.active.v1", "mission_id": "../escape"}),
                encoding="utf-8",
            )
            result = run_codex1("status", "--json", "--repo-root", repo_dir)
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        self.assertTrue(payload["ok"])
        self.assertIsNone(payload["mission_id"])
        self.assertIsNone(payload["mission_root"])
        self.assertEqual(payload["verdict"], "inactive")
        self.assertIn("warnings", payload)
        self.assertIn("must not contain", payload["warnings"][0])

    def test_status_reports_non_object_state_as_invalid_state_json(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            mission.mkdir(parents=True)
            (mission / "STATE.json").write_text("[]", encoding="utf-8")
            result = run_codex1("status", "--json", "--repo-root", repo_dir, "--mission", "demo")
        self.assertEqual(result.returncode, 0, result.stderr)
        payload = json.loads(result.stdout)
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["mission_id"], "demo")
        self.assertEqual(payload["verdict"], "invalid_state")
        self.assertEqual(payload["next_action"]["kind"], "explain_and_stop")
        self.assertTrue(payload["stop"]["allow"])
        self.assertIn("expected a JSON object", payload["warnings"][0])

    def test_status_discovers_mission_from_current_directory(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            write_state(mission)
            result = run_codex1("status", "--json", cwd=mission)
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        self.assertEqual(payload["repo_root"], str(Path(repo_dir).resolve()))
        self.assertEqual(payload["mission_id"], "demo")
        self.assertEqual(payload["mission_root"], str(mission.resolve()))

    def test_status_rejects_state_mission_id_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            write_state(mission, mission_id="other")
            result = run_codex1("status", "--json", "--repo-root", repo_dir, "--mission", "demo")
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        self.assertEqual(payload["mission_id"], "demo")
        self.assertEqual(payload["verdict"], "invalid_state")
        self.assertIn("does not match selected mission", payload["warnings"][0])

    def test_active_pointer_rejects_state_mission_id_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            plans = repo / "PLANS"
            mission = plans / "demo"
            write_state(mission, mission_id="other")
            (plans / "ACTIVE.json").write_text(
                json.dumps({"schema_version": "codex1.active.v1", "mission_id": "demo"}),
                encoding="utf-8",
            )
            result = run_codex1("status", "--json", "--repo-root", repo_dir)
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        self.assertEqual(payload["mission_id"], "demo")
        self.assertEqual(payload["verdict"], "invalid_state")
        self.assertIn("does not match selected mission", payload["warnings"][0])

    def test_status_surfaces_required_replan_before_active_fallback(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            write_state(
                mission,
                loop={"active": True, "paused": False, "mode": "execute"},
                replan={"required": True, "reason": "repair_budget_exhausted"},
            )
            result = run_codex1("status", "--json", "--repo-root", repo_dir, "--mission", "demo")
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        self.assertEqual(payload["verdict"], "replan_required")
        self.assertEqual(payload["next_action"]["kind"], "replan")
        self.assertTrue(payload["next_action"]["autonomous"])
        self.assertFalse(payload["stop"]["allow"])
        self.assertEqual(payload["stop"]["reason"], "block_replan_required")

    def test_status_surfaces_close_complete_when_close_is_ready(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            write_state(
                mission,
                loop={"active": True, "paused": False, "mode": "execute"},
                outcome={"ratified": True},
                plan={"locked": True},
                close={"state": "close_complete_ready", "requires_mission_close_review": False},
            )
            result = run_codex1("status", "--json", "--repo-root", repo_dir, "--mission", "demo")
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        self.assertEqual(payload["verdict"], "close_required")
        self.assertEqual(payload["next_action"]["kind"], "close_complete")
        self.assertTrue(payload["close"]["ready"])
        self.assertTrue(payload["close"]["required"])
        self.assertFalse(payload["stop"]["allow"])
        self.assertEqual(payload["stop"]["reason"], "block_close_complete_required")

    def test_status_does_not_trust_close_ready_without_close_gates(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            write_state(
                mission,
                loop={"active": True, "paused": False, "mode": "execute"},
                close={"state": "close_complete_ready", "requires_mission_close_review": False},
            )
            result = run_codex1("status", "--json", "--repo-root", repo_dir, "--mission", "demo")
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        self.assertEqual(payload["verdict"], "continue_required")
        self.assertFalse(payload["close"]["ready"])
        self.assertFalse(payload["close"]["required"])
        self.assertTrue(payload["stop"]["allow"])

    def test_inactive_loop_takes_priority_over_replan_and_close_ready(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            write_state(
                mission,
                loop={"active": False, "paused": False, "mode": "none"},
                outcome={"ratified": True},
                plan={"locked": True},
                replan={"required": True, "reason": "repair_budget_exhausted"},
                close={"state": "close_complete_ready", "requires_mission_close_review": False},
            )
            result = run_codex1("status", "--json", "--repo-root", repo_dir, "--mission", "demo")
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        self.assertEqual(payload["verdict"], "inactive")
        self.assertEqual(payload["next_action"]["kind"], "none")
        self.assertTrue(payload["stop"]["allow"])
        self.assertEqual(payload["stop"]["reason"], "inactive_loop")

    def test_status_rejects_non_boolean_loop_and_replan_values(self) -> None:
        cases = [
            {"loop": {"active": "false", "paused": False, "mode": "execute"}, "replan": {"required": True}},
            {"loop": {"active": True, "paused": False, "mode": "execute"}, "replan": {"required": "true"}},
        ]
        for overrides in cases:
            with self.subTest(overrides=overrides), tempfile.TemporaryDirectory() as repo_dir:
                repo = Path(repo_dir)
                mission = repo / "PLANS" / "demo"
                write_state(mission, **overrides)
                result = run_codex1("status", "--json", "--repo-root", repo_dir, "--mission", "demo")
                self.assertEqual(result.stderr, "")
                payload = json.loads(result.stdout)
                self.assertEqual(payload["verdict"], "invalid_state")
                self.assertEqual(payload["next_action"]["kind"], "explain_and_stop")
                self.assertTrue(payload["stop"]["allow"])
                self.assertIn("must be a boolean", payload["warnings"][0])

    def test_ralph_stop_hook_allows_when_no_mission_is_selected(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            result = run_codex1("ralph", "stop-hook", "--json", "--repo-root", repo_dir)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {})

    def test_ralph_stop_hook_blocks_required_autonomous_status(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            write_state(
                mission,
                loop={"active": True, "paused": False, "mode": "execute"},
                replan={"required": True, "reason": "repair_budget_exhausted"},
            )
            result = run_codex1("ralph", "stop-hook", "--json", "--repo-root", repo_dir, "--mission", "demo")
        self.assertEqual(result.returncode, 0, result.stderr)
        payload = json.loads(result.stdout)
        self.assertEqual(payload["decision"], "block")
        self.assertIn("replan is required", payload["reason"])

    def test_ralph_stop_hook_allows_when_stop_hook_active(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            write_state(
                mission,
                loop={"active": True, "paused": False, "mode": "execute"},
                replan={"required": True, "reason": "repair_budget_exhausted"},
            )
            result = run_codex1(
                "ralph",
                "stop-hook",
                "--json",
                "--repo-root",
                repo_dir,
                "--mission",
                "demo",
                input_text=json.dumps({"stop_hook_active": True}),
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {})

    def test_doctor_json_is_non_invasive_and_honest(self) -> None:
        result = run_codex1("doctor", "--json", "--repo-root", str(REPO_ROOT))
        self.assertEqual(result.returncode, 0, result.stderr)
        payload = json.loads(result.stdout)
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["schema_version"], "codex1.doctor.v1")
        checks = {check["id"]: check for check in payload["checks"]}
        self.assertTrue(checks["handoff_docs_present"]["ok"])
        self.assertEqual(checks["codex_hook_config_parser_integration"]["severity"], "info")
        self.assertFalse(checks["codex_hook_config_parser_integration"]["ok"])

    def test_doctor_reports_non_object_state_as_warning_check(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            mission.mkdir(parents=True)
            (mission / "STATE.json").write_text("[]", encoding="utf-8")
            result = run_codex1("doctor", "--json", "--repo-root", repo_dir, "--mission", "demo")
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        checks = {check["id"]: check for check in payload["checks"]}
        drift = checks["state_event_revision_drift"]
        self.assertFalse(drift["ok"])
        self.assertEqual(drift["severity"], "warning")
        self.assertIn("expected a JSON object", drift["message"])

    def test_doctor_reports_missing_events_for_revised_state_as_drift(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            mission.mkdir(parents=True)
            (mission / "STATE.json").write_text(
                json.dumps({"schema_version": "codex1.state.v1", "mission_id": "demo", "revision": 3}),
                encoding="utf-8",
            )
            result = run_codex1("doctor", "--json", "--repo-root", repo_dir, "--mission", "demo")
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        checks = {check["id"]: check for check in payload["checks"]}
        drift = checks["state_event_revision_drift"]
        self.assertFalse(drift["ok"])
        self.assertEqual(drift["state_revision"], 3)
        self.assertIsNone(drift["latest_event_revision"])
        self.assertIn("missing audit for revision 3", drift["message"])

    def test_doctor_reports_malformed_event_rows_without_crashing(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            mission.mkdir(parents=True)
            (mission / "STATE.json").write_text(
                json.dumps({"schema_version": "codex1.state.v1", "mission_id": "demo", "revision": 2}),
                encoding="utf-8",
            )
            (mission / "EVENTS.jsonl").write_text(
                "[]\n" + json.dumps({"revision_after": 2}) + "\n",
                encoding="utf-8",
            )
            result = run_codex1("doctor", "--json", "--repo-root", repo_dir, "--mission", "demo")
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        checks = {check["id"]: check for check in payload["checks"]}
        drift = checks["state_event_revision_drift"]
        self.assertFalse(drift["ok"])
        self.assertEqual(drift["latest_event_revision"], 2)
        self.assertEqual(drift["malformed_event_rows"], 1)
        self.assertIn("malformed event", drift["message"])

    def test_doctor_reports_event_rows_without_revision_as_malformed(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            write_state(mission, revision=2)
            (mission / "EVENTS.jsonl").write_text(
                json.dumps({"type": "missing_revision"}) + "\n" + json.dumps({"revision_after": 2}) + "\n",
                encoding="utf-8",
            )
            result = run_codex1("doctor", "--json", "--repo-root", repo_dir, "--mission", "demo")
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        checks = {check["id"]: check for check in payload["checks"]}
        drift = checks["state_event_revision_drift"]
        self.assertFalse(drift["ok"])
        self.assertEqual(drift["latest_event_revision"], 2)
        self.assertEqual(drift["malformed_event_rows"], 1)

    def test_doctor_reports_unreadable_events_without_traceback(self) -> None:
        with tempfile.TemporaryDirectory() as repo_dir:
            repo = Path(repo_dir)
            mission = repo / "PLANS" / "demo"
            write_state(mission, revision=2)
            (mission / "EVENTS.jsonl").mkdir()
            result = run_codex1("doctor", "--json", "--repo-root", repo_dir, "--mission", "demo")
        self.assertEqual(result.stderr, "")
        payload = json.loads(result.stdout)
        checks = {check["id"]: check for check in payload["checks"]}
        drift = checks["state_event_revision_drift"]
        self.assertFalse(drift["ok"])
        self.assertIn("Cannot read EVENTS.jsonl", drift["message"])

    def test_installed_command_check_runs_outside_source_without_source_pythonpath(self) -> None:
        from codex1.doctor import _command_on_path_check

        with tempfile.TemporaryDirectory() as temp_dir:
            temp = Path(temp_dir)
            bin_dir = temp / "bin"
            bin_dir.mkdir()
            marker = temp / "marker.json"
            command = bin_dir / "codex1"
            command.write_text(
                "#!/usr/bin/env python3\n"
                "import json, os, pathlib\n"
                f"pathlib.Path({str(marker)!r}).write_text(json.dumps({{'cwd': os.getcwd(), 'pythonpath': os.environ.get('PYTHONPATH')}}))\n"
                "print('Deterministic CLI substrate status doctor')\n",
                encoding="utf-8",
            )
            command.chmod(0o755)
            with mock.patch.dict(
                os.environ,
                {"PATH": str(bin_dir) + os.pathsep + os.environ.get("PATH", ""), "PYTHONPATH": PYTHONPATH},
                clear=False,
            ):
                check = _command_on_path_check()
            self.assertTrue(check["ok"], check)
            marker_payload = json.loads(marker.read_text(encoding="utf-8"))
            self.assertNotEqual(marker_payload["cwd"], str(REPO_ROOT))
            self.assertNotIn(PYTHONPATH, marker_payload.get("pythonpath") or "")

    def test_module_command_runs_from_outside_source_checkout(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            result = run_codex1("status", "--json", "--repo-root", str(REPO_ROOT), cwd=Path(temp_dir))
        self.assertEqual(result.returncode, 0, result.stderr)
        payload = json.loads(result.stdout)
        self.assertEqual(payload["repo_root"], str(REPO_ROOT))

    def test_json_error_shape_for_unimplemented_command(self) -> None:
        result = run_codex1("init", "--json")
        self.assertEqual(result.returncode, 1)
        payload = json.loads(result.stderr)
        self.assertFalse(payload["ok"])
        self.assertEqual(payload["schema_version"], "codex1.error.v1")
        self.assertEqual(payload["code"], "NOT_IMPLEMENTED")
        self.assertFalse(payload["retryable"])


if __name__ == "__main__":
    unittest.main()
