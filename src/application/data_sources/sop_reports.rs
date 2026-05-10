use anyhow::Result;
use serde::Serialize;
use std::collections::BTreeMap;

use crate::application::backtest::pre_bayes_soft_evidence_diff;
use crate::application::belief::{
    adapt_factor_pipeline_debug_report, pre_bayes_policy_lineage_summary, pre_bayes_report_summary,
    AdaptFactorPipelineDebugReportInput, ExpansionFactorPipelineReport, FactorPipelineDebugReport,
};
use crate::application::data_sources::{run_clean_futures_multi_timeframe, CleanFuturesReport};
use crate::application::decision_utils::{research_objective_label, ResearchObjectiveMode};
use crate::application::factor_lifecycle::ExpansionFactorScore;
use crate::application::factor_lifecycle::{
    apply_factor_mutation_spec, build_expansion_sop_metrics_from_market_reports,
    build_expansion_sop_mutation_metrics, evaluate_expansion_sop_mutation,
    expansion_regression_reasons_by_market,
};
use crate::application::multi_timeframe_inputs::resolved_multi_timeframe_inputs_for_market;
use crate::config::{env_f64, shell_quote};
use crate::factors::FactorRegistry;
use crate::state::{
    load_pre_bayes_policy_history, FactorMutationEvaluation, FactorMutationSpec,
    PreBayesEntryQualityBridge, PreBayesEvidencePolicy, PreBayesPolicyLineageSummary,
    PreBayesSoftEvidenceNodeDiff,
};

pub struct FuturesSopMarketInput {
    pub market: String,
    pub output_path: String,
    pub aggregated_candles: usize,
    pub state_dir: String,
    pub multi_timeframe_inputs:
        crate::application::multi_timeframe_inputs::ResolvedMultiTimeframeInputs,
}

pub struct ExpansionSopMarketInput {
    pub market: String,
    pub output_path: String,
    pub aggregated_candles: usize,
}

