from __future__ import annotations

import argparse
import csv
import json
import re
import shutil
import subprocess
from collections import Counter
from pathlib import Path
from typing import Any, Callable, Iterable


TSV_FIELDS = [
    "window",
    "gate_status",
    "branch",
    "execution_bias",
    "execution_score",
    "decision_hint",
    "execution_readiness",
    "prediction_vote_score",
    "hybrid_transition_hazard",
    "duration_remaining_expected_bars",
    "branch_probability",
    "path_ranker_score_used_by_execution_tree",
    "path_ranker_score_visible_to_execution_tree",
    "ranker_score_path_id",
    "ranker_score_runtime_source",
    "ranker_score_raw_path_score",
    "ranker_score_calibrated_path_prob",
    "ranker_score_path_prob_lower_bound",
    "ranker_score_execution_gate_status",
    "path_ranker_runtime_source",
    "path_ranker_model_family",
    "ranker_validation_ready",
    "readiness_gap_to_observe",
    "readiness_gap_to_ready",
    "top_positive_feature",
    "top_positive_contribution",
    "top_negative_feature",
    "top_negative_contribution",
    "pythagorean_overstretch",
    "ising_phase_transition_risk",
    "spectral_entropy",
    "dominant_cycle_energy",
    "cycle_phase_alignment",
    "consumer_reason",
]

NUMERIC_FIELDS = [
    "execution_readiness",
    "prediction_vote_score",
    "hybrid_transition_hazard",
    "duration_remaining_expected_bars",
    "branch_probability",
    "execution_score",
    "ranker_score_raw_path_score",
    "ranker_score_calibrated_path_prob",
    "ranker_score_path_prob_lower_bound",
    "readiness_gap_to_observe",
    "readiness_gap_to_ready",
    "top_positive_contribution",
    "top_negative_contribution",
    "pythagorean_overstretch",
    "ising_phase_transition_risk",
    "spectral_entropy",
    "dominant_cycle_energy",
    "cycle_phase_alignment",
]


def _as_float(value: Any) -> float | None:
    if value is None:
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def _fmt_float(value: Any) -> str:
    number = _as_float(value)
    if number is None:
        return ""
    return f"{number:.6f}"


def _fmt_bool(value: Any) -> str:
    if value is True:
        return "true"
    if value is False:
        return "false"
    return ""


def _shap_rows(trace: dict[str, Any]) -> list[dict[str, Any]]:
    rows = trace.get("execution_shap_top_k", [])
    if not isinstance(rows, list) or not rows:
        rows = trace.get("output", {}).get("execution_shap_top_k", [])
    if not isinstance(rows, list):
        return []
    return [row for row in rows if isinstance(row, dict)]


def _shap_feature_value(rows: list[dict[str, Any]], feature: str) -> float | None:
    for row in rows:
        if row.get("feature") == feature:
            return _as_float(row.get("feature_value"))
    return None


def _top_contribution(
    rows: list[dict[str, Any]], *, positive: bool
) -> tuple[str, float | None]:
    best_feature = ""
    best_value: float | None = None
    for row in rows:
        contribution = _as_float(row.get("contribution"))
        if contribution is None:
            continue
        if positive and contribution <= 0:
            continue
        if not positive and contribution >= 0:
            continue
        if best_value is None:
            best_feature = str(row.get("feature") or "")
            best_value = contribution
            continue
        if positive and contribution > best_value:
            best_feature = str(row.get("feature") or "")
            best_value = contribution
        elif not positive and contribution < best_value:
            best_feature = str(row.get("feature") or "")
            best_value = contribution
    return best_feature, best_value


def _lineage(trace: dict[str, Any]) -> list[str]:
    lines = trace.get("output", {}).get("split_reason_lineage", [])
    if isinstance(lines, list):
        return [str(line) for line in lines]
    return []


def _extract_lineage_float(lines: Iterable[str], pattern: str) -> float | None:
    compiled = re.compile(pattern)
    for line in lines:
        match = compiled.search(line)
        if match:
            return _as_float(match.group(1))
    return None


def _lineage_value(line: str, key: str) -> str | None:
    needle = f"{key}="
    for part in line.split():
        if needle in part:
            value = part.split(needle, 1)[1]
            return value.strip().strip(",;")
    return None


