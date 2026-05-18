use crate::types::Direction;

use super::{
    AnalysisArtifact, ExecutionPlanArtifact, PolicyEngine, PolicyFeatureVector,
    QualificationArtifact, StagedArtifacts,
};
use crate::factor_lab::FactorDiagnostics;
use crate::state::PreBayesEvidenceFilter;

fn parse_summary_value<'a>(summary: &'a [String], key: &str) -> Option<&'a str> {
    summary
        .iter()
        .find_map(|item| item.strip_prefix(&format!("{key}=")))
}

fn derive_session_model(summary: &[String]) -> String {
    let source_mode = parse_summary_value(summary, "multi_timeframe_source").unwrap_or_default();
    let source_mode_lower = source_mode.to_ascii_lowercase();
    if source_mode_lower.contains("silver") {
        "silver_bullet".to_string()
    } else if source_mode_lower.contains("judas") {
        "judas".to_string()
    } else if source_mode_lower.contains("turtle") {
        "turtle_soup".to_string()
    } else {
        "standard".to_string()
    }
}

fn derive_signal_bar_pattern(summary: &[String], active_pda_count: usize) -> String {
    let source_mode = parse_summary_value(summary, "multi_timeframe_source")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if source_mode.contains("judas") || source_mode.contains("turtle") {
        "sweep_reject".to_string()
    } else if active_pda_count > 0 {
        "displacement".to_string()
    } else {
        "none".to_string()
    }
}

fn map_timed_pda_label_to_setup_family(label: &str) -> String {
    let concept = label.split(':').next().unwrap_or_default();
    match concept {
        "FairValueGap" => "fair_value_gap",
        "InversionFairValueGap" => "inverse_fvg",
        "BalancedPriceRange" => "breaker_block",
        "LiquidityPool" => "liquidity_void",
        "EqualHighsLows" => "turtle_soup",
        "OptimalTradeEntry" => "ote_confluence",
        "Ndog" | "Nwog" | "OpenRangeGap" => "silver_bullet",
        "SwingFailurePattern" => "judas_swing",
        _ => "none",
    }
    .to_string()
}

fn parse_band_value(label: &str, key: &str) -> Option<f64> {
    label.split('|').find_map(|part| {
        part.trim()
            .strip_prefix(&format!("{key}="))
            .and_then(|value| value.parse::<f64>().ok())
    })
}

fn parse_bps_value(label: &str, key: &str) -> Option<f64> {
    label.split('|').find_map(|part| {
        part.trim()
            .strip_prefix(&format!("{key}="))
            .and_then(|value| value.parse::<f64>().ok())
    })
}

fn nearest_active_band(label: Option<&str>) -> Option<(f64, f64)> {
    label.and_then(|raw| {
        let top = parse_band_value(raw, "top")?;
        let bottom = parse_band_value(raw, "bottom")?;
        Some((top.max(bottom), top.min(bottom)))
    })
}

fn midpoint(top: f64, bottom: f64) -> f64 {
    (top + bottom) / 2.0
}

fn bps_distance(anchor: f64, target: f64) -> f64 {
    if anchor.abs() <= f64::EPSILON {
        0.0
    } else {
        ((target - anchor).abs() / anchor.abs()) * 10_000.0
    }
}

#[derive(Debug, Clone)]
pub struct StagedArtifactsInput<'a> {
    pub diagnostics: &'a FactorDiagnostics,
    pub decision_hint: &'a str,
    pub filter: &'a PreBayesEvidenceFilter,
    pub multi_timeframe_summary: &'a [String],
    pub selected_entry_quality: &'a str,
    pub direction: Direction,
    pub risk_reward: f64,
    pub kelly_fraction: f64,
    pub recommended_command: &'a str,
}

