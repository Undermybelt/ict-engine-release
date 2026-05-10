use anyhow::{bail, Result};
use serde::Serialize;

use crate::application::backtest::load_control_matrix_research_artifacts;
use crate::pda_timeline::{
    append_promoted_canonical_setup, build_promoted_canonical_setup_spec,
    repo_root_from_manifest_dir,
};
use crate::types::Direction;

pub struct PromoteCanonicalSetupCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub setup_name: &'a str,
    pub sequence_label: &'a str,
    pub direction: Option<&'a str>,
    pub sweep_id: Option<&'a str>,
    pub horizon_bars: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PromoteCanonicalSetupReport {
    pub symbol: String,
    pub setup_name: String,
    pub sequence_label: String,
    pub direction: Option<String>,
    pub source_sweep_id: String,
    pub config_path: String,
    pub generated_path: String,
    pub promoted_candidate_count: usize,
}

pub fn auto_quant_promote_canonical_setup_command(
    input: PromoteCanonicalSetupCommandInput<'_>,
) -> Result<PromoteCanonicalSetupReport> {
    auto_quant_promote_canonical_setup_command_with_repo_root(input, repo_root_from_manifest_dir())
}

fn auto_quant_promote_canonical_setup_command_with_repo_root(
    input: PromoteCanonicalSetupCommandInput<'_>,
    repo_root: impl AsRef<std::path::Path>,
) -> Result<PromoteCanonicalSetupReport> {
    let PromoteCanonicalSetupCommandInput {
        symbol,
        state_dir,
        setup_name,
        sequence_label,
        direction,
        sweep_id,
        horizon_bars,
    } = input;
    let artifacts = load_control_matrix_research_artifacts(state_dir, symbol)?;
    let artifact = match sweep_id {
        Some(target) => artifacts
            .iter()
            .rev()
            .find(|item| item.sweep_id == target)
            .ok_or_else(|| anyhow::anyhow!("unknown pb12 sweep_id '{}'", target))?,
        None => artifacts
            .last()
            .ok_or_else(|| anyhow::anyhow!("no pb12 research artifacts found for {}", symbol))?,
    };
    let parsed_direction = direction.map(parse_direction).transpose()?;
    let candidate = artifact
        .discovery_summary
        .top_candidates
        .iter()
        .find(|item| {
            item.sequence_label == sequence_label
                && parsed_direction
                    .map(|value| item.direction == value)
                    .unwrap_or(true)
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "sequence '{}' was not found in promoted discovery candidates for sweep {}",
                sequence_label,
                artifact.sweep_id
            )
        })?;
    if artifact.discovery_summary.top_candidates.is_empty() {
        bail!("latest pb12 artifact contains no promotable discovery candidates");
    }
    let spec = build_promoted_canonical_setup_spec(
        setup_name,
        &candidate.sequence_label,
        Some(candidate.direction),
        horizon_bars,
        &artifact.sweep_id,
        symbol,
    )?;
    let (config_path, generated_path) = append_promoted_canonical_setup(repo_root, spec.clone())?;
    Ok(PromoteCanonicalSetupReport {
        symbol: symbol.to_string(),
        setup_name: spec.name,
        sequence_label: candidate.sequence_label.clone(),
        direction: Some(direction_label(candidate.direction).to_string()),
        source_sweep_id: artifact.sweep_id.clone(),
        config_path,
        generated_path,
        promoted_candidate_count: artifact.discovery_summary.promoted_candidate_count,
    })
}

fn parse_direction(raw: &str) -> Result<Direction> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "bull" | "bullish" => Ok(Direction::Bull),
        "bear" | "bearish" => Ok(Direction::Bear),
        "neutral" => Ok(Direction::Neutral),
        other => bail!("unsupported direction '{}'", other),
    }
}

fn direction_label(direction: Direction) -> &'static str {
    match direction {
        Direction::Bull => "bull",
        Direction::Bear => "bear",
        Direction::Neutral => "neutral",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::backtest::{
        append_control_matrix_research_artifact, build_control_matrix_research_artifact,
        ControlMatrixDiscoveryBaseline, ControlMatrixDiscoveryCandidate,
        ControlMatrixDiscoverySummary, ControlMatrixPlan, ControlMatrixResearchArtifactInput,
        ControlMatrixResearchRunSummary,
    };
    use crate::application::data_sources::build_control_matrix_provider_summary;
    use chrono::Utc;

    #[test]
    fn promote_canonical_setup_writes_repo_manifest_and_generated_file() {
        let temp = tempfile::tempdir().unwrap();
        let artifact = build_control_matrix_research_artifact(ControlMatrixResearchArtifactInput {
            symbol: "NQ",
            sweep_id: "pb12:NQ:test",
            research_objective: "generic",
            generated_at: Utc::now(),
            control_matrix_plan: ControlMatrixPlan::pb12(),
            discovery_summary: ControlMatrixDiscoverySummary {
                status: "candidates_above_threshold".to_string(),
                threshold_probability: 0.95,
                hold_bars: 6,
                candidate_horizon_bars: 30,
                evaluated_candidate_count: 1,
                promoted_candidate_count: 1,
                baseline: Some(ControlMatrixDiscoveryBaseline {
                    source: "strategy_library_weighted_win_rate".to_string(),
                    weighted_win_rate: 0.52,
                    strategy_count: 1,
                    total_trade_count: 50,
                }),
                top_candidates: vec![ControlMatrixDiscoveryCandidate {
                    sequence_label: "liquidity_sweep -> market_structure_shift".to_string(),
                    direction: Direction::Bull,
                    sample_count: 5,
                    win_count: 5,
                    empirical_win_rate: 1.0,
                    posterior_mean_win_rate: 0.85,
                    posterior_prob_beats_baseline: Some(0.98),
                    average_signed_return: 0.01,
                    first_confirm_bar: 10,
                    latest_confirm_bar: 40,
                }],
            },
            provider_summary: build_control_matrix_provider_summary(&ControlMatrixPlan::pb12()),
            runs: vec![ControlMatrixResearchRunSummary {
                run_number: 1,
                run_label: "pb12_run_01".to_string(),
                baseline: false,
                enabled_toggles: vec!["use_greeks".to_string()],
                disabled_toggles: vec!["use_oi".to_string()],
                best_factor: Some("trend".to_string()),
                aggregate_return: 0.02,
                feedback_records_generated: 1,
                feedback_records_applied: 1,
                dataset_comparable: true,
                recommended_next_command: "ict-engine factor-research".to_string(),
                runtime_notes: Vec::new(),
            }],
        });
        append_control_matrix_research_artifact(temp.path(), "NQ", artifact).unwrap();

        let report = auto_quant_promote_canonical_setup_command_with_repo_root(
            PromoteCanonicalSetupCommandInput {
                symbol: "NQ",
                state_dir: temp.path().to_str().unwrap(),
                setup_name: "Sweep Mss Continuation",
                sequence_label: "liquidity_sweep -> market_structure_shift",
                direction: Some("bull"),
                sweep_id: None,
                horizon_bars: 30,
            },
            temp.path(),
        )
        .unwrap();

        assert_eq!(report.setup_name, "SweepMssContinuation");
        assert!(std::path::Path::new(&report.config_path).exists());
        assert!(std::path::Path::new(&report.generated_path).exists());
    }
}