def _extract_ranker_score_fields(lines: Iterable[str]) -> dict[str, str]:
    line = next((item for item in lines if "ranker_score=" in item), "")
    if not line:
        return {
            "ranker_score_path_id": "",
            "ranker_score_runtime_source": "",
            "ranker_score_raw_path_score": "",
            "ranker_score_calibrated_path_prob": "",
            "ranker_score_path_prob_lower_bound": "",
            "ranker_score_execution_gate_status": "",
        }

    def value(key: str) -> str:
        found = _lineage_value(line, key)
        if found is None or found == "none":
            return ""
        return found

    return {
        "ranker_score_path_id": value("path_id"),
        "ranker_score_runtime_source": value("runtime_source"),
        "ranker_score_raw_path_score": _fmt_float(value("raw_path_score")),
        "ranker_score_calibrated_path_prob": _fmt_float(value("calibrated_path_prob")),
        "ranker_score_path_prob_lower_bound": _fmt_float(value("path_prob_lower_bound")),
        "ranker_score_execution_gate_status": value("execution_gate_status"),
    }


def summarize_window(
    window_id: str,
    analyze_payload: dict[str, Any],
    trace_payload: dict[str, Any],
) -> dict[str, str]:
    output = trace_payload.get("output") if isinstance(trace_payload.get("output"), dict) else {}
    triage = (
        analyze_payload.get("execution_triage")
        if isinstance(analyze_payload.get("execution_triage"), dict)
        else {}
    )
    lines = _lineage(trace_payload)
    execution_readiness = _extract_lineage_float(
        lines, r"execution_readiness=([-+]?\d+(?:\.\d+)?)"
    )
    prediction_vote_score = _extract_lineage_float(
        lines, r"prediction_vote_score=([-+]?\d+(?:\.\d+)?)"
    )
    hybrid_transition_hazard = _extract_lineage_float(
        lines, r"hybrid_transition_hazard=([-+]?\d+(?:\.\d+)?)"
    )
    duration_remaining_expected_bars = _extract_lineage_float(
        lines, r"duration_remaining_expected_bars=([-+]?\d+(?:\.\d+)?)"
    )
    ranker_score_fields = _extract_ranker_score_fields(lines)
    shap_rows = _shap_rows(trace_payload)
    top_positive_feature, top_positive_contribution = _top_contribution(
        shap_rows, positive=True
    )
    top_negative_feature, top_negative_contribution = _top_contribution(
        shap_rows, positive=False
    )
    readiness_number = _as_float(execution_readiness)
    readiness_gap_to_observe = (
        max(0.0, 0.45 - readiness_number)
        if readiness_number is not None
        else None
    )
    readiness_gap_to_ready = (
        max(0.0, 0.65 - readiness_number)
        if readiness_number is not None
        else None
    )

    return {
        "window": window_id,
        "gate_status": str(output.get("gate_status") or triage.get("gate_status") or ""),
        "branch": str(output.get("branch") or triage.get("branch") or ""),
        "execution_bias": str(
            output.get("execution_bias") or triage.get("execution_bias") or ""
        ),
        "execution_score": _fmt_float(
            output.get("execution_score", triage.get("execution_score"))
        ),
        "decision_hint": str(output.get("decision_hint") or triage.get("decision_hint") or ""),
        "execution_readiness": _fmt_float(execution_readiness),
        "prediction_vote_score": _fmt_float(prediction_vote_score),
        "hybrid_transition_hazard": _fmt_float(hybrid_transition_hazard),
        "duration_remaining_expected_bars": _fmt_float(duration_remaining_expected_bars),
        "branch_probability": _fmt_float(
            output.get("branch_probability", triage.get("branch_probability"))
        ),
        "path_ranker_score_used_by_execution_tree": _fmt_bool(
            output.get("path_ranker_score_used_by_execution_tree")
        ),
        "path_ranker_score_visible_to_execution_tree": _fmt_bool(
            output.get("path_ranker_score_visible_to_execution_tree")
        ),
        **ranker_score_fields,
        "path_ranker_runtime_source": str(output.get("path_ranker_runtime_source") or ""),
        "path_ranker_model_family": str(output.get("path_ranker_model_family") or ""),
        "ranker_validation_ready": _fmt_bool(output.get("ranker_validation_ready")),
        "readiness_gap_to_observe": _fmt_float(readiness_gap_to_observe),
        "readiness_gap_to_ready": _fmt_float(readiness_gap_to_ready),
        "top_positive_feature": top_positive_feature,
        "top_positive_contribution": _fmt_float(top_positive_contribution),
        "top_negative_feature": top_negative_feature,
        "top_negative_contribution": _fmt_float(top_negative_contribution),
        "pythagorean_overstretch": _fmt_float(
            _shap_feature_value(shap_rows, "pythagorean_overstretch")
        ),
        "ising_phase_transition_risk": _fmt_float(
            _shap_feature_value(shap_rows, "ising_phase_transition_risk")
        ),
        "spectral_entropy": _fmt_float(_shap_feature_value(shap_rows, "spectral_entropy")),
        "dominant_cycle_energy": _fmt_float(
            _shap_feature_value(shap_rows, "dominant_cycle_energy")
        ),
        "cycle_phase_alignment": _fmt_float(
            _shap_feature_value(shap_rows, "cycle_phase_alignment")
        ),
        "consumer_reason": str(output.get("consumer_reason") or triage.get("consumer_reason") or ""),
    }


