from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from statistics import mean, pstdev
from typing import Any


def _realized_r(trade: dict[str, Any]) -> float:
    if "realized_R" in trade:
        return float(trade["realized_R"])
    return float(trade.get("gross_R", 0.0)) - float(trade.get("cost_R", 0.0))


def _safe_mean(values: list[float]) -> float:
    return mean(values) if values else 0.0


def _sample_skew(values: list[float]) -> float:
    if len(values) < 3:
        return 0.0
    mu = mean(values)
    sigma = pstdev(values)
    if sigma == 0.0:
        return 0.0
    return sum(((value - mu) / sigma) ** 3 for value in values) / len(values)


def _sample_kurtosis(values: list[float]) -> float:
    if len(values) < 4:
        return 3.0
    mu = mean(values)
    sigma = pstdev(values)
    if sigma == 0.0:
        return 3.0
    return sum(((value - mu) / sigma) ** 4 for value in values) / len(values)


def _sharpe(values: list[float], periods_per_year: int) -> float:
    if len(values) < 2:
        return 0.0
    sigma = pstdev(values)
    if sigma == 0.0:
        return 0.0
    return mean(values) / sigma * math.sqrt(periods_per_year)


def _sortino(values: list[float], periods_per_year: int) -> float:
    if len(values) < 2:
        return 0.0
    downside = [min(0.0, value) for value in values]
    downside_sigma = math.sqrt(sum(value * value for value in downside) / len(downside))
    if downside_sigma == 0.0:
        return 0.0
    return mean(values) / downside_sigma * math.sqrt(periods_per_year)


