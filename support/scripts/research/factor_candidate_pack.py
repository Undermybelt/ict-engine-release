from __future__ import annotations

import argparse
import ast
import json
from pathlib import Path
from statistics import mean
from typing import Any
import zipfile


def _trade_density_label(trade_count: int | None) -> str:
    if trade_count is None:
        return "external_evidence"
    if trade_count <= 0:
        return "invalid"
    if trade_count < 10:
        return "anecdotal"
    if trade_count < 30:
        return "probe_only"
    if trade_count < 80:
        return "thin"
    return "preferred_density"


def _market_status(metrics: dict[str, Any]) -> str:
    if metrics.get("trade_count") is None:
        return "external_evidence" if metrics.get("sharpe") is not None else "flat"
    trade_count = int(metrics.get("trade_count", 0) or 0)
    return "covered" if trade_count > 0 else "flat"


def _win_rate_pct(value: Any) -> float | None:
    if value is None:
        return None
    win_rate = float(value)
    if win_rate <= 1.0:
        win_rate *= 100.0
    return round(win_rate, 6)


def _max_drawdown_pct(value: Any) -> float | None:
    if value is None:
        return None
    drawdown = abs(float(value))
    if drawdown <= 1.0:
        drawdown *= 100.0
    return round(drawdown, 6)


def _docstring_metadata(source_text: str) -> dict[str, Any]:
    try:
        module = ast.parse(source_text)
    except SyntaxError:
        return {}
    doc = ast.get_docstring(module) or ""
    metadata: dict[str, Any] = {}
    for raw_line in doc.splitlines():
        line = raw_line.strip()
        if ":" not in line:
            continue
        key, _, value = line.partition(":")
        normalized_key = key.strip().lower().replace(" ", "_")
        value = value.strip()
        if not value:
            continue
        if normalized_key == "paradigm":
            metadata["paradigm"] = value
        elif normalized_key == "hypothesis":
            metadata["hypothesis"] = value
        elif normalized_key == "parent":
            metadata["parent_strategy"] = value
        elif normalized_key == "status":
            metadata["status_hint"] = value
        elif normalized_key == "external_data":
            metadata["external_data"] = value
        elif normalized_key == "uses_mtf":
            metadata["uses_mtf"] = value.lower() == "yes"
    return metadata


