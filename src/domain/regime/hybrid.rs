use anyhow::Result;
use std::collections::BTreeMap;

use crate::config::{regime_feature_vector, FrameFeatures};
use crate::pda_sequence::PdaSequenceArtifactSummary;

use super::{
    estimate_duration_state, geometric_duration, negative_binomial_duration, timeframe_alignment,
    DurationDistribution, RegimeGovernor, RegimeSegmentationPacket, WassersteinClassifier,
};

pub const HYBRID_REGIME_METHOD: &str = "hybrid_regime_first_pass_v1";
pub const HYBRID_REGIME_SEGMENTATION_VERSION: &str = "v2";

fn regime_family_from_label(label: &str) -> &'static str {
    match label {
        "trend_impulse" | "trend_decay" => "trend",
        "range_calm" | "range_choppy" => "range",
        _ => "transition",
    }
}

fn empirical_duration_distribution(samples: &[usize]) -> Option<DurationDistribution> {
    if samples.len() < 3 {
        return None;
    }
    let mean = samples.iter().map(|value| *value as f64).sum::<f64>() / samples.len() as f64;
    let variance = samples
        .iter()
        .map(|value| {
            let diff = *value as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / samples.len() as f64;
    Some(negative_binomial_duration(
        mean.max(1.0),
        variance.max(mean + 1.0),
    ))
}

fn baseline_duration_distribution(label: &str, market: Option<&str>) -> DurationDistribution {
    let family = regime_family_from_label(label);
    match (family, market.map(|value| value.to_ascii_uppercase())) {
        ("trend", Some(market)) if market == "CL" => negative_binomial_duration(4.0, 12.0),
        ("trend", Some(market)) if market == "NQ" => negative_binomial_duration(7.0, 21.0),
        ("trend", _) => negative_binomial_duration(6.0, 18.0),
        ("range", Some(market)) if market == "GC" => geometric_duration(5.0),
        ("range", _) => geometric_duration(4.0),
        _ => geometric_duration(3.0),
    }
}

pub fn build_hybrid_regime_packet(
    higher_timeframe: Option<&FrameFeatures>,
    current: &FrameFeatures,
    previous_label: Option<&str>,
    current_regime_age_bars: Option<usize>,
    market: Option<&str>,
    historical_regime_ages: &[usize],
    pda_sequence_summary: Option<&PdaSequenceArtifactSummary>,
) -> Result<RegimeSegmentationPacket> {
    let features = regime_feature_vector(current);
    let feature_attribution: BTreeMap<String, f64> = [
        ("trend_distance".to_string(), features[0]),
        ("mean_reversion_pressure".to_string(), features[1]),
        ("fvg_share".to_string(), features[2]),
        ("sweep_share".to_string(), features[3]),
    ]
    .into_iter()
    .collect();

    let classifier = WassersteinClassifier::default();
    let classification = classifier.classify(&features)?;
    let governor = RegimeGovernor::new(0.20, 2.0, 3);
    let decision = governor.decide_with_previous(
        &classification.label,
        &classification.membership,
        0,
        previous_label,
        usize::MAX,
    )?;

    let alignment = higher_timeframe.and_then(|higher| {
        let higher_classification = classifier.classify(&regime_feature_vector(higher)).ok()?;
        Some(timeframe_alignment(
            &higher_classification.label,
            &decision.selected_label,
        ))
    });
    let pda_alignment = pda_sequence_summary.and_then(|summary| {
        summary.primary_cluster_family.as_deref().map(|family| {
            let hybrid_family = regime_family_from_label(&decision.selected_label);
            (family == hybrid_family) || family == "transition"
        })
    });
    let elapsed_bars = current_regime_age_bars.unwrap_or(1).max(1);
    let duration_distribution = empirical_duration_distribution(historical_regime_ages)
        .unwrap_or_else(|| baseline_duration_distribution(&decision.selected_label, market));
    let duration_state = estimate_duration_state(elapsed_bars, &duration_distribution);
    let adjusted_confidence = if matches!(pda_alignment, Some(false)) {
        (decision.confidence - 0.10).clamp(0.0, 1.0)
    } else {
        decision.confidence
    };
    let transition_hazard = if matches!(pda_alignment, Some(false)) {
        duration_state
            .hazard_rate
            .max((1.0 - adjusted_confidence + 0.25).clamp(0.0, 1.0))
    } else {
        duration_state
            .hazard_rate
            .max((1.0 - adjusted_confidence).clamp(0.0, 1.0))
    };

    let mut evidence = vec![
        format!("wasserstein_label={}", classification.label),
        format!("wasserstein_distance={:.4}", classification.distance),
        format!("duration_elapsed_bars={}", duration_state.elapsed_bars),
        format!("duration_model={}", duration_state.model),
        format!("duration_hazard_rate={:.4}", duration_state.hazard_rate),
        format!("duration_survival_prob={:.4}", duration_state.survival_prob),
        format!(
            "duration_remaining_expected_bars={:.4}",
            duration_state.remaining_expected_bars
        ),
        format!("duration_history_samples={}", historical_regime_ages.len()),
    ];
    evidence.extend(decision.evidence.clone());
    if let Some(summary) = pda_sequence_summary {
        evidence.push(format!(
            "pda_cluster_family={}",
            summary
                .primary_cluster_family
                .as_deref()
                .unwrap_or("unknown")
        ));
        evidence.push(format!(
            "pda_cluster_label={}",
            summary
                .primary_cluster_label
                .as_deref()
                .unwrap_or("unknown")
        ));
        evidence.push(format!(
            "pda_hybrid_alignment={}",
            pda_alignment.unwrap_or(false)
        ));
    }
    if let Some(alignment) = &alignment {
        evidence.push(format!("timeframe_alignment={}", alignment.aligned));
        evidence.push(format!("timeframe_alignment_score={:.4}", alignment.score));
        evidence.extend(alignment.evidence.clone());
    }

    Ok(RegimeSegmentationPacket {
        method: HYBRID_REGIME_METHOD.to_string(),
        segmentation_version: HYBRID_REGIME_SEGMENTATION_VERSION.to_string(),
        active_regime_cluster: Some(decision.selected_label),
        transition_hazard: Some(transition_hazard),
        duration_elapsed_bars: Some(duration_state.elapsed_bars),
        duration_model: Some(duration_state.model),
        duration_remaining_expected_bars: Some(duration_state.remaining_expected_bars),
        regime_membership: classification.membership,
        feature_attribution,
        evidence,
        wasserstein_label: Some(classification.label),
        wasserstein_distance: Some(classification.distance),
        governor_confidence: Some(adjusted_confidence),
        governor_entropy: Some(decision.entropy),
        governor_min_hold_active: Some(decision.min_hold_active),
        timeframe_alignment: alignment.as_ref().map(|item| item.aligned),
        timeframe_alignment_score: alignment.as_ref().map(|item| item.score),
    })
}
