//! Beta-Binomial / Dirichlet-tempered prior init for `trade_outcome`.
//!
//! Auto-Quant validated strategies report aggregate `(trade_count,
//! win_rate_pct)` for each backtest. We turn this empirical evidence
//! into Dirichlet pseudo-counts and add them — tempered by a `temper`
//! factor in `[0, 1]` — to a single chosen `parent_config` row of the
//! trading network's `trade_outcome` CPT.
//!
//! This is **prior** initialization: it seeds the starting belief
//! before any real-trade evidence flows in. Posterior updates from
//! real trades are out of scope and continue to flow through
//! `apply_feedback_to_trade_outcome_network`.
//!
//! Math (per strategy, per CPT row [w, be, l]):
//!
//! ```text
//! α_w  = p_w  * prior_strength
//! α_be = p_be * prior_strength
//! α_l  = p_l  * prior_strength
//!
//! n_w  = round(trade_count * win_rate_pct / 100)
//! n_l  = trade_count - n_w
//! n_be = 0                          (FreqTrade buckets breakeven into win/loss)
//!
//! α'_w  = α_w  + temper * n_w
//! α'_be = α_be + temper * n_be     (unchanged)
//! α'_l  = α_l  + temper * n_l
//!
//! probs' = normalize([α'_w, α'_be, α'_l])
//! ```
//!
//! When multiple strategies are applied, they are folded sequentially:
//! the post-state of strategy *i* is the prior of strategy *i+1*.
//! This makes the operation associative-in-effect (overall pseudo-counts
//! sum) and idempotent under repeated application *with the same
//! manifest*.

use std::collections::BTreeSet;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::bbn::dag::BayesianNetwork;

use super::manifest::{
    StrategyLibraryEntry, StrategyLibraryEntryStatus, StrategyLibraryManifest,
    StrategyLibraryValidationMetrics,
};

/// Default temper factor: backtest pseudo-counts are weighted at half
/// strength relative to a real trade with the same outcome.
pub const DEFAULT_TEMPER: f64 = 0.5;

/// Default Dirichlet concentration applied to the existing CPT row
/// before adding tempered Auto-Quant counts.
pub const DEFAULT_PRIOR_STRENGTH: f64 = 4.0;

/// Default `(entry_quality, factor_alignment, factor_uncertainty)`
/// indices: `(0, 0, 0)` = high entry quality, aligned, low uncertainty.
/// Auto-Quant validated runs are presumed to fire under this row.
pub const DEFAULT_DEFAULT_PARENT_CONFIG: [usize; 3] = [0, 0, 0];

/// `trade_outcome` node id in the trading network.
const TRADE_OUTCOME_NODE_ID: &str = "trade_outcome";