#[derive(Debug, Serialize)]
pub struct FuturesSopReport {
    pub sop_version: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub root: String,
    pub output_dir: String,
    pub cleaned_dir: String,
    pub state_dir: String,
    pub interval: String,
    pub selection_policy: String,
    pub clean_report: CleanFuturesReport,
    pub market_reports: Vec<FuturesSopMarketReport>,
    pub global_factor_leaderboard: Vec<FuturesSopFactorLeaderboardEntry>,
    pub recommended_global_factor: Option<String>,
    pub recommended_global_pre_bayes_policy: Option<PreBayesEvidencePolicy>,
    pub recommended_global_pre_bayes_entry_quality_bridge: Option<PreBayesEntryQualityBridge>,
    pub recommended_global_pre_bayes_summary: Vec<String>,
    pub recommended_global_pre_bayes_policy_lineage: Option<PreBayesPolicyLineageSummary>,
    pub recommended_global_pre_bayes_soft_evidence_diff: Vec<PreBayesSoftEvidenceNodeDiff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_global_pipeline_debug: Option<FactorPipelineDebugReport>,
    pub recommended_market_factors: BTreeMap<String, String>,
    pub warnings: Vec<String>,
    pub recommended_commands: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FuturesSopMarketReport {
    pub market: String,
    pub cleaned_path: String,
    pub candle_count: usize,
    pub multi_timeframe_summary: Vec<String>,
    pub best_factor: Option<String>,
    pub promotion_status: String,
    pub rollback_scope: String,
    pub workflow_phase: String,
    pub artifact_gate_status: String,
    pub recommended_next_command: String,
    pub aggregate_return: f64,
    pub aggregate_return_warning: Option<String>,
    pub top_scorecards: Vec<FuturesSopScorecard>,
    pub pipeline: Option<ExpansionFactorPipelineReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FuturesSopScorecard {
    pub factor_name: String,
    pub composite_score: f64,
    pub grade: String,
    pub iteration_action: String,
}

#[derive(Debug, Serialize)]
pub struct FuturesSopFactorLeaderboardEntry {
    pub factor_name: String,
    pub markets_seen: usize,
    pub first_place_markets: usize,
    pub average_composite_score: f64,
    pub best_composite_score: f64,
}

#[derive(Debug, Serialize)]
pub struct ExpansionSopReport {
    pub sop_version: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub root: String,
    pub output_dir: String,
    pub cleaned_dir: String,
    pub interval: String,
    pub expansion_lookback: usize,
    pub expansion_atr_multiplier: f64,
    pub clean_report: CleanFuturesReport,
    pub market_reports: Vec<ExpansionMarketReport>,
    pub global_factor_leaderboard: Vec<ExpansionFactorLeaderboardEntry>,
    pub recommended_global_factor: Option<String>,
    pub recommended_global_pre_bayes_policy: Option<PreBayesEvidencePolicy>,
    pub recommended_global_pre_bayes_entry_quality_bridge: Option<PreBayesEntryQualityBridge>,
    pub recommended_global_pre_bayes_summary: Vec<String>,
    pub recommended_global_pre_bayes_policy_lineage: Option<PreBayesPolicyLineageSummary>,
    pub recommended_global_pre_bayes_soft_evidence_diff: Vec<PreBayesSoftEvidenceNodeDiff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_global_pipeline_debug: Option<FactorPipelineDebugReport>,
    pub recommended_market_factors: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation_spec: Option<crate::state::FactorMutationSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub factor_mutation_evaluation: Option<FactorMutationEvaluation>,
    pub warnings: Vec<String>,
    pub recommended_commands: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ExpansionMarketReport {
    pub market: String,
    pub cleaned_path: String,
    pub total_candles: usize,
    pub expansion_samples: usize,
    pub bull_expansion_samples: usize,
    pub bear_expansion_samples: usize,
    pub best_factor: Option<String>,
    pub top_factors: Vec<ExpansionFactorScore>,
    pub multi_timeframe_summary: Vec<String>,
    pub pipeline: Option<ExpansionFactorPipelineReport>,
}

#[derive(Debug, Serialize)]
pub struct ExpansionFactorLeaderboardEntry {
    pub factor_name: String,
    pub markets_seen: usize,
    pub first_place_markets: usize,
    pub average_fit_score: f64,
    pub average_balanced_accuracy: f64,
    pub average_directional_accuracy: f64,
}

pub struct BuildFuturesSopReportInput {
    pub root: String,
    pub output_dir: String,
    pub cleaned_dir: String,
    pub state_dir: String,
    pub interval: String,
    pub clean_report: CleanFuturesReport,
    pub market_reports: Vec<FuturesSopMarketReport>,
    pub factor_scores: BTreeMap<String, Vec<f64>>,
    pub factor_first_places: BTreeMap<String, usize>,
    pub recommended_market_factors: BTreeMap<String, String>,
    pub warnings: Vec<String>,
}

pub fn run_futures_sop_with<F>(
    root: &str,
    output_dir: &str,
    interval: &str,
    mut run_market: F,
) -> Result<FuturesSopReport>
where
    F: FnMut(
        FuturesSopMarketInput,
    ) -> Result<(
        crate::factor_lab::ResearchReport,
        Option<ExpansionFactorPipelineReport>,
    )>,
{
    let cleaned_dir = std::path::Path::new(output_dir)
        .join(format!("cleaned-{}", interval))
        .to_string_lossy()
        .to_string();
    let state_dir = std::path::Path::new(output_dir)
        .join("state")
        .to_string_lossy()
        .to_string();
    std::fs::create_dir_all(&state_dir)?;

    let multi_timeframe_clean_report = run_clean_futures_multi_timeframe(root, output_dir)?;
    let clean_report = multi_timeframe_clean_report
        .reports
        .iter()
        .find(|report| report.interval == interval)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("missing cleaned report for interval '{}'", interval))?;
    let mut market_reports = Vec::new();
    let mut factor_scores = BTreeMap::<String, Vec<f64>>::new();
    let mut factor_first_places = BTreeMap::<String, usize>::new();
    let mut warnings = Vec::new();
    let mut recommended_market_factors = BTreeMap::<String, String>::new();

    for dataset in &clean_report.datasets {
        let (report, pipeline) = run_market(FuturesSopMarketInput {
            market: dataset.market.clone(),
            output_path: dataset.output_path.clone(),
            aggregated_candles: dataset.summary.aggregated_candles,
            state_dir: state_dir.clone(),
            multi_timeframe_inputs: resolved_multi_timeframe_inputs_for_market(
                &multi_timeframe_clean_report,
                &dataset.market,
            ),
        })?;
        for scorecard in &report.backtest.scorecards {
            factor_scores
                .entry(scorecard.factor_name.clone())
                .or_default()
                .push(scorecard.composite_score);
        }
        if let Some(best_factor) = &report.best_factor {
            *factor_first_places.entry(best_factor.clone()).or_default() += 1;
            recommended_market_factors.insert(dataset.market.clone(), best_factor.clone());
        }
        let (market_report, aggregate_return_warning) = build_futures_sop_market_report(
            &dataset.market,
            &dataset.output_path,
            dataset.summary.aggregated_candles,
            &report,
            pipeline,
        );
        if let Some(warning) = &aggregate_return_warning {
            warnings.push(format!("{}:{}", dataset.market, warning));
        }
        market_reports.push(market_report);
    }

    build_futures_sop_report(BuildFuturesSopReportInput {
        root: root.to_string(),
        output_dir: output_dir.to_string(),
        cleaned_dir,
        state_dir,
        interval: interval.to_string(),
        clean_report,
        market_reports,
        factor_scores,
        factor_first_places,
        recommended_market_factors,
        warnings,
    })
}

fn suspicious_aggregate_return(value: f64) -> bool {
    !value.is_finite() || value.abs() > 1_000_000.0
}

pub fn build_futures_sop_market_report(
    market: &str,
    cleaned_path: &str,
    candle_count: usize,
    report: &crate::factor_lab::ResearchReport,
    pipeline: Option<ExpansionFactorPipelineReport>,
) -> (FuturesSopMarketReport, Option<String>) {
    let top_scorecards = report
        .backtest
        .scorecards
        .iter()
        .map(|item| FuturesSopScorecard {
            factor_name: item.factor_name.clone(),
            composite_score: item.composite_score,
            grade: item.grade.clone(),
            iteration_action: item.iteration_action.clone(),
        })
        .take(5)
        .collect::<Vec<_>>();
    let aggregate_return_warning =
        suspicious_aggregate_return(report.aggregate_return).then(|| {
            format!(
                "aggregate_return={} looks unstable; prefer composite_score for factor selection",
                report.aggregate_return
            )
        });
    (
        FuturesSopMarketReport {
            market: market.to_string(),
            cleaned_path: cleaned_path.to_string(),
            candle_count,
            multi_timeframe_summary: report.multi_timeframe_summary.clone(),
            best_factor: report.best_factor.clone(),
            promotion_status: report.promotion_decision.status.clone(),
            rollback_scope: report.rollback_recommendation.scope.clone(),
            workflow_phase: report.workflow_state.phase.clone(),
            artifact_gate_status: report
                .artifact_decision_summary
                .consumed_trend_status
                .clone(),
            recommended_next_command: report.recommended_next_command.clone(),
            aggregate_return: report.aggregate_return,
            aggregate_return_warning: aggregate_return_warning.clone(),
            top_scorecards,
            pipeline,
        },
        aggregate_return_warning,
    )
}

pub struct BuildExpansionSopMarketReportInput {
    pub market: String,
    pub cleaned_path: String,
    pub total_candles: usize,
    pub expansion_samples: usize,
    pub bull_expansion_samples: usize,
    pub bear_expansion_samples: usize,
    pub best_factor: Option<String>,
    pub top_factors: Vec<ExpansionFactorScore>,
    pub multi_timeframe_summary: Vec<String>,
    pub pipeline: Option<ExpansionFactorPipelineReport>,
}

pub fn build_expansion_sop_market_report(
    input: BuildExpansionSopMarketReportInput,
) -> (ExpansionMarketReport, Vec<String>) {
    let BuildExpansionSopMarketReportInput {
        market,
        cleaned_path,
        total_candles,
        expansion_samples,
        bull_expansion_samples,
        bear_expansion_samples,
        best_factor,
        top_factors,
        multi_timeframe_summary,
        pipeline,
    } = input;
    let mut warnings = Vec::new();
    if expansion_samples == 0 {
        warnings.push("no_expansion_samples".to_string());
    }
    if bull_expansion_samples == 0 || bear_expansion_samples == 0 {
        warnings.push(format!(
            "unbalanced_expansion_labels bull={} bear={}",
            bull_expansion_samples, bear_expansion_samples
        ));
    }
    (
        ExpansionMarketReport {
            market,
            cleaned_path,
            total_candles,
            expansion_samples,
            bull_expansion_samples,
            bear_expansion_samples,
            best_factor,
            top_factors,
            multi_timeframe_summary,
            pipeline,
        },
        warnings,
    )
}

pub struct BuildExpansionSopReportInput {
    pub root: String,
    pub output_dir: String,
    pub cleaned_dir: String,
    pub state_dir: String,
    pub interval: String,
    pub lookback: usize,
    pub atr_multiplier: f64,
    pub objective_mode: ResearchObjectiveMode,
    pub clean_report: CleanFuturesReport,
    pub market_reports: Vec<ExpansionMarketReport>,
    pub global_scores: BTreeMap<String, Vec<ExpansionFactorScore>>,
    pub recommended_market_factors: BTreeMap<String, String>,
    pub mutation_spec: Option<FactorMutationSpec>,
    pub factor_mutation_evaluation: Option<FactorMutationEvaluation>,
    pub warnings: Vec<String>,
}

pub struct RunExpansionSopInput<'a> {
    pub root: &'a str,
    pub output_dir: &'a str,
    pub interval: &'a str,
    pub lookback: usize,
    pub atr_multiplier: f64,
    pub objective_mode: ResearchObjectiveMode,
    pub mutation_spec: Option<&'a FactorMutationSpec>,
}

pub fn run_expansion_sop_with<F>(
    input: RunExpansionSopInput<'_>,
    mut build_market: F,
) -> Result<ExpansionSopReport>
where
    F: FnMut(ExpansionSopMarketInput, &str, &FactorRegistry) -> Result<ExpansionMarketReport>,
{
    let RunExpansionSopInput {
        root,
        output_dir,
        interval,
        lookback,
        atr_multiplier,
        objective_mode,
        mutation_spec,
    } = input;

    let cleaned_dir = std::path::Path::new(output_dir)
        .join(format!("cleaned-{}", interval))
        .to_string_lossy()
        .to_string();
    let state_dir = std::path::Path::new(output_dir)
        .join("state")
        .to_string_lossy()
        .to_string();
    std::fs::create_dir_all(&state_dir)?;
    let multi_timeframe_clean_report = run_clean_futures_multi_timeframe(root, output_dir)?;
    let clean_report = multi_timeframe_clean_report
        .reports
        .iter()
        .find(|report| report.interval == interval)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("missing cleaned report for interval '{}'", interval))?;

