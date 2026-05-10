use crate::application::orchestration::ExecutionTreeOutput;
use crate::domain::regime::RegimeSegmentationPacket;
use crate::types::TradePlan;

pub fn apply_duration_sizing_adjustment(
    mut trade_plan: TradePlan,
    market: &str,
    hybrid_regime: &RegimeSegmentationPacket,
) -> TradePlan {
    let Some(remaining) = hybrid_regime.duration_remaining_expected_bars else {
        return trade_plan;
    };
    let family = hybrid_regime
        .active_regime_cluster
        .as_deref()
        .map(|label| {
            if label.contains("trend") {
                "trend"
            } else if label.contains("range") {
                "range"
            } else {
                "transition"
            }
        })
        .unwrap_or("transition");
    let scale = duration_sizing_scale(market, family, remaining);
    if scale < 1.0 {
        trade_plan.kelly_fraction *= scale;
        trade_plan.position_size *= scale;
        trade_plan.uncertainties.push(format!(
            "duration_sizing_scale={scale:.2} remaining_expected_bars={remaining:.3} market={} family={}",
            market,
            family
        ));
        if scale == 0.0 {
            trade_plan
                .uncertainties
                .push("duration_window_too_short_for_execution_size_zeroed".to_string());
        }
    }
    trade_plan
}

pub fn duration_sizing_scale(market: &str, family: &str, remaining_expected_bars: f64) -> f64 {
    let _ = market;
    match family {
        "trend" => {
            if remaining_expected_bars <= 1.5 {
                0.0
            } else if remaining_expected_bars <= 2.5 {
                0.25
            } else if remaining_expected_bars <= 4.0 {
                0.50
            } else {
                1.0
            }
        }
        "range" => {
            if remaining_expected_bars <= 1.0 {
                0.0
            } else if remaining_expected_bars <= 2.0 {
                0.35
            } else if remaining_expected_bars <= 3.5 {
                0.60
            } else {
                1.0
            }
        }
        _ => {
            if remaining_expected_bars <= 1.5 {
                0.0
            } else if remaining_expected_bars <= 3.0 {
                0.40
            } else if remaining_expected_bars <= 5.0 {
                0.70
            } else {
                1.0
            }
        }
    }
}

pub fn apply_regime_execution_guardrail(
    mut output: ExecutionTreeOutput,
    hybrid_regime: &RegimeSegmentationPacket,
) -> ExecutionTreeOutput {
    let high_transition_hazard = hybrid_regime.transition_hazard.unwrap_or_default() >= 0.60;
    let pda_disagreement = hybrid_regime
        .evidence
        .iter()
        .any(|line| line == "pda_hybrid_alignment=false");
    let low_remaining_duration = hybrid_regime
        .duration_remaining_expected_bars
        .unwrap_or(f64::INFINITY)
        <= 1.5;
    let short_remaining_duration = hybrid_regime
        .duration_remaining_expected_bars
        .unwrap_or(f64::INFINITY)
        <= 2.5;
    if high_transition_hazard || pda_disagreement || low_remaining_duration {
        output.gate_status = "observe".to_string();
        output.branch = "transition_guardrail".to_string();
        output.execution_bias = "guarded".to_string();
        output.branch_probability = output.branch_probability.min(0.50);
        output.posterior_uncertainty = output.posterior_uncertainty.max(0.60);
        output.decision_hint = if low_remaining_duration {
            "execution_guarded_due_to_low_remaining_regime_duration".to_string()
        } else if pda_disagreement {
            "execution_guarded_due_to_pda_hybrid_disagreement".to_string()
        } else {
            "execution_guarded_due_to_high_transition_hazard".to_string()
        };
        output.split_reason_lineage.push(format!(
            "hybrid_transition_hazard={:.3}",
            hybrid_regime.transition_hazard.unwrap_or_default()
        ));
        if pda_disagreement {
            output
                .split_reason_lineage
                .push("pda_hybrid_alignment=false".to_string());
        }
        if low_remaining_duration || short_remaining_duration {
            output.split_reason_lineage.push(format!(
                "duration_remaining_expected_bars={:.3}",
                hybrid_regime
                    .duration_remaining_expected_bars
                    .unwrap_or_default()
            ));
        }
    } else if short_remaining_duration {
        output.execution_bias = "passive".to_string();
        output.split_reason_lineage.push(format!(
            "duration_remaining_expected_bars={:.3} → execution_bias=passive",
            hybrid_regime
                .duration_remaining_expected_bars
                .unwrap_or_default()
        ));
    }
    output
}
