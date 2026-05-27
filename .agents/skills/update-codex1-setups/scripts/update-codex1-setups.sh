#!/usr/bin/env bash
set -euo pipefail

case "${1:---dry-run}" in
  --dry-run)
    mode=dry-run
    ;;
  --apply)
    mode=apply
    ;;
  *)
    echo "usage: $0 [--dry-run|--apply]" >&2
    exit 2
    ;;
esac

source_root=/Users/joel/codex1

if [[ "$mode" == apply ]]; then
  if [[ -n "$(git -C "$source_root" status --porcelain --untracked-files=all)" ]]; then
    echo "refusing apply: $source_root has uncommitted changes" >&2
    exit 1
  fi
  git -C "$source_root" pull --ff-only
fi

cargo build --manifest-path "$source_root/Cargo.toml" >/dev/null
bin="$source_root/target/debug/codex1"
count=0

while IFS= read -r -d "" marker; do
  repo=${marker%/.codex1/setup-bundle.json}
  top=$(git -C "$repo" rev-parse --show-toplevel 2>/dev/null || true)
  [[ -n "$top" && "$repo" == "$top" ]] || continue
  git -C "$repo" ls-files --error-unmatch .codex1/setup-bundle.json >/dev/null 2>&1 || continue

  count=$((count + 1))
  printf "== %s ==\n" "$repo"
  if [[ "$mode" == dry-run ]]; then
    "$bin" --repo-root "$repo" setup install --dry-run
  else
    "$bin" --repo-root "$repo" setup install
  fi
done < <(
  find /Users/joel \
    \( -path /Users/joel/Library -o -path /Users/joel/.Trash -o -name node_modules -o -name target -o -name "*.photoslibrary" -o -name "Photo Booth Library" \) -prune -o \
    -path "*/.codex1/setup-bundle.json" -type f \
    -not -path "*/.codex1/setup-backups/*" \
    -print0 2>/dev/null
)

if [[ "$count" == 0 ]]; then
  echo "no valid Codex1 setup repos found" >&2
  exit 1
fi

echo "$mode complete for $count repos"
