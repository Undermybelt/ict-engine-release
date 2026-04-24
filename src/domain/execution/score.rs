#[derive(Debug, Clone, Copy, Default)]
pub struct ExecutionEdgeSplit {
    pub execution_edge_share: f64,
    pub prediction_edge_share: f64,
}

pub fn execution_edge_split(execution_score: f64, prediction_score: f64) -> ExecutionEdgeSplit {
    let execution = execution_score.max(0.0);
    let prediction = prediction_score.max(0.0);
    let total = (execution + prediction).max(f64::EPSILON);
    ExecutionEdgeSplit {
        execution_edge_share: execution / total,
        prediction_edge_share: prediction / total,
    }
}

pub fn execution_readiness(
    execution_score: f64,
    evidence_quality: f64,
    overextension_distance: Option<f64>,
    reversion_speed: Option<f64>,
) -> f64 {
    let mut score = execution_score.max(0.0) * 0.50 + evidence_quality.max(0.0) * 0.30;
    if let Some(speed) = reversion_speed {
        score += speed.clamp(0.0, 1.0) * 0.20;
    }
    if let Some(distance) = overextension_distance {
        score -= distance.abs().min(1.0) * 0.20;
    }
    score.clamp(0.0, 1.0)
}
