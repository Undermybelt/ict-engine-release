from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

MARKET_PRESETS_PATH = Path("config/market_data_harness_presets.json")
MARKET_RELATIONSHIPS_PATH = Path("config/market_relationships.json")
PROVIDER_PROFILE_DIR = Path("examples/provider_profiles")


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _normalized(value: str) -> str:
    return value.strip().lower().replace("-", "_").replace(" ", "_")


def _load_market_configs(repo_root: Path) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    presets = _load_json(repo_root / MARKET_PRESETS_PATH).get("markets", [])
    relationships = {
        _normalized(item["market_key"]): item
        for item in _load_json(repo_root / MARKET_RELATIONSHIPS_PATH).get("markets", [])
    }
    return presets, relationships


def _resolve_market(
    presets: list[dict[str, Any]],
    relationships: dict[str, dict[str, Any]],
    selector: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    wanted = _normalized(selector)
    for preset in presets:
        aliases = preset.get("aliases", [])
        if _normalized(preset["market_key"]) == wanted or any(
            _normalized(alias) == wanted for alias in aliases
        ):
            relationship = relationships.get(_normalized(preset["market_key"]), {})
            return preset, relationship
    raise ValueError(f"unknown market selector '{selector}'")


def _load_provider_profiles(repo_root: Path) -> list[dict[str, Any]]:
    profile_dir = repo_root / PROVIDER_PROFILE_DIR
    profiles: list[dict[str, Any]] = []
    for path in sorted(profile_dir.glob("*.json")):
        payload = _load_json(path)
        payload["_source_path"] = str(path)
        payload["_source_stem"] = path.stem
        profiles.append(payload)
    return profiles


def _resolve_profile(
    repo_root: Path,
    selector: str | None,
) -> dict[str, Any] | None:
    if not selector:
        return None
    wanted = _normalized(selector)
    for profile in _load_provider_profiles(repo_root):
        if wanted in {
            _normalized(profile["profile_id"]),
            _normalized(profile.get("display_name", "")),
            _normalized(profile.get("_source_stem", "")),
        }:
            return profile
    raise ValueError(f"unknown provider profile '{selector}'")


def _collect_default_provider_candidates(preset: dict[str, Any]) -> list[str]:
    ordered: list[str] = []
    for spec in preset.get("related", {}).values():
        if spec.get("yfinance") and "yfinance" not in ordered:
            ordered.append("yfinance")
        if spec.get("tradingview_mcp") and "tradingview_mcp" not in ordered:
            ordered.append("tradingview_mcp")
        if spec.get("ibkr") and "ibkr" not in ordered:
            ordered.append("ibkr")
    return ordered


def _build_related_symbols(preset: dict[str, Any]) -> list[dict[str, Any]]:
    symbols: list[dict[str, Any]] = []
    for role, spec in sorted(preset.get("related", {}).items()):
        provider_symbols: dict[str, Any] = {}
        if spec.get("display_symbol"):
            provider_symbols["display_symbol"] = spec["display_symbol"]
        if spec.get("yfinance"):
            provider_symbols["yfinance"] = spec["yfinance"]
        if spec.get("tradingview_mcp"):
            provider_symbols["tradingview_mcp"] = spec["tradingview_mcp"]
        if spec.get("ibkr"):
            provider_symbols["ibkr"] = spec["ibkr"]
        symbols.append(
            {
                "role": role,
                "display_symbol": spec.get("display_symbol"),
                "provider_symbols": provider_symbols,
            }
        )
    return symbols


def _resolved_live_defaults(preset: dict[str, Any]) -> dict[str, Any]:
    live_defaults = dict(preset.get("live_defaults") or {})
    related = preset.get("related", {})
    spot_role = live_defaults.get("spot_role")
    options_role = live_defaults.get("options_role")
    if spot_role and spot_role in related:
        live_defaults["spot_symbol"] = related[spot_role].get("display_symbol")
    if options_role and options_role in related:
        live_defaults["options_symbol"] = related[options_role].get("display_symbol")
    return live_defaults


def _dataset_entries_from_profile(
    profile: dict[str, Any],
    default_providers: list[str],
) -> list[dict[str, Any]]:
    datasets: list[dict[str, Any]] = []
    for contract in profile.get("data_contracts", []):
        path_hint = contract.get("path_hint")
        datasets.append(
            {
                "dataset_id": contract["contract_id"],
                "category": contract["category"],
                "required": contract["required"],
                "label": contract["label"],
                "symbols": contract.get("symbols", []),
                "timeframes": contract.get("timeframes", []),
                "path_hint": path_hint,
                "path_exists": bool(path_hint) and Path(path_hint).expanduser().exists(),
                "opt_in_only": profile.get("opt_in_only", False),
                "selection_profile_id": profile["profile_id"],
                "provider_candidates": list(default_providers),
                "notes": contract.get("notes", []),
            }
        )
    return datasets


def _generic_dataset_entries(
    preset: dict[str, Any],
    relationship: dict[str, Any],
    default_providers: list[str],
) -> list[dict[str, Any]]:
    live_defaults = preset.get("live_defaults") or {}
    datasets = [
        {
            "dataset_id": f"{preset['market_key'].lower()}_market_defaults",
            "category": "market_defaults",
            "required": True,
            "label": f"{preset['market_key']} zero-config market defaults",
            "symbols": [
                live_defaults.get("futures_symbol"),
                *[
                    item.get("display_symbol")
                    for item in preset.get("related", {}).values()
                    if item.get("display_symbol")
                ],
            ],
            "timeframes": [],
            "path_hint": None,
            "path_exists": False,
            "opt_in_only": False,
            "selection_profile_id": None,
            "provider_candidates": list(default_providers),
            "notes": [
                "Generated from repo market presets and relationships.",
                "Use this zero-config lane when no personal data profile is selected.",
            ],
        }
    ]
    if relationship:
        datasets.append(
            {
                "dataset_id": f"{preset['market_key'].lower()}_relationship_context",
                "category": "relationship_context",
                "required": False,
                "label": f"{preset['market_key']} cross-market relationship context",
                "symbols": relationship.get("related_etf_companions", [])
                + relationship.get("related_futures_symbols", [])
                + relationship.get("related_crypto_symbols", []),
                "timeframes": [],
                "path_hint": None,
                "path_exists": False,
                "opt_in_only": False,
                "selection_profile_id": None,
                "provider_candidates": list(default_providers),
                "notes": [
                    "Context-only companion symbols. This does not guarantee a reusable local dataset path."
                ],
            }
        )
    return datasets


def _selected_profile_surface(profile: dict[str, Any] | None) -> dict[str, Any] | None:
    if not profile:
        return None
    return {
        "profile_id": profile["profile_id"],
        "display_name": profile["display_name"],
        "opt_in_only": profile.get("opt_in_only", False),
        "summary": profile.get("summary", ""),
    }


def build_resolution_bundle(
    repo_root: Path | str,
    market_selector: str,
    profile_selector: str | None = None,
    timeframes: list[str] | None = None,
    bar_count: int | None = None,
) -> dict[str, Any]:
    repo_root = Path(repo_root).resolve()
    timeframes = list(timeframes or [])
    presets, relationships = _load_market_configs(repo_root)
    preset, relationship = _resolve_market(presets, relationships, market_selector)
    profile = _resolve_profile(repo_root, profile_selector)
    default_providers = _collect_default_provider_candidates(preset)

    if profile:
        datasets = _dataset_entries_from_profile(profile, default_providers)
        selection_mode = "profile_opt_in"
        selection_label = f"{profile['display_name']} ({profile['profile_id']})"
    else:
        datasets = _generic_dataset_entries(preset, relationship, default_providers)
        selection_mode = "generic_zero_config"
        selection_label = "Generic zero-config dataset resolver lane"

    live_defaults = _resolved_live_defaults(preset)
    symbol_resolution = {
        "schema_version": "symbol-resolution/v1",
        "requested_selector": market_selector,
        "market_key": preset["market_key"],
        "aliases": preset.get("aliases", []),
        "live_defaults": live_defaults,
        "relationships": relationship,
        "related_symbols": _build_related_symbols(preset),
        "selected_profile": _selected_profile_surface(profile),
    }

    data_catalog = {
        "schema_version": "data-catalog/v1",
        "market_key": preset["market_key"],
        "summary": {
            "selection_mode": selection_mode,
            "selection_label": selection_label,
            "requested_timeframes": timeframes,
            "requested_bar_count": bar_count,
            "default_provider_candidates": default_providers,
            "dataset_count": len(datasets),
        },
        "datasets": datasets,
    }

    dataset_available = any(item.get("path_hint") for item in datasets)
    normalized_dataset_summary = {
        "schema_version": "normalized-dataset-summary/v1",
        "market_key": preset["market_key"],
        "selection_mode": selection_mode,
        "selection_label": selection_label,
        "requested_timeframes": timeframes,
        "requested_bar_count": bar_count,
        "resolution_ready": True,
        "dataset_available": dataset_available,
        "provider_candidates": default_providers,
        "required_dataset_ids": [
            item["dataset_id"] for item in datasets if item.get("required")
        ],
        "optional_dataset_ids": [
            item["dataset_id"] for item in datasets if not item.get("required")
        ],
    }

    return {
        "symbol_resolution": symbol_resolution,
        "data_catalog": data_catalog,
        "normalized_dataset_summary": normalized_dataset_summary,
    }


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.write_text(
        json.dumps(payload, indent=2, sort_keys=False) + "\n",
        encoding="utf-8",
    )


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build a generic dataset resolver artifact bundle from repo market presets."
    )
    parser.add_argument(
        "--repo-root",
        default=str(Path(__file__).resolve().parents[2]),
        help="Repo root containing config/ and examples/provider_profiles/",
    )
    parser.add_argument("--market", required=True, help="Market key or alias to resolve.")
    parser.add_argument(
        "--profile",
        default=None,
        help="Optional provider profile selector for personal opt-in data lanes.",
    )
    parser.add_argument(
        "--output-dir",
        required=True,
        help="Directory where artifact JSON files will be written.",
    )
    parser.add_argument(
        "--timeframe",
        action="append",
        default=[],
        help="Requested timeframe. Repeat for multiple values.",
    )
    parser.add_argument(
        "--bar-count",
        type=int,
        default=None,
        help="Optional requested bar count for the downstream dataset request.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    bundle = build_resolution_bundle(
        repo_root=args.repo_root,
        market_selector=args.market,
        profile_selector=args.profile,
        timeframes=args.timeframe,
        bar_count=args.bar_count,
    )
    output_dir = Path(args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    _write_json(output_dir / "symbol_resolution.json", bundle["symbol_resolution"])
    _write_json(output_dir / "data_catalog.json", bundle["data_catalog"])
    _write_json(
        output_dir / "normalized_dataset_summary.json",
        bundle["normalized_dataset_summary"],
    )
    print(
        json.dumps(
            {
                "ok": True,
                "output_dir": str(output_dir),
                "market_key": bundle["symbol_resolution"]["market_key"],
                "selection_mode": bundle["normalized_dataset_summary"]["selection_mode"],
                "artifacts": [
                    "symbol_resolution.json",
                    "data_catalog.json",
                    "normalized_dataset_summary.json",
                ],
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
