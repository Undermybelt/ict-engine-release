use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Duration, Utc};

use crate::domain::belief::ObjectiveMarketCredibilityShrink;
use crate::domain::regime::{JumpModelRegimeSummary, RegimeDisagreementSummary, RegimeFeatures};
use crate::state::{
    load_state_or_default, save_state, BacktestRunRecord, ResearchRunRecord, WorkflowSnapshot,
};

const MARKET_JUMP_CALIBRATION_FILE: &str = "market_jump_calibration.json";
const MARKET_JUMP_OBJECTIVE_CALIBRATION_FILE: &str = "market_jump_objective_calibration.json";

#[derive(Debug, Clone, Copy)]
struct MarketJumpCalibration {
    trend_weight: f64,
    balance_weight: f64,
    transition_weight: f64,
    backtest_edge: f64,
}

const MARKET_JUMP_CALIBRATION_MIN_SAMPLES: usize = 3;
const MARKET_JUMP_CALIBRATION_COOLDOWN_HOURS: i64 = 12;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PersistedMarketJumpCalibration {
    trend_weight: f64,
    balance_weight: f64,
    transition_weight: f64,
    backtest_edge: f64,
    #[serde(default)]
    sample_count: usize,
    #[serde(default)]
    updated_at: Option<DateTime<Utc>>,
}

fn market_jump_calibration(
    market_family: Option<&str>,
    market_behavior_profile: Option<&str>,
) -> MarketJumpCalibration {
    let family = match market_family {
        Some("futures_index") => MarketJumpCalibration {
            trend_weight: 1.08,
            balance_weight: 0.95,
            transition_weight: 1.01,
            backtest_edge: 0.06,
        },
        Some("metals") => MarketJumpCalibration {
            trend_weight: 0.93,
            balance_weight: 1.09,
            transition_weight: 0.94,
            backtest_edge: -0.02,
        },
        Some("energy") => MarketJumpCalibration {
            trend_weight: 0.82,
            balance_weight: 0.88,
            transition_weight: 1.24,
            backtest_edge: 0.18,
        },
        _ => MarketJumpCalibration {
            trend_weight: 1.0,
            balance_weight: 1.0,
            transition_weight: 1.0,
            backtest_edge: 0.0,
        },
    };

    match market_behavior_profile {
        Some("energy_volatility_shock_sensitive") => MarketJumpCalibration {
            transition_weight: family.transition_weight + 0.08,
            backtest_edge: family.backtest_edge + 0.06,
            ..family
        },
        Some("metals_defensive_liquidity_sensitive") => MarketJumpCalibration {
            balance_weight: family.balance_weight + 0.04,
            transition_weight: (family.transition_weight - 0.04).max(0.75),
            backtest_edge: family.backtest_edge - 0.02,
            ..family
        },
        Some("index_beta_regime_sensitive") => MarketJumpCalibration {
            trend_weight: family.trend_weight + 0.02,
            transition_weight: family.transition_weight + 0.01,
            backtest_edge: family.backtest_edge + 0.01,
            ..family
        },
        _ => family,
    }
}

fn calibration_key(market_family: Option<&str>, market_behavior_profile: Option<&str>) -> String {
    format!(
        "{}::{}",
        market_family.unwrap_or("generic"),
        market_behavior_profile.unwrap_or("generic")
    )
}

fn objective_calibration_key(market_family: Option<&str>, objective: Option<&str>) -> String {
    format!(
        "{}::{}",
        market_family.unwrap_or("generic"),
        objective.unwrap_or("generic")
    )
}

fn dynamic_market_jump_calibration<P: AsRef<Path>>(
    state_dir: P,
    symbol: &str,
    market_family: Option<&str>,
    market_behavior_profile: Option<&str>,
) -> MarketJumpCalibration {
    let baseline = market_jump_calibration(market_family, market_behavior_profile);
    let persisted: BTreeMap<String, PersistedMarketJumpCalibration> =
        load_state_or_default(&state_dir, symbol, MARKET_JUMP_CALIBRATION_FILE).unwrap_or_default();
    let objective_persisted: BTreeMap<String, PersistedMarketJumpCalibration> =
        load_state_or_default(state_dir, symbol, MARKET_JUMP_OBJECTIVE_CALIBRATION_FILE)
            .unwrap_or_default();
    let overlay = persisted.get(&calibration_key(market_family, market_behavior_profile));
    let objective_overlay = objective_persisted.get(&objective_calibration_key(
        market_family,
        market_behavior_profile,
    ));
    let Some(blended_overlay) = overlay.or(objective_overlay) else {
        return baseline;
    };

    let trend_overlay = overlay
        .map(|item| item.trend_weight)
        .or_else(|| objective_overlay.map(|item| item.trend_weight))
        .unwrap_or(blended_overlay.trend_weight);
    let balance_overlay = overlay
        .map(|item| item.balance_weight)
        .or_else(|| objective_overlay.map(|item| item.balance_weight))
        .unwrap_or(blended_overlay.balance_weight);
    let transition_overlay = overlay
        .map(|item| item.transition_weight)
        .or_else(|| objective_overlay.map(|item| item.transition_weight))
        .unwrap_or(blended_overlay.transition_weight);
    let edge_overlay = overlay
        .map(|item| item.backtest_edge)
        .or_else(|| objective_overlay.map(|item| item.backtest_edge))
        .unwrap_or(blended_overlay.backtest_edge);

    MarketJumpCalibration {
        trend_weight: ((baseline.trend_weight + trend_overlay) / 2.0).clamp(0.75, 1.35),
        balance_weight: ((baseline.balance_weight + balance_overlay) / 2.0).clamp(0.75, 1.35),
        transition_weight: ((baseline.transition_weight + transition_overlay) / 2.0)
            .clamp(0.75, 1.45),
        backtest_edge: ((baseline.backtest_edge + edge_overlay) / 2.0).clamp(-0.25, 0.35),
    }
}

