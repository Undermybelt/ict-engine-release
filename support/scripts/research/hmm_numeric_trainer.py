#!/usr/bin/env python3
"""Bounded offline HMM numeric trainer harness.

This script explores a tiny explicit grid over named HMM numeric knobs,
executes ict-engine's existing analyze path in isolated state dirs, and emits:

- hmm_numeric_trainer_artifact.json
- candidate_history.jsonl
- replay_summary.json

It never writes repo-default state. All runtime state stays under the caller's
explicit --state-dir.
"""

from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

sys_path_root = Path(__file__).resolve().parents[2]
if str(sys_path_root) not in sys.path:
    sys.path.insert(0, str(sys_path_root))
from path_defaults import resolve_binary_path  # noqa: E402


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run a bounded offline HMM numeric trainer over explicit state dirs."
    )
    parser.add_argument("--symbol", required=True)
    parser.add_argument("--data", required=True, help="Single cleaned candle JSON reused for htf/mtf/ltf")
    parser.add_argument("--state-dir", required=True)
    parser.add_argument("--out", required=True, help="Output hmm_numeric_trainer_artifact.json path")
    parser.add_argument("--bin", dest="bin_path", help="Override ict-engine binary path")
    parser.add_argument("--max-candidates", type=int, default=8)
    return parser.parse_args()


def resolve_binary(args: argparse.Namespace) -> Path:
    if args.bin_path:
        return Path(args.bin_path).expanduser().resolve()
    env_bin = os.environ.get("ICT_ENGINE_BIN")
    if env_bin:
        return Path(env_bin).expanduser().resolve()
    return resolve_binary_path(__file__)


def data_hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:16]


def build_candidates(max_candidates: int) -> list[dict[str, float]]:
    transition_smoothing = [0.0, 0.2, 0.4, 0.6]
    emission_std_floor = [0.5, 1.0, 1.5]
    posterior_temperature = [0.9, 1.1]
    candidates = []
    for idx, (smooth, floor, temp) in enumerate(
        itertools.product(transition_smoothing, emission_std_floor, posterior_temperature)
    ):
        if idx >= max_candidates:
            break
        candidates.append(
            {
                "transition_smoothing": smooth,
                "emission_std_floor": floor,
                "posterior_temperature": temp,
            }
        )
    return candidates


def write_hmm_artifact(state_dir: Path, symbol: str, params: dict[str, float], source_hash: str, split_id: str, iteration: int) -> Path:
    symbol_dir = state_dir / symbol
    symbol_dir.mkdir(parents=True, exist_ok=True)
    artifact = {
        "protocol_version": "hmm-numeric-trainer-artifact-v1",
        "parameter_vector": [
            params["transition_smoothing"],
            params["emission_std_floor"],
            params["posterior_temperature"],
        ],
        "parameter_names": [
            "transition_smoothing",
            "emission_std_floor",
            "posterior_temperature",
        ],
        "bounds": [
            {"name": "transition_smoothing", "lower": 0.0, "upper": 1.0},
            {"name": "emission_std_floor", "lower": 0.1, "upper": 5.0},
            {"name": "posterior_temperature", "lower": 0.1, "upper": 5.0},
        ],
        "objective_breakdown": {},
        "seed": 20260505,
        "split_id": split_id,
        "best_iteration": iteration,
        "source_data_hash": source_hash,
        "state_count": 3,
    }
    path = symbol_dir / "hmm_numeric_trainer_artifact.json"
    path.write_text(json.dumps(artifact, indent=2, sort_keys=True) + "\n")
    return path


def run_analyze(binary: Path, symbol: str, data_path: Path, state_dir: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            str(binary),
            "analyze",
            "--symbol",
            symbol,
            "--data-htf",
            str(data_path),
            "--data-mtf",
            str(data_path),
            "--data-ltf",
            str(data_path),
            "--state-dir",
            str(state_dir),
            "--human",
        ],
        capture_output=True,
        text=True,
        check=False,
    )


def parse_execution_histogram(summary: str) -> dict[str, int]:
    out: dict[str, int] = {}
    for item in summary.split(";"):
        if "=" not in item:
            continue
        key, value = item.split("=", 1)
        key = key.strip()
        value = value.strip()
        if not key or not value.isdigit():
            continue
        out[key] = int(value)
    return out


def load_mece_artifact(candidate_state_dir: Path, symbol: str) -> dict[str, Any]:
    path = candidate_state_dir / symbol / "mece_recovery_artifact.json"
    if not path.exists():
        raise FileNotFoundError(path)
    return json.loads(path.read_text())