    let baseline_registry = FactorRegistry::default();
    let mut registry = FactorRegistry::default();
    crate::factors::FactorHotplugConfig::apply_to_registry_if_present(&state_dir, &mut registry);
    if let Some(spec) = mutation_spec {
        apply_factor_mutation_spec(&mut registry, spec)?;
    }
    let baseline_metrics = mutation_spec
        .map(|_| {
            build_expansion_sop_mutation_metrics(
                &baseline_registry,
                &clean_report,
                lookback,
                atr_multiplier,
                objective_mode,
            )
        })
        .transpose()?;

    let mut market_reports = Vec::new();
    let mut warnings = Vec::new();
    let mut recommended_market_factors = BTreeMap::<String, String>::new();
    let mut global_scores = BTreeMap::<String, Vec<ExpansionFactorScore>>::new();

    for dataset in &clean_report.datasets {
        let report = build_market(
            ExpansionSopMarketInput {
                market: dataset.market.clone(),
                output_path: dataset.output_path.clone(),
                aggregated_candles: dataset.summary.aggregated_candles,
            },
            &state_dir,
            &registry,
        )?;
        if report.expansion_samples == 0 {
            warnings.push(format!(
                "{}:no_expansion_samples_for_lookback_{}_atr_{:.2}",
                dataset.market, lookback, atr_multiplier
            ));
        }
        if report.bull_expansion_samples == 0 || report.bear_expansion_samples == 0 {
            warnings.push(format!(
                "{}:unbalanced_expansion_labels bull={} bear={}",
                dataset.market, report.bull_expansion_samples, report.bear_expansion_samples
            ));
        }
        for score in &report.top_factors {
            global_scores
                .entry(score.factor_name.clone())
                .or_default()
                .push(score.clone());
        }
        if let Some(best_factor) = &report.best_factor {
            recommended_market_factors.insert(dataset.market.clone(), best_factor.clone());
        }
        market_reports.push(report);
    }

