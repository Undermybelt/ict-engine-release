from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any
import zipfile

import factor_candidate_pack as pack
import regime_artifact_bundle as regime_bundle

PRESET_PATH = Path("config/factor_candidate_harness_presets.json")
PROFILE_DIR = Path("examples/factor_candidate_profiles")
NAMING_CONTRACT_VERSION = "factor-artifact-naming/v1"


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _normalized(value: str) -> str:
    return value.strip().lower().replace("-", "_").replace(" ", "_")


def _load_presets(repo_root: Path) -> list[dict[str, Any]]:
    return _load_json(repo_root / PRESET_PATH).get("candidates", [])


def _load_profiles(repo_root: Path) -> list[dict[str, Any]]:
    profiles: list[dict[str, Any]] = []
    for path in sorted((repo_root / PROFILE_DIR).glob("*.json")):
        payload = _load_json(path)
        payload["_source_path"] = str(path)
        payload["_source_stem"] = path.stem
        profiles.append(payload)
    return profiles


def _resolve_profile(repo_root: Path, selector: str | None) -> dict[str, Any] | None:
    if not selector:
        return None
    wanted = _normalized(selector)
    for profile in _load_profiles(repo_root):
        if wanted in {
            _normalized(profile["profile_id"]),
            _normalized(profile.get("display_name", "")),
            _normalized(profile.get("_source_stem", "")),
        }:
            return profile
    raise ValueError(f"unknown factor candidate profile '{selector}'")


def _deep_merge(base: dict[str, Any], override: dict[str, Any]) -> dict[str, Any]:
    merged = dict(base)
    for key, value in override.items():
        if (
            isinstance(value, dict)
            and isinstance(merged.get(key), dict)
            and key not in {"cross_market_metrics"}
        ):
            merged[key] = _deep_merge(merged[key], value)
        else:
            merged[key] = value
    return merged


def _selected_profile_surface(profile: dict[str, Any] | None) -> dict[str, Any] | None:
    if not profile:
        return None
    return {
        "profile_id": profile["profile_id"],
        "display_name": profile["display_name"],
        "opt_in_only": profile.get("opt_in_only", False),
        "summary": profile.get("summary", ""),
    }


def _freqtrade_zip_validation_reason(path: Path) -> str | None:
    if not path.exists():
        return f"missing_artifact:{path}"
    try:
        with zipfile.ZipFile(path) as archive:
            corrupt_member = archive.testzip()
    except zipfile.BadZipFile as exc:
        return f"invalid_artifact:{path}:{exc}"
    if corrupt_member:
        return f"invalid_artifact:{path}:crc_failed:{corrupt_member}"
    return None


def _artifact_kind(candidate: dict[str, Any]) -> str:
    artifact_source = candidate.get("artifact_source", {})
    if artifact_source.get("freqtrade_backtest_zip"):
        return "freqtrade_backtest_zip"
    if artifact_source.get("strategy_library_json"):
        return "strategy_library_json"
    if artifact_source.get("regime_benchmark_jsons") or candidate.get(
        "reusable_input_kind"
    ) == "regime_benchmark_json":
        return "regime_benchmark_json"
    if candidate.get("promotion_state") == "regime_only":
        return "regime_gate_placeholder"
    return "candidate_placeholder"


