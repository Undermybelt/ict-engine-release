use std::collections::BTreeMap;

use crate::application::belief::{
    build_jump_model_regime_sidecar_with_history, historical_market_jump_weight,
};
use crate::domain::belief::{BeliefEvidencePacket, BeliefNodePosteriorSnapshot};
use crate::domain::regime::{
    JumpModelRegimeSummary, RegimeFeatures, RegimeGateDecision, RegimePosterior,
    RegimeSegmentationPacket,
};
use crate::domain::strategy::StrategyRecommendation;
use crate::pda_sequence::{summarize_pda_sequence_artifact, PdaSequenceAnalysisArtifact};
use crate::state::{FactorPipelineLabelSource, PreBayesEvidenceFilter, PreBayesEvidencePacket};

fn market_category(market: Option<&str>) -> Option<String> {
    let market = market?.to_ascii_uppercase();
    let category = match market.as_str() {
        "NQ" | "ES" | "YM" => "futures_index",
        "GC" => "metals",
        "CL" => "energy",
        _ => return None,
    };
    Some(category.to_string())
}

fn market_behavior_profile(category: &str) -> &'static str {
    match category {
        "futures_index" => "index_beta_regime_sensitive",
        "metals" => "metals_defensive_liquidity_sensitive",
        "energy" => "energy_volatility_shock_sensitive",
        _ => "generic",
    }
}

fn packet_market_family(packet: &BeliefEvidencePacket) -> Option<String> {
    packet
        .market_evidence
        .iter()
        .find_map(|line| line.strip_prefix("market_category="))
        .map(str::to_string)
}

fn packet_market_behavior_profile(packet: &BeliefEvidencePacket) -> Option<String> {
    packet
        .market_evidence
        .iter()
        .find_map(|line| line.strip_prefix("market_behavior_profile="))
        .map(str::to_string)
}