def build_manifest_from_freqtrade_backtest_zip(zip_path: Path) -> dict[str, Any]:
    with zipfile.ZipFile(zip_path) as archive:
        result_name = next(
            (
                name
                for name in archive.namelist()
                if name.endswith(".json") and not name.endswith("_config.json")
            ),
            None,
        )
        if not result_name:
            raise ValueError(f"{zip_path} does not contain a backtest result JSON")

        config_name = next(
            (name for name in archive.namelist() if name.endswith("_config.json")),
            None,
        )
        result_payload = json.loads(archive.read(result_name))
        config_payload = (
            json.loads(archive.read(config_name)) if config_name else {}
        )

        strategies: list[dict[str, Any]] = []
        for strategy_name, payload in (result_payload.get("strategy") or {}).items():
            strategy_source_name = next(
                (
                    name
                    for name in archive.namelist()
                    if name.endswith(f"{strategy_name}.py")
                ),
                None,
            )
            strategy_metadata = {}
            if strategy_source_name:
                strategy_metadata = _docstring_metadata(
                    archive.read(strategy_source_name).decode("utf-8")
                )

            pair_metrics: dict[str, Any] = {}
            total_pair_metrics: dict[str, Any] | None = None
            for item in payload.get("results_per_pair", []):
                key = item.get("key")
                if not key:
                    continue
                metrics = {
                    "sharpe": item.get("sharpe"),
                    "trade_count": int(item.get("trades", 0) or 0),
                    "win_rate_pct": _win_rate_pct(item.get("winrate")),
                    "profit_factor": item.get("profit_factor"),
                    "total_profit_pct": item.get("profit_total_pct"),
                    "max_drawdown_pct": _max_drawdown_pct(
                        item.get("max_drawdown_account")
                    ),
                }
                if key == "TOTAL":
                    total_pair_metrics = metrics
                else:
                    pair_metrics[key] = metrics

            aggregate_trade_count = int(
                payload.get("total_trades")
                or (total_pair_metrics or {}).get("trade_count")
                or 0
            )
            total_profit_pct = None
            if payload.get("profit_total") is not None:
                total_profit_pct = round(float(payload["profit_total"]) * 100.0, 6)
            elif total_pair_metrics:
                total_profit_pct = total_pair_metrics.get("total_profit_pct")

            strategies.append(
                {
                    "name": payload.get("strategy_name") or strategy_name,
                    "status": "ok",
                    "metadata": {
                        "strategy": strategy_name,
                        "hypothesis": strategy_metadata.get("hypothesis", ""),
                        "paradigm": strategy_metadata.get("paradigm"),
                        "source_artifact": str(zip_path),
                        "strategy_source_name": strategy_source_name,
                        "parent_strategy": strategy_metadata.get("parent_strategy"),
                        "status_hint": strategy_metadata.get("status_hint"),
                        "uses_mtf": strategy_metadata.get("uses_mtf"),
                        "external_data": strategy_metadata.get("external_data"),
                    },
                    "validation_metrics": {
                        "sharpe": payload.get("sharpe"),
                        "trade_count": aggregate_trade_count,
                        "win_rate_pct": _win_rate_pct(
                            (
                                float(payload.get("wins")) / aggregate_trade_count
                                if payload.get("wins") is not None and aggregate_trade_count
                                else None
                            )
                        ),
                        "profit_factor": payload.get("profit_factor"),
                        "total_profit_pct": total_profit_pct,
                        "max_drawdown_pct": _max_drawdown_pct(
                            payload.get("max_drawdown_account")
                        ),
                    },
                    "per_pair_metrics": pair_metrics,
                }
            )

    if not strategies:
        raise ValueError(f"{zip_path} does not contain strategy results")

    timeframe = strategies[0]["validation_metrics"].get("timeframe") or config_payload.get(
        "timeframe"
    )
    if not timeframe:
        timeframe = next(
            (
                payload.get("timeframe")
                for payload in (result_payload.get("strategy") or {}).values()
                if payload.get("timeframe")
            ),
            None,
        )

    return {
        "manifest_version": "freqtrade-backtest-manifest/v1",
        "timeframe": timeframe,
        "strategies": strategies,
    }


def build_strategy_library_manifest_from_freqtrade_backtest_zip(
    zip_path: Path,
    *,
    repo_url: str = "",
    pinned_ref: str = "",
    config_path: str = "",
    log_path: str = "",
    exported_at: str | None = None,
) -> dict[str, Any]:
    manifest = build_manifest_from_freqtrade_backtest_zip(zip_path)
    strategies: list[dict[str, Any]] = []
    for strategy in manifest.get("strategies", []):
        metadata = strategy.get("metadata", {})
        strategy_name = strategy.get("name", "")
        per_pair_metrics = strategy.get("per_pair_metrics") or {}
        strategies.append(
            {
                "name": strategy_name,
                "file_path": metadata.get("strategy_source_name", ""),
                "metadata": {
                    "strategy": metadata.get("strategy", strategy_name),
                    "mutation_id": metadata.get("mutation_id", ""),
                    "base_factor": metadata.get("base_factor", ""),
                    "hypothesis": metadata.get("hypothesis", ""),
                    "paradigm": metadata.get("paradigm", ""),
                    "expected_regime": metadata.get("expected_regime", ""),
                    "factors_used": metadata.get("factors_used", []),
                    "parent": metadata.get("parent_strategy", ""),
                    "asset_class": metadata.get("asset_class", ""),
                    "status": metadata.get("status_hint", "active"),
                    "created": metadata.get("created", ""),
                },
                "status": strategy.get("status", "ok"),
                "validation_metrics": strategy.get("validation_metrics"),
                "per_pair_metrics": per_pair_metrics,
                "pairs": list(per_pair_metrics.keys()),
                "timerange": (
                    f"{strategy.get('validation_metrics', {}).get('backtest_start', '')}"
                ),
                "commit": pinned_ref,
                "error": None,
            }
        )
    return {
        "manifest_version": "1.0",
        "exported_at": exported_at or "",
        "auto_quant_repo_url": repo_url,
        "auto_quant_pinned_ref": pinned_ref,
        "config_path": config_path,
        "timeframe": manifest.get("timeframe", ""),
        "log_path": log_path,
        "strategies": strategies,
        "validation_errors": [],
    }


