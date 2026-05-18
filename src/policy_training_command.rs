use super::*;
use ict_engine::application::entry_models::{
    apply_structural_path_ranking_external_scores_command,
    clear_structural_path_ranking_trainer_artifact_command,
    disable_structural_path_ranking_runtime_command,
    enable_structural_path_ranking_runtime_command, export_structural_path_ranking_target_command,
    policy_training_status_command, register_structural_path_ranking_trainer_artifact_command,
    PolicyTrainingStatusSurface,
};

fn render_policy_training_status_low_token(surface: &PolicyTrainingStatusSurface) -> String {
    [
        surface.summary_line.as_str(),
        surface.factor_candidate_packs.summary_line.as_str(),
        surface.regime_confidence_assets.summary_line.as_str(),
        surface.structural_path_ranking_runtime_summary.as_str(),
        surface.structural_path_ranking_validation_summary.as_str(),
    ]
    .join("\n")
}

pub(crate) fn policy_training_status_shell(
    symbol: &str,
    state_dir: &str,
    entry_model: Option<&str>,
    output_format: &str,
) -> Result<()> {
    ensure_state_dir_ready(state_dir)?;
    match output_format {
        "json" | "agent" => policy_training_status_command(state_dir, symbol, entry_model),
        "compact" | "human" => {
            let surface = ict_engine::application::entry_models::policy_training_status(
                state_dir,
                symbol,
                entry_model,
            )?;
            println!("{}", render_policy_training_status_low_token(&surface));
            Ok(())
        }
        other => anyhow::bail!(
            "unsupported policy-training-status output format '{}'; expected json, compact, agent, or human",
            other
        ),
    }
}

pub(crate) fn register_structural_path_ranking_trainer_artifact_shell(
    symbol: &str,
    state_dir: &str,
    artifact_uri: &str,
    model_family: &str,
    score_column: &str,
    trained_rows: Option<usize>,
    calibration_rows: Option<usize>,
) -> Result<()> {
    ensure_state_dir_ready(state_dir)?;
    register_structural_path_ranking_trainer_artifact_command(
        state_dir,
        symbol,
        artifact_uri,
        model_family,
        Some(score_column),
        trained_rows,
        calibration_rows,
    )
}

pub(crate) fn clear_structural_path_ranking_trainer_artifact_shell(
    symbol: &str,
    state_dir: &str,
) -> Result<()> {
    ensure_state_dir_ready(state_dir)?;
    clear_structural_path_ranking_trainer_artifact_command(state_dir, symbol)
}

pub(crate) fn enable_structural_path_ranking_runtime_shell(
    symbol: &str,
    state_dir: &str,
    reuse_mode: &str,
) -> Result<()> {
    ensure_state_dir_ready(state_dir)?;
    enable_structural_path_ranking_runtime_command(state_dir, symbol, reuse_mode)
}

pub(crate) fn disable_structural_path_ranking_runtime_shell(
    symbol: &str,
    state_dir: &str,
) -> Result<()> {
    ensure_state_dir_ready(state_dir)?;
    disable_structural_path_ranking_runtime_command(state_dir, symbol)
}

pub(crate) fn export_structural_path_ranking_target_shell(
    symbol: &str,
    state_dir: &str,
) -> Result<()> {
    ensure_state_dir_ready(state_dir)?;
    export_structural_path_ranking_target_command(state_dir, symbol)
}

pub(crate) fn apply_structural_path_ranking_external_scores_shell(
    symbol: &str,
    state_dir: &str,
    scores_file: &str,
) -> Result<()> {
    ensure_state_dir_ready(state_dir)?;
    apply_structural_path_ranking_external_scores_command(state_dir, symbol, scores_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_policy_training_status_low_token_emits_three_summary_lines() {
        let surface = PolicyTrainingStatusSurface {
            summary_line: "top-level".to_string(),
            factor_candidate_packs:
                ict_engine::application::entry_models::FactorCandidatePackTrainingStatusSurface {
                    summary_line: "candidate-packs".to_string(),
                    ..Default::default()
                },
            regime_confidence_assets:
                ict_engine::application::entry_models::RegimeConfidenceAssetTrainingStatusSurface {
                    summary_line: "regime-assets".to_string(),
                    ..Default::default()
                },
            structural_path_ranking_runtime_summary: "runtime-line".to_string(),
            structural_path_ranking_validation_summary: "validation-line".to_string(),
            ..PolicyTrainingStatusSurface::default()
        };

        let rendered = render_policy_training_status_low_token(&surface);
        let lines = rendered.lines().collect::<Vec<_>>();

        assert_eq!(
            lines,
            vec![
                "top-level",
                "candidate-packs",
                "regime-assets",
                "runtime-line",
                "validation-line"
            ]
        );
    }
}