pub fn build_staged_artifacts(
    input: StagedArtifactsInput<'_>,
    policy_engine: &dyn PolicyEngine,
) -> StagedArtifacts {
    let StagedArtifactsInput {
        diagnostics,
        decision_hint,
        filter,
        multi_timeframe_summary,
        selected_entry_quality,
        direction,
        risk_reward,
        kelly_fraction,
        recommended_command,
    } = input;

    let plan_status = if matches!(direction, Direction::Neutral) {
        "plan_blocked"
    } else {
        "plan_ready"
    };
    let selected_direction = format!("{:?}", direction);
    let session_model = derive_session_model(multi_timeframe_summary);
    let signal_bar_pattern =
        derive_signal_bar_pattern(multi_timeframe_summary, filter.active_pda_count);
    let overlap_ratio = filter
        .filtered_multi_timeframe_entry_alignment_score
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let entry_alignment = parse_summary_value(
        multi_timeframe_summary,
        "lower_timeframe_entry_alignment_score",
    )
    .and_then(|value| value.parse::<f64>().ok())
    .unwrap_or(0.5);
    let htf_alignment =
        parse_summary_value(multi_timeframe_summary, "higher_timeframe_alignment_score")
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.5);
    let entry_style = if matches!(direction, Direction::Neutral) {
        "observe".to_string()
    } else if filter.active_pda_count > 0 || entry_alignment >= 0.6 || htf_alignment >= 0.6 {
        "limit_pullback".to_string()
    } else {
        "market_confirmation".to_string()
    };
    let active_band = nearest_active_band(filter.nearest_active_pda.as_deref());
    let pda_distance_bps = active_band
        .map(|(top, bottom)| {
            let reference = midpoint(top, bottom);
            bps_distance(
                reference,
                match direction {
                    Direction::Bull => bottom,
                    Direction::Bear => top,
                    Direction::Neutral => reference,
                },
            )
        })
        .unwrap_or(0.0);
    let pda_width_bps = active_band
        .map(|(top, bottom)| bps_distance(midpoint(top, bottom), top))
        .unwrap_or(0.0)
        * 2.0;
    let entry_price_offset_bps = active_band
        .map(|(top, bottom)| {
            let reference = midpoint(top, bottom);
            if entry_style == "limit_pullback" {
                bps_distance(reference, bottom)
            } else {
                bps_distance(reference, top)
            }
        })
        .unwrap_or(0.0);
    let sl_distance_bps = active_band
        .map(|(top, bottom)| {
            let reference = midpoint(top, bottom);
            match direction {
                Direction::Bull => bps_distance(reference, bottom),
                Direction::Bear => bps_distance(reference, top),
                Direction::Neutral => 0.0,
            }
        })
        .unwrap_or(0.0);
    let sweep_depth_bps = filter
        .nearest_active_pda
        .as_deref()
        .and_then(|label| parse_bps_value(label, "sweep_depth_bps"))
        .unwrap_or_else(|| {
            if filter.raw_liquidity_context_label.contains("sweep")
                || filter.filtered_liquidity_context_label.contains("sweep")
            {
                10.0
            } else {
                0.0
            }
        });
    let features = PolicyFeatureVector {
        factor_alignment: diagnostics.alignment_label.clone(),
        factor_uncertainty: diagnostics.uncertainty_label.clone(),
        gating_status: filter.gating_status.clone(),
        selected_entry_quality: selected_entry_quality.to_string(),
        recommended_command: recommended_command.to_string(),
        evidence_quality_score: filter.evidence_quality_score,
        selected_direction: selected_direction.clone(),
        risk_reward,
        kelly_fraction,
        setup_family: filter
            .nearest_active_pda
            .as_deref()
            .map(map_timed_pda_label_to_setup_family)
            .unwrap_or_else(|| "none".to_string()),
        entry_style,
        risk_template: if matches!(direction, Direction::Neutral) {
            "observe_only".to_string()
        } else if filter.inversed_pda_count > 0 {
            "tight_external".to_string()
        } else {
            "structure_external".to_string()
        },
        setup_quality: if filter.inversed_pda_count > filter.active_pda_count {
            "low".to_string()
        } else if filter.active_pda_count > 0 {
            selected_entry_quality.to_string()
        } else {
            "low".to_string()
        },
        signal_bar_pattern,
        session_model,
        higher_tf_bias_match: !matches!(direction, Direction::Neutral)
            && filter.filtered_multi_timeframe_direction_bias != "neutral",
        discount_premium_correct: !matches!(direction, Direction::Neutral)
            && filter.active_pda_count > 0,
        liquidity_swept: filter.raw_liquidity_context_label.contains("sweep")
            || filter.filtered_liquidity_context_label.contains("sweep"),
        signal_bar_present: filter.active_pda_count > 0,
        pda_signal_overlap: filter.active_pda_count > 0
            && filter
                .filtered_multi_timeframe_entry_alignment_score
                .unwrap_or(0.0)
                >= 0.5,
        timed_pda_active_nearby: filter.active_pda_count > 0,
        timed_pda_inversed_nearby: filter.inversed_pda_count > 0,
        timed_pda_stale_nearby: filter.stale_pda_count > 0,
        pda_distance_bps,
        pda_width_bps,
        overlap_ratio,
        displacement_strength: filter.evidence_quality_score.clamp(0.0, 1.0),
        sweep_depth_bps,
        entry_price_offset_bps,
        sl_distance_bps,
        tp_rr_ratio: risk_reward,
        // Flowtree features: defaults in trace_builder (populated via ensemble path)
        atr_consumption_ratio: 0.0,
        htf_dol_distance_ratio: 1.0,
        htf_eqx_swept: filter.raw_liquidity_context_label.contains("sweep")
            || filter.filtered_liquidity_context_label.contains("sweep"),
        htf_rb_type: "none".to_string(),
        event_b_consecutive_count: (filter.stale_pda_count + filter.inversed_pda_count).min(255)
            as u8,
        event_a_sequence_stage: 0,
        ltf_path_label: "none".to_string(),
        ote_0705_offset: 0.0,
        structure_break_count: 0,
        latest_break_type: "none".to_string(),
        fractal_sync_confirmed: false,
        killswitch_completion: 0,
        fvgs_open: 0,
        order_blocks_nearby: 0,
        cisd_ltf_confirmed: false,
        cisd_htf_confirmed: false,
        rb_pinbar_detected: false,
        pda_bull_count: 0,
        liquidity_sweep_count: 0,
        red_alert_active: (filter.stale_pda_count + filter.inversed_pda_count) >= 3,
        recovery_event_a_streak: 0,
        pda_survival_regime: "unknown".to_string(),
        setup_model_id: String::new(),
        setup_progress_state: String::new(),
        cisd_run_length_observed: 0.0,
        cisd_impulse_atr: 0.0,
        cisd_body_ratio_mean: 0.0,
        rb_wick_body_ratio: 0.0,
        rb_close_location_ratio: 0.0,
        bars_between_cisd_and_rb: 0.0,
        seq_window_hit: false,
        ema19_distance_bps: 0.0,
        realized_vol_zscore: 0.0,
        hmm_accumulation_prob: 0.0,
        hmm_manipulation_expansion_prob: 0.0,
        hmm_distribution_prob: 0.0,
    };
    let policy_decision = policy_engine.infer(&features);

    StagedArtifacts {
        analysis: AnalysisArtifact {
            stage: "analysis".to_string(),
            factor_alignment: diagnostics.alignment_label.clone(),
            factor_uncertainty: diagnostics.uncertainty_label.clone(),
            decision_hint: decision_hint.to_string(),
            summary: format!(
                "alignment={} uncertainty={} decision_hint={}",
                diagnostics.alignment_label, diagnostics.uncertainty_label, decision_hint
            ),
        },
        qualification: QualificationArtifact {
            stage: "qualification".to_string(),
            gating_status: filter.gating_status.clone(),
            selected_entry_quality: selected_entry_quality.to_string(),
            evidence_quality_score: filter.evidence_quality_score,
            summary: format!(
                "gate={} selected_entry_quality={} evidence_quality={:.4}",
                filter.gating_status, selected_entry_quality, filter.evidence_quality_score
            ),
        },
        execution_plan: ExecutionPlanArtifact {
            stage: "execution_plan".to_string(),
            plan_status: plan_status.to_string(),
            selected_direction: selected_direction.clone(),
            summary: format!(
                "direction={:?} rr={:.4} kelly={:.4}",
                direction, risk_reward, kelly_fraction
            ),
        },
        policy_decision,
    }
}
