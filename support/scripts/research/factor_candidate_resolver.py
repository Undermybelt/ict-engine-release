from __future__ import annotations

import argparse
import json
from pathlib import Path
import shutil
from typing import Any
import zipfile

import factor_candidate_pack as pack
import regime_artifact_bundle as regime_bundle

PRESET_PATH = Path("config/factor_candidate_harness_presets.json")
PROFILE_DIR = Path("support/examples/factor_candidate_profiles")
NAMING_CONTRACT_VERSION = "factor-artifact-naming/v1"
REQUIRED_CANDIDATE_PACK_FILES = (
    "factor_expression.json",
    "factor_eval_grid_summary.json",
    "transfer_score.json",
)


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _normalized(value: str) -> str:
    return value.strip().lower().replace("-", "_").replace(" ", "_")


def _load_presets(repo_root: Path) -> list[dict[str, Any]]:
    return _load_json(repo_root / PRESET_PATH).get("candidates", [])


def _artifact_path(path: str | Path, repo_root: Path) -> Path:
    artifact_path = Path(path).expanduser()
    if artifact_path.is_absolute():
        return artifact_path
    return (repo_root / artifact_path).resolve()


def _artifact_ref(path: str | Path) -> str:
    artifact_path = Path(path).expanduser()
    return str(artifact_path)


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


def _candidate_pack_validation_reason(path: Path) -> str | None:
    if not path.exists():
        return f"missing_artifact:{path}"
    if not path.is_dir():
        return f"invalid_artifact:{path}:not_directory"
    missing = [name for name in REQUIRED_CANDIDATE_PACK_FILES if not (path / name).exists()]
    if missing:
        return f"invalid_artifact:{path}:missing_files:{','.join(missing)}"
    try:
        for name in REQUIRED_CANDIDATE_PACK_FILES:
            _load_json(path / name)
    except Exception as exc:
        return f"invalid_artifact:{path}:{exc}"
    return None


def _artifact_kind(candidate: dict[str, Any]) -> str:
    artifact_source = candidate.get("artifact_source", {})
    if artifact_source.get("candidate_pack_dir"):
        return "candidate_pack_dir"
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


def _artifact_plan(candidate: dict[str, Any], repo_root: Path) -> dict[str, Any]:
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
            "curation_decision": "needs_named_prerequisite",
        }

    candidate_pack_dir = artifact_source.get("candidate_pack_dir")
    if candidate_pack_dir:
        pack_dir = _artifact_path(candidate_pack_dir, repo_root)
        validation_reason = _candidate_pack_validation_reason(pack_dir)
        artifact_ready = validation_reason is None
        return {
            "artifact_kind": "candidate_pack_dir",
            "artifact_ready": artifact_ready,
            "build_mode": "candidate_pack_dir" if artifact_ready else None,
            "candidate_pack_dir_path": pack_dir,
            "candidate_pack_dir_ref": _artifact_ref(candidate_pack_dir),
            "pack_build_reason": (
                "buildable_from_repo_candidate_pack"
                if artifact_ready
                else validation_reason
            ),
            "evidence_status": "buildable" if artifact_ready else "missing_reusable_artifact",
            "curation_decision": (
                "promote_to_candidate_pack"
                if artifact_ready
                else "discard_until_reusable_artifact"
            ),
        }

    backtest_zip = artifact_source.get("freqtrade_backtest_zip")
    if backtest_zip:
        zip_path = _artifact_path(backtest_zip, repo_root)
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
            "evidence_status": "buildable" if artifact_ready else "missing_reusable_artifact",
            "curation_decision": (
                "promote_to_candidate_pack"
                if artifact_ready
                else "discard_until_reusable_artifact"
            ),
        }

    strategy_library_json = artifact_source.get("strategy_library_json")
    if strategy_library_json:
        manifest_path = _artifact_path(strategy_library_json, repo_root)
        if not manifest_path.exists():
            return {
                "artifact_kind": "strategy_library_json",
                "artifact_ready": False,
                "build_mode": None,
                "pack_build_reason": f"missing_artifact:{manifest_path}",
                "evidence_status": "missing_reusable_artifact",
                "curation_decision": "discard_until_reusable_artifact",
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
                "evidence_status": "missing_reusable_artifact",
                "curation_decision": "discard_until_reusable_artifact",
            }
        return {
            "artifact_kind": "strategy_library_json",
            "artifact_ready": True,
            "build_mode": "strategy_library_json",
            "strategy_library_json_path": manifest_path,
            "pack_build_reason": "buildable_from_reusable_artifact",
            "evidence_status": "buildable",
            "curation_decision": "promote_to_candidate_pack",
        }

    regime_benchmark_jsons = artifact_source.get("regime_benchmark_jsons") or []
    if regime_benchmark_jsons or artifact_kind == "regime_benchmark_json":
        benchmark_paths = [_artifact_path(path, repo_root) for path in regime_benchmark_jsons]
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
            evidence_status = "missing_reusable_artifact"
            build_mode = None
        else:
            pack_build_reason = "missing_regime_benchmark_jsons"
            evidence_status = "missing_reusable_artifact"
            build_mode = None
        return {
            "artifact_kind": "regime_benchmark_json",
            "artifact_ready": artifact_ready,
            "build_mode": build_mode,
            "regime_benchmark_paths": benchmark_paths,
            "pack_build_reason": pack_build_reason,
            "evidence_status": evidence_status,
            "curation_decision": (
                "promote_to_regime_artifact_bundle"
                if artifact_ready
                else "discard_until_reusable_artifact"
            ),
        }

    return {
        "artifact_kind": artifact_kind,
        "artifact_ready": False,
        "build_mode": None,
        "pack_build_reason": "missing_reusable_input",
        "evidence_status": "missing_reusable_artifact",
        "curation_decision": "discard_until_reusable_artifact",
    }