#[derive(Debug, Clone)]
pub struct AutoQuantPriorInitInput<'a> {
    pub manifest: &'a StrategyLibraryManifest,
    /// If `Some`, only apply the named strategies; otherwise, apply
    /// every entry whose `status == "ok"` with non-empty metrics.
    pub strategy_filter: Option<&'a [String]>,
    /// Parent configuration to update. Length and per-index range must
    /// match the `trade_outcome` node's parent topology.
    pub parent_config: Vec<usize>,
    /// `temper ∈ [0, 1]`. 0 = no prior shift; 1 = backtest counts at
    /// full conviction.
    pub temper: f64,
    /// Dirichlet concentration on the *existing* CPT row.
    pub prior_strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoQuantPriorInitStrategyEffect {
    pub strategy_name: String,
    pub mutation_id: String,
    pub trade_count: u32,
    pub n_win: u32,
    pub n_loss: u32,
    pub n_breakeven: u32,
    pub temper: f64,
    pub before_probs: Vec<f64>,
    pub after_probs: Vec<f64>,
    pub diff: Vec<f64>,
    #[serde(default)]
    pub bbn_entropy_before: f64,
    #[serde(default)]
    pub bbn_entropy_after: f64,
    #[serde(default)]
    pub bbn_entropy_reduction: f64,
    #[serde(default)]
    pub bbn_log_loss_before: f64,
    #[serde(default)]
    pub bbn_log_loss_after: f64,
    #[serde(default)]
    pub bbn_log_loss_delta: f64,
    #[serde(default)]
    pub bbn_contradiction_before: f64,
    #[serde(default)]
    pub bbn_contradiction_after: f64,
    #[serde(default)]
    pub bbn_contradiction_lift: f64,
    #[serde(default)]
    pub evidence_value_gate_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoQuantPriorInitOutcome {
    pub parent_config: Vec<usize>,
    pub initial_probs: Vec<f64>,
    pub final_probs: Vec<f64>,
    pub strategies_applied: Vec<AutoQuantPriorInitStrategyEffect>,
    pub strategies_skipped: Vec<(String, String)>,
    pub temper: f64,
    pub prior_strength: f64,
    #[serde(default)]
    pub bbn_entropy_reduction: f64,
    #[serde(default)]
    pub bbn_log_loss_delta: f64,
    #[serde(default)]
    pub bbn_contradiction_lift: f64,
    #[serde(default)]
    pub evidence_value_gate_passed: bool,
}

/// Apply the manifest's validated strategies as Dirichlet pseudo-counts
/// on a single `trade_outcome` CPT row. Mutates `network` in place.
pub fn apply_strategy_library_prior_init(
    network: &mut BayesianNetwork,
    input: AutoQuantPriorInitInput<'_>,
) -> Result<AutoQuantPriorInitOutcome> {
    if !(0.0..=1.0).contains(&input.temper) {
        bail!("temper must lie in [0, 1]; got {}", input.temper);
    }
    if input.prior_strength <= 0.0 {
        bail!("prior_strength must be > 0; got {}", input.prior_strength);
    }

    let filter_set: Option<BTreeSet<&str>> = input
        .strategy_filter
        .map(|names| names.iter().map(String::as_str).collect());

    // Capture the initial state of the targeted row, then perform a
    // pre-flight validation against the node topology before mutating.
    let initial_probs = read_cpt_row(network, &input.parent_config)?;
    let states_len = initial_probs.len();

    let mut current = initial_probs.clone();
    let mut applied: Vec<AutoQuantPriorInitStrategyEffect> = Vec::new();
    let mut skipped: Vec<(String, String)> = Vec::new();

    for entry in &input.manifest.strategies {
        if let Some(filter) = filter_set.as_ref() {
            if !filter.contains(entry.name.as_str()) {
                continue;
            }
        }
        match classify_entry_for_prior_init(entry) {
            EntryClassification::Apply(metrics) => {
                let effect =
                    compute_effect(entry, metrics, &current, input.temper, input.prior_strength)?;
                if !effect.evidence_value_gate_passed {
                    skipped.push((
                        entry.name.clone(),
                        format!(
                            "evidence_value_gate_failed: entropy_reduction={:.6} log_loss_delta={:.6} contradiction_lift={:.6}",
                            effect.bbn_entropy_reduction,
                            effect.bbn_log_loss_delta,
                            effect.bbn_contradiction_lift
                        ),
                    ));
                    continue;
                }
                if effect.after_probs.len() != states_len {
                    bail!(
                        "internal: strategy '{}' produced effect with wrong arity {} (expected {})",
                        entry.name,
                        effect.after_probs.len(),
                        states_len
                    );
                }
                current = effect.after_probs.clone();
                applied.push(effect);
            }
            EntryClassification::Skip(reason) => {
                skipped.push((entry.name.clone(), reason));
            }
        }
    }

    if !applied.is_empty() {
        write_cpt_row(network, &input.parent_config, current.clone())?;
    }
    let bbn_entropy_reduction = applied
        .iter()
        .map(|effect| effect.bbn_entropy_reduction.max(0.0))
        .sum();
    let bbn_log_loss_delta = applied
        .iter()
        .map(|effect| effect.bbn_log_loss_delta.max(0.0))
        .sum();
    let bbn_contradiction_lift = applied
        .iter()
        .map(|effect| effect.bbn_contradiction_lift.max(0.0))
        .sum();

    Ok(AutoQuantPriorInitOutcome {
        parent_config: input.parent_config,
        initial_probs,
        final_probs: current,
        strategies_applied: applied,
        strategies_skipped: skipped,
        temper: input.temper,
        prior_strength: input.prior_strength,
        bbn_entropy_reduction,
        bbn_log_loss_delta,
        bbn_contradiction_lift,
        evidence_value_gate_passed: bbn_entropy_reduction > 0.0
            || bbn_log_loss_delta > 0.0
            || bbn_contradiction_lift > 0.0,
    })
}

enum EntryClassification<'a> {
    Apply(&'a StrategyLibraryValidationMetrics),
    Skip(String),
}

fn classify_entry_for_prior_init(entry: &StrategyLibraryEntry) -> EntryClassification<'_> {
    match entry.status_kind() {
        StrategyLibraryEntryStatus::Ok => {}
        StrategyLibraryEntryStatus::Error => {
            return EntryClassification::Skip("status=error".to_string());
        }
        StrategyLibraryEntryStatus::NotRun => {
            return EntryClassification::Skip("status=not_run".to_string());
        }
        StrategyLibraryEntryStatus::Other(other) => {
            return EntryClassification::Skip(format!("status={}", other));
        }
    }
    let Some(metrics) = entry.validation_metrics.as_ref() else {
        return EntryClassification::Skip("no validation_metrics".to_string());
    };
    if metrics.trade_count == 0 {
        return EntryClassification::Skip("trade_count=0".to_string());
    }
    if !(0.0..=100.0).contains(&metrics.win_rate_pct) {
        return EntryClassification::Skip(format!(
            "win_rate_pct={} out of [0,100]",
            metrics.win_rate_pct
        ));
    }
    EntryClassification::Apply(metrics)
}

