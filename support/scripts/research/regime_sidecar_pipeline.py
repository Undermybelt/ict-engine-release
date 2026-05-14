from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Any

import regime_conformal_calibration_report as r6
import regime_consumer_bundle as r10
import regime_distributional_agreement_report as r7
import regime_expert_trainer as r5
import regime_feature_builder as r3
import regime_high_confidence_decision as r9
import regime_ontology_manifest as r2
import regime_transition_governor as r8


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _label_id_to_feature_truth(label_id: str) -> tuple[str, str] | None:
    if "::" not in label_id:
        return None
    level, label = label_id.split("::", 1)
    if not level or not label:
        return None
    return f"{level}_label", label


def _join_truth_labels_into_features(features_path: Path, truth_path: Path | None) -> int:
    if truth_path is None or not truth_path.exists():
        return 0
    truth_rows = [
        json.loads(line)
        for line in truth_path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    truth_by_timestamp = {
        str(row.get("timestamp", "")): _label_id_to_feature_truth(str(row.get("label_id", row.get("label", ""))))
        for row in truth_rows
    }
    with features_path.open(newline="", encoding="utf-8") as handle:
        rows = [dict(row) for row in csv.DictReader(handle)]
    if not rows:
        return 0
    columns = list(rows[0].keys())
    joined = 0
    for row in rows:
        mapped = truth_by_timestamp.get(str(row.get("timestamp", "")))
        if not mapped:
            continue
        field, label = mapped
        row[field] = label
        if field not in columns:
            columns.append(field)
        joined += 1
    if joined:
        with features_path.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.DictWriter(handle, fieldnames=columns)
            writer.writeheader()
            writer.writerows(rows)
    return joined


def _input_contract(output_dir: Path) -> dict[str, Any]:
    return {
        "schema_version": "regime-sidecar-pipeline/v1",
        "status": "input_required",
        "input_contract": {
            "required": ["--ohlcv"],
            "optional": ["--output-dir", "--label-prefix", "--auxiliary-evidence", "--truth"],
            "output_dir": str(output_dir),
            "no_repo_root_state": True,
        },
    }


def _artifact_paths(output_dir: Path) -> dict[str, Path]:
    return {
        "ontology_json": output_dir / "regime_ontology_manifest.json",
        "ontology_jsonl": output_dir / "regime_expert_bank_manifest.jsonl",
        "features": output_dir / "regime_features.csv",
        "feature_quality": output_dir / "feature_quality_report.json",
        "scores": output_dir / "regime_expert_scores.jsonl",
        "training_report": output_dir / "regime_expert_training_report.json",
        "conformal": output_dir / "regime_conformal_calibration_report.json",
        "distributional": output_dir / "regime_distributional_agreement_report.json",
        "governor": output_dir / "regime_transition_governor_report.json",
        "decision": output_dir / "regime_high_confidence_decision.json",
        "bundle": output_dir / "regime_consumer_bundle.json",
        "pipeline": output_dir / "regime_sidecar_pipeline_report.json",
    }


def run_pipeline(
    *,
    ohlcv_path: Path | None,
    output_dir: Path,
    auxiliary_evidence_path: Path | None = None,
    truth_path: Path | None = None,
    label_prefix: str = "primary::Trend",
) -> dict[str, Any]:
    if ohlcv_path is None:
        return _input_contract(output_dir)
    if not ohlcv_path.exists():
        result = _input_contract(output_dir)
        result["input_contract"]["missing_path"] = str(ohlcv_path)
        return result

    output_dir.mkdir(parents=True, exist_ok=True)
    paths = _artifact_paths(output_dir)

    manifest = r2.build_manifest()
    r2._write_json(paths["ontology_json"], manifest)
    r2._write_jsonl(paths["ontology_jsonl"], manifest["experts"])

    r3.build_feature_artifacts(
        ohlcv_path=ohlcv_path,
        output_features=paths["features"],
        output_report=paths["feature_quality"],
        auxiliary_path=auxiliary_evidence_path,
    )
    truth_joined_rows = _join_truth_labels_into_features(paths["features"], truth_path)
    r5.build_expert_training_artifacts(
        ontology_path=paths["ontology_json"],
        features_path=paths["features"],
        output_scores=paths["scores"],
        output_report=paths["training_report"],
        precision_first=False,
    )
    r6.build_conformal_calibration_report(
        scores_path=paths["scores"],
        training_report_path=paths["training_report"],
        truth_path=truth_path,
        label_prefix=label_prefix,
        output_json=paths["conformal"],
    )
    r7.build_distributional_agreement_report(
        features_path=paths["features"],
        scores_path=paths["scores"],
        conformal_report_path=paths["conformal"],
        label_prefix=label_prefix,
        output_json=paths["distributional"],
    )
    r8.build_transition_governor_report(
        scores_path=paths["scores"],
        conformal_report_path=paths["conformal"],
        distributional_report_path=paths["distributional"],
        label_prefix=label_prefix,
        min_duration=3,
        output_json=paths["governor"],
    )
    decision = r9.build_high_confidence_decision(
        scores_path=paths["scores"],
        conformal_report_path=paths["conformal"],
        distributional_report_path=paths["distributional"],
        governor_report_path=paths["governor"],
        label_prefix=label_prefix,
        output_json=paths["decision"],
    )
    bundle = r10.build_consumer_bundle(artifact_dir=output_dir, output_json=paths["bundle"])

    result = {
        "schema_version": "regime-sidecar-pipeline/v1",
        "status": "ok",
        "output_dir": str(output_dir),
        "label_prefix": label_prefix,
        "bundle_path": str(paths["bundle"]),
        "decision_path": str(paths["decision"]),
        "final_decision": bundle.get("latest_decision", {
            "decision_state": decision.get("decision_state", ""),
            "trade_usable": decision.get("trade_usable", False),
        }),
        "artifacts": {key: str(path) for key, path in paths.items() if key != "pipeline"},
        "consumer_contract": {
            "zero_config": True,
            "main_runtime_mutation": "none",
            "optional_for_consumers": True,
            "no_repo_root_state": True,
        },
        "truth_joined_rows": truth_joined_rows,
    }
    _write_json(paths["pipeline"], result)
    return result


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run ICT regime sidecar chain R2-R10 into a token-friendly bundle.")
    parser.add_argument("--ohlcv", help="Required OHLCV CSV/JSONL input.")
    parser.add_argument("--output-dir", default="/tmp/ict-regime-sidecar", help="Output artifact directory. Defaults to /tmp.")
    parser.add_argument("--label-prefix", default="primary::Trend", help="Hot-plug label scope for consumer decision.")
    parser.add_argument("--auxiliary-evidence", help="Optional timestamp-joined auxiliary evidence CSV/JSONL.")
    parser.add_argument("--truth", help="Optional truth labels JSONL keyed by timestamp.")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    result = run_pipeline(
        ohlcv_path=Path(args.ohlcv) if args.ohlcv else None,
        output_dir=Path(args.output_dir),
        auxiliary_evidence_path=Path(args.auxiliary_evidence) if args.auxiliary_evidence else None,
        truth_path=Path(args.truth) if args.truth else None,
        label_prefix=args.label_prefix,
    )
    print(json.dumps({
        "status": result["status"],
        "bundle_path": result.get("bundle_path", ""),
        "final_decision": result.get("final_decision", {}),
        "input_contract": result.get("input_contract", {}),
    }, indent=2))
    return 0 if result["status"] == "ok" else 2


if __name__ == "__main__":
    raise SystemExit(main())
