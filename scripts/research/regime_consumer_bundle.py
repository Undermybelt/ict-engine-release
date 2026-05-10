from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

DEFAULT_ARTIFACTS = {
    "ontology": "regime_ontology_manifest.json",
    "feature_quality": "feature_quality_report.json",
    "expert_training": "regime_expert_training_report.json",
    "conformal": "regime_conformal_calibration_report.json",
    "distributional": "regime_distributional_agreement_report.json",
    "transition_governor": "regime_transition_governor_report.json",
    "decision": "regime_high_confidence_decision.json",
}


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def _parse_include_artifact(value: str) -> tuple[str, Path]:
    if "=" not in value:
        raise ValueError(f"include artifact must be key=path, got: {value}")
    key, raw_path = value.split("=", 1)
    key = key.strip()
    if not key:
        raise ValueError(f"include artifact key is empty: {value}")
    return key, Path(raw_path).expanduser()


def _artifact_inputs(artifact_dir: Path | None, include_artifacts: list[str]) -> dict[str, Path]:
    inputs: dict[str, Path] = {}
    if artifact_dir is not None:
        for key, filename in DEFAULT_ARTIFACTS.items():
            inputs[key] = artifact_dir / filename
    for item in include_artifacts:
        key, path = _parse_include_artifact(item)
        inputs[key] = path
    return inputs


def _artifact_summary(key: str, path: Path) -> dict[str, Any]:
    if not path.exists():
        return {"status": "missing", "path": str(path)}
    payload = _load_json(path)
    summary: dict[str, Any] = {
        "status": "present",
        "path": str(path),
        "schema_version": payload.get("schema_version", ""),
    }
    for field in (
        "timestamp",
        "row_count",
        "expert_count",
        "confidence_95",
        "confidence_99",
        "decision_state",
        "trade_usable",
        "final_label",
        "execution_tree_hint",
        "transition_hazard",
        "agreement",
    ):
        if field in payload:
            summary[field] = payload[field]
    if key == "decision":
        summary["label_set"] = payload.get("label_set", [])
        summary["abstain_reasons"] = payload.get("abstain_reasons", [])
    return summary


def _latest_decision(artifacts: dict[str, dict[str, Any]], loaded: dict[str, dict[str, Any]]) -> dict[str, Any]:
    decision = loaded.get("decision", {})
    if not decision:
        return {
            "decision_state": "missing",
            "trade_usable": False,
            "final_label": "",
            "label_set": [],
            "abstain_reasons": ["decision_artifact_missing"],
        }
    return {
        "timestamp": decision.get("timestamp", ""),
        "decision_state": decision.get("decision_state", ""),
        "trade_usable": bool(decision.get("trade_usable", False)),
        "final_label": decision.get("final_label", ""),
        "label_set": decision.get("label_set", []),
        "abstain_reasons": decision.get("abstain_reasons", []),
    }


def _consumer_hints(loaded: dict[str, dict[str, Any]], latest_decision: dict[str, Any]) -> dict[str, Any]:
    decision = loaded.get("decision", {})
    governor = loaded.get("transition_governor", {})
    execution_hint = decision.get("execution_tree_hint") or governor.get("execution_tree_hint") or "unknown_abstain"
    return {
        "execution_tree_hint": execution_hint,
        "bbn_evidence_hint": decision.get("bbn_evidence_hint", governor.get("bbn_evidence_hint", {})),
        "path_ranker_context": decision.get("path_ranker_context", {}),
        "user_vrp_nq_context": decision.get("user_vrp_nq_context", {}),
        "trade_usable": latest_decision.get("trade_usable", False),
    }


def build_consumer_bundle(
    *,
    output_json: Path,
    include_artifacts: list[str] | None = None,
    artifact_dir: Path | None = None,
) -> dict[str, Any]:
    includes = include_artifacts or []
    inputs = _artifact_inputs(artifact_dir, includes)
    artifacts: dict[str, dict[str, Any]] = {}
    loaded: dict[str, dict[str, Any]] = {}
    missing: list[str] = []

    for key, path in inputs.items():
        summary = _artifact_summary(key, path)
        artifacts[key] = summary
        if summary["status"] == "missing":
            missing.append(key)
        else:
            loaded[key] = _load_json(path)

    latest = _latest_decision(artifacts, loaded)
    hints = _consumer_hints(loaded, latest)
    bundle = {
        "schema_version": "regime-consumer-bundle/v1",
        "artifact_count": len(artifacts),
        "missing_artifacts": missing,
        "latest_decision": latest,
        "consumer_hints": hints,
        "artifacts": artifacts,
        "consumer_contract": {
            "zero_config": True,
            "hotplug_scope": "include_artifact",
            "main_runtime_mutation": "none",
            "optional_for_consumers": True,
            "token_friendly": True,
        },
    }
    _write_json(output_json, bundle)
    return bundle


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Bundle regime sidecar artifacts into one token-friendly consumer manifest.")
    parser.add_argument("--artifact-dir", help="Optional directory containing default R2-R9 artifact filenames.")
    parser.add_argument("--include-artifact", action="append", default=[], help="Hot-plug artifact input as key=path. May be repeated.")
    parser.add_argument("--output-json", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    result = build_consumer_bundle(
        artifact_dir=Path(args.artifact_dir) if args.artifact_dir else None,
        include_artifacts=args.include_artifact,
        output_json=Path(args.output_json),
    )
    print(json.dumps({"ok": True, "output": args.output_json, "artifact_count": result["artifact_count"], "decision_state": result["latest_decision"]["decision_state"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