def _select_strategy(
    manifest: dict[str, Any],
    strategy_name: str | None,
) -> dict[str, Any]:
    strategies = manifest.get("strategies", [])
    if not strategies:
        raise ValueError("manifest contains no strategies")
    if strategy_name:
        for strategy in strategies:
            if strategy.get("name") == strategy_name:
                return strategy
        raise ValueError(f"strategy '{strategy_name}' not found in manifest")
    return strategies[0]


def _candidate_expression(
    strategy: dict[str, Any],
    manifest: dict[str, Any],
    candidate_spec: dict[str, Any],
) -> dict[str, Any]:
    metadata = strategy.get("metadata", {})
    operator_set = candidate_spec.get("operator_set") or metadata.get("factors_used", [])
    return {
        "schema_version": "factor-expression/v1",
        "candidate_id": candidate_spec.get("candidate_id"),
        "display_name": candidate_spec.get("display_name"),
        "family": candidate_spec.get("family"),
        "status": candidate_spec.get("status"),
        "promotion_state": candidate_spec.get("promotion_state"),
        "strategy_name": strategy.get("name"),
        "mutation_id": candidate_spec.get("mutation_id") or metadata.get("mutation_id"),
        "base_factor": candidate_spec.get("base_factor") or metadata.get("base_factor"),
        "expression_text": candidate_spec.get("expression_text")
        or metadata.get("hypothesis", ""),
        "operator_set": operator_set,
        "complexity": candidate_spec.get("complexity", len(operator_set)),
        "paradigm": candidate_spec.get("paradigm") or metadata.get("paradigm"),
        "expected_regime": candidate_spec.get("expected_regime")
        or metadata.get("expected_regime"),
        "target_market_hypothesis": candidate_spec.get(
            "target_market_hypothesis",
            list(strategy.get("per_pair_metrics", {}).keys()),
        ),
        "base_timeframe": candidate_spec.get("base_timeframe", manifest.get("timeframe")),
        "context_timeframes": candidate_spec.get("context_timeframes", []),
        "regime_role": candidate_spec.get("regime_role", "mixed"),
        "evidence_window": candidate_spec.get("evidence_window"),
        "strategy_source": candidate_spec.get("strategy_source"),
        "filter_belief_execution_mapping": {
            "pre_bayes_targets": candidate_spec.get("pre_bayes_targets", []),
            "belief_targets": candidate_spec.get("belief_targets", []),
            "path_ranking_targets": candidate_spec.get("path_ranking_targets", []),
            "execution_tree_targets": candidate_spec.get("execution_tree_targets", []),
            "execution_tree_blockers_intended": candidate_spec.get(
                "execution_tree_blockers_intended", []
            ),
            "structural_feedback_required": candidate_spec.get(
                "structural_feedback_required", False
            ),
        },
    }