def _tail_loss(values: list[float], percentile: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = max(0, min(len(ordered) - 1, int(percentile * len(ordered)) - 1))
    return ordered[index]


def _cvar(values: list[float], percentile: float = 0.05) -> float:
    if not values:
        return 0.0
    cutoff = _tail_loss(values, percentile)
    tail = [value for value in values if value <= cutoff]
    return mean(tail) if tail else cutoff


def _tail_ratio(values: list[float]) -> float:
    if not values:
        return 0.0
    upper = abs(_tail_loss(values, 0.95))
    lower = abs(_tail_loss(values, 0.05))
    if lower == 0.0:
        return 0.0
    return upper / lower


def _normal_cdf(value: float) -> float:
    return 0.5 * (1.0 + math.erf(value / math.sqrt(2.0)))


def _probabilistic_sharpe_ratio(
    sharpe: float,
    benchmark: float,
    sample_count: int,
    skew: float,
    kurtosis: float,
) -> float:
    if sample_count < 2:
        return 0.0
    denominator = math.sqrt(
        max(1e-12, 1.0 - skew * sharpe + ((kurtosis - 1.0) / 4.0) * sharpe * sharpe)
    )
    statistic = (sharpe - benchmark) * math.sqrt(sample_count - 1.0) / denominator
    return min(1.0, max(0.0, _normal_cdf(statistic)))


def _deflated_sharpe_benchmark(values: list[float], nb_trials: int, periods_per_year: int) -> float:
    if len(values) < 2 or nb_trials <= 1:
        return 0.0
    if pstdev(values) == 0.0:
        return 0.0
    sharpe_std = math.sqrt(periods_per_year / max(1, len(values) - 1))
    trial_penalty = math.sqrt(2.0 * math.log(max(2, nb_trials)))
    return sharpe_std * trial_penalty


def _max_drawdown(cumulative: list[float]) -> float:
    peak = 0.0
    worst = 0.0
    for value in cumulative:
        peak = max(peak, value)
        worst = min(worst, value - peak)
    return worst


def _payoff_shape(values: list[float], hit_rate: float, avg_win: float, avg_loss: float) -> str:
    if not values:
        return "empty"
    if avg_win > abs(avg_loss) * 1.5:
        return "trend_convexity"
    if hit_rate >= 0.6 and abs(avg_loss) <= avg_win:
        return "high_hit_rate_positive_skew"
    if hit_rate >= 0.55:
        return "mean_reversion_snapback"
    if _sample_skew(values) < -0.5 and hit_rate > 0.5:
        return "carry_tail_risk"
    return "mixed"


def _promotion_gate(
    trade_count: int,
    net_return: float,
    sharpe: float,
    oos_sharpe_lcb: float,
    dsr: float,
    pbo: float,
    failure_tags: list[str],
) -> str:
    hard_failures = {"under_trades", "cost_blind_negative_edge", "negative_edge", "tail_risk_hidden"}
    if hard_failures.intersection(failure_tags) or net_return <= 0.0:
        return "reject"
    if trade_count >= 80 and sharpe > 0.0 and oos_sharpe_lcb > 0.0 and dsr >= 0.8 and pbo <= 0.1:
        return "promote"
    return "probe"


def _oos_sharpe_lcb(sharpe: float, sample_count: int) -> float:
    if sample_count <= 1:
        return 0.0
    return sharpe - 1.96 / math.sqrt(sample_count)


def _pbo_proxy(dsr: float, trade_count: int, nb_trials: int) -> float:
    trial_pressure = min(1.0, math.log(max(2, nb_trials)) / 10.0)
    density_relief = min(0.5, trade_count / 160.0)
    return min(1.0, max(0.0, (1.0 - dsr) * 0.7 + trial_pressure - density_relief))


def build_payoff_shape_report(
    *,
    candidate_id: str,
    trades: list[dict[str, Any]],
    nb_trials: int,
    periods_per_year: int = 252,
) -> dict[str, Any]:
    values = [_realized_r(trade) for trade in trades]
    wins = [value for value in values if value > 0.0]
    losses = [value for value in values if value < 0.0]
    cumulative: list[float] = []
    running = 0.0
    for value in values:
        running += value
        cumulative.append(running)

    trade_count = len(values)
    net_return = sum(values)
    gross_return = sum(float(trade.get("gross_R", trade.get("realized_R", 0.0))) for trade in trades)
    cost_total = sum(float(trade.get("cost_R", 0.0)) for trade in trades)
    hit_rate = len(wins) / trade_count if trade_count else 0.0
    avg_win = _safe_mean(wins)
    avg_loss = _safe_mean(losses)
    sharpe = _sharpe(values, periods_per_year)
    sortino = _sortino(values, periods_per_year)
    skew = _sample_skew(values)
    kurtosis = _sample_kurtosis(values)
    effective_trials = max(1, int(nb_trials))
    effective_sample_size = trade_count
    deflated_benchmark = _deflated_sharpe_benchmark(values, effective_trials, periods_per_year)
    psr = _probabilistic_sharpe_ratio(sharpe, 0.0, effective_sample_size, skew, kurtosis)
    dsr = _probabilistic_sharpe_ratio(sharpe, deflated_benchmark, effective_sample_size, skew, kurtosis)
    oos_sharpe_lcb = _oos_sharpe_lcb(sharpe, effective_sample_size)
    pbo = _pbo_proxy(dsr, trade_count, effective_trials)
    max_drawdown = _max_drawdown(cumulative)
    cvar_95 = _cvar(values, 0.05)
    tail_loss_p95 = _tail_loss(values, 0.05)
    tail_ratio = _tail_ratio(values)
    profit_factor = sum(wins) / abs(sum(losses)) if losses else 0.0
    avg_rr = avg_win / abs(avg_loss) if avg_loss else 0.0
    calmar = net_return / abs(max_drawdown) if max_drawdown else 0.0
    failure_tags: list[str] = []

    if trade_count == 0:
        failure_tags.append("under_trades")
    elif trade_count < 30:
        failure_tags.append("thin_density")
    if gross_return > 0.0 and net_return <= 0.0:
        failure_tags.append("cost_blind_negative_edge")
    if net_return <= 0.0 and "cost_blind_negative_edge" not in failure_tags:
        failure_tags.append("negative_edge")
    if dsr < 0.8:
        failure_tags.append("low_dsr")
    if pbo > 0.1:
        failure_tags.append("high_pbo")
    if tail_ratio < 0.5 and cvar_95 < -1.0:
        failure_tags.append("tail_risk_hidden")

    return {
        "schema_version": "factor-payoff-shape/v1",
        "candidate_id": candidate_id,
        "trade_count": trade_count,
        "nb_trials": nb_trials,
        "gross_return_R": gross_return,
        "cost_total_R": cost_total,
        "net_return_R": net_return,
        "hit_rate": hit_rate,
        "avg_win": avg_win,
        "avg_loss": avg_loss,
        "win_loss_ratio": avg_rr,
        "avg_rr": avg_rr,
        "profit_factor": profit_factor,
        "sharpe": sharpe,
        "sortino": sortino,
        "calmar": calmar,
        "oos_sharpe_lcb": oos_sharpe_lcb,
        "psr": psr,
        "dsr": dsr,
        "pbo": pbo,
        "deflated_sharpe_benchmark": deflated_benchmark,
        "effective_trials": effective_trials,
        "effective_sample_size": effective_sample_size,
        "max_drawdown_R": max_drawdown,
        "cvar_95": cvar_95,
        "skew": skew,
        "kurtosis": kurtosis,
        "tail_loss_p95": tail_loss_p95,
        "tail_ratio": tail_ratio,
        "payoff_shape": _payoff_shape(values, hit_rate, avg_win, avg_loss),
        "failure_tags": failure_tags,
        "promotion_gate": _promotion_gate(
            trade_count,
            net_return,
            sharpe,
            oos_sharpe_lcb,
            dsr,
            pbo,
            failure_tags,
        ),
    }


def _load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build factor payoff-shape report from trade JSONL.")
    parser.add_argument("--candidate-id", required=True)
    parser.add_argument("--trades-jsonl", required=True)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--nb-trials", type=int, default=1)
    parser.add_argument("--periods-per-year", type=int, default=252)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    report = build_payoff_shape_report(
        candidate_id=args.candidate_id,
        trades=_load_jsonl(Path(args.trades_jsonl)),
        nb_trials=args.nb_trials,
        periods_per_year=args.periods_per_year,
    )
    out = Path(args.output_json)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2, sort_keys=False) + "\n", encoding="utf-8")
    print(json.dumps({"ok": True, "candidate_id": args.candidate_id, "output": str(out)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())