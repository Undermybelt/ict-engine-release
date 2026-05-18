#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys

SCAN_ROOTS = (
    ".github",
    "config",
    "src",
    "support/examples",
    "support/scripts",
    "tests",
)
SKIP_DIRS = {
    ".git",
    "__pycache__",
    "target",
}
SKIP_SUFFIXES = {
    ".pyc",
    ".pyo",
    ".so",
    ".dylib",
    ".rlib",
    ".rmeta",
    ".png",
    ".jpg",
    ".jpeg",
    ".gif",
    ".zip",
    ".gz",
    ".feather",
    ".parquet",
}
DOCS_PLANS_PATTERN = re.compile(r"support/docs/plans/[^\s'\"`),\]}]+\.md")


def _is_skipped(path: Path, script_path: Path) -> bool:
    if path == script_path:
        return True
    if any(part in SKIP_DIRS for part in path.parts):
        return True
    return path.suffix.lower() in SKIP_SUFFIXES


def _candidate_files(root: Path, extra_files: list[Path], script_path: Path) -> list[Path]:
    files: list[Path] = []
    for rel in SCAN_ROOTS:
        scan_root = root / rel
        if scan_root.is_file():
            if not _is_skipped(scan_root, script_path):
                files.append(scan_root)
            continue
        if not scan_root.exists():
            continue
        for path in scan_root.rglob("*"):
            if path.is_file() and not _is_skipped(path, script_path):
                files.append(path)
    for path in extra_files:
        resolved = path.resolve()
        if resolved.is_file() and not _is_skipped(resolved, script_path):
            files.append(resolved)
    return sorted(set(files))


def _read_text(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return None


def find_violations(root: Path, extra_files: list[Path], script_path: Path) -> list[str]:
    violations: list[str] = []
    for path in _candidate_files(root, extra_files, script_path):
        text = _read_text(path)
        if text is None:
            continue
        for lineno, line in enumerate(text.splitlines(), start=1):
            for match in DOCS_PLANS_PATTERN.finditer(line):
                try:
                    display_path = path.relative_to(root)
                except ValueError:
                    display_path = path
                violations.append(f"{display_path}:{lineno}: {match.group(0)}")
    return violations


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Fail when runtime/code surfaces reference support/docs/plans markdown."
    )
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--extra-file", action="append", type=Path, default=[])
    args = parser.parse_args(argv)

    root = args.root.resolve()
    script_path = Path(__file__).resolve()
    violations = find_violations(root, args.extra_file, script_path)
    if violations:
        print("docs runtime isolation violation:")
        for violation in violations:
            print(violation)
        return 1
    print("docs runtime isolation ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