def _eval_grid_summary(
    strategy: dict[str, Any],
    manifest: dict[str, Any],
    candidate_spec: dict[str, Any],
    autoresearch_status: dict[str, Any],
) -> dict[str, Any]:
    aggregate = strategy.get("validation_metrics") or {}
    per_pair = strategy.get("per_pair_metrics") or {}
    breadth_matrix: dict[str, Any] = {}
    for market, metrics in per_pair.items():
        trade_count = int(metrics.get("trade_count", 0) or 0)
        breadth_matrix[market] = {
            "status": _market_status(metrics),
            "trade_count": trade_count,
            "trade_density_label": _trade_density_label(trade_count),
            "sharpe": metrics.get("sharpe"),
            "win_rate_pct": metrics.get("win_rate_pct"),
            "profit_factor": metrics.get("profit_factor"),
            "total_profit_pct": metrics.get("total_profit_pct"),
            "max_drawdown_pct": metrics.get("max_drawdown_pct"),
        }
    for market, metrics in (candidate_spec.get("cross_market_metrics") or {}).items():
        if market in breadth_matrix:
            continue
        trade_count = metrics.get("trade_count")
        breadth_matrix[market] = {
            "status": _market_status(metrics),
            "trade_count": trade_count,
            "trade_density_label": _trade_density_label(
                int(trade_count) if trade_count is not None else None
            ),
            "sharpe": metrics.get("sharpe"),
            "win_rate_pct": metrics.get("win_rate_pct"),
            "profit_factor": metrics.get("profit_factor"),
            "total_profit_pct": metrics.get("total_profit_pct"),
            "max_drawdown_pct": metrics.get("max_drawdown_pct"),
            "source_window": metrics.get("window"),
            "evidence_source": metrics.get("evidence_source", "candidate_spec"),
            "notes": metrics.get("notes", []),
        }
    aggregate_trade_count = int(aggregate.get("trade_count", 0) or 0)
    return {
        "schema_version": "factor-eval-grid-summary/v1",
        "selected_strategy": strategy.get("name"),
        "timeframe": manifest.get("timeframe"),
        "candidate_status": candidate_spec.get("status"),
        "promotion_state": candidate_spec.get("promotion_state"),
        "breadth_matrix": breadth_matrix,
        "trade_density_summary": {
            "aggregate_trade_count": aggregate_trade_count,
            "aggregate_label": _trade_density_label(aggregate_trade_count),
            "covered_market_count": sum(
                1
                for item in breadth_matrix.values()
                if item["status"] in {"covered", "external_evidence"}
            ),
        },
        "aggregate_metrics": {
            "sharpe": aggregate.get("sharpe"),
            "win_rate_pct": aggregate.get("win_rate_pct"),
            "profit_factor": aggregate.get("profit_factor"),
            "total_profit_pct": aggregate.get("total_profit_pct"),
            "max_drawdown_pct": aggregate.get("max_drawdown_pct"),
            "trade_count": aggregate_trade_count,
        },
        "resonance_summary": candidate_spec.get(
            "resonance_summary",
            {
                "base_timeframe": candidate_spec.get(
                    "base_timeframe", manifest.get("timeframe")
                ),
                "context_stack": candidate_spec.get("context_timeframes", []),
                "resonance_by_timeframe": {},
            },
        ),
        "autoresearch": {
            "effective_status": autoresearch_status.get("effective_status"),
            "decision_counts": autoresearch_status.get("decision_counts", {}),
            "failure_tag_counts": autoresearch_status.get("failure_tag_counts", {}),
            "best_attempt_score_delta": (
                (autoresearch_status.get("best_attempt") or {})
                .get("decision", {})
                .get("score_delta")
            ),
        },
        "cross_market_evidence": candidate_spec.get("cross_market_metrics", {}),
    }


def _transfer_score(
    strategy: dict[str, Any],
    manifest: dict[str, Any],
    candidate_spec: dict[str, Any],
) -> dict[str, Any]:
    per_pair = strategy.get("per_pair_metrics") or {}
    market_evidence: dict[str, Any] = {
        market: {**metrics, "evidence_source": "manifest"}
        for market, metrics in per_pair.items()
    }
    for market, metrics in (candidate_spec.get("cross_market_metrics") or {}).items():
        market_evidence.setdefault(
            market,
            {**metrics, "evidence_source": metrics.get("evidence_source", "candidate_spec")},
        )

    covered = []
    sharpe_values = []
    trade_counts = []
    markets_without_trade_counts = []
    for market, metrics in market_evidence.items():
        trade_count = metrics.get("trade_count")
        has_trade_count = trade_count is not None and int(trade_count or 0) > 0
        has_quality_signal = metrics.get("sharpe") is not None
        if has_trade_count or has_quality_signal:
            covered.append(market)
            if metrics.get("sharpe") is not None:
                sharpe_values.append(float(metrics.get("sharpe", 0.0) or 0.0))
            if has_trade_count:
                trade_counts.append(int(trade_count or 0))
            else:
                markets_without_trade_counts.append(market)
    covered_count = len(covered)
    if covered_count <= 1:
        status = "single_market_only"
        overall_transfer_score = 0.0
    else:
        avg_sharpe = mean(sharpe_values) if sharpe_values else 0.0
        avg_trade_count = mean(trade_counts) if trade_counts else 0.0
        density_score = min(avg_trade_count / 80.0, 1.0)
        sharpe_score = max(min(avg_sharpe / 2.0, 1.0), 0.0)
        breadth_score = min(covered_count / 3.0, 1.0)
        overall_transfer_score = round(
            density_score * 0.35 + sharpe_score * 0.35 + breadth_score * 0.30,
            6,
        )
        status = "cross_market_candidate"
    return {
        "schema_version": "transfer-score/v1",
        "strategy_name": strategy.get("name"),
        "covered_market_count": covered_count,
        "covered_markets": covered,
        "markets_without_trade_counts": markets_without_trade_counts,
        "status": status,
        "overall_transfer_score": overall_transfer_score,
        "average_sharpe": round(mean(sharpe_values), 6) if sharpe_values else 0.0,
        "average_trade_count": round(mean(trade_counts), 6) if trade_counts else 0.0,
        "timeframe": manifest.get("timeframe"),
        "market_evidence": market_evidence,
        "evidence_source": (
            "manifest+candidate_spec"
            if candidate_spec.get("cross_market_metrics")
            else "manifest_only"
        ),
    }