def objective_from_artifact(artifact: dict[str, Any]) -> tuple[float, dict[str, float]]:
    accuracy = float(artifact.get("accuracy", 0.0))
    macro_f1 = float(artifact.get("macro_f1", 0.0))
    histogram = parse_execution_histogram(str(artifact.get("execution_validity_summary", "")))
    execution_coverage = min(len([count for count in histogram.values() if count > 0]) / 3.0, 1.0)
    segments = artifact.get("segments", []) or []
    segment_quality = 0.0
    if segments:
        segment_quality = sum(float(segment.get("accuracy", 0.0)) for segment in segments) / len(segments)
    breakdown = {
        "accuracy": accuracy,
        "macro_f1": macro_f1,
        "execution_validity_histogram": execution_coverage,
        "rollout_segment_quality": segment_quality,
    }
    score = (
        accuracy * 0.50
        + macro_f1 * 0.20
        + execution_coverage * 0.15
        + segment_quality * 0.15
    )
    return score, breakdown


def main() -> None:
    args = parse_args()
    binary = resolve_binary(args)
    data_path = Path(args.data).expanduser().resolve()
    state_root = Path(args.state_dir).expanduser().resolve()
    out_path = Path(args.out).expanduser().resolve()
    out_dir = out_path.parent
    out_dir.mkdir(parents=True, exist_ok=True)
    if not binary.exists():
        raise SystemExit(f"ict-engine binary not found: {binary}")
    if not data_path.exists():
        raise SystemExit(f"data path not found: {data_path}")

    source_hash = data_hash(data_path)
    split_id = f"{args.symbol}:demo-grid:{source_hash}"
    history_path = out_dir / "candidate_history.jsonl"
    replay_summary_path = out_dir / "replay_summary.json"

    best_candidate: dict[str, Any] | None = None
    candidate_history: list[dict[str, Any]] = []

    for iteration, params in enumerate(build_candidates(args.max_candidates), start=1):
        candidate_state_dir = state_root / "hmm_numeric_candidates" / f"candidate_{iteration:03d}"
        write_hmm_artifact(candidate_state_dir, args.symbol, params, source_hash, split_id, iteration)
        run = run_analyze(binary, args.symbol, data_path, candidate_state_dir)
        artifact_path = candidate_state_dir / args.symbol / "mece_recovery_artifact.json"
        if run.returncode != 0 or not artifact_path.exists():
            candidate = {
                "iteration": iteration,
                "status": "analyze_failed",
                "returncode": run.returncode,
                "params": params,
                "stdout_tail": run.stdout.strip().splitlines()[-5:],
                "stderr_tail": run.stderr.strip().splitlines()[-5:],
                "candidate_state_dir": str(candidate_state_dir),
            }
            candidate_history.append(candidate)
            continue
        artifact = load_mece_artifact(candidate_state_dir, args.symbol)
        objective_score, breakdown = objective_from_artifact(artifact)
        candidate = {
            "iteration": iteration,
            "status": "ok",
            "params": params,
            "objective_score": objective_score,
            "objective_breakdown": breakdown,
            "mece_recovery_artifact_path": str(artifact_path),
            "candidate_state_dir": str(candidate_state_dir),
        }
        candidate_history.append(candidate)
        if best_candidate is None or objective_score > best_candidate["objective_score"]:
            best_candidate = candidate

    history_path.write_text(
        "".join(json.dumps(row, sort_keys=True) + "\n" for row in candidate_history)
    )

    if best_candidate is None:
        raise SystemExit("no successful HMM numeric trainer candidates were produced")

    best_params = best_candidate["params"]
    final_artifact = {
        "protocol_version": "hmm-numeric-trainer-artifact-v1",
        "parameter_vector": [
            best_params["transition_smoothing"],
            best_params["emission_std_floor"],
            best_params["posterior_temperature"],
        ],
        "parameter_names": [
            "transition_smoothing",
            "emission_std_floor",
            "posterior_temperature",
        ],
        "bounds": [
            {"name": "transition_smoothing", "lower": 0.0, "upper": 1.0},
            {"name": "emission_std_floor", "lower": 0.1, "upper": 5.0},
            {"name": "posterior_temperature", "lower": 0.1, "upper": 5.0},
        ],
        "objective_breakdown": best_candidate["objective_breakdown"],
        "seed": 20260505,
        "split_id": split_id,
        "best_iteration": best_candidate["iteration"],
        "source_data_hash": source_hash,
        "state_count": 3,
    }
    out_path.write_text(json.dumps(final_artifact, indent=2, sort_keys=True) + "\n")

    replay_summary = {
        "symbol": args.symbol,
        "data_path": str(data_path),
        "state_dir": str(state_root),
        "binary": str(binary),
        "candidate_count": len(candidate_history),
        "successful_candidates": len([row for row in candidate_history if row["status"] == "ok"]),
        "best_iteration": best_candidate["iteration"],
        "best_objective_score": best_candidate["objective_score"],
        "artifact_path": str(out_path),
        "candidate_history_path": str(history_path),
    }
    replay_summary_path.write_text(json.dumps(replay_summary, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
