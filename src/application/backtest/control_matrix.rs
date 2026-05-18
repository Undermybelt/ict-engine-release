use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use statrs::distribution::{Beta, ContinuousCDF};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use crate::application::auto_quant::results::{
    load_strategy_library_manifest, StrategyLibraryEntryStatus, STRATEGY_LIBRARY_FILE,
};
use crate::application::data_sources::{
    build_control_matrix_provider_summary, ControlMatrixProviderSummary,
};
use crate::data::load_candles;
use crate::indicators::compute_atr;
use crate::pda_timeline::{build_pda_timeline, match_all_setups_extended, SetupContext};
use crate::state::{
    append_artifact_ledger_entry, artifact_state_path, load_state_or_default, save_state,
    ArtifactLedgerEntry,
};
use crate::types::{Candle, Direction};

pub const CONTROL_MATRIX_RESEARCH_RUNS_FILE: &str = "auto_quant_pb12_research_runs.json";
pub const CONTROL_MATRIX_RESEARCH_ARTIFACT_KIND: &str = "auto_quant_pb12_research_run";
pub const CONTROL_MATRIX_RESEARCH_REVIEW_RULE_VERSION: &str = "auto-quant-pb12-research-v1";
pub const CONTROL_MATRIX_TOP_RUN_LIMIT: usize = 3;
pub const CONTROL_MATRIX_DISCOVERY_MIN_SAMPLES: usize = 3;
pub const CONTROL_MATRIX_DISCOVERY_THRESHOLD_PROBABILITY: f64 = 0.95;
pub const CONTROL_MATRIX_DISCOVERY_MAX_SEQUENCE_LEN: usize = 3;
pub const CONTROL_MATRIX_DISCOVERY_HORIZON_BARS: usize = 30;
pub const CONTROL_MATRIX_DISCOVERY_HOLD_BARS: usize = 6;
pub const CONTROL_MATRIX_DISCOVERY_ATR_PERIOD: usize = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControlMatrixKind {
    #[serde(rename = "pb12", alias = "Pb12")]
    Pb12,
}

impl ControlMatrixKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pb12 => "pb12",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pb12Toggle {
    UseGreeks,
    UseOi,
    UseIv,
    UseEtf,
    UseCfd,
    UseVix,
    UseDailyStructure,
    UseWeeklyStructure,
}

impl Pb12Toggle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UseGreeks => "use_greeks",
            Self::UseOi => "use_oi",
            Self::UseIv => "use_iv",
            Self::UseEtf => "use_etf",
            Self::UseCfd => "use_cfd",
            Self::UseVix => "use_vix",
            Self::UseDailyStructure => "use_daily_structure",
            Self::UseWeeklyStructure => "use_weekly_structure",
        }
    }
}

pub const PB12_TOGGLES: [Pb12Toggle; 8] = [
    Pb12Toggle::UseGreeks,
    Pb12Toggle::UseOi,
    Pb12Toggle::UseIv,
    Pb12Toggle::UseEtf,
    Pb12Toggle::UseCfd,
    Pb12Toggle::UseVix,
    Pb12Toggle::UseDailyStructure,
    Pb12Toggle::UseWeeklyStructure,
];

