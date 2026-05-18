from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


PERSONAL_OPTIONAL_FIELDS = [
    "qqq_hv_level",
    "qqq_hv_pct_rank_252",
    "nq_vs_200d_pct",
    "vix3m_level",
    "vvix_over_vix",
    "vrp",
    "iv_rank",
    "hv_rank",
]


def _candidate(
    candidate_id: str,
    *,
    family: str,
    formula: str,
    source_refs: list[str],
    markets: list[str],
    timeframes: list[str],
    required_fields: list[str],
    optional_fields: list[str] | None = None,
    regime_targets: list[str] | None = None,
    bbn_targets: list[str] | None = None,
    path_ranking_targets: list[str] | None = None,
    execution_tree_targets: list[str] | None = None,
    operators: list[str] | None = None,
) -> dict[str, Any]:
    merged_optional = list(dict.fromkeys([*(optional_fields or []), *PERSONAL_OPTIONAL_FIELDS]))
    return {
        "candidate_id": candidate_id,
        "source_refs": source_refs,
        "family": family,
        "markets": markets,
        "timeframes": timeframes,
        "required_fields": required_fields,
        "optional_fields": merged_optional,
        "missing_optional_policy": "emit_missing_optional_and_continue",
        "promotion_gate": "probe",
        "factor_expression": {
            "expression_version": 1,
            "operators": operators or [
                "delay",
                "delta",
                "rolling_mean",
                "rolling_std",
                "rank",
                "ts_rank",
                "zscore",
            ],
            "formula": formula,
            "normalization": "rolling_zscore",
            "lookahead_safe": True,
        },
        "chain_targets": {
            "regime_targets": regime_targets or [
                "TrendExpansion",
                "RangeConsolidation",
                "ExtremeStress",
                "ReversalBrewing",
                "Unknown",
            ],
            "bbn_targets": bbn_targets or [
                "factor_alignment",
                "factor_uncertainty",
            ],
            "path_ranking_targets": path_ranking_targets or [
                "risk_adjusted_path_utility",
                "current_posterior",
            ],
            "execution_tree_targets": execution_tree_targets or [
                "execution_readiness",
                "recommendation_delta",
            ],
        },
        "artifact_contract": {
            "writes": [
                "candidate_spec.json",
                "factor_expression.json",
                "summary.json",
                "chain_verdict.json",
            ],
            "state_root": "/tmp/ict-hl/<symbol>/<session_id>/candidates/<candidate_id>/",
            "runtime_dependency_policy": "sidecar_only_no_large_framework_import",
        },
    }