def build_factor_candidate_pack(
    *,
    manifest: dict[str, Any],
    strategy_name: str | None = None,
    candidate_spec: dict[str, Any] | None = None,
    autoresearch_status: dict[str, Any] | None = None,
) -> dict[str, Any]:
    candidate_spec = candidate_spec or {}
    autoresearch_status = autoresearch_status or {}
    strategy = _select_strategy(manifest, strategy_name)
    return {
        "factor_expression": _candidate_expression(strategy, manifest, candidate_spec),
        "factor_eval_grid_summary": _eval_grid_summary(
            strategy, manifest, candidate_spec, autoresearch_status
        ),
        "transfer_score": _transfer_score(strategy, manifest, candidate_spec),
    }


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build a white-box factor candidate pack from Auto-Quant manifest evidence."
    )
    source_group = parser.add_mutually_exclusive_group(required=True)
    source_group.add_argument("--manifest-json")
    source_group.add_argument("--freqtrade-backtest-zip")
    parser.add_argument("--strategy-name")
    parser.add_argument("--candidate-spec-json")
    parser.add_argument("--autoresearch-status-json")
    parser.add_argument("--emit-strategy-library-json")
    parser.add_argument("--repo-url", default="")
    parser.add_argument("--pinned-ref", default="")
    parser.add_argument("--config-path", default="")
    parser.add_argument("--log-path", default="")
    parser.add_argument("--output-dir", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    manifest = (
        _load_json(Path(args.manifest_json))
        if args.manifest_json
        else build_manifest_from_freqtrade_backtest_zip(
            Path(args.freqtrade_backtest_zip)
        )
    )
    candidate_spec = (
        _load_json(Path(args.candidate_spec_json)) if args.candidate_spec_json else {}
    )
    autoresearch_status = (
        _load_json(Path(args.autoresearch_status_json))
        if args.autoresearch_status_json
        else {}
    )
    bundle = build_factor_candidate_pack(
        manifest=manifest,
        strategy_name=args.strategy_name,
        candidate_spec=candidate_spec,
        autoresearch_status=autoresearch_status,
    )
    output_dir = Path(args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    for name, payload in bundle.items():
        _write_json(output_dir / f"{name}.json", payload)
    if args.emit_strategy_library_json:
        if not args.freqtrade_backtest_zip:
            raise ValueError("--emit-strategy-library-json requires --freqtrade-backtest-zip")
        strategy_manifest = build_strategy_library_manifest_from_freqtrade_backtest_zip(
            Path(args.freqtrade_backtest_zip),
            repo_url=args.repo_url,
            pinned_ref=args.pinned_ref,
            config_path=args.config_path,
            log_path=args.log_path,
        )
        _write_json(Path(args.emit_strategy_library_json).resolve(), strategy_manifest)
    print(
        json.dumps(
            {
                "ok": True,
                "output_dir": str(output_dir),
                "strategy_name": bundle["factor_expression"]["strategy_name"],
                "artifacts": [f"{name}.json" for name in bundle],
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