    let factor_mutation_evaluation = mutation_spec.map(|spec| {
        let mut metrics_after = build_expansion_sop_metrics_from_market_reports(&market_reports);
        metrics_after.regression_reasons_by_market = expansion_regression_reasons_by_market(
            &baseline_registry,
            &registry,
            &clean_report
                .datasets
                .iter()
                .map(|dataset| (dataset.market.as_str(), dataset.output_path.as_str()))
                .collect::<Vec<_>>(),
            lookback,
            atr_multiplier,
        )
        .unwrap_or_default();
        metrics_after.regressed_markets = metrics_after
            .regression_reasons_by_market
            .keys()
            .cloned()
            .collect();
        evaluate_expansion_sop_mutation(
            spec,
            root,
            interval,
            lookback,
            atr_multiplier,
            baseline_metrics.as_ref(),
            metrics_after,
        )
    });

    build_expansion_sop_report(BuildExpansionSopReportInput {
        root: root.to_string(),
        output_dir: output_dir.to_string(),
        cleaned_dir,
        state_dir,
        interval: interval.to_string(),
        lookback,
        atr_multiplier,
        objective_mode,
        clean_report,
        market_reports,
        global_scores,
        recommended_market_factors,
        mutation_spec: mutation_spec.cloned(),
        factor_mutation_evaluation,
        warnings,
    })
}