const PB12_SIGN_MATRIX: [[bool; 8]; 12] = [
    [true, true, false, true, true, true, false, false],
    [false, true, true, false, true, true, true, false],
    [true, false, true, true, false, true, true, true],
    [false, true, false, true, true, false, true, true],
    [false, false, true, false, true, true, false, true],
    [false, false, false, true, false, true, true, false],
    [true, false, false, false, true, false, true, true],
    [true, true, false, false, false, true, false, true],
    [true, true, true, false, false, false, true, false],
    [false, true, true, true, false, false, false, true],
    [true, false, true, true, true, false, false, false],
    [false, false, false, false, false, false, false, false],
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pb12RunSpec {
    pub run_number: usize,
    pub baseline: bool,
    pub use_greeks: bool,
    pub use_oi: bool,
    pub use_iv: bool,
    pub use_etf: bool,
    pub use_cfd: bool,
    pub use_vix: bool,
    pub use_daily_structure: bool,
    pub use_weekly_structure: bool,
}

impl Pb12RunSpec {
    pub fn toggle_value(&self, toggle: Pb12Toggle) -> bool {
        match toggle {
            Pb12Toggle::UseGreeks => self.use_greeks,
            Pb12Toggle::UseOi => self.use_oi,
            Pb12Toggle::UseIv => self.use_iv,
            Pb12Toggle::UseEtf => self.use_etf,
            Pb12Toggle::UseCfd => self.use_cfd,
            Pb12Toggle::UseVix => self.use_vix,
            Pb12Toggle::UseDailyStructure => self.use_daily_structure,
            Pb12Toggle::UseWeeklyStructure => self.use_weekly_structure,
        }
    }

    pub fn enabled_toggles(&self) -> Vec<&'static str> {
        PB12_TOGGLES
            .iter()
            .copied()
            .filter(|toggle| self.toggle_value(*toggle))
            .map(Pb12Toggle::as_str)
            .collect()
    }

    pub fn disabled_toggles(&self) -> Vec<&'static str> {
        PB12_TOGGLES
            .iter()
            .copied()
            .filter(|toggle| !self.toggle_value(*toggle))
            .map(Pb12Toggle::as_str)
            .collect()
    }

    pub fn compact_label(&self) -> String {
        format!(
            "pb12_run_{:02}{}",
            self.run_number,
            if self.baseline { "_baseline" } else { "" }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlMatrixPlan {
    pub kind: ControlMatrixKind,
    pub runs: Vec<Pb12RunSpec>,
}

impl ControlMatrixPlan {
    pub fn pb12() -> Self {
        let runs = PB12_SIGN_MATRIX
            .iter()
            .enumerate()
            .map(|(idx, row)| Pb12RunSpec {
                run_number: idx + 1,
                baseline: idx + 1 == 12,
                use_greeks: row[0],
                use_oi: row[1],
                use_iv: row[2],
                use_etf: row[3],
                use_cfd: row[4],
                use_vix: row[5],
                use_daily_structure: row[6],
                use_weekly_structure: row[7],
            })
            .collect();
        Self {
            kind: ControlMatrixKind::Pb12,
            runs,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlMatrixResearchRunSummary {
    pub run_number: usize,
    pub run_label: String,
    pub baseline: bool,
    pub enabled_toggles: Vec<String>,
    pub disabled_toggles: Vec<String>,
    pub best_factor: Option<String>,
    pub aggregate_return: f64,
    pub feedback_records_generated: usize,
    pub feedback_records_applied: usize,
    pub dataset_comparable: bool,
    pub recommended_next_command: String,
    #[serde(default)]
    pub runtime_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlMatrixResearchArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub sweep_id: String,
    pub research_objective: String,
    pub control_matrix_plan: ControlMatrixPlan,
    pub run_count: usize,
    pub runs: Vec<ControlMatrixResearchRunSummary>,
    pub baseline_run: Option<ControlMatrixResearchRunSummary>,
    pub top_runs: Vec<ControlMatrixResearchRunSummary>,
    #[serde(default)]
    pub discovery_summary: ControlMatrixDiscoverySummary,
    #[serde(default)]
    pub provider_summary: ControlMatrixProviderSummary,
}

pub struct ControlMatrixResearchArtifactInput<'a> {
    pub symbol: &'a str,
    pub sweep_id: &'a str,
    pub research_objective: &'a str,
    pub generated_at: DateTime<Utc>,
    pub control_matrix_plan: ControlMatrixPlan,
    pub runs: Vec<ControlMatrixResearchRunSummary>,
    pub discovery_summary: ControlMatrixDiscoverySummary,
    pub provider_summary: ControlMatrixProviderSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlMatrixDiscoveryBaseline {
    pub source: String,
    pub weighted_win_rate: f64,
    pub strategy_count: usize,
    pub total_trade_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlMatrixDiscoveryCandidate {
    pub sequence_label: String,
    pub direction: Direction,
    pub sample_count: usize,
    pub win_count: usize,
    pub empirical_win_rate: f64,
    pub posterior_mean_win_rate: f64,
    pub posterior_prob_beats_baseline: Option<f64>,
    pub average_signed_return: f64,
    pub first_confirm_bar: usize,
    pub latest_confirm_bar: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ControlMatrixDiscoverySummary {
    pub status: String,
    pub threshold_probability: f64,
    pub hold_bars: usize,
    pub candidate_horizon_bars: usize,
    pub evaluated_candidate_count: usize,
    pub promoted_candidate_count: usize,
    pub baseline: Option<ControlMatrixDiscoveryBaseline>,
    pub top_candidates: Vec<ControlMatrixDiscoveryCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlMatrixDiscoveryObservation {
    sequence_label: String,
    direction: Direction,
    confirm_bar: usize,
    signed_return_bps: i64,
    win: bool,
}

impl ControlMatrixDiscoveryObservation {
    pub fn new(
        sequence_label: String,
        direction: Direction,
        confirm_bar: usize,
        signed_return: f64,
        win: bool,
    ) -> Self {
        Self {
            sequence_label,
            direction,
            confirm_bar,
            signed_return_bps: (signed_return * 10_000.0).round() as i64,
            win,
        }
    }

    fn signed_return(&self) -> f64 {
        self.signed_return_bps as f64 / 10_000.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ControlMatrixDiscoveryKey {
    sequence_label: String,
    direction: Direction,
}

#[derive(Debug, Clone, Default)]
struct ControlMatrixDiscoveryAggregate {
    sample_count: usize,
    win_count: usize,
    signed_return_sum: f64,
    first_confirm_bar: usize,
    latest_confirm_bar: usize,
}

pub fn build_control_matrix_research_artifact(
    input: ControlMatrixResearchArtifactInput<'_>,
) -> ControlMatrixResearchArtifact {
    let ControlMatrixResearchArtifactInput {
        symbol,
        sweep_id,
        research_objective,
        generated_at,
        control_matrix_plan,
        runs,
        discovery_summary,
        provider_summary,
    } = input;
    let baseline_run = runs.iter().find(|run| run.baseline).cloned();
    let mut top_runs = runs.clone();
    top_runs.sort_by(|left, right| {
        right
            .aggregate_return
            .total_cmp(&left.aggregate_return)
            .then_with(|| left.run_number.cmp(&right.run_number))
    });
    top_runs.truncate(CONTROL_MATRIX_TOP_RUN_LIMIT);
    ControlMatrixResearchArtifact {
        artifact_id: format!(
            "auto_quant_pb12_research_run_{}_{}",
            symbol,
            generated_at.format("%Y%m%dT%H%M%S%.9fZ")
        ),
        generated_at,
        symbol: symbol.to_string(),
        sweep_id: sweep_id.to_string(),
        research_objective: research_objective.to_string(),
        control_matrix_plan,
        run_count: runs.len(),
        runs,
        baseline_run,
        top_runs,
        discovery_summary,
        provider_summary,
    }
}

pub fn build_control_matrix_discovery_summary(
    candles: &[Candle],
    baseline: Option<ControlMatrixDiscoveryBaseline>,
) -> ControlMatrixDiscoverySummary {
    summarize_control_matrix_discovery_candidates(
        baseline,
        build_control_matrix_discovery_observations(candles),
    )
}

pub fn load_control_matrix_discovery_baseline<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<Option<ControlMatrixDiscoveryBaseline>> {
    let manifest_path = dir.as_ref().join(symbol).join(STRATEGY_LIBRARY_FILE);
    if !manifest_path.exists() {
        return Ok(None);
    }
    let manifest = load_strategy_library_manifest(&manifest_path)?;
    let mut strategy_count = 0usize;
    let mut total_trade_count = 0u32;
    let mut weighted_win_sum = 0.0;
    for strategy in &manifest.strategies {
        if strategy.status_kind() != StrategyLibraryEntryStatus::Ok {
            continue;
        }
        let Some(metrics) = strategy.validation_metrics.as_ref() else {
            continue;
        };
        if metrics.trade_count == 0 || !(0.0..=100.0).contains(&metrics.win_rate_pct) {
            continue;
        }
        strategy_count += 1;
        total_trade_count = total_trade_count.saturating_add(metrics.trade_count);
        weighted_win_sum += metrics.trade_count as f64 * (metrics.win_rate_pct / 100.0);
    }
    if strategy_count == 0 || total_trade_count == 0 {
        return Ok(None);
    }
    Ok(Some(ControlMatrixDiscoveryBaseline {
        source: "strategy_library_weighted_win_rate".to_string(),
        weighted_win_rate: (weighted_win_sum / total_trade_count as f64).clamp(0.0, 1.0),
        strategy_count,
        total_trade_count,
    }))
}

pub fn build_control_matrix_discovery_summary_for_symbol<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    data_path: &str,
) -> Result<ControlMatrixDiscoverySummary> {
    let candles = load_candles(data_path)?;
    let baseline = load_control_matrix_discovery_baseline(dir, symbol)?;
    Ok(build_control_matrix_discovery_summary(&candles, baseline))
}

pub fn build_control_matrix_provider_summary_for_plan(
    plan: &ControlMatrixPlan,
) -> ControlMatrixProviderSummary {
    build_control_matrix_provider_summary(plan)
}

pub fn append_control_matrix_research_artifact<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    artifact: ControlMatrixResearchArtifact,
) -> Result<Vec<ControlMatrixResearchArtifact>> {
    let path = artifact_state_path(&dir, symbol, CONTROL_MATRIX_RESEARCH_RUNS_FILE);
    let mut history: Vec<ControlMatrixResearchArtifact> =
        load_state_or_default(&dir, symbol, CONTROL_MATRIX_RESEARCH_RUNS_FILE)?;
    history.push(artifact.clone());
    save_state(&dir, symbol, CONTROL_MATRIX_RESEARCH_RUNS_FILE, &history)?;
    append_artifact_ledger_entry(
        &dir,
        symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: CONTROL_MATRIX_RESEARCH_ARTIFACT_KIND.to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: 1,
            generated_at: artifact.generated_at,
            symbol: symbol.to_string(),
            source_phase: "factor-research".to_string(),
            source_run_id: Some(artifact.sweep_id.clone()),
            path,
            status: "sweep_complete".to_string(),
            promote_candidate: false,
            actionable: false,
            decision_hint: "review_pb12_summary".to_string(),
            review_reason: format!(
                "control_matrix={} runs={} top_run={} baseline_run={} discovery_status={} discovery_candidates={} provider_prompts={}",
                artifact.control_matrix_plan.kind.as_str(),
                artifact.run_count,
                artifact
                    .top_runs
                    .first()
                    .map(|run| run.run_label.as_str())
                    .unwrap_or("n/a"),
                artifact
                    .baseline_run
                    .as_ref()
                    .map(|run| run.run_label.as_str())
                    .unwrap_or("n/a"),
                artifact.discovery_summary.status,
                artifact.discovery_summary.promoted_candidate_count,
                artifact.provider_summary.actionable_install_prompts.len()
            ),
            review_rule_version: CONTROL_MATRIX_RESEARCH_REVIEW_RULE_VERSION.to_string(),
            top_factor_name: artifact
                .top_runs
                .first()
                .and_then(|run| run.best_factor.clone()),
            top_factor_action: None,
            family_scores: std::collections::BTreeMap::new(),
            supersedes_artifact_id: None,
            quality_score: control_matrix_quality_score(&artifact),
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;
    Ok(history)
}

pub fn load_control_matrix_research_artifacts<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<Vec<ControlMatrixResearchArtifact>> {
    load_state_or_default(dir, symbol, CONTROL_MATRIX_RESEARCH_RUNS_FILE)
}

fn control_matrix_quality_score(artifact: &ControlMatrixResearchArtifact) -> i32 {
    artifact
        .top_runs
        .first()
        .map(|run| (run.aggregate_return * 100.0).round().clamp(-100.0, 100.0) as i32)
        .unwrap_or_default()
}

fn summarize_control_matrix_discovery_candidates(
    baseline: Option<ControlMatrixDiscoveryBaseline>,
    observations: Vec<ControlMatrixDiscoveryObservation>,
) -> ControlMatrixDiscoverySummary {
    let mut grouped = HashMap::<ControlMatrixDiscoveryKey, ControlMatrixDiscoveryAggregate>::new();
    for observation in observations {
        let key = ControlMatrixDiscoveryKey {
            sequence_label: observation.sequence_label.clone(),
            direction: observation.direction,
        };
        let aggregate = grouped
            .entry(key)
            .or_insert_with(|| ControlMatrixDiscoveryAggregate {
                first_confirm_bar: observation.confirm_bar,
                latest_confirm_bar: observation.confirm_bar,
                ..ControlMatrixDiscoveryAggregate::default()
            });
        aggregate.sample_count += 1;
        aggregate.win_count += usize::from(observation.win);
        aggregate.signed_return_sum += observation.signed_return();
        aggregate.first_confirm_bar = aggregate.first_confirm_bar.min(observation.confirm_bar);
        aggregate.latest_confirm_bar = aggregate.latest_confirm_bar.max(observation.confirm_bar);
    }

    let evaluated_candidate_count = grouped.len();
    let mut candidates = grouped
        .into_iter()
        .filter_map(|(key, aggregate)| {
            if aggregate.sample_count < CONTROL_MATRIX_DISCOVERY_MIN_SAMPLES {
                return None;
            }
            let sample_count = aggregate.sample_count;
            let win_count = aggregate.win_count;
            let empirical_win_rate = win_count as f64 / sample_count as f64;
            let posterior_mean_win_rate = (win_count as f64 + 1.0) / (sample_count as f64 + 2.0);
            let posterior_prob_beats_baseline = baseline.as_ref().and_then(|baseline| {
                let beta = Beta::new(
                    win_count as f64 + 1.0,
                    (sample_count - win_count) as f64 + 1.0,
                )
                .ok()?;
                let baseline_rate = baseline.weighted_win_rate.clamp(0.0, 1.0);
                Some((1.0 - beta.cdf(baseline_rate)).clamp(0.0, 1.0))
            });
            Some(ControlMatrixDiscoveryCandidate {
                sequence_label: key.sequence_label,
                direction: key.direction,
                sample_count,
                win_count,
                empirical_win_rate,
                posterior_mean_win_rate,
                posterior_prob_beats_baseline,
                average_signed_return: aggregate.signed_return_sum / sample_count as f64,
                first_confirm_bar: aggregate.first_confirm_bar,
                latest_confirm_bar: aggregate.latest_confirm_bar,
            })
        })
        .collect::<Vec<_>>();

    if baseline.is_some() {
        candidates.retain(|candidate| {
            candidate.posterior_prob_beats_baseline.unwrap_or_default()
                >= CONTROL_MATRIX_DISCOVERY_THRESHOLD_PROBABILITY
        });
    } else {
        candidates.clear();
    }

    candidates.sort_by(|left, right| {
        right
            .posterior_prob_beats_baseline
            .unwrap_or_default()
            .total_cmp(&left.posterior_prob_beats_baseline.unwrap_or_default())
            .then_with(|| right.sample_count.cmp(&left.sample_count))
            .then_with(|| {
                right
                    .posterior_mean_win_rate
                    .total_cmp(&left.posterior_mean_win_rate)
            })
            .then_with(|| {
                right
                    .average_signed_return
                    .total_cmp(&left.average_signed_return)
            })
            .then_with(|| left.sequence_label.cmp(&right.sequence_label))
    });
    candidates.truncate(CONTROL_MATRIX_TOP_RUN_LIMIT);

    let promoted_candidate_count = candidates.len();
    let status = if baseline.is_none() {
        "baseline_unavailable".to_string()
    } else if evaluated_candidate_count == 0 {
        "no_candidates".to_string()
    } else if promoted_candidate_count == 0 {
        "no_candidates_above_threshold".to_string()
    } else {
        "candidates_above_threshold".to_string()
    };

    ControlMatrixDiscoverySummary {
        status,
        threshold_probability: CONTROL_MATRIX_DISCOVERY_THRESHOLD_PROBABILITY,
        hold_bars: CONTROL_MATRIX_DISCOVERY_HOLD_BARS,
        candidate_horizon_bars: CONTROL_MATRIX_DISCOVERY_HORIZON_BARS,
        evaluated_candidate_count,
        promoted_candidate_count,
        baseline,
        top_candidates: candidates,
    }
}

fn build_control_matrix_discovery_observations(
    candles: &[Candle],
) -> Vec<ControlMatrixDiscoveryObservation> {
    if candles.len() <= CONTROL_MATRIX_DISCOVERY_HOLD_BARS + 1 {
        return Vec::new();
    }
    let atr = pad_indicator(
        compute_atr(candles, CONTROL_MATRIX_DISCOVERY_ATR_PERIOD),
        candles.len(),
        0.0,
    );
    let timeline = build_pda_timeline(candles, &atr);
    if timeline.len() < 2 {
        return Vec::new();
    }
    let setup_context = SetupContext {
        primary_candles: Some(candles),
        ..SetupContext::default()
    };
    let canonical_matches = match_all_setups_extended(
        &timeline,
        &setup_context,
        CONTROL_MATRIX_DISCOVERY_HORIZON_BARS,
    );
    let canonical_bar_sequences = canonical_matches
        .iter()
        .map(|setup| bar_signature(&setup.event_bars))
        .collect::<BTreeSet<_>>();

    let mut observations = Vec::new();
    for sequence_len in 2..=CONTROL_MATRIX_DISCOVERY_MAX_SEQUENCE_LEN {
        for window in timeline.windows(sequence_len) {
            if !window_has_strict_precedence(window) {
                continue;
            }
            let confirm_bar = window
                .last()
                .map(|event| event.bar_index)
                .unwrap_or_default();
            if confirm_bar + CONTROL_MATRIX_DISCOVERY_HOLD_BARS >= candles.len() {
                continue;
            }
            let span = confirm_bar.saturating_sub(window[0].bar_index);
            if span > CONTROL_MATRIX_DISCOVERY_HORIZON_BARS {
                continue;
            }
            let bar_indexes = window
                .iter()
                .map(|event| event.bar_index)
                .collect::<Vec<_>>();
            if canonical_bar_sequences.contains(&bar_signature(&bar_indexes)) {
                continue;
            }
            let direction = window
                .last()
                .map(|event| event.direction)
                .unwrap_or(Direction::Neutral);
            if direction == Direction::Neutral {
                continue;
            }
            let entry_price = candles[confirm_bar].close;
            let exit_price = candles[confirm_bar + CONTROL_MATRIX_DISCOVERY_HOLD_BARS].close;
            if entry_price.abs() <= f64::EPSILON {
                continue;
            }
            let raw_return = (exit_price - entry_price) / entry_price.abs();
            let signed_return = match direction {
                Direction::Bull => raw_return,
                Direction::Bear => -raw_return,
                Direction::Neutral => 0.0,
            };
            observations.push(ControlMatrixDiscoveryObservation::new(
                window
                    .iter()
                    .map(|event| event.kind.as_str().to_string())
                    .collect::<Vec<_>>()
                    .join(" -> "),
                direction,
                confirm_bar,
                signed_return,
                signed_return > 0.0,
            ));
        }
    }
    observations
}

fn window_has_strict_precedence(window: &[crate::pda_timeline::PdaEvent]) -> bool {
    window
        .windows(2)
        .all(|pair| pair[0].bar_index < pair[1].bar_index)
}

fn bar_signature(bars: &[usize]) -> String {
    bars.iter()
        .map(|bar| bar.to_string())
        .collect::<Vec<_>>()
        .join(">")
}

fn pad_indicator(values: Vec<f64>, target_len: usize, fill: f64) -> Vec<f64> {
    if values.len() >= target_len {
        return values;
    }
    let mut padded = vec![fill; target_len - values.len()];
    padded.extend(values);
    padded
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::state::{load_artifact_ledger, ARTIFACT_LEDGER_FILE};

    #[test]
    fn pb12_has_twelve_runs() {
        let plan = ControlMatrixPlan::pb12();
        assert_eq!(plan.kind, ControlMatrixKind::Pb12);
        assert_eq!(plan.runs.len(), 12);
    }

    #[test]
    fn pb12_baseline_run_is_all_off() {
        let plan = ControlMatrixPlan::pb12();
        let baseline = plan.runs.iter().find(|run| run.baseline).unwrap();
        assert_eq!(baseline.run_number, 12);
        for toggle in PB12_TOGGLES {
            assert!(!baseline.toggle_value(toggle));
        }
        assert!(baseline.enabled_toggles().is_empty());
        assert_eq!(baseline.disabled_toggles().len(), PB12_TOGGLES.len());
    }

    #[test]
    fn pb12_non_baseline_runs_are_unique() {
        let plan = ControlMatrixPlan::pb12();
        let mut seen = BTreeSet::new();
        for run in &plan.runs {
            let key = PB12_TOGGLES
                .iter()
                .map(|toggle| if run.toggle_value(*toggle) { '1' } else { '0' })
                .collect::<String>();
            assert!(seen.insert(key), "duplicate run {}", run.run_number);
        }
    }

    #[test]
    fn pb12_columns_are_balanced_across_toggles() {
        let plan = ControlMatrixPlan::pb12();
        for toggle in PB12_TOGGLES {
            let on_count = plan
                .runs
                .iter()
                .filter(|run| run.toggle_value(toggle))
                .count();
            assert_eq!(on_count, 6, "toggle {} lost PB12 balance", toggle.as_str());
        }
    }

    #[test]
    fn compact_label_marks_baseline() {
        let plan = ControlMatrixPlan::pb12();
        assert_eq!(plan.runs[0].compact_label(), "pb12_run_01");
        assert_eq!(plan.runs[11].compact_label(), "pb12_run_12_baseline");
    }

    #[test]
    fn control_matrix_kind_serializes_to_pb12_and_accepts_legacy_variant() {
        let value = serde_json::to_value(ControlMatrixPlan::pb12()).unwrap();
        assert_eq!(value["kind"], serde_json::json!("pb12"));

        let parsed: ControlMatrixPlan =
            serde_json::from_value(serde_json::json!({"kind": "Pb12", "runs": []})).unwrap();
        assert_eq!(parsed.kind, ControlMatrixKind::Pb12);
    }

    #[test]
    fn test_control_matrix_artifact_persistence_writes_ledger_entry() {
        let temp = tempfile::tempdir().unwrap();
        let artifact = build_control_matrix_research_artifact(ControlMatrixResearchArtifactInput {
            symbol: "NQ",
            sweep_id: "pb12:NQ:test",
            research_objective: "generic",
            generated_at: Utc::now(),
            control_matrix_plan: ControlMatrixPlan::pb12(),
            discovery_summary: ControlMatrixDiscoverySummary::default(),
            provider_summary: build_control_matrix_provider_summary(&ControlMatrixPlan::pb12()),
            runs: vec![ControlMatrixResearchRunSummary {
                run_number: 12,
                run_label: "pb12_run_12_baseline".to_string(),
                baseline: true,
                enabled_toggles: Vec::new(),
                disabled_toggles: PB12_TOGGLES
                    .iter()
                    .map(|toggle| toggle.as_str().to_string())
                    .collect(),
                best_factor: Some("trend".to_string()),
                aggregate_return: 0.0125,
                feedback_records_generated: 8,
                feedback_records_applied: 8,
                dataset_comparable: true,
                recommended_next_command: "ict-engine factor-research".to_string(),
                runtime_notes: Vec::new(),
            }],
        });

        let history =
            append_control_matrix_research_artifact(temp.path(), "NQ", artifact.clone()).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0], artifact);

        let ledger = load_artifact_ledger(temp.path(), "NQ").unwrap();
        assert_eq!(ledger.len(), 1);
        assert_eq!(
            ledger[0].artifact_kind,
            CONTROL_MATRIX_RESEARCH_ARTIFACT_KIND
        );
        assert_eq!(ledger[0].source_run_id.as_deref(), Some("pb12:NQ:test"));
        assert_eq!(
            ledger[0].path,
            temp.path()
                .join("NQ")
                .join(CONTROL_MATRIX_RESEARCH_RUNS_FILE)
                .to_string_lossy()
        );
        assert!(
            temp.path()
                .join("NQ")
                .join(CONTROL_MATRIX_RESEARCH_RUNS_FILE)
                .exists(),
            "artifact history file must be written"
        );
        assert!(
            temp.path().join("NQ").join(ARTIFACT_LEDGER_FILE).exists(),
            "ledger file must be written"
        );
    }

    #[test]
    fn control_matrix_discovery_summary_ranks_candidates_against_baseline() {
        let summary = summarize_control_matrix_discovery_candidates(
            Some(ControlMatrixDiscoveryBaseline {
                source: "strategy_library".to_string(),
                weighted_win_rate: 0.52,
                strategy_count: 2,
                total_trade_count: 100,
            }),
            vec![
                ControlMatrixDiscoveryObservation::new(
                    "liquidity_sweep -> market_structure_shift".to_string(),
                    crate::types::Direction::Bull,
                    10,
                    0.012,
                    true,
                ),
                ControlMatrixDiscoveryObservation::new(
                    "liquidity_sweep -> market_structure_shift".to_string(),
                    crate::types::Direction::Bull,
                    16,
                    0.010,
                    true,
                ),
                ControlMatrixDiscoveryObservation::new(
                    "liquidity_sweep -> market_structure_shift".to_string(),
                    crate::types::Direction::Bull,
                    22,
                    0.009,
                    true,
                ),
                ControlMatrixDiscoveryObservation::new(
                    "liquidity_sweep -> market_structure_shift".to_string(),
                    crate::types::Direction::Bull,
                    28,
                    0.006,
                    true,
                ),
                ControlMatrixDiscoveryObservation::new(
                    "liquidity_sweep -> market_structure_shift".to_string(),
                    crate::types::Direction::Bull,
                    34,
                    0.008,
                    true,
                ),
                ControlMatrixDiscoveryObservation::new(
                    "order_block -> rejection_block".to_string(),
                    crate::types::Direction::Bear,
                    14,
                    -0.006,
                    true,
                ),
                ControlMatrixDiscoveryObservation::new(
                    "order_block -> rejection_block".to_string(),
                    crate::types::Direction::Bear,
                    20,
                    0.003,
                    false,
                ),
                ControlMatrixDiscoveryObservation::new(
                    "order_block -> rejection_block".to_string(),
                    crate::types::Direction::Bear,
                    26,
                    -0.002,
                    true,
                ),
            ],
        );

        assert_eq!(summary.status, "candidates_above_threshold");
        assert_eq!(summary.evaluated_candidate_count, 2);
        assert_eq!(summary.promoted_candidate_count, 1);
        assert_eq!(summary.top_candidates.len(), 1);
        assert_eq!(
            summary.top_candidates[0].sequence_label,
            "liquidity_sweep -> market_structure_shift"
        );
        assert_eq!(summary.top_candidates[0].sample_count, 5);
        assert_eq!(summary.top_candidates[0].win_count, 5);
        assert!(
            summary.top_candidates[0]
                .posterior_prob_beats_baseline
                .unwrap_or_default()
                >= 0.95
        );
    }
}