def _artifact_plan(candidate: dict[str, Any]) -> dict[str, Any]:
    artifact_source = candidate.get("artifact_source", {})
    artifact_kind = _artifact_kind(candidate)
    explicit_reason = candidate.get("pack_readiness_reason")
    if explicit_reason:
        return {
            "artifact_kind": artifact_kind,
            "artifact_ready": False,
            "build_mode": None,
            "pack_build_reason": explicit_reason,
            "evidence_status": "deferred",
        }

    backtest_zip = artifact_source.get("freqtrade_backtest_zip")
    if backtest_zip:
        zip_path = Path(backtest_zip).expanduser()
        validation_reason = _freqtrade_zip_validation_reason(zip_path)
        artifact_ready = validation_reason is None
        return {
            "artifact_kind": "freqtrade_backtest_zip",
            "artifact_ready": artifact_ready,
            "build_mode": "freqtrade_backtest_zip" if artifact_ready else None,
            "freqtrade_backtest_zip_path": zip_path,
            "pack_build_reason": (
                "buildable_from_reusable_artifact"
                if artifact_ready
                else validation_reason
            ),
            "evidence_status": "buildable" if artifact_ready else "board_evidence_only",
        }

    strategy_library_json = artifact_source.get("strategy_library_json")
    if strategy_library_json:
        manifest_path = Path(strategy_library_json).expanduser()
        if not manifest_path.exists():
            return {
                "artifact_kind": "strategy_library_json",
                "artifact_ready": False,
                "build_mode": None,
                "pack_build_reason": f"missing_artifact:{manifest_path}",
                "evidence_status": "board_evidence_only",
            }
        try:
            manifest = _load_json(manifest_path)
            strategies = manifest.get("strategies") or []
            if not strategies:
                raise ValueError("manifest contains no strategies")
        except Exception as exc:
            return {
                "artifact_kind": "strategy_library_json",
                "artifact_ready": False,
                "build_mode": None,
                "pack_build_reason": f"invalid_artifact:{manifest_path}:{exc}",
                "evidence_status": "board_evidence_only",
            }
        return {
            "artifact_kind": "strategy_library_json",
            "artifact_ready": True,
            "build_mode": "strategy_library_json",
            "strategy_library_json_path": manifest_path,
            "pack_build_reason": "buildable_from_reusable_artifact",
            "evidence_status": "buildable",
        }

    regime_benchmark_jsons = artifact_source.get("regime_benchmark_jsons") or []
    if regime_benchmark_jsons or artifact_kind == "regime_benchmark_json":
        benchmark_paths = [Path(path).expanduser() for path in regime_benchmark_jsons]
        missing_paths = [path for path in benchmark_paths if not path.exists()]
        artifact_ready = bool(benchmark_paths) and not missing_paths
        if artifact_ready:
            pack_build_reason = "buildable_from_reusable_artifact"
            evidence_status = "buildable"
            build_mode = "regime_benchmark_json"
        elif benchmark_paths:
            pack_build_reason = "missing_artifact:" + ",".join(
                str(path) for path in missing_paths
            )
            evidence_status = "board_evidence_only"
            build_mode = None
        else:
            pack_build_reason = "missing_regime_benchmark_jsons"
            evidence_status = "board_evidence_only"
            build_mode = None
        return {
            "artifact_kind": "regime_benchmark_json",
            "artifact_ready": artifact_ready,
            "build_mode": build_mode,
            "regime_benchmark_paths": benchmark_paths,
            "pack_build_reason": pack_build_reason,
            "evidence_status": evidence_status,
        }

    return {
        "artifact_kind": artifact_kind,
        "artifact_ready": False,
        "build_mode": None,
        "pack_build_reason": "missing_reusable_input",
        "evidence_status": "board_evidence_only",
    }


def _reusable_input_refs(candidate: dict[str, Any]) -> list[str]:
    refs: list[str] = []
    artifact_source = candidate.get("artifact_source", {})
    backtest_zip = artifact_source.get("freqtrade_backtest_zip")
    if backtest_zip:
        refs.append(str(Path(backtest_zip).expanduser()))
    strategy_library_json = artifact_source.get("strategy_library_json")
    if strategy_library_json:
        refs.append(str(Path(strategy_library_json).expanduser()))
    for benchmark_json in artifact_source.get("regime_benchmark_jsons", []):
        refs.append(str(Path(benchmark_json).expanduser()))
    strategy_source = candidate.get("strategy_source")
    if strategy_source:
        refs.append(str(strategy_source))
    return refs