fn dynamic_market_jump_objective_calibration<P: AsRef<Path>>(
    state_dir: P,
    symbol: &str,
    market_family: Option<&str>,
    objective: Option<&str>,
) -> Option<PersistedMarketJumpCalibration> {
    let persisted: BTreeMap<String, PersistedMarketJumpCalibration> =
        load_state_or_default(state_dir, symbol, MARKET_JUMP_OBJECTIVE_CALIBRATION_FILE)
            .unwrap_or_default();
    persisted
        .get(&objective_calibration_key(market_family, objective))
        .copied()
}

fn calibration_sample_weight(score: f64) -> f64 {
    if !score.is_finite() {
        return 0.0;
    }
    score.abs().clamp(0.0, 1.0)
}

fn blend_calibration_sample(
    baseline: MarketJumpCalibration,
    aggregate: &mut (f64, f64, f64, f64, f64, usize),
    sample_weight: f64,
) {
    if sample_weight <= 0.0 {
        return;
    }
    aggregate.0 += baseline.trend_weight * sample_weight;
    aggregate.1 += baseline.balance_weight * sample_weight;
    aggregate.2 += baseline.transition_weight * sample_weight;
    aggregate.3 += baseline.backtest_edge * sample_weight;
    aggregate.4 += sample_weight;
}

fn market_jump_calibration_from_records<'a, I>(
    records: I,
    updated_at: DateTime<Utc>,
) -> BTreeMap<String, PersistedMarketJumpCalibration>
where
    I: IntoIterator<Item = (Option<&'a str>, Option<&'a str>, f64)>,
{
    let mut grouped: BTreeMap<String, (f64, f64, f64, f64, f64, usize)> = BTreeMap::new();
    for (market_family, market_behavior_profile, score) in records {
        let sample_weight = calibration_sample_weight(score);
        if sample_weight <= 0.0 {
            continue;
        }
        let key = calibration_key(market_family, market_behavior_profile);
        let baseline = market_jump_calibration(market_family, market_behavior_profile);
        let aggregate = grouped.entry(key).or_insert((0.0, 0.0, 0.0, 0.0, 0.0, 0));
        blend_calibration_sample(baseline, aggregate, sample_weight);
        aggregate.5 += 1;
    }

    grouped
        .into_iter()
        .filter_map(
            |(key, (trend, balance, transition, edge, total_weight, sample_count))| {
                if total_weight <= 0.0 {
                    return None;
                }
                Some((
                    key,
                    PersistedMarketJumpCalibration {
                        trend_weight: (trend / total_weight).clamp(0.75, 1.35),
                        balance_weight: (balance / total_weight).clamp(0.75, 1.35),
                        transition_weight: (transition / total_weight).clamp(0.75, 1.45),
                        backtest_edge: (edge / total_weight).clamp(-0.25, 0.35),
                        sample_count,
                        updated_at: Some(updated_at),
                    },
                ))
            },
        )
        .collect()
}

fn should_skip_market_jump_calibration_update(
    existing: Option<&PersistedMarketJumpCalibration>,
    candidate: &PersistedMarketJumpCalibration,
) -> bool {
    if candidate.sample_count < MARKET_JUMP_CALIBRATION_MIN_SAMPLES {
        return true;
    }

    let Some(existing) = existing else {
        return false;
    };
    let Some(existing_updated_at) = existing.updated_at else {
        return false;
    };
    let Some(candidate_updated_at) = candidate.updated_at else {
        return false;
    };

    candidate_updated_at
        < existing_updated_at + Duration::hours(MARKET_JUMP_CALIBRATION_COOLDOWN_HOURS)
}

