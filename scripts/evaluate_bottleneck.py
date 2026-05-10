#!/usr/bin/env python3
"""Public bottleneck-evaluation entrypoint."""
import argparse
import os
import pathlib
import subprocess
import sys
from path_defaults import (
    cleaned_data_root_ready,
    resolve_binary_path,
    resolve_cleaned_data_root,
    resolve_repo_root,
)

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
  python3 scripts/evaluate_bottleneck.py --show-config
  python3 scripts/evaluate_bottleneck.py --run --data-root /path/to/ict-cleaned-mtf
  python3 scripts/evaluate_bottleneck.py --run

Notes:
  - Default mode is safe: it prints this help and does not start a long run.
  - Use --backend-help for a non-executing backend summary.
  - `--run` now requires a resolved cleaned-data root with the expected interval folders.
  - Override local assumptions with `--data-root`, `--bin`, or `--repo-root`.
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
        '--show-config',
        action='store_true',
        help='print resolved repo/data/bin paths and whether cleaned data looks ready',
    )
    parser.add_argument('--repo-root', help='override repo root for backend execution')
    parser.add_argument('--data-root', help='override cleaned multi-timeframe data root for backend execution')
    parser.add_argument('--bin', dest='bin_path', help='override ict-engine binary path for backend execution')
    parser.add_argument(
        'args',
        nargs=argparse.REMAINDER,
        help='extra arguments passed to the backend script after --run; archived backends may ignore them',
    )
    return parser


def resolved_config(ns) -> dict[str, str | bool]:
    repo_root = pathlib.Path(ns.repo_root).expanduser().resolve() if ns.repo_root else resolve_repo_root(__file__)
    data_root = pathlib.Path(ns.data_root).expanduser().resolve() if ns.data_root else resolve_cleaned_data_root(__file__)
    bin_path = pathlib.Path(ns.bin_path).expanduser().resolve() if ns.bin_path else resolve_binary_path(__file__)
    return {
        'repo_root': str(repo_root),
        'data_root': str(data_root),
        'bin_path': str(bin_path),
        'cleaned_data_ready': cleaned_data_root_ready(__file__, data_root),
    }


def main(argv=None) -> int:
    parser = build_parser()
    ns = parser.parse_args(argv)
    config = resolved_config(ns)
    if ns.target:
        print(f"archived backend: {TARGET.relative_to(REPO)}")
        return 0
    if ns.backend_help:
        print(BACKEND_HELP)
        return 0
    if ns.show_config:
        for key, value in config.items():
            print(f"{key}={value}")
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
    if not config['cleaned_data_ready']:
        print(
            "refusing to run archived backend: cleaned data root is not ready.\n"
            "use --show-config to inspect resolved paths and pass --data-root /path/to/ict-cleaned-mtf "
            "or set ICT_ENGINE_DATA_ROOT explicitly.",
            file=sys.stderr,
        )
        return 2
    env = os.environ.copy()
    env['ICT_ENGINE_REPO_ROOT'] = str(config['repo_root'])
    env['ICT_ENGINE_DATA_ROOT'] = str(config['data_root'])
    env['ICT_ENGINE_BIN'] = str(config['bin_path'])
    return subprocess.run([sys.executable, str(TARGET), *extra], cwd=REPO, env=env).returncode


if __name__ == '__main__':
    raise SystemExit(main())
