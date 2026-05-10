use serde::{Deserialize, Serialize};

pub const TINY_LEG_RAW_CLUSTER_COUNT: usize = 16;
pub const TINY_LEG_MERGED_CLUSTER_COUNT: usize = 6;
pub const TINY_LEG_FEATURE_COUNT: usize = 5;
pub const DEFAULT_EVALUATION_PERIODS: [&str; 6] = ["1d", "4h", "1h", "15m", "5m", "1m"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegimeProbeMode {
    Oracle,
    TinyLegResearch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProbePromotionBoundary {
    ResearchOnly,
    RequiresLiveNowcastBranch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayerContributionSurface {
    pub layer_id: String,
    pub layer_kind: String,
    pub contribution_score: f64,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TinyLegRegimeProbeConfig {
    pub mode: RegimeProbeMode,
    pub zigzag_profile: String,
    pub leg_feature_names: Vec<String>,
    pub raw_cluster_count: usize,
    pub merged_cluster_count: usize,
    pub evaluation_periods: Vec<String>,
    pub promotion_boundary: ProbePromotionBoundary,
    pub retrospective_only: bool,
}

impl Default for TinyLegRegimeProbeConfig {
    fn default() -> Self {
        Self {
            mode: RegimeProbeMode::TinyLegResearch,
            zigzag_profile: "small_zigzag".to_string(),
            leg_feature_names: vec![
                "leg_slope".to_string(),
                "path_efficiency".to_string(),
                "leg_length".to_string(),
                "leg_time".to_string(),
                "max_drawdown_within_leg".to_string(),
            ],
            raw_cluster_count: TINY_LEG_RAW_CLUSTER_COUNT,
            merged_cluster_count: TINY_LEG_MERGED_CLUSTER_COUNT,
            evaluation_periods: DEFAULT_EVALUATION_PERIODS
                .iter()
                .map(|value| value.to_string())
                .collect(),
            promotion_boundary: ProbePromotionBoundary::RequiresLiveNowcastBranch,
            retrospective_only: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OracleRegimeProbeReport {
    pub mode: RegimeProbeMode,
    pub summary: String,
    pub evaluation_periods: Vec<String>,
    pub promotion_boundary: ProbePromotionBoundary,
    pub live_truth_rule: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_contributions: Vec<LayerContributionSurface>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OracleRegimeProbeAgentSurface {
    pub summary: String,
    pub live_truth_rule: String,
    pub contribution_count: usize,
    pub layers: Vec<LayerContributionSurface>,
}

pub fn default_tiny_leg_regime_probe_config() -> TinyLegRegimeProbeConfig {
    TinyLegRegimeProbeConfig::default()
}

pub fn build_oracle_regime_probe_report(
    summary: impl Into<String>,
    layer_contributions: Vec<LayerContributionSurface>,
) -> OracleRegimeProbeReport {
    let config = default_tiny_leg_regime_probe_config();
    OracleRegimeProbeReport {
        mode: config.mode,
        summary: summary.into(),
        evaluation_periods: config.evaluation_periods,
        promotion_boundary: config.promotion_boundary,
        live_truth_rule:
            "retrospective tiny-leg or zigzag outputs are not sufficient by themselves for live regime truth"
                .to_string(),
        layer_contributions,
    }
}

pub fn build_oracle_regime_probe_agent_surface(
    report: &OracleRegimeProbeReport,
) -> OracleRegimeProbeAgentSurface {
    OracleRegimeProbeAgentSurface {
        summary: report.summary.clone(),
        live_truth_rule: report.live_truth_rule.clone(),
        contribution_count: report.layer_contributions.len(),
        layers: report.layer_contributions.clone(),
    }
}

pub fn render_oracle_regime_probe_human_lines(report: &OracleRegimeProbeReport) -> Vec<String> {
    let mut lines = vec![
        format!("Probe: {}", report.summary),
        format!("Live truth rule: {}", report.live_truth_rule),
    ];
    lines.extend(
        report
            .layer_contributions
            .iter()
            .enumerate()
            .map(|(idx, layer)| {
                format!(
                    "Layer {}: kind={} score={:.3} rationale={}",
                    idx + 1,
                    layer.layer_kind,
                    layer.contribution_score,
                    layer.rationale
                )
            }),
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_leg_probe_defaults_stay_research_only_and_six_period() {
        let config = default_tiny_leg_regime_probe_config();
        assert_eq!(config.mode, RegimeProbeMode::TinyLegResearch);
        assert!(config.retrospective_only);
        assert_eq!(
            config.promotion_boundary,
            ProbePromotionBoundary::RequiresLiveNowcastBranch
        );
        assert_eq!(config.raw_cluster_count, TINY_LEG_RAW_CLUSTER_COUNT);
        assert_eq!(config.merged_cluster_count, TINY_LEG_MERGED_CLUSTER_COUNT);
        assert_eq!(config.leg_feature_names.len(), TINY_LEG_FEATURE_COUNT);
        assert_eq!(
            config.evaluation_periods.len(),
            DEFAULT_EVALUATION_PERIODS.len()
        );
    }

    #[test]
    fn oracle_probe_report_carries_live_truth_rule_and_layer_contributions() {
        let report = build_oracle_regime_probe_report(
            "research-only tiny-leg probe",
            vec![LayerContributionSurface {
                layer_id: "layer-1".to_string(),
                layer_kind: "retrospective_cluster".to_string(),
                contribution_score: 0.42,
                rationale: "small zigzag leg clustering evidence".to_string(),
            }],
        );
        assert_eq!(report.mode, RegimeProbeMode::TinyLegResearch);
        assert_eq!(
            report.evaluation_periods.len(),
            DEFAULT_EVALUATION_PERIODS.len()
        );
        assert_eq!(
            report.promotion_boundary,
            ProbePromotionBoundary::RequiresLiveNowcastBranch
        );
        assert!(report.live_truth_rule.contains("not sufficient"));
        assert_eq!(report.layer_contributions.len(), 1);
    }

    #[test]
    fn oracle_probe_surfaces_render_layer_contributions_for_humans_and_agents() {
        let report = build_oracle_regime_probe_report(
            "research-only tiny-leg probe",
            vec![LayerContributionSurface {
                layer_id: "layer-1".to_string(),
                layer_kind: "retrospective_cluster".to_string(),
                contribution_score: 0.42,
                rationale: "small zigzag leg clustering evidence".to_string(),
            }],
        );
        let agent = build_oracle_regime_probe_agent_surface(&report);
        let human = render_oracle_regime_probe_human_lines(&report);
        assert_eq!(agent.contribution_count, 1);
        assert_eq!(agent.layers[0].layer_kind, "retrospective_cluster");
        assert!(human[0].contains("Probe:"));
        assert!(human[1].contains("Live truth rule:"));
        assert!(human[2].contains("Layer 1:"));
        assert!(human[2].contains("score=0.420"));
    }
}
