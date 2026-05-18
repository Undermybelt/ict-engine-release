#!/usr/bin/env python3
import json
import subprocess
import sys

if len(sys.argv) < 4:
    print(json.dumps({
        "error": "usage: compact_router.py <command> <symbol> <state_dir> [extra args...]"
    }))
    sys.exit(1)

command = sys.argv[1]
symbol = sys.argv[2]
state_dir = sys.argv[3]
extra = sys.argv[4:]

allowed = {
    "next-action": ["cargo", "run", "--quiet", "--", "next-action", "--symbol", symbol, "--state-dir", state_dir],
    "research-compact": ["cargo", "run", "--quiet", "--", "research-compact", "--symbol", symbol, "--state-dir", state_dir],
    "market-fork-status": ["cargo", "run", "--quiet", "--", "market-fork-status", "--symbol", symbol, "--state-dir", state_dir],
    "pre-bayes-compact": ["cargo", "run", "--quiet", "--", "pre-bayes-compact", "--symbol", symbol, "--state-dir", state_dir],
    "artifact-gate-compact": ["cargo", "run", "--quiet", "--", "artifact-gate-compact", "--symbol", symbol, "--state-dir", state_dir],
}

if command not in allowed:
    print(json.dumps({"error": f"unsupported command: {command}"}))
    sys.exit(2)

proc = subprocess.run(allowed[command] + extra, capture_output=True, text=True)
if proc.returncode != 0:
    print(json.dumps({"error": proc.stderr.strip() or proc.stdout.strip()}))
    sys.exit(proc.returncode)

print(proc.stdout.strip())