def build_candidate_registry(
    repo_root: Path | str,
    profile_selector: str | None = None,
) -> dict[str, Any]:
    repo_root = Path(repo_root).resolve()
    presets = _load_presets(repo_root)
    profile = _resolve_profile(repo_root, profile_selector)
    overrides = {
        item["candidate_id"]: item
        for item in (profile or {}).get("candidate_overrides", [])
    }

    candidates: list[dict[str, Any]] = []
    buildable_count = 0
    for preset in presets:
        candidate = _deep_merge(preset, overrides.get(preset["candidate_id"], {}))
        artifact_plan = _artifact_plan(candidate)
        if artifact_plan["artifact_ready"]:
            buildable_count += 1
        strategy_source = candidate.get("strategy_source")
        if strategy_source and not Path(strategy_source).is_absolute():
            candidate["strategy_source"] = str((repo_root / strategy_source).resolve())
        candidate["artifact_ready"] = artifact_plan["artifact_ready"]
        candidate["selected_profile_id"] = profile["profile_id"] if profile else None
        candidate["pack_build_reason"] = artifact_plan["pack_build_reason"]
        candidate["evidence_status"] = artifact_plan["evidence_status"]
        candidate["artifact_kind"] = artifact_plan["artifact_kind"]
        candidate["board_evidence_status"] = "board_recorded"
        candidate["board_ref"] = "docs/plans/2026-05-05-execution-tree-factor-auto-quant-todo.md"
        candidate["reusable_input_refs"] = _reusable_input_refs(candidate)
        candidate["naming_contract"] = {
            "version": NAMING_CONTRACT_VERSION,
            "artifact_layers": [
                "board_record",
                "reusable_input",
                "candidate_pack",
                "temp_state_dir",
            ],
            "state_term_scope": "runtime_or_temp_state_only",
        }
        candidates.append(candidate)

    selection_mode = "profile_opt_in" if profile else "generic_zero_config"
    selection_label = (
        f"{profile['display_name']} ({profile['profile_id']})"
        if profile
        else "Generic zero-config factor candidate registry"
    )
    return {
        "schema_version": "factor-candidate-registry/v1",
        "selected_profile": _selected_profile_surface(profile),
        "summary": {
            "naming_contract_version": NAMING_CONTRACT_VERSION,
            "selection_mode": selection_mode,
            "selection_label": selection_label,
            "candidate_count": len(candidates),
            "buildable_count": buildable_count,
        },
        "candidates": candidates,
    }


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def write_candidate_specs(output_dir: Path, candidates: list[dict[str, Any]]) -> list[dict[str, Any]]:
    spec_entries: list[dict[str, Any]] = []
    for candidate in candidates:
        candidate_id = candidate["candidate_id"]
        spec_payload = {
            key: value
            for key, value in candidate.items()
            if key
            not in {
                "artifact_ready",
                "selected_profile_id",
            }
        }
        spec_path = output_dir / "specs" / f"{candidate_id}.json"
        _write_json(spec_path, spec_payload)
        spec_entries.append(
            {
                "candidate_id": candidate_id,
                "strategy_name": candidate.get("strategy_name"),
                "spec_path": str(spec_path),
            }
        )
    return spec_entries


