use crate::state::{PreBayesEntryQualityBridge, PreBayesEvidenceFilter};

use super::artifact::ExecutionInputSnapshot;

pub struct ExecutionInputSources<'a> {
    pub pre_bayes_evidence_filter: &'a PreBayesEvidenceFilter,
    pub pre_bayes_entry_quality_bridge: &'a PreBayesEntryQualityBridge,
    pub selected_entry_quality_distribution: &'a [f64],
    pub selected_win_probability: f64,
}

pub fn derive_execution_inputs(src: &ExecutionInputSources<'_>) -> ExecutionInputSnapshot {
    let prediction_score = src
        .selected_entry_quality_distribution
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .clamp(0.0, 1.0);
    let aggression_bias = (src.pre_bayes_entry_quality_bridge.long_signal_probability
        - src.pre_bayes_entry_quality_bridge.short_signal_probability)
        .clamp(-1.0, 1.0);
    let completion_pressure = src.selected_win_probability.clamp(0.0, 1.0);
    let liquidity_absorption_bias = match src.pre_bayes_evidence_filter.gating_status.as_str() {
        "pass_hard" => 0.85,
        "pass_neutralized" => 0.60,
        _ => 0.35,
    };

    ExecutionInputSnapshot {
        aggression_bias,
        completion_pressure,
        liquidity_absorption_bias,
        evidence_quality: src
            .pre_bayes_evidence_filter
            .evidence_quality_score
            .clamp(0.0, 1.0),
        prediction_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_execution_inputs_from_pre_bayes_sources() {
        let filter = PreBayesEvidenceFilter {
            gating_status: "pass_hard".to_string(),
            evidence_quality_score: 0.91,
            ..PreBayesEvidenceFilter::default()
        };

        let bridge = PreBayesEntryQualityBridge {
            long_signal_probability: 0.72,
            short_signal_probability: 0.21,
            ..PreBayesEntryQualityBridge::default()
        };

        let snapshot = derive_execution_inputs(&ExecutionInputSources {
            pre_bayes_evidence_filter: &filter,
            pre_bayes_entry_quality_bridge: &bridge,
            selected_entry_quality_distribution: &[0.15, 0.82, 0.03],
            selected_win_probability: 0.67,
        });

        assert!((snapshot.aggression_bias - 0.51).abs() < 1e-9);
        assert!((snapshot.completion_pressure - 0.67).abs() < 1e-9);
        assert!((snapshot.liquidity_absorption_bias - 0.85).abs() < 1e-9);
        assert!((snapshot.evidence_quality - 0.91).abs() < 1e-9);
        assert!((snapshot.prediction_score - 0.82).abs() < 1e-9);
    }
}
