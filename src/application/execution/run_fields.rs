use crate::domain::execution::classify_execution_gate;

#[derive(Debug, Clone, Default)]
pub struct ExecutionPhaseFields {
    pub execution_edge_share: Option<f64>,
    pub prediction_edge_share: Option<f64>,
    pub execution_readiness: Option<f64>,
    pub execution_gate_status: Option<String>,
}

pub fn build_execution_phase_fields(
    execution_edge_share: f64,
    prediction_edge_share: f64,
    execution_readiness: f64,
) -> ExecutionPhaseFields {
    ExecutionPhaseFields {
        execution_edge_share: Some(execution_edge_share),
        prediction_edge_share: Some(prediction_edge_share),
        execution_readiness: Some(execution_readiness),
        execution_gate_status: Some(classify_execution_gate(execution_readiness).to_string()),
    }
}

pub fn derive_research_execution_fields(
    comparable: bool,
    approved: bool,
    should_rollback: bool,
    feedback_records_applied: usize,
    aggregate_return: f64,
    has_best_factor: bool,
) -> ExecutionPhaseFields {
    let execution_edge_share = if has_best_factor {
        if approved {
            0.62
        } else {
            0.54
        }
    } else {
        45.0 / 100.0
    };
    let prediction_edge_share = (1.0_f64 - execution_edge_share).clamp(0.0, 1.0);
    let execution_readiness = (comparable as u8 as f64 * 0.20
        + approved as u8 as f64 * 0.30
        + should_rollback as u8 as f64 * -0.20
        + feedback_records_applied.min(5) as f64 * 0.05
        + if aggregate_return > 0.0 { 0.20 } else { 0.05 })
    .clamp(0.0, 1.0);
    build_execution_phase_fields(
        execution_edge_share,
        prediction_edge_share,
        execution_readiness,
    )
}

pub fn derive_backtest_execution_fields(
    trade_count: usize,
    total_return: f64,
    regime_break_penalty: f64,
    approved: bool,
) -> ExecutionPhaseFields {
    let execution_edge_share = if trade_count > 0 { 0.58 } else { 45.0 / 100.0 };
    let prediction_edge_share = (1.0_f64 - execution_edge_share).clamp(0.0, 1.0);
    let execution_readiness = (if total_return > 0.0 { 0.30 } else { 0.10 }
        + (trade_count.min(20) as f64 / 20.0) * 0.30
        + (1.0 - regime_break_penalty.clamp(0.0, 1.0)) * 0.20
        + approved as u8 as f64 * 0.20)
        .clamp(0.0, 1.0);
    build_execution_phase_fields(
        execution_edge_share,
        prediction_edge_share,
        execution_readiness,
    )
}

pub fn derive_update_execution_fields(
    feedback_records_applied: usize,
    realized_outcome: &str,
    duplicate_feedback_skipped: bool,
    approved: bool,
) -> ExecutionPhaseFields {
    let execution_edge_share = if feedback_records_applied > 0 {
        0.60
    } else {
        0.50
    };
    let prediction_edge_share = (1.0_f64 - execution_edge_share).clamp(0.0, 1.0);
    let execution_readiness = (if realized_outcome == "win" {
        0.35
    } else {
        0.15
    } + feedback_records_applied.min(5) as f64 * 0.07
        + (!duplicate_feedback_skipped) as u8 as f64 * 0.15
        + approved as u8 as f64 * 0.15)
        .clamp(0.0, 1.0);
    build_execution_phase_fields(
        execution_edge_share,
        prediction_edge_share,
        execution_readiness,
    )
}
