from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Any

import bbn_evidence_value_report as bbn_value
import factor_payoff_shape_report as payoff
import factor_formula_library as formula_library
import labeling_triple_barrier as labeling
import paper2code_adapters as paper2code
import payoff_to_path_ranker_target as path_target
import purged_cv_backtest_guard as purged_cv


DEFAULT_PROFILE: dict[str, Any] = {
    "profile_id": "ict-default-v1",
    "enabled": True,
    "pt_mult": 0.02,
    "sl_mult": 0.01,
    "max_holding_bars": 16,
    "cost_bps": 0.0,
    "nb_trials": 1,
    "periods_per_year": 252,
    "purged_cv_enabled": True,
    "embargo_bars": 1,
    "fold_count": 4,
    "formula_families": [],
    "bbn_evidence_rows_jsonl": "",
    "auxiliary_fields": [
        "qqq_hv_level",
        "nq_vs_200d_pct",
        "vix3m_level",
        "qqq_hv_pct_rank_252",
        "vvix_over_vix",
    ],
    "artifact_names": {
        "labels": "labels.jsonl",
        "payoff_report": "payoff_report.json",
        "handoff_summary": "handoff_summary.json",
    },
}


def _load_profile(path: Path | None) -> dict[str, Any]:
    profile = dict(DEFAULT_PROFILE)
    profile["artifact_names"] = dict(DEFAULT_PROFILE["artifact_names"])
    if path is None:
        return profile
    override = json.loads(path.read_text(encoding="utf-8"))
    profile.update(override)
    if "artifact_names" in override:
        artifact_names = dict(DEFAULT_PROFILE["artifact_names"])
        artifact_names.update(override["artifact_names"])
        profile["artifact_names"] = artifact_names
    return profile


def _read_csv(path: Path) -> list[dict[str, Any]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def _load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=False) + "\n")


def run_pipeline(
    *,
    input_csv: Path,
    output_dir: Path,
    symbol: str,
    candidate_id: str,
    profile_json: Path | None = None,
) -> dict[str, Any]:
    """Run zero-config payoff labeling into an isolated output directory."""
    profile = _load_profile(profile_json)
    if not bool(profile.get("enabled", True)):
        result = {
            "ok": True,
            "skipped": True,
            "reason": "profile_disabled",
            "symbol": symbol,
            "candidate_id": candidate_id,
            "profile": profile,
        }
        _write_json(output_dir / "handoff_summary.json", result)
        return result

    rows = _read_csv(input_csv)
    labels = labeling.triple_barrier_labels(
        rows,
        pt_mult=float(profile["pt_mult"]),
        sl_mult=float(profile["sl_mult"]),
        max_holding_bars=int(profile["max_holding_bars"]),
        cost_bps=float(profile.get("cost_bps", 0.0)),
    )
    report = payoff.build_payoff_shape_report(
        candidate_id=candidate_id,
        trades=labels,
        nb_trials=int(profile.get("nb_trials", 1)),
        periods_per_year=int(profile.get("periods_per_year", 252)),
    )

    artifact_names = profile["artifact_names"]
    labels_path = output_dir / artifact_names["labels"]
    report_path = output_dir / artifact_names["payoff_report"]
    summary_path = output_dir / artifact_names["handoff_summary"]
    _write_jsonl(labels_path, labels)
    _write_json(report_path, report)
    purged_cv_guard = purged_cv.build_guard_report(
        labels=labels,
        nb_trials=int(profile.get("nb_trials", 1)),
        embargo_bars=int(profile.get("embargo_bars", 1)),
        fold_count=int(profile.get("fold_count", 4)),
    ) if bool(profile.get("purged_cv_enabled", True)) else {
        "schema_version": "purged-cv-backtest-guard/v1",
        "purged_cv_gate": "disabled",
        "leakage_flags": [],
    }
    purged_cv_path = output_dir / "purged_cv_guard.json"
    _write_json(purged_cv_path, purged_cv_guard)
    report.update(
        {
            "pbo": purged_cv_guard.get("pbo"),
            "oos_sharpe_lcb": purged_cv_guard.get("oos_sharpe_lcb"),
            "embargo_bars": purged_cv_guard.get("embargo_bars"),
            "leakage_flags": purged_cv_guard.get("leakage_flags", []),
            "purged_cv_gate": purged_cv_guard.get("purged_cv_gate"),
        }
    )
    _write_json(report_path, report)
    path_ranker_handoff = path_target.export_targets(
        labels_jsonl=labels_path,
        payoff_report_json=report_path,
        output_dir=output_dir,
        symbol=symbol,
        auxiliary_fields=list(profile.get("auxiliary_fields", [])),
    )
    formula_library_payload = formula_library.build_formula_library(families=list(profile.get("formula_families", [])))
    formula_library_path = output_dir / "factor_formula_library.json"
    _write_json(formula_library_path, formula_library_payload)
    paper2code_report = paper2code.build_adapter_report(rows=rows, candidate_id=candidate_id)
    paper2code_path = output_dir / "paper2code_adapter_report.json"
    _write_json(paper2code_path, paper2code_report)
    sidecar_closure = {
        "schema_version": "heuristic-sidecar-closure/v1",
        "formula_library": formula_library_payload,
        "paper2code_adapter_report": paper2code_report,
        "artifact_paths": {
            "factor_formula_library": str(formula_library_path),
            "paper2code_adapter_report": str(paper2code_path),
        },
    }
    bbn_rows_path = str(profile.get("bbn_evidence_rows_jsonl", "")).strip()
    if bbn_rows_path:
        bbn_report = bbn_value.build_evidence_value_report(
            rows=_load_jsonl(Path(bbn_rows_path)),
            candidate_id=candidate_id,
        )
        bbn_report_path = output_dir / "bbn_evidence_value_report.json"
        _write_json(bbn_report_path, bbn_report)
        sidecar_closure["bbn_evidence_value_report"] = bbn_report
        sidecar_closure["artifact_paths"]["bbn_evidence_value_report"] = str(bbn_report_path)

    result = {
        "ok": True,
        "skipped": False,
        "symbol": symbol,
        "candidate_id": candidate_id,
        "profile": profile,
        "input_csv": str(input_csv),
        "output_dir": str(output_dir),
        "artifact_paths": {
            "labels": str(labels_path),
            "payoff_report": str(report_path),
            "purged_cv_guard": str(purged_cv_path),
            "handoff_summary": str(summary_path),
        },
        "label_count": len(labels),
        "payoff_gate": report["promotion_gate"],
        "failure_tags": report["failure_tags"],
        "purged_cv_guard": purged_cv_guard,
        "path_ranker_handoff": path_ranker_handoff,
        "sidecar_closure": sidecar_closure,
        "next_recommended_layer": "regime_bbn_path_ranker" if report["promotion_gate"] != "reject" else "rewrite_factor_or_data",
    }
    _write_json(summary_path, result)
    return result


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Zero-config heuristic payoff labeling pipeline.")
    parser.add_argument("--input-csv", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--symbol", required=True)
    parser.add_argument("--candidate-id", required=True)
    parser.add_argument("--profile-json")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    result = run_pipeline(
        input_csv=Path(args.input_csv).resolve(),
        output_dir=Path(args.output_dir).resolve(),
        symbol=args.symbol,
        candidate_id=args.candidate_id,
        profile_json=Path(args.profile_json).resolve() if args.profile_json else None,
    )
    print(json.dumps(result, indent=2, sort_keys=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())