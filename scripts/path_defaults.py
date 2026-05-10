#!/usr/bin/env python3
from __future__ import annotations

import os
from pathlib import Path


def _path_from_env(name: str) -> Path | None:
    value = os.environ.get(name)
    if not value:
        return None
    return Path(value).expanduser().resolve()


def _discover_repo_root(anchor: str | Path) -> Path:
    start = Path(anchor).expanduser().resolve()
    current = start if start.is_dir() else start.parent
    for candidate in [current, *current.parents]:
        if (candidate / "Cargo.toml").exists() and (candidate / "src").exists():
            return candidate
    raise RuntimeError(f"could not discover repo root from {anchor}")


def resolve_repo_root(anchor: str | Path) -> Path:
    return _path_from_env("ICT_ENGINE_REPO_ROOT") or _discover_repo_root(anchor)


def resolve_tomac_root(anchor: str | Path) -> Path:
    override = _path_from_env("ICT_ENGINE_TOMAC_ROOT")
    if override is not None:
        return override
    repo = resolve_repo_root(anchor)
    candidates = [
        repo / "data" / "Tomac",
        repo.parent / "Tomac",
        repo.parent.parent / "Downloads" / "Tomac",
    ]
    for candidate in candidates:
        if candidate.exists():
            return candidate.resolve()
    return candidates[-1].resolve()


def resolve_cleaned_data_root(anchor: str | Path) -> Path:
    override = _path_from_env("ICT_ENGINE_DATA_ROOT")
    if override is not None:
        return override
    repo = resolve_repo_root(anchor)
    tomac_root = resolve_tomac_root(anchor)
    candidates = [
        repo / "data" / "ict-cleaned-mtf",
        tomac_root / "ict-cleaned-mtf",
    ]
    for candidate in candidates:
        if candidate.exists():
            return candidate.resolve()
    return candidates[-1].resolve()


def resolve_binary_path(anchor: str | Path) -> Path:
    override = _path_from_env("ICT_ENGINE_BIN")
    if override is not None:
        return override
    repo = resolve_repo_root(anchor)
    release_bin = repo / "target" / "release" / "ict-engine"
    debug_bin = repo / "target" / "debug" / "ict-engine"
    if release_bin.exists():
        return release_bin.resolve()
    if debug_bin.exists():
        return debug_bin.resolve()
    return release_bin.resolve()


def resolve_policy_training_dir(anchor: str | Path) -> Path:
    override = _path_from_env("ICT_ENGINE_POLICY_TRAINING_DIR")
    if override is not None:
        return override
    return (resolve_repo_root(anchor) / "state" / "policy_training").resolve()


def cleaned_data_root_ready(anchor: str | Path, data_root: str | Path | None = None) -> bool:
    root = (
        Path(data_root).expanduser().resolve()
        if data_root is not None
        else resolve_cleaned_data_root(anchor)
    )
    expected = [
        root / "cleaned-1m",
        root / "cleaned-5m",
        root / "cleaned-15m",
        root / "cleaned-1h",
        root / "cleaned-4h",
        root / "cleaned-1d",
    ]
    return all(path.exists() for path in expected)
