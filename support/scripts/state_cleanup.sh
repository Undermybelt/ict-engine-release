#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

echo "State directory usage under $repo_root"
echo

if ! find . -maxdepth 1 -type d \( -name 'state' -o -name 'state_*' -o -name 'state*' \) | grep -q .; then
  echo "No state directories found."
  exit 0
fi

du -sh ./state* 2>/dev/null | sort -hr
echo
echo "Suggested review commands:"
echo "  ls -la ./state*"
echo "  find ./state* -maxdepth 2 -type f | head"
echo
echo "Suggested removal pattern after review:"
echo "  rm -rf ./state_old_name"