fn compute_effect(
    entry: &StrategyLibraryEntry,
    metrics: &StrategyLibraryValidationMetrics,
    before: &[f64],
    temper: f64,
    prior_strength: f64,
) -> Result<AutoQuantPriorInitStrategyEffect> {
    if before.len() != 3 {
        bail!(
            "trade_outcome row arity must be 3 [win, breakeven, loss]; got {}",
            before.len()
        );
    }

    let n = metrics.trade_count;
    let n_win = ((metrics.win_rate_pct / 100.0) * (n as f64)).round() as i64;
    let n_win = n_win.clamp(0, n as i64) as u32;
    let n_loss = n.saturating_sub(n_win);
    let n_breakeven: u32 = 0;
    let empirical = empirical_distribution(n_win, n_breakeven, n_loss)?;

    let alpha_w = before[0] * prior_strength + temper * (n_win as f64);
    let alpha_be = before[1] * prior_strength + temper * (n_breakeven as f64);
    let alpha_l = before[2] * prior_strength + temper * (n_loss as f64);
    let total = alpha_w + alpha_be + alpha_l;
    if total <= 0.0 || !total.is_finite() {
        bail!(
            "prior init produced non-finite total mass for strategy '{}': {} {} {}",
            entry.name,
            alpha_w,
            alpha_be,
            alpha_l
        );
    }
    let after = vec![alpha_w / total, alpha_be / total, alpha_l / total];
    let diff = before
        .iter()
        .zip(after.iter())
        .map(|(b, a)| a - b)
        .collect();
    let bbn_entropy_before = shannon_entropy(before);
    let bbn_entropy_after = shannon_entropy(&after);
    let bbn_entropy_reduction = (bbn_entropy_before - bbn_entropy_after).max(0.0);
    let bbn_log_loss_before = cross_entropy_loss(&empirical, before)?;
    let bbn_log_loss_after = cross_entropy_loss(&empirical, &after)?;
    let bbn_log_loss_delta = (bbn_log_loss_before - bbn_log_loss_after).max(0.0);
    let bbn_contradiction_before = contradiction_score(&empirical, before);
    let bbn_contradiction_after = contradiction_score(&empirical, &after);
    let bbn_contradiction_lift = (bbn_contradiction_before - bbn_contradiction_after).max(0.0);
    let evidence_value_gate_passed =
        bbn_entropy_reduction > 0.0 || bbn_log_loss_delta > 0.0 || bbn_contradiction_lift > 0.0;

    Ok(AutoQuantPriorInitStrategyEffect {
        strategy_name: entry.name.clone(),
        mutation_id: entry.metadata.mutation_id.clone(),
        trade_count: n,
        n_win,
        n_loss,
        n_breakeven,
        temper,
        before_probs: before.to_vec(),
        after_probs: after,
        diff,
        bbn_entropy_before,
        bbn_entropy_after,
        bbn_entropy_reduction,
        bbn_log_loss_before,
        bbn_log_loss_after,
        bbn_log_loss_delta,
        bbn_contradiction_before,
        bbn_contradiction_after,
        bbn_contradiction_lift,
        evidence_value_gate_passed,
    })
}