pub fn belief_evidence_packet_from_pre_bayes_filter(
    symbol: &str,
    market: Option<&str>,
    filter: &PreBayesEvidenceFilter,
    raw_market_regime_trace: Option<&FactorPipelineLabelSource>,
    raw_liquidity_context_trace: Option<&FactorPipelineLabelSource>,
    raw_multi_timeframe_resonance_trace: Option<&FactorPipelineLabelSource>,
) -> BeliefEvidencePacket {
    let mut market_evidence = Vec::new();
    if let Some(category) = market_category(market.or(Some(symbol))) {
        market_evidence.push(format!("market_category={category}"));
        market_evidence.push(format!(
            "market_behavior_profile={}",
            market_behavior_profile(&category)
        ));
    }
    if let Some(trace) = raw_market_regime_trace {
        market_evidence.extend(trace.evidence.clone());
    }
    if let Some(trace) = raw_liquidity_context_trace {
        market_evidence.extend(trace.evidence.clone());
    }

    let mut factor_evidence = filter.rationale.clone();
    if let Some(trace) = raw_multi_timeframe_resonance_trace {
        factor_evidence.extend(trace.evidence.clone());
    }

    let mut timed_pda_summary = BTreeMap::new();
    timed_pda_summary.insert(
        "active_pda_count".to_string(),
        filter.active_pda_count.to_string(),
    );
    timed_pda_summary.insert(
        "inversed_pda_count".to_string(),
        filter.inversed_pda_count.to_string(),
    );
    timed_pda_summary.insert(
        "stale_pda_count".to_string(),
        filter.stale_pda_count.to_string(),
    );
    if let Some(nearest) = &filter.nearest_active_pda {
        timed_pda_summary.insert("nearest_active_pda".to_string(), nearest.clone());
    }
    if let Some(nearest) = &filter.nearest_inversed_pda {
        timed_pda_summary.insert("nearest_inversed_pda".to_string(), nearest.clone());
    }

    let mut multi_timeframe_evidence = BTreeMap::new();
    multi_timeframe_evidence.insert(
        "raw_direction_bias".to_string(),
        filter.raw_multi_timeframe_direction_bias.clone(),
    );
    multi_timeframe_evidence.insert(
        "filtered_direction_bias".to_string(),
        filter.filtered_multi_timeframe_direction_bias.clone(),
    );
    multi_timeframe_evidence.insert(
        "raw_resonance_label".to_string(),
        filter.raw_multi_timeframe_resonance_label.clone(),
    );
    multi_timeframe_evidence.insert(
        "filtered_resonance_label".to_string(),
        filter.filtered_multi_timeframe_resonance_label.clone(),
    );
    if let Some(score) = filter.raw_multi_timeframe_alignment_score {
        multi_timeframe_evidence.insert("raw_alignment_score".to_string(), format!("{score:.4}"));
    }
    if let Some(score) = filter.raw_multi_timeframe_entry_alignment_score {
        multi_timeframe_evidence.insert(
            "raw_entry_alignment_score".to_string(),
            format!("{score:.4}"),
        );
    }

    let entry_logic_id = filter.entry_logic_id.clone();
    let logic_family = filter.logic_family.clone();
    if let Some(value) = &entry_logic_id {
        factor_evidence.push(format!("entry_logic_id={value}"));
    }
    if let Some(value) = &logic_family {
        factor_evidence.push(format!("logic_family={value}"));
    }

    BeliefEvidencePacket {
        symbol: symbol.to_string(),
        market: market.map(str::to_string),
        timestamp: None,
        entry_logic_id,
        logic_family,
        regime_features: RegimeFeatures {
            market_regime_label: Some(filter.filtered_market_regime_label.clone()),
            volatility_regime_label: Some(filter.filtered_factor_uncertainty.clone()),
            liquidity_regime_label: Some(filter.filtered_liquidity_context_label.clone()),
            stress_score: Some(1.0 - filter.evidence_quality_score),
            transition_score: Some(
                if filter.filtered_multi_timeframe_resonance_label == "dislocated" {
                    1.0
                } else if filter.filtered_multi_timeframe_resonance_label == "mixed" {
                    0.5
                } else {
                    0.0
                },
            ),
            evidence: filter.conflict_flags.clone(),
            segmentation_context: None,
            structural_break_context: None,
        },
        market_evidence,
        factor_evidence,
        timed_pda_summary,
        multi_timeframe_evidence,
        evidence_assignments: filter.evidence_assignments.clone(),
        uses_soft_evidence: filter.uses_soft_evidence,
        soft_market_regime_distribution: filter.soft_market_regime_distribution.clone(),
        soft_liquidity_context_distribution: filter.soft_liquidity_context_distribution.clone(),
        soft_factor_alignment_distribution: filter.soft_factor_alignment_distribution.clone(),
        soft_factor_uncertainty_distribution: filter.soft_factor_uncertainty_distribution.clone(),
        soft_multi_timeframe_resonance_distribution: filter
            .soft_multi_timeframe_resonance_distribution
            .clone(),
        microstructure_context: None,
        market_policy: None,
    }
}

pub fn belief_evidence_packet_from_pre_bayes_packet(
    symbol: &str,
    market: Option<&str>,
    packet: &PreBayesEvidencePacket,
) -> BeliefEvidencePacket {
    let mut belief = belief_evidence_packet_from_pre_bayes_filter(
        symbol,
        market,
        &packet.filter,
        Some(&packet.raw_market_regime_trace),
        Some(&packet.raw_liquidity_context_trace),
        Some(&packet.raw_multi_timeframe_resonance_trace),
    );
    belief.timed_pda_summary.insert(
        "summary_active_pda_count".to_string(),
        packet.timed_pda_summary.active_pda_count.to_string(),
    );
    belief.timed_pda_summary.insert(
        "summary_inversed_pda_count".to_string(),
        packet.timed_pda_summary.inversed_pda_count.to_string(),
    );
    belief
}

