#!/usr/bin/env python3
"""Public bottleneck-evaluation entrypoint."""
import argparse
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
TARGET = REPO / 'scripts' / 'archive' / 'pre_bayes_policy_tuning.py'

BACKEND_HELP = """Archived Pre-Bayes policy tuning backend. It evaluates evidence/gate/shrink/bridge bottleneck configs.

Backend: scripts/archive/pre_bayes_policy_tuning.py

This archived backend does not expose a stable public argparse surface.
Use --run only when you intend to execute it.
"""

EPILOG = """
When to use:
  - Use when score is pinned by evidence quality, Pre-Bayes gate, shrink, or bridge gap.
  - Best after factor-pipeline-debug shows the factor itself is not the only blocker.

Examples:
  python3 scripts/evaluate_bottleneck.py
  python3 scripts/evaluate_bottleneck.py --target
  python3 scripts/evaluate_bottleneck.py --backend-help
  python3 scripts/evaluate_bottleneck.py --run

Notes:
  - Default mode is safe: it prints this help and does not start a long run.
  - Use --backend-help for a non-executing backend summary.
  - Use --run only when you intend to execute the archived backend script.
  - --run can read/write state and may take minutes.
"""


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description='Run or inspect the archived bottleneck-evaluation experiment entrypoint.',
        epilog=EPILOG,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        '--run',
        action='store_true',
        help='execute the archived backend script; omitted means show this help only',
    )
    parser.add_argument(
        '--target',
        action='store_true',
        help='print the archived backend script path and exit',
    )
    parser.add_argument(
        '--backend-help',
        action='store_true',
        help='print non-executing backend summary and exit',
    )
    parser.add_argument(
        'args',
        nargs=argparse.REMAINDER,
        help='extra arguments passed to the backend script after --run; archived backends may ignore them',
    )
    return parser


def main(argv=None) -> int:
    parser = build_parser()
    ns = parser.parse_args(argv)
    if ns.target:
        print(f"archived backend: {TARGET.relative_to(REPO)}")
        return 0
    if ns.backend_help:
        print(BACKEND_HELP)
        return 0
    extra = ns.args
    if extra and extra[0] == '--':
        extra = extra[1:]
    if ns.run and '--help' in extra:
        print(BACKEND_HELP)
        return 0
    if not ns.run:
        parser.print_help()
        return 0
    return subprocess.run([sys.executable, str(TARGET), *extra], cwd=REPO).returncode


if __name__ == '__main__':
    raise SystemExit(main())