pub fn build_futures_sop_report(input: BuildFuturesSopReportInput) -> Result<FuturesSopReport> {
    let BuildFuturesSopReportInput {
        root,
        output_dir,
        cleaned_dir,
        state_dir,
        interval,
        clean_report,
        market_reports,
        factor_scores,
        factor_first_places,
        recommended_market_factors,
        warnings,
    } = input;

    let mut global_factor_leaderboard = factor_scores
        .into_iter()
        .map(|(factor_name, scores)| FuturesSopFactorLeaderboardEntry {
            first_place_markets: factor_first_places.get(&factor_name).copied().unwrap_or(0),
            markets_seen: scores.len(),
            average_composite_score: scores.iter().sum::<f64>() / scores.len() as f64,
            best_composite_score: scores.iter().copied().fold(f64::MIN, f64::max),
            factor_name,
        })
        .collect::<Vec<_>>();
    global_factor_leaderboard.sort_by(|a, b| {
        b.first_place_markets
            .cmp(&a.first_place_markets)
            .then_with(|| {
                b.average_composite_score
                    .partial_cmp(&a.average_composite_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let recommended_global_factor = global_factor_leaderboard
        .first()
        .map(|entry| entry.factor_name.clone());
    let recommended_global_pre_bayes_policy = market_reports
        .iter()
        .find(|market| {
            market.best_factor.as_deref() == recommended_global_factor.as_deref()
                && market.pipeline.is_some()
        })
        .and_then(|market| {
            market
                .pipeline
                .as_ref()
                .map(|pipeline| pipeline.bbn_support.pre_bayes_filter.policy.clone())
        });
    let recommended_global_pre_bayes_entry_quality_bridge = market_reports
        .iter()
        .find(|market| {
            market.best_factor.as_deref() == recommended_global_factor.as_deref()
                && market.pipeline.is_some()
        })
        .and_then(|market| {
            market
                .pipeline
                .as_ref()
                .map(|pipeline| pipeline.entry_quality_bridge.clone())
        });
    let recommended_global_pre_bayes_summary = pre_bayes_report_summary(
        recommended_global_pre_bayes_policy.as_ref(),
        recommended_global_pre_bayes_entry_quality_bridge.as_ref(),
    );
    let recommended_global_pre_bayes_policy_lineage = market_reports
        .iter()
        .find(|market| {
            market.best_factor.as_deref() == recommended_global_factor.as_deref()
                && market.pipeline.is_some()
        })
        .and_then(|market| {
            let history = load_pre_bayes_policy_history(&state_dir, &market.market).ok()?;
            let gate_status = market
                .pipeline
                .as_ref()
                .map(|pipeline| pipeline.bbn_support.pre_bayes_filter.gating_status.as_str())
                .unwrap_or("");
            Some(pre_bayes_policy_lineage_summary(&history, gate_status))
        });
    let recommended_global_pre_bayes_soft_evidence_diff = market_reports
        .iter()
        .find(|market| {
            market.best_factor.as_deref() == recommended_global_factor.as_deref()
                && market.pipeline.is_some()
        })
        .and_then(|market| market.pipeline.as_ref())
        .map(|pipeline| pre_bayes_soft_evidence_diff(&pipeline.bbn_support.pre_bayes_filter))
        .unwrap_or_default();
    let recommended_global_pipeline_debug = market_reports
        .iter()
        .find(|market| {
            market.best_factor.as_deref() == recommended_global_factor.as_deref()
                && market.pipeline.is_some()
        })
        .and_then(|market| {
            market.pipeline.as_ref().and_then(|pipeline| {
                adapt_factor_pipeline_debug_report(AdaptFactorPipelineDebugReportInput {
                    symbol: &market.market,
                    data: &market.cleaned_path,
                    objective: research_objective_label(ResearchObjectiveMode::Generic),
                    pipeline,
                    multi_timeframe_summary: &market.multi_timeframe_summary,
                    raw_pre_bayes_labels: BTreeMap::from([
                        (
                            "market_regime".to_string(),
                            pipeline.bbn_support.market_regime_label.clone(),
                        ),
                        (
                            "liquidity_context".to_string(),
                            pipeline.bbn_support.liquidity_context_label.clone(),
                        ),
                        (
                            "factor_alignment".to_string(),
                            pipeline.probability_support.alignment_label.clone(),
                        ),
                        (
                            "factor_uncertainty".to_string(),
                            pipeline.probability_support.uncertainty_label.clone(),
                        ),
                        (
                            "multi_timeframe_resonance".to_string(),
                            pipeline
                                .bbn_support
                                .pre_bayes_filter
                                .raw_multi_timeframe_resonance_label
                                .clone(),
                        ),
                    ]),
                    soft_evidence_divergence: pre_bayes_soft_evidence_diff(
                        &pipeline.bbn_support.pre_bayes_filter,
                    ),
                    bridge_gap_clear_threshold: env_f64(
                        "ICT_ENGINE_BRIDGE_GAP_CLEAR_THRESHOLD",
                        0.12,
                    ),
                    paired_market_quality_report: None,
                })
                .ok()
            })
        });

    Ok(FuturesSopReport {
        sop_version: "futures-sop-v1".to_string(),
        generated_at: chrono::Utc::now(),
        root: root.clone(),
        output_dir: output_dir.clone(),
        cleaned_dir,
        state_dir: state_dir.clone(),
        interval: interval.clone(),
        selection_policy:
            "continuous_front_contract_multi_timeframe_cleaning_then_factor_research_with_1m_5m_15m_1h_4h_1d_context_then_global_leaderboard_by_first_place_and_average_composite_score"
                .to_string(),
        clean_report,
        market_reports,
        global_factor_leaderboard,
        recommended_global_factor,
        recommended_global_pre_bayes_policy,
        recommended_global_pre_bayes_entry_quality_bridge,
        recommended_global_pre_bayes_summary,
        recommended_global_pre_bayes_policy_lineage,
        recommended_global_pre_bayes_soft_evidence_diff,
        recommended_global_pipeline_debug,
        recommended_market_factors,
        warnings,
        recommended_commands: vec![
            format!(
                "ict-engine futures-sop --root {} --output-dir {} --interval {}",
                shell_quote(&root),
                shell_quote(&output_dir),
                shell_quote(&interval)
            ),
            format!(
                "ict-engine factor-research --symbol NQ --data {} --state-dir {}",
                shell_quote(
                    &std::path::Path::new(&output_dir)
                        .join(format!("cleaned-{}/nq.continuous-{}.json", interval, interval))
                        .to_string_lossy()
                ),
                shell_quote(&state_dir)
            ),
            format!(
                "ict-engine clean-futures --root {} --output-dir {} --multi-timeframe",
                shell_quote(&root),
                shell_quote(&output_dir)
            ),
        ],
    })
}

pub fn build_expansion_sop_report(
    input: BuildExpansionSopReportInput,
) -> Result<ExpansionSopReport> {
    let BuildExpansionSopReportInput {
        root,
        output_dir,
        cleaned_dir,
        state_dir,
        interval,
        lookback,
        atr_multiplier,
        objective_mode,
        clean_report,
        market_reports,
        global_scores,
        recommended_market_factors,
        mutation_spec,
        factor_mutation_evaluation,
        warnings,
    } = input;

    let mut global_factor_leaderboard = global_scores
        .into_iter()
        .map(|(factor_name, scores)| ExpansionFactorLeaderboardEntry {
            first_place_markets: market_reports
                .iter()
                .filter(|market| market.best_factor.as_deref() == Some(factor_name.as_str()))
                .count(),
            markets_seen: scores.len(),
            average_fit_score: scores.iter().map(|score| score.fit_score).sum::<f64>()
                / scores.len() as f64,
            average_balanced_accuracy: scores
                .iter()
                .map(|score| score.balanced_accuracy)
                .sum::<f64>()
                / scores.len() as f64,
            average_directional_accuracy: scores
                .iter()
                .map(|score| score.directional_accuracy)
                .sum::<f64>()
                / scores.len() as f64,
            factor_name,
        })
        .collect::<Vec<_>>();
    global_factor_leaderboard.sort_by(|a, b| {
        b.first_place_markets
            .cmp(&a.first_place_markets)
            .then_with(|| {
                b.average_fit_score
                    .partial_cmp(&a.average_fit_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let recommended_global_factor = global_factor_leaderboard
        .first()
        .map(|entry| entry.factor_name.clone());
    let recommended_global_pre_bayes_policy = market_reports
        .iter()
        .find(|market| {
            market.best_factor.as_deref() == recommended_global_factor.as_deref()
                && market.pipeline.is_some()
        })
        .and_then(|market| {
            market
                .pipeline
                .as_ref()
                .map(|pipeline| pipeline.bbn_support.pre_bayes_filter.policy.clone())
        });
    let recommended_global_pre_bayes_entry_quality_bridge = market_reports
        .iter()
        .find(|market| {
            market.best_factor.as_deref() == recommended_global_factor.as_deref()
                && market.pipeline.is_some()
        })
        .and_then(|market| {
            market
                .pipeline
                .as_ref()
                .map(|pipeline| pipeline.entry_quality_bridge.clone())
        });
    let recommended_global_pre_bayes_summary = pre_bayes_report_summary(
        recommended_global_pre_bayes_policy.as_ref(),
        recommended_global_pre_bayes_entry_quality_bridge.as_ref(),
    );
    let recommended_global_pre_bayes_policy_lineage = market_reports
        .iter()
        .find(|market| {
            market.best_factor.as_deref() == recommended_global_factor.as_deref()
                && market.pipeline.is_some()
        })
        .and_then(|market| {
            let history = load_pre_bayes_policy_history(&state_dir, &market.market).ok()?;
            let gate_status = market
                .pipeline
                .as_ref()
                .map(|pipeline| pipeline.bbn_support.pre_bayes_filter.gating_status.as_str())
                .unwrap_or("");
            Some(pre_bayes_policy_lineage_summary(&history, gate_status))
        });
    let recommended_global_pre_bayes_soft_evidence_diff = market_reports
        .iter()
        .find(|market| {
            market.best_factor.as_deref() == recommended_global_factor.as_deref()
                && market.pipeline.is_some()
        })
        .and_then(|market| market.pipeline.as_ref())
        .map(|pipeline| pre_bayes_soft_evidence_diff(&pipeline.bbn_support.pre_bayes_filter))
        .unwrap_or_default();
    let recommended_policy_history_symbol = market_reports
        .iter()
        .find(|market| market.best_factor.as_deref() == recommended_global_factor.as_deref())
        .map(|market| market.market.clone())
        .unwrap_or_else(|| "NQ".to_string());
    let recommended_global_pipeline_debug = market_reports
        .iter()
        .find(|market| {
            market.best_factor.as_deref() == recommended_global_factor.as_deref()
                && market.pipeline.is_some()
        })
        .and_then(|market| {
            market.pipeline.as_ref().and_then(|pipeline| {
                adapt_factor_pipeline_debug_report(AdaptFactorPipelineDebugReportInput {
                    symbol: &market.market,
                    data: &market.cleaned_path,
                    objective: research_objective_label(objective_mode),
                    pipeline,
                    multi_timeframe_summary: &market.multi_timeframe_summary,
                    raw_pre_bayes_labels: BTreeMap::from([
                        (
                            "market_regime".to_string(),
                            pipeline.bbn_support.market_regime_label.clone(),
                        ),
                        (
                            "liquidity_context".to_string(),
                            pipeline.bbn_support.liquidity_context_label.clone(),
                        ),
                        (
                            "factor_alignment".to_string(),
                            pipeline.probability_support.alignment_label.clone(),
                        ),
                        (
                            "factor_uncertainty".to_string(),
                            pipeline.probability_support.uncertainty_label.clone(),
                        ),
                        (
                            "multi_timeframe_resonance".to_string(),
                            pipeline
                                .bbn_support
                                .pre_bayes_filter
                                .raw_multi_timeframe_resonance_label
                                .clone(),
                        ),
                    ]),
                    soft_evidence_divergence: pre_bayes_soft_evidence_diff(
                        &pipeline.bbn_support.pre_bayes_filter,
                    ),
                    bridge_gap_clear_threshold: env_f64(
                        "ICT_ENGINE_BRIDGE_GAP_CLEAR_THRESHOLD",
                        0.12,
                    ),
                    paired_market_quality_report: None,
                })
                .ok()
            })
        });

    Ok(ExpansionSopReport {
        sop_version: "expansion-sop-v1".to_string(),
        generated_at: chrono::Utc::now(),
        root: root.clone(),
        output_dir: output_dir.clone(),
        cleaned_dir,
        interval: interval.clone(),
        expansion_lookback: lookback,
        expansion_atr_multiplier: atr_multiplier,
        clean_report,
        market_reports,
        global_factor_leaderboard,
        recommended_global_factor: recommended_global_factor.clone(),
        recommended_global_pre_bayes_policy,
        recommended_global_pre_bayes_entry_quality_bridge,
        recommended_global_pre_bayes_summary,
        recommended_global_pre_bayes_policy_lineage,
        recommended_global_pre_bayes_soft_evidence_diff,
        recommended_global_pipeline_debug,
        recommended_market_factors,
        mutation_spec,
        factor_mutation_evaluation,
        warnings,
        recommended_commands: vec![
            format!(
                "ict-engine expansion-sop --root {} --output-dir {} --interval {} --lookback {} --atr-multiplier {:.2} --objective {}",
                shell_quote(&root),
                shell_quote(&output_dir),
                shell_quote(&interval),
                lookback,
                atr_multiplier,
                shell_quote(research_objective_label(objective_mode))
            ),
            format!(
                "ict-engine expansion-sop --root {} --output-dir {} --interval {} --lookback {} --atr-multiplier {:.2} --objective {} --mutation-spec <spec.json> --emit-mutation-evaluation",
                shell_quote(&root),
                shell_quote(&output_dir),
                shell_quote(&interval),
                lookback,
                atr_multiplier,
                shell_quote(research_objective_label(objective_mode))
            ),
            format!(
                "ict-engine factor-pipeline-debug --symbol {} --data {} --factor {} --objective {}",
                shell_quote(&recommended_policy_history_symbol),
                shell_quote(
                    &std::path::Path::new(&output_dir)
                        .join(format!("cleaned-{}/{}.continuous-{}.json", interval, recommended_policy_history_symbol.to_ascii_lowercase(), interval))
                        .to_string_lossy()
                ),
                shell_quote(
                    recommended_global_factor
                        .clone()
                        .unwrap_or_else(|| "structure_ict".to_string())
                        .as_str()
                ),
                shell_quote(research_objective_label(objective_mode))
            ),
            format!(
                "ict-engine workflow-status --symbol {} --state-dir {} --phase pre-bayes-policy-history",
                shell_quote(&recommended_policy_history_symbol),
                shell_quote(&state_dir)
            ),
            format!(
                "ict-engine clean-futures --root {} --output-dir {} --multi-timeframe",
                shell_quote(&root),
                shell_quote(&output_dir)
            ),
        ],
    })
}
