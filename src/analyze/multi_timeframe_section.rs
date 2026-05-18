use serde::Serialize;

use super::multi_timeframe_parse::parse_multi_timeframe_evidence;
use super::types::AnalyzeMultiTimeframeInterval;
use crate::state::PreBayesEvidenceFilter;

#[derive(Debug, Serialize)]
pub struct AnalyzeMultiTimeframeSection {
    pub probability_role: String,
    pub source_mode: String,
    pub direction_bias: String,
    pub alignment_score: Option<f64>,
    pub entry_alignment_score: Option<f64>,
    pub resonance_label: String,
    pub intervals: Vec<AnalyzeMultiTimeframeInterval>,
    pub summary: Vec<String>,
    pub narrative: String,
}

fn parse_multi_timeframe_interval_entry(item: &str) -> Option<AnalyzeMultiTimeframeInterval> {
    let (interval, rest) = item.split_once(':')?;
    let (bars_part, source_part) = rest.split_once(" bars ")?;
    let bars = bars_part.parse::<usize>().ok()?;
    Some(AnalyzeMultiTimeframeInterval {
        interval: interval.to_string(),
        bars,
        source_detail: source_part.to_string(),
    })
}

pub fn build_analyze_multi_timeframe_section(
    summary: &[String],
    filter: Option<&PreBayesEvidenceFilter>,
) -> AnalyzeMultiTimeframeSection {
    let evidence = parse_multi_timeframe_evidence(summary);
    let source_mode = summary
        .iter()
        .find_map(|item| item.strip_prefix("multi_timeframe_source="))
        .unwrap_or("primary_only")
        .to_string();
    let resonance_label = filter
        .map(|filter| filter.filtered_multi_timeframe_resonance_label.clone())
        .unwrap_or_default();
    let narrative = if resonance_label == "aligned" {
        "six_timeframe_resonance_is_supportive_but_still_evidence_only".to_string()
    } else if resonance_label == "dislocated" {
        "six_timeframe_resonance_is_dislocated_so_pre_bayes_should_downweight_directional_commitment".to_string()
    } else {
        "six_timeframe_resonance_is_mixed_and_requires_soft_evidence_handling".to_string()
    };
    AnalyzeMultiTimeframeSection {
        probability_role:
            "six_timeframe_context_shapes_regime_selection_pre_bayes_resonance_and_execution_bias"
                .to_string(),
        source_mode,
        direction_bias: evidence.direction_bias,
        alignment_score: evidence.alignment_score,
        entry_alignment_score: evidence.entry_alignment_score,
        resonance_label,
        intervals: summary
            .iter()
            .filter_map(|item| parse_multi_timeframe_interval_entry(item))
            .collect(),
        summary: summary.to_vec(),
        narrative,
    }
}
