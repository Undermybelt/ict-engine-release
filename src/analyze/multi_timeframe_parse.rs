#[derive(Debug, Clone, Default)]
pub struct ParsedMultiTimeframeEvidence {
    pub direction_bias: String,
    pub alignment_score: Option<f64>,
    pub entry_alignment_score: Option<f64>,
    /// Number of timeframe intervals actually covered (0-6).
    pub covered_count: usize,
}

pub fn multi_timeframe_direction_conflicts_with(label: &str, direction_bias: &str) -> bool {
    matches!(
        (label, direction_bias),
        ("bull", "bearish") | ("bear", "bullish") | ("bullish", "bearish") | ("bearish", "bullish")
    )
}

pub fn classify_multi_timeframe_resonance(
    policy: &crate::state::PreBayesEvidencePolicy,
    direction_conflict: bool,
    evidence: &ParsedMultiTimeframeEvidence,
) -> String {
    // Graceful degradation: with fewer than 3 timeframes, neutral bias is
    // expected (no long-term bucket) — treat as "aligned" if alignment scores
    // are healthy, rather than penalising to "mixed".
    let sparse_coverage = evidence.covered_count > 0 && evidence.covered_count < 3;
    let alignment = evidence.alignment_score.unwrap_or(0.5);
    let entry_alignment = evidence.entry_alignment_score.unwrap_or(0.5);
    if direction_conflict
        || alignment < policy.min_multi_timeframe_alignment_score * 0.8
        || entry_alignment < policy.min_multi_timeframe_entry_alignment_score * 0.8
    {
        "dislocated".to_string()
    } else if !sparse_coverage
        && (evidence.direction_bias == "neutral"
            || alignment < policy.min_multi_timeframe_alignment_score
            || entry_alignment < policy.min_multi_timeframe_entry_alignment_score)
    {
        "mixed".to_string()
    } else if sparse_coverage
        && alignment >= policy.min_multi_timeframe_alignment_score
        && entry_alignment >= policy.min_multi_timeframe_entry_alignment_score
    {
        "aligned".to_string()
    } else if sparse_coverage {
        "mixed".to_string()
    } else {
        "aligned".to_string()
    }
}

pub fn parse_multi_timeframe_evidence(
    multi_timeframe_summary: &[String],
) -> ParsedMultiTimeframeEvidence {
    let direction_bias = multi_timeframe_summary
        .iter()
        .find_map(|item| item.strip_prefix("higher_timeframe_direction_bias="))
        .unwrap_or("neutral")
        .to_string();
    let alignment_score = multi_timeframe_summary
        .iter()
        .find_map(|item| item.strip_prefix("higher_timeframe_alignment_score="))
        .and_then(|value| value.parse::<f64>().ok());
    let entry_alignment_score = multi_timeframe_summary
        .iter()
        .find_map(|item| item.strip_prefix("lower_timeframe_entry_alignment_score="))
        .and_then(|value| value.parse::<f64>().ok());
    let covered_count = multi_timeframe_summary
        .iter()
        .find_map(|item| {
            item.strip_prefix("multi_timeframe_source=")
                .and_then(|rest| rest.split(" covered_intervals=").nth(1))
                .map(|intervals| intervals.split(',').filter(|s| !s.is_empty()).count())
        })
        .unwrap_or(0);
    ParsedMultiTimeframeEvidence {
        direction_bias,
        alignment_score,
        entry_alignment_score,
        covered_count,
    }
}
