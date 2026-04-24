"""
ict-engine paper integration bridge.

Imports safety/execution modules from paper2code/ and provides
a unified interface for ict-engine's factor mutation, autoresearch,
and Ising overlay pipelines.

Usage:
    from paper2code_bridge import (
        check_friction_barrier,
        check_survivor_bias,
        check_mode_collapse,
        track_capital_decay,
        estimate_execution_cost,
        validate_ising_overlay,
        get_mutation_hints,
    )
"""

import sys
from pathlib import Path

# Add paper2code subdirs to path (skip rammstein — needs torch)
_p2c = Path(__file__).parent
for sub in ["red_queens_trap", "kyle_stochastic_liquidity", "crowded_trades",
            "factor_engine", "ising_stylized_facts", "phi4_field_theory",
            "wasserstein_kelly", "backtest_overfitting"]:
    p = _p2c / sub
    if p.exists() and str(p) not in sys.path:
        sys.path.insert(0, str(p))

import numpy as np
import importlib
import importlib.util

def _load_module(name: str, subdir: str, filename: str):
    """Load a module from paper2code subdir without __init__.py side effects."""
    mod_path = _p2c / subdir / "src" / filename
    spec = importlib.util.spec_from_file_location(name, mod_path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

# ── Red Queen's Trap ───────────────────────────────────────────────────

_friction = _load_module("friction_barrier", "red_queens_trap", "friction_barrier.py")
_survivor = _load_module("survivor_bias", "red_queens_trap", "survivor_bias.py")
_collapse = _load_module("mode_collapse", "red_queens_trap", "mode_collapse.py")
_decay = _load_module("capital_decay", "red_queens_trap", "capital_decay.py")

FrictionCosts = _friction.FrictionCosts
_breakeven = _friction.breakeven_win_rate
_survivor_check = _survivor.apply_to_factor_mutation
_collapse_check = _collapse.apply_to_factor_diversity
_decay_check = _decay.apply_to_autoresearch


def check_friction_barrier(
    model_accuracy: float,
    reward_risk_ratio: float = 1.0,
    risk_pct: float = 1.0,
    costs: FrictionCosts = None,
) -> dict:
    """Check if model accuracy exceeds breakeven win rate.
    
    Returns:
        {"breakeven": float, "exceeds": bool, "margin": float}
    """
    if costs is None:
        costs = FrictionCosts()
    w_be = _breakeven(reward_risk_ratio, risk_pct, costs)
    return {
        "breakeven_win_rate": w_be,
        "model_accuracy": model_accuracy,
        "exceeds_breakeven": model_accuracy > w_be,
        "margin": model_accuracy - w_be,
    }


def check_survivor_bias(
    scores_before: np.ndarray,
    scores_after: np.ndarray,
    accepted: np.ndarray,
) -> dict:
    """Check for survivor bias in factor mutation results."""
    return _survivor_check(scores_before, scores_after, accepted)


def check_mode_collapse(
    factor_scores: np.ndarray,
    factor_directions: np.ndarray,
) -> dict:
    """Check if factors have collapsed to single strategy."""
    return _collapse_check(factor_scores, factor_directions)


def track_capital_decay(
    mutation_deltas: np.ndarray,
    accepted: np.ndarray,
) -> dict:
    """Track cumulative capital decay in autoresearch."""
    return _decay_check(mutation_deltas, accepted)


# ── Kyle's Model ───────────────────────────────────────────────────────

_kyle = _load_module("kyle_lambda", "kyle_stochastic_liquidity", "kyle_lambda.py")
_kyle_est = _kyle.estimate_kyle_lambda
_kyle_cost = _kyle.execution_cost_from_kyle
_kyle_submart = _kyle.check_submartingale


def estimate_execution_cost(
    price_changes: np.ndarray,
    order_flow: np.ndarray,
    order_size: float = 100.0,
    price: float = 10000.0,
) -> dict:
    """Estimate execution cost using Kyle's Lambda."""
    est = _kyle_est(price_changes, order_flow)
    if not est.valid:
        return {"valid": False, "lambda": 0.0, "cost_bps": 0.0}
    
    cost = _kyle_cost(est.lambda_t, order_size, price)
    submart = _kyle_submart(price_changes)  # proxy: use price changes as lambda proxy
    
    return {
        "valid": True,
        "lambda": est.lambda_t,
        "market_depth": est.market_depth,
        "r_squared": est.r_squared,
        "cost_bps": cost["cost_bps"],
        "is_submartingale": submart.get("is_submartingale", False),
        "implication": submart.get("implication", ""),
    }


# ── Crowded Trades ─────────────────────────────────────────────────────

_crowding_mod = _load_module("crowding", "crowded_trades", "crowding.py")
_crowding_check = _crowding_mod.apply_to_ict_ising_overlay
_herding_check = _crowding_mod.herding_bias_from_ising


def validate_ising_crowding(
    ising_magnetization: float,
    ising_coupling: float,
    factor_directions: np.ndarray,
) -> dict:
    """Validate Ising overlay against crowded trades evidence."""
    return _crowding_check(ising_magnetization, ising_coupling, factor_directions)


# ── FactorEngine ───────────────────────────────────────────────────────

_fe_mod = _load_module("experience_kb", "factor_engine", "experience_kb.py")
ExperienceKnowledgeBase = _fe_mod.ExperienceKnowledgeBase
MutationExperience = _fe_mod.MutationExperience

# Global knowledge base instance
_kb = None


def get_kb(storage_path: str = None) -> ExperienceKnowledgeBase:
    """Get or create global knowledge base."""
    global _kb
    if _kb is None:
        _kb = ExperienceKnowledgeBase(storage_path)
    return _kb


def get_mutation_hints(factor_name: str, n: int = 3) -> list[str]:
    """Get 'avoid these' hints from failure history."""
    kb = get_kb()
    return kb.get_avoidance_hints(factor_name, n)


def record_mutation(
    factor_name: str,
    spec: dict,
    score_before: float,
    score_after: float,
    accepted: bool,
    failure_tags: list[str],
    regime: str = "",
    objective: str = "",
):
    """Record a mutation experience for future learning."""
    kb = get_kb()
    kb.record(MutationExperience(
        mutation_id=f"{factor_name}_{hash(str(spec))}",
        timestamp=__import__('time').time(),
        factor_name=factor_name,
        spec_hash=MutationExperience.spec_hash_fn(spec),
        score_before=score_before,
        score_after=score_after,
        score_delta=score_after - score_before,
        accepted=accepted,
        failure_tags=failure_tags,
        failure_reasons=[f"tag: {t}" for t in failure_tags],
        regime=regime,
        objective=objective,
        params_after=spec,
    ))


# ── Ising Stylized Facts ──────────────────────────────────────────────

_ising_mod = _load_module("cluster_persistence", "ising_stylized_facts", "cluster_persistence.py")
_cluster_metrics = _ising_mod.cluster_persistence_metrics


# ── Wasserstein-Kelly ─────────────────────────────────────────────────

_kelly_mod = _load_module("kelly", "wasserstein_kelly", "kelly.py")
standard_kelly = _kelly_mod.standard_kelly
wasserstein_kelly = _kelly_mod.wasserstein_kelly
fractional_kelly = _kelly_mod.fractional_kelly
kelly_for_single_bet = _kelly_mod.kelly_for_single_bet


# ── Backtest Overfitting ──────────────────────────────────────────────

_overfit_mod = _load_module("overfitting", "backtest_overfitting", "overfitting.py")
estimate_overfitting_probability = _overfit_mod.estimate_overfitting_probability
combinatorial_cv = _overfit_mod.combinatorial_cv
apply_to_factor_overfitting = _overfit_mod.apply_to_factor_mutation


def validate_ising_cluster(
    spins_t: np.ndarray,
    spins_t1: np.ndarray,
) -> dict:
    """Validate Ising cluster persistence."""
    stats = _cluster_metrics(spins_t, spins_t1)
    return {
        "persistence": stats.persistence,
        "absolute_overlap": stats.absolute_overlap,
        "n_clusters": stats.n_clusters,
        "giant_component_frac": stats.giant_component_frac,
        "valid": stats.valid,
    }


# ── Quick check all ────────────────────────────────────────────────────

def run_all_checks(
    scores_before: np.ndarray = None,
    scores_after: np.ndarray = None,
    accepted: np.ndarray = None,
    factor_scores: np.ndarray = None,
    factor_directions: np.ndarray = None,
    mutation_deltas: np.ndarray = None,
) -> dict:
    """Run all safety checks on current state.
    
    Returns a consolidated risk report.
    """
    report = {}
    
    if scores_before is not None and scores_after is not None and accepted is not None:
        report["survivor_bias"] = check_survivor_bias(scores_before, scores_after, accepted)
        report["capital_decay"] = track_capital_decay(scores_after - scores_before, accepted)
    
    if factor_scores is not None and factor_directions is not None:
        report["mode_collapse"] = check_mode_collapse(factor_scores, factor_directions)
    
    # Friction barrier (always check with default params)
    report["friction_barrier"] = check_friction_barrier(model_accuracy=0.52)
    
    return report


if __name__ == "__main__":
    # Smoke test
    import numpy as np
    
    print("=== paper2code_bridge smoke test ===")
    
    # Friction
    fb = check_friction_barrier(0.52)
    print(f"Friction: W_BE={fb['breakeven_win_rate']:.1%}, exceeds={fb['exceeds_breakeven']}")
    
    # Survivor bias
    sb = check_survivor_bias(
        np.random.randn(20)*0.01, np.random.randn(20)*0.01,
        np.random.rand(20) > 0.5
    )
    print(f"Survivor bias: gap={sb['survivor_bias_pct']:.2f}%")
    
    # Mode collapse
    mc = check_mode_collapse(np.array([0.8,0.7,0.6]), np.array([1,1,1]))
    print(f"Mode collapse: diverse={mc['is_diverse']}")
    
    # Mutation hints
    hints = get_mutation_hints("structure_ict")
    print(f"Hints: {len(hints)}")
    
    print("\nAll bridge checks passed.")
