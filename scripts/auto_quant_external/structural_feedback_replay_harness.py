from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parents[1]
ICT_ENGINE_BIN = REPO_ROOT / "target" / "debug" / "ict-engine"
ENRICHER = SCRIPT_DIR / "structural_feedback_trade_enricher.py"
PATH_RANKER_TRAINER = SCRIPT_DIR / "pandas_path_ranker_trainer.py"


def load_candles(path: Path) -> list[dict[str, Any]]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(payload, dict):
        candles = payload.get("candles", [])
    else:
        candles = payload
    if not isinstance(candles, list) or not candles:
        raise ValueError(f"no candles found in {path}")
    return candles


def write_candles(path: Path, symbol: str, candles: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"symbol": symbol, "candles": candles}, indent=2) + "\n", encoding="utf-8")


def run(cmd: list[str], *, cwd: Path = REPO_ROOT) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(cmd, cwd=str(cwd), capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(cmd)}\nSTDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}")
    return result


def outcome_from_forward_window(candles: list[dict[str, Any]], entry_index: int, horizon: int, threshold: float) -> tuple[str, float, float]:
    entry_close = float(candles[entry_index]["close"])
    future = candles[entry_index + 1 : entry_index + 1 + horizon]
    if not future:
        return "breakeven", 0.0, entry_close
    exit_close = float(future[-1]["close"])
    pnl = (exit_close / entry_close) - 1.0
    max_up = max((float(row["high"]) / entry_close) - 1.0 for row in future)
    max_down = min((float(row["low"]) / entry_close) - 1.0 for row in future)
    if pnl > threshold:
        return "win", pnl, exit_close
    if pnl < -threshold:
        return "loss", pnl, exit_close
    if max_down < -threshold * 2.0 and pnl <= 0.0:
        return "invalidated", pnl, exit_close
    return "breakeven", pnl, exit_close


def copy_prior_state(prior_state: Path | None, target_state: Path) -> None:
    if not prior_state:
        return
    if prior_state.resolve() == target_state.resolve():
        return
    if target_state.exists():
        shutil.rmtree(target_state)
    shutil.copytree(prior_state, target_state)


def generate_observation(
    *,
    symbol: str,
    candles: list[dict[str, Any]],
    output_root: Path,
    prior_state: Path | None,
    index: int,
    lookback: int,
    horizon: int,
    threshold: float,
    observation_id: int,
) -> dict[str, Any]:
    state_dir = output_root / "state"
    copy_prior_state(prior_state, state_dir)
    data_path = output_root / "windows" / f"{symbol.lower()}_15m_obs_{observation_id:02d}.json"
    feedback_path = output_root / "feedback" / f"structural_feedback_obs_{observation_id:02d}.json"
    start = index - lookback + 1
    if start < 0 or index + horizon >= len(candles):
        raise ValueError(f"invalid window index={index} lookback={lookback} horizon={horizon}")
    write_candles(data_path, symbol, candles[start : index + 1])

    run([
        str(ICT_ENGINE_BIN),
        "analyze",
        "--symbol",
        symbol,
        "--data-ltf",
        str(data_path),
        "--data-mtf",
        str(data_path),
        "--data-htf",
        str(data_path),
        "--state-dir",
        str(state_dir),
        "--human",
    ])
    run([
        str(ICT_ENGINE_BIN),
        "export-structural-path-ranking-target",
        "--symbol",
        symbol,
        "--state-dir",
        str(state_dir),
    ])

    target_csv = state_dir / symbol / "policy_training" / "structural_path_ranking_target.csv"
    model_dir = state_dir / symbol / "policy_training" / "path_ranker_model"
    scores_path = output_root / "scores" / f"scores_obs_{observation_id:02d}.csv"
    scores_path.parent.mkdir(parents=True, exist_ok=True)
    if model_dir.exists():
        run([
            sys.executable,
            str(PATH_RANKER_TRAINER),
            "--apply",
            "--model-dir",
            str(model_dir),
            "--target-csv",
            str(target_csv),
            "--output-scores",
            str(scores_path),
        ])
        run([
            str(ICT_ENGINE_BIN),
            "apply-structural-path-ranking-external-scores",
            "--symbol",
            symbol,
            "--state-dir",
            str(state_dir),
            "--scores-file",
            str(scores_path),
        ])
        run([
            str(ICT_ENGINE_BIN),
            "export-structural-path-ranking-target",
            "--symbol",
            symbol,
            "--state-dir",
            str(state_dir),
        ])
    outcome, pnl, exit_close = outcome_from_forward_window(candles, index, horizon, threshold)
    run([
        sys.executable,
        str(ENRICHER),
        "emit-probe",
        "--target-csv",
        str(target_csv),
        "--output",
        str(feedback_path),
        "--rank",
        "1",
        "--realized-outcome",
        outcome,
        "--pnl",
        str(pnl),
        "--exit-reason",
        f"forward_{horizon}_bar_close",
        "--notes",
        f"semi_auto_replay observation={observation_id} data_index={index} threshold={threshold}",
    ])
    update = run([
        str(ICT_ENGINE_BIN),
        "update",
        "--symbol",
        symbol,
        "--outcome",
        outcome,
        "--entry-signal",
        "medium",
        "--state-dir",
        str(state_dir),
        f"--pnl={pnl}",
        "--feedback-file",
        str(feedback_path),
    ])
    export = run([
        str(ICT_ENGINE_BIN),
        "export-structural-path-ranking-target",
        "--symbol",
        symbol,
        "--state-dir",
        str(state_dir),
    ])
    summary = json.loads(export.stdout)
    return {
        "observation_id": observation_id,
        "data_path": str(data_path),
        "feedback_path": str(feedback_path),
        "window_start": candles[start]["timestamp"],
        "entry_timestamp": candles[index]["timestamp"],
        "exit_timestamp": candles[index + horizon]["timestamp"],
        "entry_close": float(candles[index]["close"]),
        "exit_close": exit_close,
        "outcome": outcome,
        "pnl": pnl,
        "mature_rows": summary.get("mature_rows"),
        "history_mature_rows": summary.get("history_mature_rows"),
        "summary_line": summary.get("summary_line"),
        "update_stdout_bytes": len(update.stdout),
    }