def build_candidate_packs(
    *,
    repo_root: Path,
    output_dir: Path,
    candidates: list[dict[str, Any]],
) -> dict[str, Any]:
    built: list[dict[str, Any]] = []
    skipped: list[dict[str, Any]] = []
    for candidate in candidates:
        artifact_plan = _artifact_plan(candidate)
        if not artifact_plan["artifact_ready"]:
            skipped.append(
                {
                    "candidate_id": candidate["candidate_id"],
                    "reason": artifact_plan["pack_build_reason"],
                }
            )
            continue
        candidate_dir = output_dir / "packs" / candidate["candidate_id"]
        candidate_dir.mkdir(parents=True, exist_ok=True)
        if artifact_plan["build_mode"] == "freqtrade_backtest_zip":
            artifact_source = candidate.get("artifact_source", {})
            zip_path = artifact_plan["freqtrade_backtest_zip_path"]
            manifest = pack.build_manifest_from_freqtrade_backtest_zip(zip_path)
            autoresearch_status_path = artifact_source.get("autoresearch_status_json")
            autoresearch_status = (
                _load_json(Path(autoresearch_status_path).expanduser())
                if autoresearch_status_path
                and Path(autoresearch_status_path).expanduser().exists()
                else {}
            )
            bundle = pack.build_factor_candidate_pack(
                manifest=manifest,
                strategy_name=candidate.get("strategy_name"),
                candidate_spec=candidate,
                autoresearch_status=autoresearch_status,
            )
            for name, payload in bundle.items():
                _write_json(candidate_dir / f"{name}.json", payload)
            built.append(
                {
                    "candidate_id": candidate["candidate_id"],
                    "strategy_name": candidate.get("strategy_name"),
                    "artifact_family": "factor_candidate_pack",
                    "pack_dir": str(candidate_dir),
                    "source_backtest_zip": str(zip_path),
                    "aggregate_trade_count": bundle["factor_eval_grid_summary"][
                        "trade_density_summary"
                    ]["aggregate_trade_count"],
                    "aggregate_label": bundle["factor_eval_grid_summary"][
                        "trade_density_summary"
                    ]["aggregate_label"],
                    "transfer_status": bundle["transfer_score"]["status"],
                }
            )
            continue

        if artifact_plan["build_mode"] == "strategy_library_json":
            manifest_path = artifact_plan["strategy_library_json_path"]
            manifest = _load_json(manifest_path)
            artifact_source = candidate.get("artifact_source", {})
            autoresearch_status_path = artifact_source.get("autoresearch_status_json")
            autoresearch_status = (
                _load_json(Path(autoresearch_status_path).expanduser())
                if autoresearch_status_path
                and Path(autoresearch_status_path).expanduser().exists()
                else {}
            )
            bundle = pack.build_factor_candidate_pack(
                manifest=manifest,
                strategy_name=candidate.get("strategy_name"),
                candidate_spec=candidate,
                autoresearch_status=autoresearch_status,
            )
            for name, payload in bundle.items():
                _write_json(candidate_dir / f"{name}.json", payload)
            built.append(
                {
                    "candidate_id": candidate["candidate_id"],
                    "strategy_name": candidate.get("strategy_name"),
                    "artifact_family": "factor_candidate_pack",
                    "pack_dir": str(candidate_dir),
                    "source_strategy_library_json": str(manifest_path),
                    "aggregate_trade_count": bundle["factor_eval_grid_summary"][
                        "trade_density_summary"
                    ]["aggregate_trade_count"],
                    "aggregate_label": bundle["factor_eval_grid_summary"][
                        "trade_density_summary"
                    ]["aggregate_label"],
                    "transfer_status": bundle["transfer_score"]["status"],
                }
            )
            continue

        if artifact_plan["build_mode"] == "regime_benchmark_json":
            benchmarks = [
                _load_json(path) for path in artifact_plan["regime_benchmark_paths"]
            ]
            bundle = regime_bundle.build_regime_artifact_bundle(
                benchmarks=benchmarks,
                candidate_id=candidate["candidate_id"],
                display_name=candidate["display_name"],
            )
            for name, payload in bundle.items():
                _write_json(candidate_dir / f"{name}.json", payload)
            classifier_summary = bundle["regime_classifier_summary"]
            transition_summary = bundle["transition_summary"]
            cross_market_summary = bundle["cross_market_summary"]
            built.append(
                {
                    "candidate_id": candidate["candidate_id"],
                    "strategy_name": candidate.get("strategy_name"),
                    "artifact_family": "regime_artifact_bundle",
                    "pack_dir": str(candidate_dir),
                    "source_benchmark_count": len(
                        artifact_plan["regime_benchmark_paths"]
                    ),
                    "covered_markets": cross_market_summary["covered_markets"],
                    "average_eval_macro_f1": classifier_summary[
                        "average_eval_macro_f1"
                    ],
                    "best_eval_macro_f1": classifier_summary["best_eval_macro_f1"],
                    "best_transition_f1": transition_summary["best_transition_f1"],
                }
            )
            continue

        skipped.append(
            {
                "candidate_id": candidate["candidate_id"],
                "reason": f"unsupported_build_mode:{artifact_plan['build_mode']}",
            }
        )

    return {
        "schema_version": "factor-candidate-pack-index/v1",
        "summary": {
            "built_count": len(built),
            "skipped_count": len(skipped),
        },
        "built_candidates": built,
        "skipped_candidates": skipped,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Resolve generic and opt-in factor candidate specs into explicit candidate-pack artifacts."
    )
    parser.add_argument(
        "--repo-root",
        default=".",
        help="Repo root containing config/ and examples/factor_candidate_profiles/",
    )
    parser.add_argument(
        "--profile",
        help="Optional factor candidate profile selector for personal opt-in evidence lanes.",
    )
    parser.add_argument("--output-dir", required=True)
    parser.add_argument(
        "--build-packs",
        action="store_true",
        help="Build candidate pack artifacts for entries with reusable evidence artifacts.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    repo_root = Path(args.repo_root).resolve()
    output_dir = Path(args.output_dir).resolve()
    registry = build_candidate_registry(repo_root=repo_root, profile_selector=args.profile)
    spec_entries = write_candidate_specs(output_dir, registry["candidates"])
    _write_json(output_dir / "candidate_registry.json", registry)
    _write_json(
        output_dir / "candidate_spec_index.json",
        {
            "schema_version": "factor-candidate-spec-index/v1",
            "specs": spec_entries,
        },
    )

    pack_index = {
        "schema_version": "factor-candidate-pack-index/v1",
        "summary": {
            "built_count": 0,
            "skipped_count": len(registry["candidates"]),
        },
        "built_candidates": [],
        "skipped_candidates": [
            {
                "candidate_id": candidate["candidate_id"],
                "reason": "pack_build_not_requested",
            }
            for candidate in registry["candidates"]
        ],
    }
    if args.build_packs:
        pack_index = build_candidate_packs(
            repo_root=repo_root,
            output_dir=output_dir,
            candidates=registry["candidates"],
        )
    _write_json(output_dir / "candidate_pack_index.json", pack_index)
    print(
        json.dumps(
            {
                "ok": True,
                "selection_mode": registry["summary"]["selection_mode"],
                "candidate_count": registry["summary"]["candidate_count"],
                "buildable_count": registry["summary"]["buildable_count"],
                "built_pack_count": pack_index["summary"]["built_count"],
                "output_dir": str(output_dir),
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
