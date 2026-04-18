use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct BacktestResultArtifact {
    pub summary: String,
    pub scorecards: Vec<String>,
    pub shrink_comparison_summary: Vec<String>,
    pub oos_quality_delta_surface: Vec<String>,
    pub market_breakdown: Vec<String>,
    pub regime_breakdown: Vec<String>,
    pub window_breakdown: Vec<String>,
    pub comparable: bool,
    pub artifacts: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BacktestResultArtifactInput {
    pub summary: String,
    pub scorecards: Vec<String>,
    pub shrink_comparison_summary: Vec<String>,
    pub oos_quality_delta_surface: Vec<String>,
    pub market_breakdown: Vec<String>,
    pub regime_breakdown: Vec<String>,
    pub window_breakdown: Vec<String>,
    pub comparable: bool,
    pub artifacts: Vec<String>,
}

pub fn build_backtest_result_artifact(
    input: BacktestResultArtifactInput,
) -> BacktestResultArtifact {
    BacktestResultArtifact {
        summary: input.summary,
        scorecards: input.scorecards,
        shrink_comparison_summary: input.shrink_comparison_summary,
        oos_quality_delta_surface: input.oos_quality_delta_surface,
        market_breakdown: input.market_breakdown,
        regime_breakdown: input.regime_breakdown,
        window_breakdown: input.window_breakdown,
        comparable: input.comparable,
        artifacts: input.artifacts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backtest_result_builder_keeps_comparable_flag() {
        let result = build_backtest_result_artifact(BacktestResultArtifactInput {
            summary: "summary".to_string(),
            scorecards: vec![],
            shrink_comparison_summary: vec!["shrink_preference=neutral".to_string()],
            oos_quality_delta_surface: vec!["oos_quality_direction=flat".to_string()],
            market_breakdown: vec![],
            regime_breakdown: vec![],
            window_breakdown: vec![],
            comparable: true,
            artifacts: vec![],
        });
        assert!(result.comparable);
        assert_eq!(
            result.shrink_comparison_summary,
            vec!["shrink_preference=neutral".to_string()]
        );
        assert_eq!(
            result.oos_quality_delta_surface,
            vec!["oos_quality_direction=flat".to_string()]
        );
    }
}