def build_seed_library() -> dict[str, Any]:
    candidates = [
        _candidate(
            "tsmom_mtf_convexity_v1",
            family="trend_momentum",
            formula="zscore(sign(ret_21) + sign(ret_63) + sign(ret_252)) / realized_vol_63",
            source_refs=[
                "doi:10.1016/j.jfineco.2011.11.003",
                "aqr:a-century-of-evidence-on-trend-following-investing",
            ],
            markets=["NQ", "ES", "GC", "CL", "BTC"],
            timeframes=["15m", "1h", "4h", "1d"],
            required_fields=["open", "high", "low", "close", "volume"],
            regime_targets=["TrendExpansion", "ExtremeStress"],
        ),
        _candidate(
            "trend_crash_guard_v1",
            family="trend_momentum",
            formula="bear_market_flag * realized_vol_zscore * market_rebound_zscore",
            source_refs=["doi:10.1016/j.jfineco.2015.12.002"],
            markets=["NQ", "QQQ", "ES", "SPY"],
            timeframes=["1h", "4h", "1d"],
            required_fields=["close", "volume"],
            optional_fields=["nq_vs_200d_pct", "qqq_hv_pct_rank_252"],
            regime_targets=["ReversalBrewing", "ExtremeStress"],
            execution_tree_targets=["transition_guardrail", "block_crowded"],
        ),
        _candidate(
            "carry_momentum_blend_v1",
            family="cross_market_smt",
            formula="rank(carry_percentile) + rank(ret_63) + rank(ret_252) - rank(realized_vol_63)",
            source_refs=["doi:10.1111/jofi.12643", "doi:10.1111/jofi.12021"],
            markets=["NQ", "ES", "GC", "CL", "EUR", "JPY"],
            timeframes=["1d", "1w"],
            required_fields=["close", "volume", "carry_percentile"],
            regime_targets=["TrendExpansion", "RangeConsolidation"],
        ),
        _candidate(
            "vrp_pressure_qqq_v1",
            family="options_hedging",
            formula="zscore(vrp, 252) + zscore(vix3m_level, 126) + rank(vvix_over_vix) - zscore(qqq_hv_pct_rank_252)",
            source_refs=["doi:10.1093/rfs/hhp008", "doi:10.1093/rfs/hhg002"],
            markets=["QQQ", "NQ", "SPY", "ES"],
            timeframes=["1d", "1w"],
            required_fields=["close"],
            optional_fields=["vrp", "vix3m_level", "vvix_over_vix", "qqq_hv_pct_rank_252"],
            regime_targets=["RangeConsolidation", "ReversalBrewing", "ExtremeStress"],
            bbn_targets=["dealer_pressure", "factor_uncertainty", "crash_risk"],
        ),
        _candidate(
            "iv_hv_spread_rank_v1",
            family="options_hedging",
            formula="rank(iv_rank - hv_rank) * liquidity_filter",
            source_refs=["doi:10.1287/mnsc.1090.1063"],
            markets=["SPY", "QQQ"],
            timeframes=["1d", "1w"],
            required_fields=["close"],
            optional_fields=["iv_rank", "hv_rank", "option_bid_ask_spread"],
            bbn_targets=["dealer_pressure", "liquidity_context"],
        ),
        _candidate(
            "option_momentum_bucket_v1",
            family="options_hedging",
            formula="rank(option_return_lookback_by_moneyness_maturity_bucket) * option_liquidity_filter",
            source_refs=["doi:10.1111/jofi.13279"],
            markets=["SPY", "QQQ"],
            timeframes=["1d"],
            required_fields=["close"],
            optional_fields=["option_return", "moneyness", "days_to_expiry", "option_bid_ask_spread"],
        ),
        _candidate(
            "ofi_book_pressure_v1",
            family="crowding_herding",
            formula="(bid_depth - ask_depth) / (bid_depth + ask_depth + eps) + signed_trade_volume_zscore",
            source_refs=["doi:10.1016/j.jfineco.2013.10.006", "github:mansoor-mamnoon/limit-order-book"],
            markets=["NQ", "ES", "BTC", "ETH"],
            timeframes=["1m", "5m", "15m"],
            required_fields=["open", "high", "low", "close", "volume"],
            optional_fields=["bid_depth", "ask_depth", "signed_trade_volume"],
            bbn_targets=["crowding_pressure", "liquidity_context"],
            execution_tree_targets=["fill_viable", "block_crowded"],
        ),
        _candidate(
            "session_liquidity_quality_v1",
            family="session_liquidity",
            formula="rank(session_volume_zscore) - rank(range_compression) - rank(spread_proxy)",
            source_refs=["github:mansoor-mamnoon/limit-order-book", "ict-engine:session-liquidity-family"],
            markets=["NQ", "ES", "GC", "CL"],
            timeframes=["1m", "5m", "15m", "30m"],
            required_fields=["open", "high", "low", "close", "volume"],
            optional_fields=["session", "spread", "liquidity_sweep_score"],
            bbn_targets=["session_quality", "liquidity_context"],
        ),
        _candidate(
            "alpha101_ts_rank_delta_v1",
            family="trend_momentum",
            formula="ts_rank(delta(close, n), m)",
            source_refs=["github:STHSF/alpha101", "github:microsoft/qlib"],
            markets=["NQ", "ES", "SPY", "QQQ", "GC", "CL", "BTC"],
            timeframes=["15m", "1h", "4h", "1d"],
            required_fields=["close"],
            operators=["delta", "ts_rank", "rank", "zscore"],
        ),
        _candidate(
            "alpha101_corr_vol_price_v1",
            family="crowding_herding",
            formula="correlation(rank(volume), rank(close / open - 1), n)",
            source_refs=["github:STHSF/alpha101"],
            markets=["NQ", "ES", "SPY", "QQQ", "BTC"],
            timeframes=["5m", "15m", "1h", "1d"],
            required_fields=["open", "close", "volume"],
            operators=["rank", "correlation", "rolling_mean", "zscore"],
            bbn_targets=["crowding_pressure", "factor_uncertainty"],
        ),
        _candidate(
            "qlib_kline_shape_v1",
            family="structure_ict",
            formula="zscore(KMID) + zscore(KLEN) + zscore(KUP - KLOW) + zscore(KSFT)",
            source_refs=["github:microsoft/qlib"],
            markets=["NQ", "ES", "SPY", "QQQ", "GC", "CL", "BTC"],
            timeframes=["5m", "15m", "1h", "4h", "1d"],
            required_fields=["open", "high", "low", "close"],
            operators=["kline_shape", "zscore", "rank"],
        ),
        _candidate(
            "qlib_slope_bundle_v1",
            family="trend_momentum",
            formula="rank(ROC_n) + rank(BETA_n) + rank(RSQR_n) - rank(STD_n)",
            source_refs=["github:microsoft/qlib"],
            markets=["NQ", "ES", "SPY", "QQQ", "GC", "CL", "BTC"],
            timeframes=["15m", "1h", "4h", "1d"],
            required_fields=["close", "volume"],
            operators=["roc", "slope", "rsquare", "std", "rank"],
        ),
        _candidate(
            "residual_ou_reversion_v1",
            family="volatility_mean_reversion",
            formula="-zscore(residual_spread, n) * half_life_stability_score * cointegration_stability_score",
            source_refs=["arxiv:2106.04028", "github:hudson-and-thames/arbitragelab"],
            markets=["SPY/QQQ", "NQ/ES", "GC/SI", "BTC/ETH"],
            timeframes=["15m", "1h", "4h", "1d"],
            required_fields=["close"],
            optional_fields=["basket_components", "hedge_ratio", "cointegration_pvalue"],
            bbn_targets=["factor_alignment", "liquidity_context"],
        ),
        _candidate(
            "fx_hml_carry_v1",
            family="cross_market_smt",
            formula="rank(short_rate_differential) + rank(spot_return_63) - rank(global_fx_vol_zscore)",
            source_refs=["doi:10.1093/rfs/hhr068", "doi:10.1111/j.1540-6261.2012.01728.x"],
            markets=["EUR", "GBP", "JPY", "AUD", "CAD"],
            timeframes=["1d", "1w"],
            required_fields=["close"],
            optional_fields=["short_rate_differential", "global_fx_vol"],
            regime_targets=["TrendExpansion", "ExtremeStress"],
        ),
        _candidate(
            "crypto_mom_liquidity_v1",
            family="trend_momentum",
            formula="rank(ret_7d) + rank(ret_28d) + rank(turnover) - rank(amihud_illiquidity)",
            source_refs=["doi:10.1111/jofi.13119", "doi:10.3386/w24877"],
            markets=["BTC", "ETH", "SOL"],
            timeframes=["1h", "4h", "1d"],
            required_fields=["close", "volume"],
            optional_fields=["market_cap", "funding_rate", "exchange_count"],
            bbn_targets=["liquidity_context", "crowding_pressure"],
        ),
        _candidate(
            "low_beta_stability_v1",
            family="equity_sidecar",
            formula="-rank(rolling_beta) + rank(beta_stability) + rank(quality_proxy)",
            source_refs=["doi:10.1016/j.jfineco.2013.10.005", "ssrn:2312432"],
            markets=["SPY", "QQQ", "IWM", "DIA", "equity_universe"],
            timeframes=["1d", "1w"],
            required_fields=["close"],
            optional_fields=["market_return", "quality_proxy", "fundamental_lag_days"],
            bbn_targets=["factor_alignment", "factor_uncertainty"],
        ),
    ]
    return {
        "schema_version": "factor-formula-seed-library/v1",
        "candidate_count": len(candidates),
        "missing_optional_policy": "emit_missing_optional_and_continue",
        "runtime_dependency_policy": "sidecar_only_no_large_framework_import",
        "promotion_gate": {
            "trade_count_preferred_min": 80,
            "oos_sharpe_lcb_min": 0.0,
            "dsr_min": 0.80,
            "pbo_max": 0.10,
            "requires_execution_tree_delta": True,
        },
        "candidates": candidates,
    }


def write_library(output: Path, payload: dict[str, Any]) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Export high-Sharpe factor seed candidates.")
    parser.add_argument("--output", required=True, help="Output JSON path.")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    payload = build_seed_library()
    write_library(Path(args.output), payload)
    print(json.dumps({"ok": True, "output": args.output, "candidate_count": payload["candidate_count"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())