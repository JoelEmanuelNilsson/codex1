#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import sqlite3
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable


DEFAULT_CODEX_HOME = Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")).expanduser()


@dataclass(frozen=True)
class JsonlRoot:
    label: str
    path: Path
    pattern: str = "**/*.jsonl"


def utc_from_timestamp(value: object) -> str:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return "n/a"

    if number > 10_000_000_000:
        number = number / 1000
    try:
        return datetime.fromtimestamp(number, tz=timezone.utc).strftime("%Y-%m-%d %H:%M:%SZ")
    except (OverflowError, OSError, ValueError):
        return "n/a"


def short_path(path: Path, home: Path) -> str:
    try:
        resolved_path = path.expanduser().resolve()
        resolved_home = home.resolve()
    except OSError:
        return str(path)

    try:
        return "~/" + str(resolved_path.relative_to(resolved_home))
    except ValueError:
        return str(resolved_path)


def iter_jsonl(root: Path, pattern: str, max_files: int | None) -> Iterable[Path]:
    if not root.exists():
        return
    seen = 0
    try:
        for path in root.rglob(pattern.removeprefix("**/")) if pattern.startswith("**/") else root.glob(pattern):
            if not path.is_file():
                continue
            yield path
            seen += 1
            if max_files is not None and seen >= max_files:
                return
    except OSError:
        return


def count_jsonl(root: Path, pattern: str, max_files: int | None) -> tuple[int, bool]:
    count = 0
    truncated = False
    for count, _ in enumerate(iter_jsonl(root, pattern, max_files), start=1):
        pass
    if max_files is not None and count >= max_files:
        truncated = True
    return count, truncated


def sqlite_connect_readonly(path: Path) -> sqlite3.Connection:
    return sqlite3.connect(f"file:{path}?mode=ro", uri=True)


def sqlite_tables(path: Path) -> list[str]:
    with sqlite_connect_readonly(path) as conn:
        rows = conn.execute(
            "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name"
        ).fetchall()
    return [row[0] for row in rows]


def sqlite_columns(path: Path, table: str) -> list[str]:
    with sqlite_connect_readonly(path) as conn:
        rows = conn.execute(f"PRAGMA table_info({quote_ident(table)})").fetchall()
    return [row[1] for row in rows]


def quote_ident(value: str) -> str:
    return '"' + value.replace('"', '""') + '"'


def recent_threads(path: Path, limit: int) -> tuple[list[str], list[dict[str, object]]]:
    columns = sqlite_columns(path, "threads")
    wanted = [
        "id",
        "updated_at",
        "created_at",
        "cwd",
        "rollout_path",
        "archived",
        "source",
        "model",
        "title",
    ]
    selected = [column for column in wanted if column in columns]
    if not selected:
        return columns, []

    order_column = "updated_at" if "updated_at" in columns else "rowid"
    sql = (
        "SELECT "
        + ", ".join(quote_ident(column) for column in selected)
        + f" FROM {quote_ident('threads')} ORDER BY {quote_ident(order_column)} DESC LIMIT ?"
    )
    with sqlite_connect_readonly(path) as conn:
        rows = conn.execute(sql, (limit,)).fetchall()
    return columns, [dict(zip(selected, row)) for row in rows]


def print_state_db(codex_home: Path, recent: int, show_titles: bool) -> None:
    state_db = codex_home / "state_5.sqlite"
    print("State DB")
    print(f"- path: {state_db}")
    print(f"- exists: {'yes' if state_db.exists() else 'no'}")
    if not state_db.exists():
        return

    try:
        tables = sqlite_tables(state_db)
    except sqlite3.Error as exc:
        print(f"- readable: no ({exc})")
        return

    print("- readable: yes")
    print(f"- tables: {', '.join(tables) if tables else 'none'}")
    if "threads" not in tables:
        print("- threads: missing; falling back to JSONL path inventory")
        return

    try:
        columns, rows = recent_threads(state_db, recent)
    except sqlite3.Error as exc:
        print(f"- threads readable: no ({exc})")
        return

    print(f"- threads columns: {', '.join(columns)}")
    print(f"- recent rows shown: {len(rows)}")
    for row in rows:
        thread_id = str(row.get("id", "n/a"))
        updated = utc_from_timestamp(row.get("updated_at"))
        rollout = str(row.get("rollout_path") or "")
        rollout_exists = Path(rollout).exists() if rollout else False
        cwd = str(row.get("cwd") or "")
        archived = row.get("archived", "n/a")
        bits = [
            f"thread={thread_id}",
            f"updated={updated}",
            f"archived={archived}",
            f"rollout_exists={'yes' if rollout_exists else 'no'}",
        ]
        if cwd:
            bits.append(f"cwd={cwd}")
        if rollout:
            bits.append(f"rollout={rollout}")
        if show_titles and row.get("title"):
            bits.append(f"title={row.get('title')}")
        print("  - " + " | ".join(bits))


def print_jsonl_roots(codex_home: Path, max_count: int | None, recent_paths: int) -> None:
    roots = [
        JsonlRoot("sessions", codex_home / "sessions"),
        JsonlRoot("archived_sessions", codex_home / "archived_sessions"),
        JsonlRoot("log", codex_home / "log"),
        JsonlRoot("history", codex_home, "history.jsonl"),
        JsonlRoot("session_index", codex_home, "session_index.jsonl"),
    ]
    home = Path.home()
    print()
    print("JSONL Sources")
    for root in roots:
        count, truncated = count_jsonl(root.path, root.pattern, max_count)
        status = "exists" if root.path.exists() else "missing"
        suffix = " (truncated)" if truncated else ""
        print(f"- {root.label}: {root.path} [{status}], jsonl={count}{suffix}")

        if recent_paths <= 0 or count == 0:
            continue
        files = sorted(
            iter_jsonl(root.path, root.pattern, max_count),
            key=lambda item: item.stat().st_mtime if item.exists() else 0,
            reverse=True,
        )[:recent_paths]
        for path in files:
            try:
                mtime = utc_from_timestamp(path.stat().st_mtime)
            except OSError:
                mtime = "n/a"
            print(f"  - {mtime} {short_path(path, home)}")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Safely inventory local Codex session sources without printing transcript contents."
        )
    )
    parser.add_argument(
        "--codex-home",
        type=Path,
        default=DEFAULT_CODEX_HOME,
        help="Codex home directory. Defaults to CODEX_HOME or ~/.codex.",
    )
    parser.add_argument(
        "--recent",
        type=int,
        default=10,
        help="Number of recent SQLite thread rows to show.",
    )
    parser.add_argument(
        "--recent-paths",
        type=int,
        default=3,
        help="Number of recent JSONL paths to show per source root.",
    )
    parser.add_argument(
        "--max-count-scan",
        type=int,
        default=100000,
        help="Maximum JSONL files to count per root before truncating.",
    )
    parser.add_argument(
        "--show-titles",
        action="store_true",
        help="Include thread titles from SQLite metadata. Off by default for privacy.",
    )
    return parser


def main() -> int:
    args = build_parser().parse_args()
    codex_home = args.codex_home.expanduser()
    max_count = None if args.max_count_scan <= 0 else args.max_count_scan

    print("Codex Session Source Scan")
    print(f"- codex_home: {codex_home}")
    print("- transcript_contents: not read")
    print("- secrets: not inspected")
    print()

    print_state_db(codex_home, max(args.recent, 0), args.show_titles)
    print_jsonl_roots(codex_home, max_count, max(args.recent_paths, 0))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