fn empirical_distribution(n_win: u32, n_breakeven: u32, n_loss: u32) -> Result<Vec<f64>> {
    let total = n_win as f64 + n_breakeven as f64 + n_loss as f64;
    if total <= 0.0 {
        bail!("cannot build empirical distribution from zero outcomes");
    }
    Ok(vec![
        n_win as f64 / total,
        n_breakeven as f64 / total,
        n_loss as f64 / total,
    ])
}

fn shannon_entropy(probs: &[f64]) -> f64 {
    probs
        .iter()
        .copied()
        .filter(|p| *p > 0.0)
        .map(|p| -p * p.ln())
        .sum()
}

fn cross_entropy_loss(empirical: &[f64], predicted: &[f64]) -> Result<f64> {
    if empirical.len() != predicted.len() {
        bail!(
            "empirical distribution arity {} does not match predicted arity {}",
            empirical.len(),
            predicted.len()
        );
    }
    const EPS: f64 = 1.0e-12;
    Ok(empirical
        .iter()
        .zip(predicted.iter())
        .filter(|(target, _)| **target > 0.0)
        .map(|(target, predicted)| -target * predicted.clamp(EPS, 1.0).ln())
        .sum())
}

fn contradiction_score(empirical: &[f64], predicted: &[f64]) -> f64 {
    empirical
        .iter()
        .zip(predicted.iter())
        .map(|(target, predicted)| (target - predicted).abs())
        .sum()
}

fn read_cpt_row(network: &BayesianNetwork, parent_config: &[usize]) -> Result<Vec<f64>> {
    let node = network
        .nodes
        .get(TRADE_OUTCOME_NODE_ID)
        .ok_or_else(|| anyhow!("network missing '{}' node", TRADE_OUTCOME_NODE_ID))?;
    if parent_config.len() != node.parents.len() {
        bail!(
            "parent_config arity {} does not match {} node parents {:?}",
            parent_config.len(),
            TRADE_OUTCOME_NODE_ID,
            node.parents
        );
    }
    let row = node
        .cpt
        .entries
        .get(parent_config)
        .ok_or_else(|| {
            anyhow!(
                "parent_config {:?} not present in '{}' CPT (parents={:?})",
                parent_config,
                TRADE_OUTCOME_NODE_ID,
                node.parents
            )
        })?
        .clone();
    if row.len() != node.states.len() {
        bail!(
            "CPT row arity {} does not match {} states {:?}",
            row.len(),
            TRADE_OUTCOME_NODE_ID,
            node.states
        );
    }
    Ok(row)
}

