#!/usr/bin/env bash
set -euo pipefail

SRC="$HOME/sam-repos/stems/"
DST="$HOME/sam-repos/monorepo/stems/"
MONOREPO="$HOME/sam-repos/monorepo"

# Files/dirs deferred to pass 2 (after the jj commit).
CONFIG=(
  '.claude'
  '.envrc'
  '.gitignore'
  '.mise.toml'
  '.vscode'
  'CLAUDE.md'
)

if [[ -e "$DST" ]]; then
  echo "destination already exists: $DST" >&2
  exit 1
fi

mkdir -p "$DST"

EXCLUDES=(
  '--filter=:- .gitignore'
  '--exclude=.git/'
  '--exclude=.jj/'
  '--exclude=.DS_Store'
  '--exclude=target/'
  '--exclude=target-rust-analyzer/'
  '--exclude=node_modules/'
  '--exclude=.venv/'
  '--exclude=.pycache/'
  '--exclude=__pycache__/'
  '--exclude=separated/'
  '--exclude=demucs-sandbox/'
  '--exclude=thoughts'
)
for c in "${CONFIG[@]}"; do
  EXCLUDES+=("--exclude=/$c")
done

cd "$MONOREPO"

echo "==> jj new (isolate the stems import from current @)"
jj new

echo "==> pass 1: copy non-config files"
rsync -a "${EXCLUDES[@]}" "$SRC" "$DST"

echo "==> jj commit"
jj commit -m "stems: import sources from ~/sam-repos/stems"

echo "==> pass 2: copy config files into new @"
for c in "${CONFIG[@]}"; do
  src_path="$SRC$c"
  if [[ -e "$src_path" ]]; then
    rsync -a "$src_path" "$DST"
    echo "  copied: $c"
  else
    echo "  skip (not present): $c"
  fi
done

echo "==> done"
du -sh "$DST"
echo
jj st