def _window_id(path: Path) -> str:
    match = re.search(r"obs_(\d+)", path.stem)
    if match:
        return match.group(1)
    return path.stem


def _json_loads(text: str) -> dict[str, Any]:
    if not text.strip():
        return {}
    payload = json.loads(text)
    if isinstance(payload, dict):
        return payload
    return {}


def _write_tsv(path: Path, rows: list[dict[str, str]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=TSV_FIELDS, delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)


def _quantile(values: list[float], q: float) -> float | None:
    if not values:
        return None
    index = round((len(values) - 1) * q)
    return values[index]


def metric_summary(rows: list[dict[str, str]]) -> dict[str, dict[str, float | int | None]]:
    summary: dict[str, dict[str, float | int | None]] = {}
    for field in NUMERIC_FIELDS:
        values = sorted(
            value
            for row in rows
            if (value := _as_float(row.get(field))) is not None
        )
        if not values:
            summary[field] = {
                "count": 0,
                "min": None,
                "p25": None,
                "median": None,
                "p75": None,
                "max": None,
                "mean": None,
            }
            continue
        summary[field] = {
            "count": len(values),
            "min": min(values),
            "p25": _quantile(values, 0.25),
            "median": _quantile(values, 0.50),
            "p75": _quantile(values, 0.75),
            "max": max(values),
            "mean": sum(values) / len(values),
        }
    return summary


def run_scan(
    *,
    ict_engine_bin: Path,
    windows_dir: Path,
    state_dir: Path,
    symbol: str,
    output_dir: Path,
    runner: Callable[..., Any] = subprocess.run,
) -> dict[str, Any]:
    output_dir.mkdir(parents=True, exist_ok=True)
    rows: list[dict[str, str]] = []
    window_paths = sorted(windows_dir.glob("*_obs_*.json"))
    for window_path in window_paths:
        window = _window_id(window_path)
        command = [
            str(ict_engine_bin),
            "analyze",
            "--symbol",
            symbol,
            "--data-htf",
            str(window_path),
            "--data-mtf",
            str(window_path),
            "--data-ltf",
            str(window_path),
            "--state-dir",
            str(state_dir),
            "--agent",
        ]
        result = runner(command, cwd=str(Path.cwd()), text=True, capture_output=True)
        analyze_path = output_dir / f"analyze_{window}.json"
        analyze_path.write_text(result.stdout, encoding="utf-8")
        if result.returncode != 0:
            stderr_path = output_dir / f"analyze_{window}.stderr.txt"
            stderr_path.write_text(result.stderr, encoding="utf-8")
            raise RuntimeError(
                f"ict-engine analyze failed for window {window} with code {result.returncode}; "
                f"stderr saved to {stderr_path}"
            )

        trace_path = state_dir / symbol / "execution_tree_trace.json"
        if not trace_path.exists():
            raise FileNotFoundError(f"missing execution tree trace after window {window}: {trace_path}")
        trace_copy_path = output_dir / f"execution_tree_trace_{window}.json"
        shutil.copyfile(trace_path, trace_copy_path)

        rows.append(
            summarize_window(
                window,
                _json_loads(result.stdout),
                _json_loads(trace_copy_path.read_text(encoding="utf-8")),
            )
        )

    _write_tsv(output_dir / "scan.tsv", rows)
    hint_counts = Counter(row["decision_hint"] for row in rows)
    gate_counts = Counter(row["gate_status"] for row in rows)
    branch_counts = Counter(row["branch"] for row in rows)
    summary = {
        "schema_version": "execution-tree-guardrail-scan/v1",
        "windows_scanned": len(rows),
        "scan_tsv": str(output_dir / "scan.tsv"),
        "gate_status_counts": dict(sorted(gate_counts.items())),
        "branch_counts": dict(sorted(branch_counts.items())),
        "decision_hint_counts": dict(sorted(hint_counts.items())),
        "metric_summary": metric_summary(rows),
    }
    (output_dir / "scan_summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return summary


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--ict-engine-bin", type=Path, default=Path("./target/debug/ict-engine"))
    parser.add_argument("--windows-dir", type=Path, required=True)
    parser.add_argument("--state-dir", type=Path, required=True)
    parser.add_argument("--symbol", default="NQ")
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args(argv)

    summary = run_scan(
        ict_engine_bin=args.ict_engine_bin,
        windows_dir=args.windows_dir,
        state_dir=args.state_dir,
        symbol=args.symbol,
        output_dir=args.output_dir,
    )
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