pub fn apply_pda_sequence_artifact_to_belief_packet(
    packet: &mut BeliefEvidencePacket,
    artifact: &PdaSequenceAnalysisArtifact,
) {
    let summary = summarize_pda_sequence_artifact(artifact);
    packet
        .timed_pda_summary
        .insert("pda_sequence_method".to_string(), summary.method.clone());
    packet.timed_pda_summary.insert(
        "pda_sequence_valid_sessions".to_string(),
        summary.valid_sessions.to_string(),
    );
    packet.timed_pda_summary.insert(
        "pda_sequence_consistency_ratio".to_string(),
        format!("{:.4}", summary.consistency_ratio),
    );
    packet.timed_pda_summary.insert(
        "pda_sequence_ensemble_mean_confidence".to_string(),
        format!("{:.4}", summary.ensemble_mean_confidence),
    );
    if let Some(label) = summary.primary_cluster_label.clone() {
        packet
            .timed_pda_summary
            .insert("pda_sequence_primary_cluster".to_string(), label.clone());
        packet
            .factor_evidence
            .push(format!("pda_sequence_primary_cluster={label}"));
    }
    if let Some(confidence) = summary.primary_cluster_confidence {
        packet.factor_evidence.push(format!(
            "pda_sequence_primary_cluster_confidence={confidence:.4}"
        ));
    }
    packet.factor_evidence.push(format!(
        "pda_sequence_consistency_ratio={:.4}",
        summary.consistency_ratio
    ));
    packet.factor_evidence.push(format!(
        "pda_sequence_ensemble_mean_confidence={:.4}",
        summary.ensemble_mean_confidence
    ));

    let latest_packet = artifact.ensemble_packets.last();
    let regime_membership = latest_packet
        .map(|packet| {
            packet
                .vote_distribution
                .iter()
                .enumerate()
                .map(|(idx, votes)| {
                    (
                        format!("cluster_{idx}"),
                        *votes as f64 / packet.votes.len() as f64,
                    )
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    packet.regime_features.segmentation_context = Some(RegimeSegmentationPacket {
        method: summary.method,
        segmentation_version: format!("kmer_k={}", summary.kmer_k),
        active_regime_cluster: summary.primary_cluster_label,
        transition_hazard: summary
            .primary_cluster_confidence
            .map(|c| (1.0 - c).clamp(0.0, 1.0)),
        duration_elapsed_bars: None,
        duration_model: None,
        duration_remaining_expected_bars: None,
        regime_membership,
        feature_attribution: BTreeMap::from([
            ("consistency_ratio".to_string(), summary.consistency_ratio),
            (
                "ensemble_mean_confidence".to_string(),
                summary.ensemble_mean_confidence,
            ),
        ]),
        evidence: vec![
            format!("valid_sessions={}", summary.valid_sessions),
            format!("kmer_k={}", summary.kmer_k),
        ],
        wasserstein_label: None,
        wasserstein_distance: None,
        governor_confidence: None,
        governor_entropy: None,
        governor_min_hold_active: None,
        timeframe_alignment: None,
        timeframe_alignment_score: None,
    });
}

pub fn apply_hybrid_regime_packet_to_belief_packet(
    packet: &mut BeliefEvidencePacket,
    hybrid: &RegimeSegmentationPacket,
) {
    let merged = if let Some(existing) = packet.regime_features.segmentation_context.clone() {
        let mut packet_out = hybrid.clone();
        packet_out.evidence.extend(existing.evidence);
        for (key, value) in existing.regime_membership {
            packet_out.regime_membership.entry(key).or_insert(value);
        }
        for (key, value) in existing.feature_attribution {
            packet_out.feature_attribution.entry(key).or_insert(value);
        }
        if packet_out.active_regime_cluster.is_none() {
            packet_out.active_regime_cluster = existing.active_regime_cluster;
        }
        if packet_out.transition_hazard.is_none() {
            packet_out.transition_hazard = existing.transition_hazard;
        }
        packet_out
    } else {
        hybrid.clone()
    };
    packet.regime_features.segmentation_context = Some(merged);
}

pub fn regime_posterior_from_pre_bayes_filter(filter: &PreBayesEvidenceFilter) -> RegimePosterior {
    let mut probabilities = BTreeMap::new();
    let market = if filter.uses_soft_evidence && !filter.soft_market_regime_distribution.is_empty()
    {
        filter.soft_market_regime_distribution.clone()
    } else {
        BTreeMap::from([(filter.filtered_market_regime_label.clone(), 1.0)])
    };

    let trend =
        market.get("bull").copied().unwrap_or(0.0) + market.get("bear").copied().unwrap_or(0.0);
    let range = market.get("range").copied().unwrap_or(0.0);
    let stress = if filter.filtered_liquidity_context_label == "hostile" {
        0.7f64.max(1.0 - filter.evidence_quality_score)
    } else {
        (1.0 - filter.evidence_quality_score) * 0.5
    };
    let transition: f64 = if filter.filtered_multi_timeframe_resonance_label == "dislocated" {
        0.8
    } else if filter.filtered_multi_timeframe_resonance_label == "mixed" {
        0.4
    } else {
        0.1
    };

    probabilities.insert("trend".to_string(), trend.clamp(0.0, 1.0));
    probabilities.insert("range".to_string(), range.clamp(0.0, 1.0));
    probabilities.insert("stress".to_string(), stress.clamp(0.0, 1.0));
    probabilities.insert("transition".to_string(), transition.clamp(0.0, 1.0));

    let active_regime = probabilities
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(k, _)| k.clone());

    RegimePosterior {
        active_regime,
        market_family: None,
        market_behavior_profile: None,
        jump_model: None,
        probabilities,
        confidence: Some(filter.evidence_quality_score),
        credible_intervals: BTreeMap::new(),
        evidence: filter.rationale.clone(),
        regime_validation: None,
    }
}

pub fn jump_model_summary_from_belief_packet(
    packet: &BeliefEvidencePacket,
) -> JumpModelRegimeSummary {
    let mut factor_evidence = packet.factor_evidence.clone();
    factor_evidence.extend(packet.market_evidence.iter().cloned());
    build_jump_model_regime_sidecar_with_history(
        std::env::temp_dir(),
        packet.symbol.as_str(),
        &packet.regime_features,
        &packet.multi_timeframe_evidence,
        &factor_evidence,
    )
}

pub fn regime_posterior_from_belief_packet(packet: &BeliefEvidencePacket) -> RegimePosterior {
    let mut posterior = regime_posterior_from_pre_bayes_filter(&PreBayesEvidenceFilter {
        filtered_market_regime_label: packet
            .regime_features
            .market_regime_label
            .clone()
            .unwrap_or_else(|| "range".to_string()),
        filtered_liquidity_context_label: packet
            .regime_features
            .liquidity_regime_label
            .clone()
            .unwrap_or_else(|| "neutral".to_string()),
        filtered_multi_timeframe_resonance_label: packet
            .multi_timeframe_evidence
            .get("filtered_resonance_label")
            .cloned()
            .unwrap_or_else(|| "mixed".to_string()),
        evidence_quality_score: 1.0
            - packet
                .regime_features
                .stress_score
                .unwrap_or(0.5)
                .clamp(0.0, 1.0),
        uses_soft_evidence: false,
        rationale: packet.factor_evidence.clone(),
        ..PreBayesEvidenceFilter::default()
    });
    posterior.market_family = packet_market_family(packet);
    posterior.market_behavior_profile = packet_market_behavior_profile(packet);
    posterior.jump_model = Some(jump_model_summary_from_belief_packet(packet));
    if let Some(family) = posterior.market_family.as_deref() {
        match family {
            "metals" => {
                if let Some(stress) = posterior.probabilities.get_mut("stress") {
                    *stress = (*stress + 0.08).clamp(0.0, 1.0);
                }
            }
            "energy" => {
                if let Some(transition) = posterior.probabilities.get_mut("transition") {
                    *transition = (*transition + 0.10).clamp(0.0, 1.0);
                }
                if let Some(stress) = posterior.probabilities.get_mut("stress") {
                    *stress = (*stress + 0.04).clamp(0.0, 1.0);
                }
            }
            _ => {}
        }
        posterior.active_regime = posterior
            .probabilities
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, _)| k.clone());
        posterior.evidence.push(format!("market_family={family}"));
    }
    if let Some(profile) = posterior.market_behavior_profile.clone() {
        posterior
            .evidence
            .push(format!("market_behavior_profile={profile}"));
    }
    posterior
}

pub fn gate_decision_from_regime_posterior(posterior: &RegimePosterior) -> RegimeGateDecision {
    let selected = posterior
        .active_regime
        .clone()
        .unwrap_or_else(|| "range".to_string());
    let market_subgraph = posterior
        .market_family
        .as_deref()
        .filter(|family| *family != "generic")
        .map(|family| format!("{}_{}_subgraph", family, selected))
        .unwrap_or_else(|| format!("{}_subgraph", selected));
    RegimeGateDecision {
        selected_subgraph: market_subgraph,
        selected_regime: selected,
        market_family: posterior.market_family.clone(),
        jump_weight: Some(historical_market_jump_weight(
            std::env::temp_dir(),
            "regime_gate",
            posterior.market_family.as_deref(),
            posterior.market_behavior_profile.as_deref(),
        )),
        rationale: posterior.evidence.clone(),
    }
}

pub fn strategy_recommendation_from_pre_bayes_filter(
    filter: &PreBayesEvidenceFilter,
    selected_direction: &str,
    selected_win_probability: f64,
    market_family: Option<&str>,
    market_behavior_profile: Option<&str>,
    selected_market_subgraph: Option<&str>,
) -> StrategyRecommendation {
    let aggression_level =
        if filter.gating_status == "pass_hard" && selected_win_probability >= 0.60 {
            "aggressive"
        } else if filter.gating_status == "observe_only" {
            "conservative"
        } else {
            "balanced"
        };

    let mut sizing_multiplier = match aggression_level {
        "aggressive" => 1.0,
        "balanced" => 0.65,
        _ => 0.35,
    };
    match market_family {
        Some("energy") => sizing_multiplier *= 0.85,
        Some("metals") => sizing_multiplier *= 0.92,
        Some("futures_index") => {}
        _ => {}
    }

    let mut invalidate_if = vec![
        format!("gating_status={}", filter.gating_status),
        format!(
            "liquidity_context={}",
            filter.filtered_liquidity_context_label
        ),
    ];
    if let Some(family) = market_family {
        invalidate_if.push(format!("market_family={family}"));
    }
    if let Some(profile) = market_behavior_profile {
        invalidate_if.push(format!("market_behavior_profile={profile}"));
    }

    let mut rationale = filter.rationale.clone();
    if let Some(subgraph) = selected_market_subgraph {
        rationale.push(format!("selected_market_subgraph={subgraph}"));
    }

    StrategyRecommendation {
        direction: selected_direction.to_string(),
        aggression_level: aggression_level.to_string(),
        sizing_multiplier,
        market_family: market_family.map(str::to_string),
        market_behavior_profile: market_behavior_profile.map(str::to_string),
        selected_market_subgraph: selected_market_subgraph.map(str::to_string),
        invalidate_if,
        rationale,
    }
}

pub fn belief_snapshot_from_distribution(
    node_id: &str,
    probabilities: &BTreeMap<String, f64>,
) -> BeliefNodePosteriorSnapshot {
    let (top_state, top_probability) = probabilities
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(state, prob)| (state.clone(), *prob))
        .unwrap_or_else(|| ("state_unavailable".to_string(), 0.0));

    let entropy = probabilities
        .values()
        .filter(|p| **p > 0.0)
        .map(|p| -p * p.ln())
        .sum();

    BeliefNodePosteriorSnapshot {
        node_id: node_id.to_string(),
        top_state,
        top_probability,
        entropy,
        probabilities: probabilities.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapts_pre_bayes_filter_into_belief_packet() {
        let filter = PreBayesEvidenceFilter {
            filtered_market_regime_label: "bull".to_string(),
            filtered_liquidity_context_label: "favorable".to_string(),
            filtered_factor_uncertainty: "low".to_string(),
            filtered_multi_timeframe_resonance_label: "aligned".to_string(),
            evidence_quality_score: 0.72,
            active_pda_count: 2,
            inversed_pda_count: 1,
            stale_pda_count: 0,
            evidence_assignments: BTreeMap::from([(
                "entry_quality".to_string(),
                "high".to_string(),
            )]),
            ..PreBayesEvidenceFilter::default()
        };

        let packet = belief_evidence_packet_from_pre_bayes_filter(
            "NQ",
            Some("futures"),
            &filter,
            None,
            None,
            None,
        );
        assert_eq!(packet.symbol, "NQ");
        assert_eq!(packet.market.as_deref(), Some("futures"));
        assert_eq!(
            packet.regime_features.market_regime_label.as_deref(),
            Some("bull")
        );
        assert_eq!(
            packet
                .timed_pda_summary
                .get("active_pda_count")
                .map(String::as_str),
            Some("2")
        );
    }

    #[test]
    fn derives_regime_gate_and_strategy() {
        let filter = PreBayesEvidenceFilter {
            filtered_market_regime_label: "bull".to_string(),
            filtered_liquidity_context_label: "hostile".to_string(),
            filtered_multi_timeframe_resonance_label: "dislocated".to_string(),
            evidence_quality_score: 0.41,
            gating_status: "observe_only".to_string(),
            rationale: vec!["weak_alignment".to_string()],
            ..PreBayesEvidenceFilter::default()
        };
        let posterior = regime_posterior_from_pre_bayes_filter(&filter);
        let gate = gate_decision_from_regime_posterior(&posterior);
        let strategy =
            strategy_recommendation_from_pre_bayes_filter(&filter, "bear", 0.54, None, None, None);
        assert!(posterior.probabilities.contains_key("stress"));
        assert!(gate.selected_subgraph.ends_with("_subgraph"));
        assert_eq!(strategy.aggression_level, "conservative");
    }

    #[test]
    fn belief_packet_market_family_changes_regime_and_subgraph() {
        let packet = BeliefEvidencePacket {
            symbol: "CL".to_string(),
            market: Some("CL".to_string()),
            market_evidence: vec![
                "market_category=energy".to_string(),
                "market_behavior_profile=energy_volatility_shock_sensitive".to_string(),
            ],
            factor_evidence: vec!["shock".to_string()],
            regime_features: RegimeFeatures {
                market_regime_label: Some("bull".to_string()),
                liquidity_regime_label: Some("hostile".to_string()),
                stress_score: Some(0.45),
                ..RegimeFeatures::default()
            },
            multi_timeframe_evidence: BTreeMap::from([(
                "filtered_resonance_label".to_string(),
                "dislocated".to_string(),
            )]),
            ..BeliefEvidencePacket::default()
        };

        let posterior = regime_posterior_from_belief_packet(&packet);
        let gate = gate_decision_from_regime_posterior(&posterior);

        assert_eq!(posterior.market_family.as_deref(), Some("energy"));
        assert_eq!(
            posterior.market_behavior_profile.as_deref(),
            Some("energy_volatility_shock_sensitive")
        );
        assert!(posterior
            .evidence
            .iter()
            .any(|line| line == "market_family=energy"));
        assert_eq!(gate.market_family.as_deref(), Some("energy"));
        assert!(gate.selected_subgraph.starts_with("energy_"));
    }

    #[test]
    fn belief_packet_can_include_pda_sequence_artifact_summary() {
        let mut packet = BeliefEvidencePacket::default();
        let artifact = PdaSequenceAnalysisArtifact {
            artifact_id: "pda-sequence-NQ-1".to_string(),
            generated_at: chrono::Utc::now(),
            symbol: "NQ".to_string(),
            method: "pda_sequence_analysis_v2".to_string(),
            k: 2,
            n_states: 3,
            kmer_k: 2,
            total_sessions: 8,
            valid_sessions: 8,
            silhouette_score: 0.5,
            consistency_ratio: 0.75,
            ensemble_mean_confidence: 0.83,
            dtw_packets: Vec::new(),
            hmm_classifications: Vec::new(),
            fcgr_labels: vec![0, 1],
            ensemble_packets: vec![crate::pda_sequence::PdaClusteringPacket {
                method: "pda_ensemble_majority_v1".to_string(),
                primary_cluster: 1,
                confidence: 1.0,
                vote_distribution: vec![0, 3],
                votes: [1, 1, 1],
                voter_names: ["dtw".to_string(), "hmm".to_string(), "fcgr".to_string()],
            }],
            provenance: crate::state::RunProvenance::default(),
        };

        apply_pda_sequence_artifact_to_belief_packet(&mut packet, &artifact);
        assert_eq!(
            packet
                .timed_pda_summary
                .get("pda_sequence_primary_cluster")
                .map(String::as_str),
            Some("cluster_1")
        );
        assert!(packet
            .factor_evidence
            .iter()
            .any(|line| line.starts_with("pda_sequence_consistency_ratio=")));
        assert_eq!(
            packet
                .regime_features
                .segmentation_context
                .as_ref()
                .and_then(|ctx| ctx.active_regime_cluster.as_deref()),
            Some("cluster_1")
        );
    }

    #[test]
    fn belief_packet_can_merge_hybrid_regime_packet_over_pda_context() {
        let mut packet = BeliefEvidencePacket::default();
        let artifact = PdaSequenceAnalysisArtifact {
            artifact_id: "pda-sequence-NQ-1".to_string(),
            generated_at: chrono::Utc::now(),
            symbol: "NQ".to_string(),
            method: "pda_sequence_analysis_v2".to_string(),
            k: 2,
            n_states: 3,
            kmer_k: 2,
            total_sessions: 8,
            valid_sessions: 8,
            silhouette_score: 0.5,
            consistency_ratio: 0.75,
            ensemble_mean_confidence: 0.83,
            dtw_packets: Vec::new(),
            hmm_classifications: Vec::new(),
            fcgr_labels: vec![0, 1],
            ensemble_packets: vec![crate::pda_sequence::PdaClusteringPacket {
                method: "pda_ensemble_majority_v1".to_string(),
                primary_cluster: 1,
                confidence: 1.0,
                vote_distribution: vec![0, 3],
                votes: [1, 1, 1],
                voter_names: ["dtw".to_string(), "hmm".to_string(), "fcgr".to_string()],
            }],
            provenance: crate::state::RunProvenance::default(),
        };
        apply_pda_sequence_artifact_to_belief_packet(&mut packet, &artifact);
        apply_hybrid_regime_packet_to_belief_packet(
            &mut packet,
            &RegimeSegmentationPacket {
                method: "hybrid_regime_first_pass_v1".to_string(),
                segmentation_version: "v2".to_string(),
                active_regime_cluster: Some("trend_impulse".to_string()),
                transition_hazard: Some(0.30),
                duration_elapsed_bars: Some(3),
                duration_model: Some("negative_binomial".to_string()),
                duration_remaining_expected_bars: Some(4.0),
                regime_membership: BTreeMap::from([("trend_impulse".to_string(), 0.7)]),
                feature_attribution: BTreeMap::from([("trend_distance".to_string(), 0.8)]),
                evidence: vec!["pda_hybrid_alignment=false".to_string()],
                wasserstein_label: Some("trend_impulse".to_string()),
                wasserstein_distance: Some(0.12),
                governor_confidence: Some(0.64),
                governor_entropy: Some(0.90),
                governor_min_hold_active: Some(false),
                timeframe_alignment: Some(true),
                timeframe_alignment_score: Some(1.0),
            },
        );
        let segmentation = packet.regime_features.segmentation_context.unwrap();
        assert_eq!(
            segmentation.active_regime_cluster.as_deref(),
            Some("trend_impulse")
        );
        assert!(segmentation
            .evidence
            .iter()
            .any(|line| line == "pda_hybrid_alignment=false"));
        assert!(segmentation
            .feature_attribution
            .contains_key("ensemble_mean_confidence"));
    }
}