def run_replay(
    *,
    candles_path: Path,
    output_root: Path,
    symbol: str,
    count: int,
    lookback: int,
    horizon: int,
    threshold: float,
    prior_state: Path | None,
) -> dict[str, Any]:
    candles = load_candles(candles_path)
    output_root.mkdir(parents=True, exist_ok=True)
    min_index = lookback - 1
    max_index = len(candles) - horizon - 1
    if max_index <= min_index:
        raise ValueError("not enough candles for replay")
    step = max(1, (max_index - min_index) // count)
    indices = [min_index + step * idx for idx in range(count)]
    observations = []
    for obs_id, index in enumerate(indices, start=1):
        observations.append(
            generate_observation(
                symbol=symbol,
                candles=candles,
                output_root=output_root,
                prior_state=prior_state if obs_id == 1 else output_root / "state",
                index=index,
                lookback=lookback,
                horizon=horizon,
                threshold=threshold,
                observation_id=obs_id,
            )
        )
    summary = {
        "ok": True,
        "symbol": symbol,
        "candles_path": str(candles_path),
        "output_root": str(output_root),
        "count": len(observations),
        "lookback": lookback,
        "horizon": horizon,
        "threshold": threshold,
        "final_mature_rows": observations[-1]["mature_rows"] if observations else None,
        "observations": observations,
    }
    (output_root / "replay_summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    return summary


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Semi-auto structural feedback replay harness")
    parser.add_argument("--candles", required=True)
    parser.add_argument("--output-root", required=True)
    parser.add_argument("--symbol", default="NQ")
    parser.add_argument("--count", type=int, default=29)
    parser.add_argument("--lookback", type=int, default=52)
    parser.add_argument("--horizon", type=int, default=16)
    parser.add_argument("--threshold", type=float, default=0.001)
    parser.add_argument("--prior-state")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    summary = run_replay(
        candles_path=Path(args.candles),
        output_root=Path(args.output_root),
        symbol=args.symbol,
        count=args.count,
        lookback=args.lookback,
        horizon=args.horizon,
        threshold=args.threshold,
        prior_state=Path(args.prior_state) if args.prior_state else None,
    )
    print(json.dumps({k: v for k, v in summary.items() if k != "observations"}, indent=2))
    print(f"[done] observations={len(summary['observations'])} final_mature_rows={summary['final_mature_rows']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
