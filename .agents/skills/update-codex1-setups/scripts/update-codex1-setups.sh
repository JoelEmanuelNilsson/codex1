#!/usr/bin/env bash
set -euo pipefail

case "${1:---dry-run}" in
  --dry-run)
    mode=dry-run
    ;;
  --apply)
    mode=apply
    ;;
  --apply-commit-push)
    mode=apply-commit-push
    ;;
  *)
    echo "usage: $0 [--dry-run|--apply|--apply-commit-push]" >&2
    exit 2
    ;;
esac

source_root=${CODEX1_SETUP_SOURCE_ROOT:-/Users/joel/codex1}
search_root=${CODEX1_SETUP_SEARCH_ROOT:-/Users/joel}
bin=${CODEX1_SETUP_BIN:-}

if [[ "$mode" != dry-run ]]; then
  dirty_source=$(git -C "$source_root" status --porcelain --untracked-files=all)
  if [[ -n "$dirty_source" ]]; then
    echo "refusing apply: source checkout has uncommitted changes: $source_root" >&2
    echo "$dirty_source" >&2
    exit 1
  fi
  git -C "$source_root" pull --ff-only
fi

marker_files() {
  python3 - "$1/.codex1/setup-bundle.json" <<'PY'
import json
import sys
from pathlib import Path

marker = Path(sys.argv[1])
if not marker.exists():
    raise SystemExit(0)
with marker.open() as handle:
    for path in json.load(handle).get("files", []):
        print(path)
PY
}

stage_path_if_present() {
  local repo=$1
  local path=$2
  if [[ -n "$(git -C "$repo" status --porcelain --untracked-files=all -- "$path")" ]]; then
    git -C "$repo" add -A -- "$path"
  fi
}

staged_path_is_allowed() {
  local staged=$1
  shift

  case "$staged" in
    .codex1/setup-bundle.json|.codex1/setup-backups/manifest.json|.codex1/setup-backups/files/*)
      return 0
      ;;
  esac

  local allowed
  for allowed in "$@"; do
    [[ "$staged" == "$allowed" ]] && return 0
  done

  return 1
}

stage_setup_changes() {
  local repo=$1
  shift

  local path
  for path in "$@"; do
    stage_path_if_present "$repo" "$path"
  done
  stage_path_if_present "$repo" ".codex1/setup-bundle.json"
  stage_path_if_present "$repo" ".codex1/setup-backups/manifest.json"
  stage_path_if_present "$repo" ".codex1/setup-backups/files"

  while IFS= read -r staged; do
    if ! staged_path_is_allowed "$staged" "$@"; then
      echo "refusing commit: staged non-setup path in $repo: $staged" >&2
      return 1
    fi
  done < <(git -C "$repo" diff --cached --name-only)
}

can_commit_push_repo() {
  local repo=$1
  local upstream
  local ahead
  local behind

  if ! git -C "$repo" symbolic-ref --quiet --short HEAD >/dev/null; then
    echo "skipping commit-push: detached HEAD in $repo" >&2
    return 1
  fi
  if ! git -C "$repo" diff --cached --quiet; then
    echo "skipping commit-push: staged changes already exist in $repo" >&2
    return 1
  fi
  if ! upstream=$(git -C "$repo" rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null); then
    echo "skipping commit-push: no upstream for $(git -C "$repo" branch --show-current) in $repo" >&2
    return 1
  fi
  read -r ahead behind < <(git -C "$repo" rev-list --left-right --count HEAD..."$upstream")
  if [[ "$ahead" != 0 || "$behind" != 0 ]]; then
    echo "skipping commit-push: branch is ahead=$ahead behind=$behind before setup commit in $repo" >&2
    return 1
  fi
}

if [[ -n "$bin" ]]; then
  if [[ ! -x "$bin" ]]; then
    echo "configured CODEX1_SETUP_BIN is not executable: $bin" >&2
    exit 1
  fi
else
  cargo build --manifest-path "$source_root/Cargo.toml" >/dev/null
  bin="$source_root/target/debug/codex1"
fi
count=0
committed=0
pushed=0
skipped_commit_push=0

while IFS= read -r -d "" marker; do
  repo=${marker%/.codex1/setup-bundle.json}
  repo=$(cd "$repo" && pwd -P)
  top=$(git -C "$repo" rev-parse --show-toplevel 2>/dev/null || true)
  if [[ -n "$top" ]]; then
    top=$(cd "$top" && pwd -P)
  fi
  [[ -n "$top" && "$repo" == "$top" ]] || continue
  git -C "$repo" ls-files --error-unmatch .codex1/setup-bundle.json >/dev/null 2>&1 || continue

  count=$((count + 1))
  printf "== %s ==\n" "$repo"
  if [[ "$mode" == dry-run ]]; then
    "$bin" --repo-root "$repo" setup install --dry-run
  elif [[ "$mode" == apply-commit-push ]]; then
    if ! can_commit_push_repo "$repo"; then
      skipped_commit_push=$((skipped_commit_push + 1))
      continue
    fi
    setup_paths_before=()
    while IFS= read -r path; do
      setup_paths_before+=("$path")
    done < <(marker_files "$repo")
    "$bin" --repo-root "$repo" setup install
    setup_paths_after=()
    while IFS= read -r path; do
      setup_paths_after+=("$path")
    done < <(marker_files "$repo")
    stage_setup_changes "$repo" "${setup_paths_before[@]}" "${setup_paths_after[@]}"
    if git -C "$repo" diff --cached --quiet; then
      echo "no setup changes to commit"
      continue
    fi
    git -C "$repo" commit -m "Update Codex1 setup guidance"
    committed=$((committed + 1))
    git -C "$repo" push
    pushed=$((pushed + 1))
  else
    "$bin" --repo-root "$repo" setup install
  fi
done < <(
  find "$search_root" \
    \( -path "$search_root/Library" -o -path "$search_root/.Trash" -o -name node_modules -o -name target -o -name "*.photoslibrary" -o -name "Photo Booth Library" \) -prune -o \
    -path "*/.codex1/setup-bundle.json" -type f \
    -not -path "*/.codex1/setup-backups/*" \
    -print0 2>/dev/null
)

if [[ "$count" == 0 ]]; then
  echo "no valid Codex1 setup repos found" >&2
  exit 1
fi

echo "$mode complete for $count repos"
if [[ "$mode" == apply-commit-push ]]; then
  echo "committed $committed repos, pushed $pushed repos, skipped commit-push for $skipped_commit_push repos"
fi