fn write_cpt_row(
    network: &mut BayesianNetwork,
    parent_config: &[usize],
    probs: Vec<f64>,
) -> Result<()> {
    let node = network
        .nodes
        .get_mut(TRADE_OUTCOME_NODE_ID)
        .ok_or_else(|| anyhow!("network missing '{}' node", TRADE_OUTCOME_NODE_ID))?;
    let entry = node
        .cpt
        .entries
        .get_mut(parent_config)
        .ok_or_else(|| anyhow!("parent_config {:?} not present in CPT", parent_config))?;
    if entry.len() != probs.len() {
        bail!(
            "internal: probs arity {} does not match existing CPT row arity {}",
            probs.len(),
            entry.len()
        );
    }
    *entry = probs;
    node.validate().with_context(|| {
        format!(
            "'{}' validation failed after prior init",
            TRADE_OUTCOME_NODE_ID
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::auto_quant::results::manifest::{
        StrategyLibraryEntry, StrategyLibraryManifest, StrategyLibraryMetadata,
        StrategyLibraryValidationMetrics,
    };
    use crate::bbn::trading::topology::build_trading_network;

    fn entry(
        name: &str,
        trade_count: u32,
        win_rate_pct: f64,
        status: &str,
    ) -> StrategyLibraryEntry {
        StrategyLibraryEntry {
            name: name.to_string(),
            file_path: format!("user_data/strategies_ibkr/{name}.py"),
            metadata: StrategyLibraryMetadata {
                strategy: name.to_string(),
                mutation_id: format!("mut-{name}"),
                ..Default::default()
            },
            status: status.to_string(),
            validation_metrics: Some(StrategyLibraryValidationMetrics {
                trade_count,
                win_rate_pct,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn manifest_with(strategies: Vec<StrategyLibraryEntry>) -> StrategyLibraryManifest {
        StrategyLibraryManifest {
            manifest_version: "1.0".to_string(),
            strategies,
            ..Default::default()
        }
    }

    /// Build a network and overwrite the targeted row with a known
    /// balanced baseline so directional assertions are CPT-init-independent.
    fn net_with_baseline_row(parent_config: &[usize], probs: Vec<f64>) -> BayesianNetwork {
        let mut net = build_trading_network().unwrap();
        let node = net.nodes.get_mut(TRADE_OUTCOME_NODE_ID).unwrap();
        node.cpt.entries.insert(parent_config.to_vec(), probs);
        node.validate().unwrap();
        net
    }

    #[test]
    fn no_op_when_no_strategies_apply() {
        let mut net = build_trading_network().unwrap();
        let m = manifest_with(vec![entry("X", 0, 0.0, "ok")]);
        let outcome = apply_strategy_library_prior_init(
            &mut net,
            AutoQuantPriorInitInput {
                manifest: &m,
                strategy_filter: None,
                parent_config: DEFAULT_DEFAULT_PARENT_CONFIG.to_vec(),
                temper: DEFAULT_TEMPER,
                prior_strength: DEFAULT_PRIOR_STRENGTH,
            },
        )
        .unwrap();
        assert!(outcome.strategies_applied.is_empty());
        assert_eq!(outcome.strategies_skipped.len(), 1);
        assert_eq!(outcome.initial_probs, outcome.final_probs);
    }

    #[test]
    fn high_win_rate_pushes_probability_toward_win_on_balanced_row() {
        let parent_config = DEFAULT_DEFAULT_PARENT_CONFIG.to_vec();
        // Balanced 33/33/34 baseline: any strategy with win_rate > 33% must
        // raise P(win) and any with win_rate < 67% must raise P(loss). At
        // 80% win rate, both conditions push P(win) up and P(loss) down.
        let baseline = vec![1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];
        let mut net = net_with_baseline_row(&parent_config, baseline.clone());
        let m = manifest_with(vec![entry("Strong", 100, 80.0, "ok")]);

        let outcome = apply_strategy_library_prior_init(
            &mut net,
            AutoQuantPriorInitInput {
                manifest: &m,
                strategy_filter: None,
                parent_config: parent_config.clone(),
                temper: DEFAULT_TEMPER,
                prior_strength: DEFAULT_PRIOR_STRENGTH,
            },
        )
        .unwrap();

        let after = &outcome.final_probs;
        let sum: f64 = after.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9, "row must normalise to 1");
        assert!(
            after[0] > baseline[0],
            "expected P(win) to rise on balanced baseline; baseline={:?} after={:?}",
            baseline,
            after
        );
        assert!(
            after[2] < baseline[2],
            "expected P(loss) to fall on balanced baseline; baseline={:?} after={:?}",
            baseline,
            after
        );
        assert_eq!(outcome.strategies_applied.len(), 1);
        let eff = &outcome.strategies_applied[0];
        assert_eq!(eff.trade_count, 100);
        assert_eq!(eff.n_win, 80);
        assert_eq!(eff.n_loss, 20);
    }

    #[test]
    fn applied_strategy_records_positive_bbn_evidence_value() {
        let parent_config = DEFAULT_DEFAULT_PARENT_CONFIG.to_vec();
        let baseline = vec![1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];
        let mut net = net_with_baseline_row(&parent_config, baseline);
        let m = manifest_with(vec![entry("Strong", 100, 80.0, "ok")]);

        let outcome = apply_strategy_library_prior_init(
            &mut net,
            AutoQuantPriorInitInput {
                manifest: &m,
                strategy_filter: None,
                parent_config,
                temper: DEFAULT_TEMPER,
                prior_strength: DEFAULT_PRIOR_STRENGTH,
            },
        )
        .unwrap();

        assert!(outcome.bbn_entropy_reduction > 0.0, "{:?}", outcome);
        assert!(outcome.bbn_log_loss_delta > 0.0, "{:?}", outcome);
        assert!(outcome.bbn_contradiction_lift > 0.0, "{:?}", outcome);
        assert!(outcome.evidence_value_gate_passed);

        let effect = &outcome.strategies_applied[0];
        assert!(effect.evidence_value_gate_passed);
        assert!(effect.bbn_entropy_reduction > 0.0, "{:?}", effect);
        assert!(effect.bbn_log_loss_delta > 0.0, "{:?}", effect);
        assert!(effect.bbn_contradiction_lift > 0.0, "{:?}", effect);
    }

    #[test]
    fn evidence_value_gate_skips_non_improving_strategy() {
        let parent_config = DEFAULT_DEFAULT_PARENT_CONFIG.to_vec();
        let mut net = net_with_baseline_row(&parent_config, vec![0.8, 0.0, 0.2]);
        let m = manifest_with(vec![entry("AlreadyPriced", 100, 80.0, "ok")]);

        let outcome = apply_strategy_library_prior_init(
            &mut net,
            AutoQuantPriorInitInput {
                manifest: &m,
                strategy_filter: None,
                parent_config,
                temper: DEFAULT_TEMPER,
                prior_strength: DEFAULT_PRIOR_STRENGTH,
            },
        )
        .unwrap();

        assert!(outcome.strategies_applied.is_empty());
        assert_eq!(outcome.initial_probs, outcome.final_probs);
        assert_eq!(outcome.bbn_entropy_reduction, 0.0);
        assert_eq!(outcome.bbn_log_loss_delta, 0.0);
        assert_eq!(outcome.bbn_contradiction_lift, 0.0);
        assert!(!outcome.evidence_value_gate_passed);
        assert_eq!(outcome.strategies_skipped.len(), 1);
        assert!(outcome.strategies_skipped[0]
            .1
            .contains("evidence_value_gate_failed"));
    }

    #[test]
    fn empirical_dominates_when_temper_is_one_and_prior_strength_is_low() {
        // With prior_strength = 1 and temper = 1 on 1000 trades, the
        // posterior should sit very close to the empirical (0.7, 0, 0.3)
        // mix regardless of where the row started.
        let parent_config = DEFAULT_DEFAULT_PARENT_CONFIG.to_vec();
        let mut net = net_with_baseline_row(&parent_config, vec![0.999_956, 0.000_022, 0.000_022]);
        let m = manifest_with(vec![entry("S", 1000, 70.0, "ok")]);

        let outcome = apply_strategy_library_prior_init(
            &mut net,
            AutoQuantPriorInitInput {
                manifest: &m,
                strategy_filter: None,
                parent_config: parent_config.clone(),
                temper: 1.0,
                prior_strength: 1.0,
            },
        )
        .unwrap();
        let after = &outcome.final_probs;
        assert!((after[0] - 0.70).abs() < 5e-3, "after={:?}", after);
        assert!(after[1].abs() < 5e-3);
        assert!((after[2] - 0.30).abs() < 5e-3, "after={:?}", after);
    }

    #[test]
    fn temper_zero_is_a_noop_on_the_row() {
        let mut net = build_trading_network().unwrap();
        let before = read_cpt_row(&net, &DEFAULT_DEFAULT_PARENT_CONFIG).unwrap();
        let m = manifest_with(vec![entry("S", 50, 60.0, "ok")]);
        let outcome = apply_strategy_library_prior_init(
            &mut net,
            AutoQuantPriorInitInput {
                manifest: &m,
                strategy_filter: None,
                parent_config: DEFAULT_DEFAULT_PARENT_CONFIG.to_vec(),
                temper: 0.0,
                prior_strength: DEFAULT_PRIOR_STRENGTH,
            },
        )
        .unwrap();
        // temper=0 means before == after up to renormalisation.
        for (b, a) in before.iter().zip(outcome.final_probs.iter()) {
            assert!((b - a).abs() < 1e-12);
        }
    }

    #[test]
    fn errors_on_invalid_parent_config_arity() {
        let mut net = build_trading_network().unwrap();
        let m = manifest_with(vec![entry("S", 50, 60.0, "ok")]);
        let err = apply_strategy_library_prior_init(
            &mut net,
            AutoQuantPriorInitInput {
                manifest: &m,
                strategy_filter: None,
                parent_config: vec![0],
                temper: DEFAULT_TEMPER,
                prior_strength: DEFAULT_PRIOR_STRENGTH,
            },
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("parent_config arity"));
    }

    #[test]
    fn errors_on_temper_out_of_range() {
        let mut net = build_trading_network().unwrap();
        let m = manifest_with(vec![entry("S", 50, 60.0, "ok")]);
        let err = apply_strategy_library_prior_init(
            &mut net,
            AutoQuantPriorInitInput {
                manifest: &m,
                strategy_filter: None,
                parent_config: DEFAULT_DEFAULT_PARENT_CONFIG.to_vec(),
                temper: 1.5,
                prior_strength: DEFAULT_PRIOR_STRENGTH,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("temper must lie in [0, 1]"));
    }

    #[test]
    fn strategy_filter_restricts_application() {
        let mut net = build_trading_network().unwrap();
        let m = manifest_with(vec![
            entry("Keep", 100, 80.0, "ok"),
            entry("Skip", 100, 80.0, "ok"),
        ]);
        let names = vec!["Keep".to_string()];
        let outcome = apply_strategy_library_prior_init(
            &mut net,
            AutoQuantPriorInitInput {
                manifest: &m,
                strategy_filter: Some(&names),
                parent_config: DEFAULT_DEFAULT_PARENT_CONFIG.to_vec(),
                temper: DEFAULT_TEMPER,
                prior_strength: DEFAULT_PRIOR_STRENGTH,
            },
        )
        .unwrap();
        assert_eq!(outcome.strategies_applied.len(), 1);
        assert_eq!(outcome.strategies_applied[0].strategy_name, "Keep");
        assert!(outcome.strategies_skipped.is_empty());
    }
}