def _reusable_input_refs(candidate: dict[str, Any], repo_root: Path) -> list[str]:
    refs: list[str] = []
    artifact_source = candidate.get("artifact_source", {})
    candidate_pack_dir = artifact_source.get("candidate_pack_dir")
    if candidate_pack_dir:
        refs.append(_artifact_ref(candidate_pack_dir))
    backtest_zip = artifact_source.get("freqtrade_backtest_zip")
    if backtest_zip:
        refs.append(_artifact_ref(backtest_zip))
    strategy_library_json = artifact_source.get("strategy_library_json")
    if strategy_library_json:
        refs.append(_artifact_ref(strategy_library_json))
    for benchmark_json in artifact_source.get("regime_benchmark_jsons", []):
        refs.append(_artifact_ref(benchmark_json))
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
        artifact_plan = _artifact_plan(candidate, repo_root)
        if artifact_plan["artifact_ready"]:
            buildable_count += 1
        strategy_source = candidate.get("strategy_source")
        if strategy_source:
            source_path = Path(strategy_source)
            if source_path.is_absolute() and repo_root in source_path.parents:
                candidate["strategy_source"] = str(source_path.relative_to(repo_root))
        candidate["artifact_ready"] = artifact_plan["artifact_ready"]
        candidate["selected_profile_id"] = profile["profile_id"] if profile else None
        candidate["pack_build_reason"] = artifact_plan["pack_build_reason"]
        candidate["evidence_status"] = artifact_plan["evidence_status"]
        candidate["artifact_kind"] = artifact_plan["artifact_kind"]
        candidate["curation_decision"] = artifact_plan["curation_decision"]
        candidate["archive_evidence_status"] = "not_runtime_input"
        candidate["archive_refs"] = []
        candidate["reusable_input_refs"] = _reusable_input_refs(candidate, repo_root)
        candidate["naming_contract"] = {
            "version": NAMING_CONTRACT_VERSION,
            "artifact_layers": [
                "archive_reference",
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


def _output_ref(path: Path, output_dir: Path) -> str:
    return str(path.relative_to(output_dir))


def _candidate_list_entry(candidate: dict[str, Any], repo_root: Path) -> dict[str, Any]:
    artifact_plan = _artifact_plan(candidate, repo_root)
    entry: dict[str, Any] = {
        "candidate_id": candidate["candidate_id"],
        "display_name": candidate.get("display_name"),
        "family": candidate.get("family"),
        "base_timeframe": candidate.get("base_timeframe"),
        "artifact_kind": artifact_plan["artifact_kind"],
        "evidence_status": artifact_plan["evidence_status"],
        "curation_decision": artifact_plan["curation_decision"],
        "reusable_input_refs": candidate.get("reusable_input_refs", []),
    }
    if artifact_plan["build_mode"] == "candidate_pack_dir":
        pack_dir = artifact_plan["candidate_pack_dir_path"]
        eval_summary = _load_json(pack_dir / "factor_eval_grid_summary.json")
        transfer_score = _load_json(pack_dir / "transfer_score.json")
        entry.update(
            {
                "aggregate_trade_count": eval_summary["trade_density_summary"][
                    "aggregate_trade_count"
                ],
                "aggregate_label": eval_summary["trade_density_summary"][
                    "aggregate_label"
                ],
                "transfer_status": transfer_score["status"],
            }
        )
    return entry


def list_buildable_candidates(
    *,
    repo_root: Path,
    candidates: list[dict[str, Any]],
) -> dict[str, Any]:
    buildable = [
        _candidate_list_entry(candidate, repo_root)
        for candidate in candidates
        if candidate["artifact_ready"]
    ]
    return {
        "schema_version": "factor-candidate-buildable-list/v1",
        "summary": {
            "buildable_count": len(buildable),
            "candidate_count": len(candidates),
        },
        "buildable_candidates": buildable,
    }


def _print_human_buildable_list(payload: dict[str, Any]) -> None:
    print(
        "buildable_count={buildable_count} candidate_count={candidate_count}".format(
            **payload["summary"]
        )
    )
    for candidate in payload["buildable_candidates"]:
        print(
            "{candidate_id}\t{aggregate_trade_count}\t{aggregate_label}\t{transfer_status}\t{reusable_ref}".format(
                candidate_id=candidate["candidate_id"],
                aggregate_trade_count=candidate.get("aggregate_trade_count", "n/a"),
                aggregate_label=candidate.get("aggregate_label", "n/a"),
                transfer_status=candidate.get("transfer_status", "n/a"),
                reusable_ref=(candidate.get("reusable_input_refs") or [""])[0],
            )
        )


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
                "spec_path": _output_ref(spec_path, output_dir),
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
        artifact_plan = _artifact_plan(candidate, repo_root)
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
        if artifact_plan["build_mode"] == "candidate_pack_dir":
            pack_dir = artifact_plan["candidate_pack_dir_path"]
            for name in REQUIRED_CANDIDATE_PACK_FILES:
                shutil.copy2(pack_dir / name, candidate_dir / name)
            eval_summary = _load_json(candidate_dir / "factor_eval_grid_summary.json")
            transfer_score = _load_json(candidate_dir / "transfer_score.json")
            built.append(
                {
                    "candidate_id": candidate["candidate_id"],
                    "strategy_name": candidate.get("strategy_name"),
                    "artifact_family": "factor_candidate_pack",
                    "pack_dir": _output_ref(candidate_dir, output_dir),
                    "source_candidate_pack_dir": artifact_plan["candidate_pack_dir_ref"],
                    "aggregate_trade_count": eval_summary["trade_density_summary"][
                        "aggregate_trade_count"
                    ],
                    "aggregate_label": eval_summary["trade_density_summary"][
                        "aggregate_label"
                    ],
                    "transfer_status": transfer_score["status"],
                }
            )
            continue

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
                    "pack_dir": _output_ref(candidate_dir, output_dir),
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
                    "pack_dir": _output_ref(candidate_dir, output_dir),
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
                    "pack_dir": _output_ref(candidate_dir, output_dir),
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
        help="Repo root containing config/ and support/examples/factor_candidate_profiles/",
    )
    parser.add_argument(
        "--profile",
        help="Optional factor candidate profile selector for personal opt-in evidence lanes.",
    )
    parser.add_argument("--output-dir")
    parser.add_argument(
        "--build-packs",
        action="store_true",
        help="Build candidate pack artifacts for entries with reusable evidence artifacts.",
    )
    parser.add_argument(
        "--list-buildable",
        action="store_true",
        help="Print the repo-local buildable candidate packs without reading historical board docs.",
    )
    parser.add_argument(
        "--output-format",
        choices=["json", "human"],
        default="json",
        help="Output format for --list-buildable.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    repo_root = Path(args.repo_root).resolve()
    registry = build_candidate_registry(repo_root=repo_root, profile_selector=args.profile)
    if args.list_buildable and not args.output_dir:
        buildable_payload = list_buildable_candidates(
            repo_root=repo_root,
            candidates=registry["candidates"],
        )
        if args.output_format == "human":
            _print_human_buildable_list(buildable_payload)
        else:
            print(json.dumps(buildable_payload, indent=2))
        return 0

    if not args.output_dir:
        raise SystemExit("--output-dir is required unless --list-buildable is used")

    output_dir = Path(args.output_dir).resolve()
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