fn merge_market_jump_calibrations(
    existing: BTreeMap<String, PersistedMarketJumpCalibration>,
    candidate: BTreeMap<String, PersistedMarketJumpCalibration>,
) -> BTreeMap<String, PersistedMarketJumpCalibration> {
    let mut merged = existing;
    for (key, candidate_overlay) in candidate {
        if should_skip_market_jump_calibration_update(merged.get(&key), &candidate_overlay) {
            continue;
        }
        merged.insert(key, candidate_overlay);
    }
    merged
}

fn market_jump_objective_calibration_from_records<'a, I>(
    records: I,
    updated_at: DateTime<Utc>,
) -> BTreeMap<String, PersistedMarketJumpCalibration>
where
    I: IntoIterator<Item = (Option<&'a str>, Option<&'a str>, f64)>,
{
    let mut grouped: BTreeMap<String, (f64, f64, f64, f64, f64, usize)> = BTreeMap::new();
    for (market_family, objective, score) in records {
        let sample_weight = calibration_sample_weight(score);
        if sample_weight <= 0.0 {
            continue;
        }
        let key = objective_calibration_key(market_family, objective);
        let baseline = market_jump_calibration(market_family, None);
        let aggregate = grouped.entry(key).or_insert((0.0, 0.0, 0.0, 0.0, 0.0, 0));
        blend_calibration_sample(baseline, aggregate, sample_weight);
        aggregate.5 += 1;
    }

    grouped
        .into_iter()
        .filter_map(
            |(key, (trend, balance, transition, edge, total_weight, sample_count))| {
                if total_weight <= 0.0 {
                    return None;
                }
                Some((
                    key,
                    PersistedMarketJumpCalibration {
                        trend_weight: (trend / total_weight).clamp(0.75, 1.35),
                        balance_weight: (balance / total_weight).clamp(0.75, 1.35),
                        transition_weight: (transition / total_weight).clamp(0.75, 1.45),
                        backtest_edge: (edge / total_weight).clamp(-0.25, 0.35),
                        sample_count,
                        updated_at: Some(updated_at),
                    },
                ))
            },
        )
        .collect()
}

fn persist_market_jump_objective_calibration_from_scores<
    'a,
    P: AsRef<Path>,
    I: IntoIterator<Item = (Option<&'a str>, Option<&'a str>, f64)>,
>(
    state_dir: P,
    symbol: &str,
    records: I,
    updated_at: DateTime<Utc>,
) -> anyhow::Result<BTreeMap<String, PersistedMarketJumpCalibration>> {
    let existing: BTreeMap<String, PersistedMarketJumpCalibration> =
        load_state_or_default(&state_dir, symbol, MARKET_JUMP_OBJECTIVE_CALIBRATION_FILE)
            .unwrap_or_default();
    let candidate = market_jump_objective_calibration_from_records(records, updated_at);
    let calibrations = merge_market_jump_calibrations(existing, candidate);
    save_state(
        &state_dir,
        symbol,
        MARKET_JUMP_OBJECTIVE_CALIBRATION_FILE,
        &calibrations,
    )?;
    Ok(calibrations)
}

pub fn persist_market_jump_calibration_from_research_runs<P: AsRef<Path>>(
    state_dir: P,
    symbol: &str,
    runs: &[ResearchRunRecord],
    market_family: Option<&str>,
    market_behavior_profile: Option<&str>,
) -> anyhow::Result<BTreeMap<String, PersistedMarketJumpCalibration>> {
    let existing: BTreeMap<String, PersistedMarketJumpCalibration> =
        load_state_or_default(&state_dir, symbol, MARKET_JUMP_CALIBRATION_FILE).unwrap_or_default();
    let candidate = market_jump_calibration_from_records(
        runs.iter()
            .map(|run| (market_family, market_behavior_profile, run.aggregate_return)),
        runs.iter()
            .map(|run| run.timestamp)
            .max()
            .unwrap_or_else(Utc::now),
    );
    let calibrations = merge_market_jump_calibrations(existing, candidate);
    save_state(
        &state_dir,
        symbol,
        MARKET_JUMP_CALIBRATION_FILE,
        &calibrations,
    )?;
    Ok(calibrations)
}

pub fn persist_market_jump_calibration_from_backtest_runs<P: AsRef<Path>>(
    state_dir: P,
    symbol: &str,
    runs: &[BacktestRunRecord],
    market_family: Option<&str>,
    market_behavior_profile: Option<&str>,
) -> anyhow::Result<BTreeMap<String, PersistedMarketJumpCalibration>> {
    let existing: BTreeMap<String, PersistedMarketJumpCalibration> =
        load_state_or_default(&state_dir, symbol, MARKET_JUMP_CALIBRATION_FILE).unwrap_or_default();
    let candidate = market_jump_calibration_from_records(
        runs.iter()
            .map(|run| (market_family, market_behavior_profile, run.total_return)),
        runs.iter()
            .map(|run| run.timestamp)
            .max()
            .unwrap_or_else(Utc::now),
    );
    let calibrations = merge_market_jump_calibrations(existing, candidate);
    save_state(
        &state_dir,
        symbol,
        MARKET_JUMP_CALIBRATION_FILE,
        &calibrations,
    )?;
    Ok(calibrations)
}

pub fn persist_market_jump_objective_calibration_from_research_runs<P: AsRef<Path>>(
    state_dir: P,
    symbol: &str,
    runs: &[ResearchRunRecord],
    market_family: Option<&str>,
    objective: Option<&str>,
) -> anyhow::Result<BTreeMap<String, PersistedMarketJumpCalibration>> {
    persist_market_jump_objective_calibration_from_scores(
        state_dir,
        symbol,
        runs.iter()
            .map(|run| (market_family, objective, run.aggregate_return)),
        runs.iter()
            .map(|run| run.timestamp)
            .max()
            .unwrap_or_else(Utc::now),
    )
}

pub fn historical_market_jump_objective_weight<P: AsRef<Path>>(
    state_dir: P,
    symbol: &str,
    market_family: Option<&str>,
    objective: Option<&str>,
) -> Option<f64> {
    dynamic_market_jump_objective_calibration(state_dir, symbol, market_family, objective)
        .map(|calibration| (1.0 + calibration.backtest_edge).clamp(0.75, 1.35))
}

pub fn objective_market_credibility_shrink(
    objective: Option<&str>,
    market_family: Option<&str>,
    credibility_score: f64,
) -> ObjectiveMarketCredibilityShrink {
    let normalized_credibility = credibility_score.clamp(0.0, 1.0);
    let objective_name = objective.unwrap_or("generic");
    let market_name = market_family.unwrap_or("generic");
    let objective_bias = match objective_name {
        "expansion_manipulation" => 0.18,
        "trend_following" => 0.10,
        "mean_reversion" => 0.08,
        _ => 0.12,
    };
    let market_bias = match market_name {
        "energy" => 0.20,
        "metals" => 0.14,
        "futures_index" => 0.10,
        _ => 0.08,
    };
    let raw_shrink = (1.0 - normalized_credibility) * (1.0 + objective_bias + market_bias);
    let shrink_weight = (1.0 - raw_shrink).clamp(0.55, 1.0);
    let shrink_triggered = shrink_weight < 0.95;
    let hard_blocked = objective_name == "expansion_manipulation"
        && market_name == "energy"
        && normalized_credibility <= 0.35;
    let mut rationale = vec![
        format!("objective={objective_name}"),
        format!("market_family={market_name}"),
        format!("credibility_score={normalized_credibility:.3}"),
        format!("objective_bias={objective_bias:.3}"),
        format!("market_bias={market_bias:.3}"),
        format!("shrink_weight={shrink_weight:.3}"),
    ];
    if shrink_triggered {
        rationale.push("objective_market_credibility_shrink=active".to_string());
    }
    if hard_blocked {
        rationale
            .push("objective_market_credibility_hard_block=return_up_oos_down_shrink".to_string());
    }

    ObjectiveMarketCredibilityShrink {
        objective: objective.map(str::to_string),
        market_family: market_family.map(str::to_string),
        credibility_score: normalized_credibility,
        shrink_weight,
        shrink_triggered,
        hard_blocked,
        rationale,
    }
}

pub fn backtest_calibrated_market_jump_weight(
    market_family: Option<&str>,
    market_behavior_profile: Option<&str>,
) -> f64 {
    let calibration = market_jump_calibration(market_family, market_behavior_profile);
    (1.0 + calibration.backtest_edge).clamp(0.75, 1.35)
}

pub fn historical_market_jump_weight<P: AsRef<Path>>(
    state_dir: P,
    symbol: &str,
    market_family: Option<&str>,
    market_behavior_profile: Option<&str>,
) -> f64 {
    let calibration =
        dynamic_market_jump_calibration(state_dir, symbol, market_family, market_behavior_profile);
    (1.0 + calibration.backtest_edge).clamp(0.75, 1.35)
}

fn normalized_distribution(entries: [(String, f64); 3]) -> BTreeMap<String, f64> {
    let total = entries.iter().map(|(_, value)| value.max(0.0)).sum::<f64>();
    entries
        .into_iter()
        .map(|(label, value)| {
            let normalized = if total > 0.0 {
                value.max(0.0) / total
            } else {
                1.0 / 3.0
            };
            (label, normalized)
        })
        .collect()
}

pub fn build_jump_model_regime_sidecar(
    features: &RegimeFeatures,
    multi_timeframe_evidence: &BTreeMap<String, String>,
    factor_evidence: &[String],
) -> JumpModelRegimeSummary {
    let market_family = factor_evidence.iter().find_map(|line| {
        line.strip_prefix("market_category=")
            .map(|value| value.to_string())
    });
    let market_behavior_profile = factor_evidence.iter().find_map(|line| {
        line.strip_prefix("market_behavior_profile=")
            .map(|value| value.to_string())
    });
    let calibration =
        market_jump_calibration(market_family.as_deref(), market_behavior_profile.as_deref());
    build_jump_model_regime_sidecar_inner(
        features,
        multi_timeframe_evidence,
        factor_evidence,
        market_family,
        market_behavior_profile,
        calibration,
    )
}

pub fn build_jump_model_regime_sidecar_with_history<P: AsRef<Path>>(
    state_dir: P,
    symbol: &str,
    features: &RegimeFeatures,
    multi_timeframe_evidence: &BTreeMap<String, String>,
    factor_evidence: &[String],
) -> JumpModelRegimeSummary {
    let market_family = factor_evidence.iter().find_map(|line| {
        line.strip_prefix("market_category=")
            .map(|value| value.to_string())
    });
    let market_behavior_profile = factor_evidence.iter().find_map(|line| {
        line.strip_prefix("market_behavior_profile=")
            .map(|value| value.to_string())
    });
    let calibration = dynamic_market_jump_calibration(
        state_dir,
        symbol,
        market_family.as_deref(),
        market_behavior_profile.as_deref(),
    );
    build_jump_model_regime_sidecar_inner(
        features,
        multi_timeframe_evidence,
        factor_evidence,
        market_family,
        market_behavior_profile,
        calibration,
    )
}

fn build_jump_model_regime_sidecar_inner(
    features: &RegimeFeatures,
    multi_timeframe_evidence: &BTreeMap<String, String>,
    factor_evidence: &[String],
    market_family: Option<String>,
    market_behavior_profile: Option<String>,
    calibration: MarketJumpCalibration,
) -> JumpModelRegimeSummary {
    let trend_weight = calibration.trend_weight;
    let balance_weight = calibration.balance_weight;
    let transition_weight = calibration.transition_weight;
    let market_jump_weight = (1.0 + calibration.backtest_edge).clamp(0.75, 1.35);
    let trend_score = match features.market_regime_label.as_deref() {
        Some("bull") | Some("bear") | Some("trend") => 0.62 * trend_weight,
        Some("range") => 0.18 * trend_weight,
        _ => 0.34 * trend_weight,
    };
    let balance_score = match features.market_regime_label.as_deref() {
        Some("range") => 0.58 * balance_weight,
        Some("bull") | Some("bear") | Some("trend") => 0.20 * balance_weight,
        _ => 0.30 * balance_weight,
    };
    let transition_hint = multi_timeframe_evidence
        .get("filtered_resonance_label")
        .map(|value| value.as_str())
        .unwrap_or("mixed");
    let volatility = features.stress_score.unwrap_or(0.5).clamp(0.0, 1.0);
    let transition_score = (match transition_hint {
        "dislocated" => 0.66,
        "mixed" => 0.42,
        "aligned" => 0.18,
        _ => 0.34,
    } + volatility * 0.18
        + features.transition_score.unwrap_or(0.0).clamp(0.0, 1.0) * 0.24)
        * transition_weight
        * market_jump_weight;

    let state_probabilities = normalized_distribution([
        ("trend_persistent".to_string(), trend_score),
        ("balance_mean_revert".to_string(), balance_score),
        ("jump_transition".to_string(), transition_score),
    ]);

    let (active_state, confidence) = state_probabilities
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(state, probability)| (state.clone(), *probability))
        .unwrap_or_else(|| ("balance_mean_revert".to_string(), 1.0 / 3.0));

    let transition_risk = state_probabilities
        .get("jump_transition")
        .copied()
        .unwrap_or_default();
    let mut evidence = vec![
        format!("jump_model.active_state={active_state}"),
        format!("jump_model.transition_hint={transition_hint}"),
        format!("jump_model.volatility={volatility:.3}"),
        format!("jump_model.market_jump_weight={market_jump_weight:.3}"),
        format!(
            "jump_model.market_family_weighting={}:trend={:.2}:balance={:.2}:transition={:.2}",
            market_family
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            trend_weight,
            balance_weight,
            transition_weight
        ),
    ];
    if let Some(profile) = market_behavior_profile.as_deref() {
        evidence.push(format!("jump_model.market_behavior_profile={profile}"));
    }
    if let Some(liquidity) = features.liquidity_regime_label.as_deref() {
        evidence.push(format!("jump_model.liquidity={liquidity}"));
    }
    evidence.extend(
        factor_evidence
            .iter()
            .take(2)
            .map(|item| format!("jump_model.factor={item}")),
    );

    JumpModelRegimeSummary {
        active_state,
        confidence,
        transition_risk,
        market_jump_weight,
        state_probabilities,
        evidence,
    }
}

pub fn jump_model_workflow_summary(snapshot: &WorkflowSnapshot) -> Option<String> {
    let vote = snapshot.latest_ensemble_vote.as_ref()?;
    let jump_model = vote
        .executor_summaries
        .iter()
        .find(|line| line.contains("jump_model"))?;
    let gating_outcome = vote
        .executor_summaries
        .iter()
        .find(|line| line.contains("jump_calibration_gate"))
        .cloned();

    Some(match gating_outcome {
        Some(gating_outcome) => format!("{jump_model}; {gating_outcome}"),
        None => jump_model.clone(),
    })
}

pub fn jump_calibration_gate_workflow_summary(snapshot: &WorkflowSnapshot) -> Option<String> {
    let vote = snapshot.latest_ensemble_vote.as_ref()?;
    vote.executor_summaries
        .iter()
        .find(|line| line.contains("jump_calibration_gate"))
        .cloned()
}

pub fn build_regime_disagreement_summary(
    hmm_active_regime: Option<&str>,
    jump_model: Option<&JumpModelRegimeSummary>,
    shrink: Option<&ObjectiveMarketCredibilityShrink>,
) -> RegimeDisagreementSummary {
    let jump_active_state = jump_model.map(|item| item.active_state.clone());
    let aligned = match (hmm_active_regime, jump_active_state.as_deref()) {
        (Some("trend"), Some("trend_persistent")) => true,
        (Some("range"), Some("balance_mean_revert")) => true,
        (Some("transition"), Some("jump_transition")) => true,
        (Some(_), Some(_)) => false,
        _ => true,
    };
    let disagreement_score = if let Some(jump_model) = jump_model {
        if aligned {
            (1.0 - jump_model.confidence).clamp(0.0, 1.0) * 0.35
        } else {
            jump_model.confidence.clamp(0.0, 1.0)
        }
    } else {
        0.0
    };
    let gate_bias = if jump_model.is_none() {
        "hmm_only".to_string()
    } else if shrink.is_some_and(|item| item.shrink_triggered) {
        "objective_market_credibility_shrink".to_string()
    } else if aligned {
        "relax_if_other_gates_clear".to_string()
    } else {
        "shrink_and_observe".to_string()
    };
    let mut evidence = Vec::new();
    if let Some(hmm) = hmm_active_regime {
        evidence.push(format!("hmm_active_regime={hmm}"));
    }
    if let Some(jump) = &jump_active_state {
        evidence.push(format!("jump_active_state={jump}"));
    }
    evidence.push(format!("aligned={aligned}"));
    evidence.push(format!("disagreement_score={disagreement_score:.3}"));
    evidence.push(format!("gate_bias={gate_bias}"));
    if let Some(shrink) = shrink {
        evidence.push(format!("credibility_score={:.3}", shrink.credibility_score));
        evidence.push(format!("shrink_weight={:.3}", shrink.shrink_weight));
    }

    RegimeDisagreementSummary {
        hmm_active_regime: hmm_active_regime.map(|value| value.to_string()),
        jump_active_state,
        aligned,
        disagreement_score,
        gate_bias,
        evidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_model_sidecar_prefers_transition_state_when_dislocated_and_stressed() {
        let summary = build_jump_model_regime_sidecar(
            &RegimeFeatures {
                market_regime_label: Some("range".to_string()),
                liquidity_regime_label: Some("hostile".to_string()),
                stress_score: Some(0.9),
                transition_score: Some(0.8),
                ..RegimeFeatures::default()
            },
            &BTreeMap::from([(
                "filtered_resonance_label".to_string(),
                "dislocated".to_string(),
            )]),
            &[
                "market_category=energy".to_string(),
                "market_behavior_profile=energy_volatility_shock_sensitive".to_string(),
                "mtf_divergence".to_string(),
            ],
        );

        assert_eq!(summary.active_state, "jump_transition");
        assert!(summary.transition_risk > 0.4);
        assert!(summary.market_jump_weight > 1.0);
        assert_eq!(summary.state_probabilities.len(), 3);
    }

    #[test]
    fn backtest_calibrated_market_jump_weight_varies_by_market() {
        let energy = backtest_calibrated_market_jump_weight(
            Some("energy"),
            Some("energy_volatility_shock_sensitive"),
        );
        let metals = backtest_calibrated_market_jump_weight(
            Some("metals"),
            Some("metals_defensive_liquidity_sensitive"),
        );

        assert!(energy > 1.0);
        assert!(metals < 1.0);
        assert!(energy > metals);
    }

    #[test]
    fn objective_market_credibility_shrink_rules_reduce_weight_for_low_credibility_energy_expansion(
    ) {
        let shrink = objective_market_credibility_shrink(
            Some("expansion_manipulation"),
            Some("energy"),
            0.32,
        );

        assert!(shrink.shrink_triggered);
        assert!(shrink.hard_blocked);
        assert!(shrink.shrink_weight < 0.95);
        assert!(shrink.shrink_weight >= 0.55);
        assert!(shrink
            .rationale
            .iter()
            .any(|line| line.contains("return_up_oos_down_shrink")));
    }

    #[test]
    fn objective_market_credibility_shrink_does_not_hard_block_higher_credibility_case() {
        let shrink = objective_market_credibility_shrink(
            Some("expansion_manipulation"),
            Some("energy"),
            0.44,
        );

        assert!(shrink.shrink_triggered);
        assert!(!shrink.hard_blocked);
    }

    #[test]
    fn regime_disagreement_prefers_objective_market_credibility_shrink_gate_when_active() {
        let jump_model = JumpModelRegimeSummary {
            active_state: "trend_persistent".to_string(),
            confidence: 0.84,
            transition_risk: 0.20,
            market_jump_weight: 1.05,
            state_probabilities: BTreeMap::new(),
            evidence: vec![],
        };
        let shrink = objective_market_credibility_shrink(
            Some("expansion_manipulation"),
            Some("energy"),
            0.35,
        );

        let summary =
            build_regime_disagreement_summary(Some("trend"), Some(&jump_model), Some(&shrink));

        assert_eq!(summary.gate_bias, "objective_market_credibility_shrink");
        assert!(summary
            .evidence
            .iter()
            .any(|line| line.contains("shrink_weight=")));
    }

    #[test]
    fn historical_market_jump_weight_uses_persisted_overlay() {
        let temp = tempfile::tempdir().unwrap();
        let symbol = "NQ";
        let path = temp.path().join(symbol);
        std::fs::create_dir_all(&path).unwrap();
        let payload = serde_json::json!({
            "energy::energy_volatility_shock_sensitive": {
                "trend_weight": 0.90,
                "balance_weight": 0.92,
                "transition_weight": 1.40,
                "backtest_edge": 0.30
            }
        });
        std::fs::write(
            path.join(MARKET_JUMP_CALIBRATION_FILE),
            serde_json::to_string_pretty(&payload).unwrap(),
        )
        .unwrap();

        let weight = historical_market_jump_weight(
            temp.path(),
            symbol,
            Some("energy"),
            Some("energy_volatility_shock_sensitive"),
        );

        assert!(
            weight
                > backtest_calibrated_market_jump_weight(
                    Some("energy"),
                    Some("energy_volatility_shock_sensitive")
                )
        );
    }

    #[test]
    fn persist_market_jump_calibration_from_research_runs_writes_expected_profile() {
        let temp = tempfile::tempdir().unwrap();
        let symbol = "NQ";
        let timestamp = Utc::now();
        let runs = vec![
            ResearchRunRecord {
                timestamp,
                aggregate_return: 0.14,
                ..ResearchRunRecord::default()
            },
            ResearchRunRecord {
                timestamp,
                aggregate_return: 0.18,
                ..ResearchRunRecord::default()
            },
            ResearchRunRecord {
                timestamp,
                aggregate_return: 0.22,
                ..ResearchRunRecord::default()
            },
        ];

        let persisted = persist_market_jump_calibration_from_research_runs(
            temp.path(),
            symbol,
            &runs,
            Some("energy"),
            Some("energy_volatility_shock_sensitive"),
        )
        .unwrap();

        let overlay = persisted
            .get("energy::energy_volatility_shock_sensitive")
            .unwrap();
        assert!(overlay.transition_weight > 1.0);
        assert!(overlay.backtest_edge > 0.0);
        assert_eq!(overlay.sample_count, 3);
        assert_eq!(overlay.updated_at, Some(timestamp));
    }

    #[test]
    fn persist_market_jump_objective_calibration_from_research_runs_writes_expected_profile() {
        let temp = tempfile::tempdir().unwrap();
        let symbol = "NQ";
        let timestamp = Utc::now();
        let runs = vec![
            ResearchRunRecord {
                timestamp,
                aggregate_return: 0.14,
                ..ResearchRunRecord::default()
            },
            ResearchRunRecord {
                timestamp,
                aggregate_return: 0.18,
                ..ResearchRunRecord::default()
            },
            ResearchRunRecord {
                timestamp,
                aggregate_return: 0.22,
                ..ResearchRunRecord::default()
            },
        ];

        let persisted = persist_market_jump_objective_calibration_from_research_runs(
            temp.path(),
            symbol,
            &runs,
            Some("futures_index"),
            Some("expansion_manipulation"),
        )
        .unwrap();

        let overlay = persisted
            .get("futures_index::expansion_manipulation")
            .unwrap();
        assert!(overlay.trend_weight > 1.0);
        assert!(overlay.backtest_edge > 0.0);
        assert_eq!(overlay.sample_count, 3);
        assert_eq!(overlay.updated_at, Some(timestamp));
        assert!(
            historical_market_jump_objective_weight(
                temp.path(),
                symbol,
                Some("futures_index"),
                Some("expansion_manipulation")
            )
            .unwrap()
                > 1.0
        );
    }

    #[test]
    fn persist_market_jump_calibration_from_research_runs_requires_sample_threshold() {
        let temp = tempfile::tempdir().unwrap();
        let symbol = "NQ";
        let runs = vec![
            ResearchRunRecord {
                timestamp: Utc::now(),
                aggregate_return: 0.14,
                ..ResearchRunRecord::default()
            },
            ResearchRunRecord {
                timestamp: Utc::now(),
                aggregate_return: 0.18,
                ..ResearchRunRecord::default()
            },
        ];

        let persisted = persist_market_jump_calibration_from_research_runs(
            temp.path(),
            symbol,
            &runs,
            Some("energy"),
            Some("energy_volatility_shock_sensitive"),
        )
        .unwrap();

        assert!(persisted.is_empty());
    }

    #[test]
    fn persist_market_jump_calibration_from_backtest_runs_writes_expected_profile() {
        let temp = tempfile::tempdir().unwrap();
        let symbol = "GC";
        let timestamp = Utc::now();
        let runs = vec![
            BacktestRunRecord {
                timestamp,
                total_return: -0.08,
                ..BacktestRunRecord::default()
            },
            BacktestRunRecord {
                timestamp,
                total_return: -0.09,
                ..BacktestRunRecord::default()
            },
            BacktestRunRecord {
                timestamp,
                total_return: -0.10,
                ..BacktestRunRecord::default()
            },
        ];

        let persisted = persist_market_jump_calibration_from_backtest_runs(
            temp.path(),
            symbol,
            &runs,
            Some("metals"),
            Some("metals_defensive_liquidity_sensitive"),
        )
        .unwrap();

        let overlay = persisted
            .get("metals::metals_defensive_liquidity_sensitive")
            .unwrap();
        assert!(overlay.balance_weight > 1.0);
        assert!(overlay.backtest_edge < 0.0);
        assert_eq!(overlay.sample_count, 3);
        assert_eq!(overlay.updated_at, Some(timestamp));
    }

    #[test]
    fn persist_market_jump_calibration_from_backtest_runs_respects_cooldown() {
        let temp = tempfile::tempdir().unwrap();
        let symbol = "GC";
        let initial_timestamp = Utc::now();
        let initial_runs = vec![
            BacktestRunRecord {
                timestamp: initial_timestamp,
                total_return: -0.08,
                ..BacktestRunRecord::default()
            },
            BacktestRunRecord {
                timestamp: initial_timestamp,
                total_return: -0.09,
                ..BacktestRunRecord::default()
            },
            BacktestRunRecord {
                timestamp: initial_timestamp,
                total_return: -0.10,
                ..BacktestRunRecord::default()
            },
        ];
        let initial = persist_market_jump_calibration_from_backtest_runs(
            temp.path(),
            symbol,
            &initial_runs,
            Some("metals"),
            Some("metals_defensive_liquidity_sensitive"),
        )
        .unwrap();
        let original = *initial
            .get("metals::metals_defensive_liquidity_sensitive")
            .unwrap();

        let cooldown_runs = vec![
            BacktestRunRecord {
                timestamp: initial_timestamp + Duration::hours(1),
                total_return: -0.20,
                ..BacktestRunRecord::default()
            },
            BacktestRunRecord {
                timestamp: initial_timestamp + Duration::hours(1),
                total_return: -0.22,
                ..BacktestRunRecord::default()
            },
            BacktestRunRecord {
                timestamp: initial_timestamp + Duration::hours(1),
                total_return: -0.24,
                ..BacktestRunRecord::default()
            },
        ];
        let persisted = persist_market_jump_calibration_from_backtest_runs(
            temp.path(),
            symbol,
            &cooldown_runs,
            Some("metals"),
            Some("metals_defensive_liquidity_sensitive"),
        )
        .unwrap();

        let cooled = persisted
            .get("metals::metals_defensive_liquidity_sensitive")
            .copied()
            .unwrap();
        assert_eq!(cooled.sample_count, original.sample_count);
        assert_eq!(cooled.updated_at, original.updated_at);
        assert!((cooled.trend_weight - original.trend_weight).abs() < 1e-12);
        assert!((cooled.balance_weight - original.balance_weight).abs() < 1e-12);
        assert!((cooled.transition_weight - original.transition_weight).abs() < 1e-12);
        assert!((cooled.backtest_edge - original.backtest_edge).abs() < 1e-12);
    }
}
