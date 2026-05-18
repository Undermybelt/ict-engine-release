use crate::types::RegimeProbs;

pub fn native_frame_weight(interval: &str) -> f64 {
    match interval {
        "1d" => 0.24,
        "4h" => 0.20,
        "1h" => 0.18,
        "15m" => 0.16,
        "5m" => 0.12,
        "1m" => 0.10,
        _ => 0.10,
    }
}

pub fn weighted_regime_probs(signals: &[(RegimeProbs, f64)]) -> RegimeProbs {
    let total_weight = signals
        .iter()
        .map(|(_, weight)| *weight)
        .sum::<f64>()
        .max(f64::EPSILON);
    RegimeProbs {
        accumulation: signals
            .iter()
            .map(|(regime_probs, weight)| regime_probs.accumulation * weight)
            .sum::<f64>()
            / total_weight,
        manipulation_expansion: signals
            .iter()
            .map(|(regime_probs, weight)| regime_probs.manipulation_expansion * weight)
            .sum::<f64>()
            / total_weight,
        distribution: signals
            .iter()
            .map(|(regime_probs, weight)| regime_probs.distribution * weight)
            .sum::<f64>()
            / total_weight,
    }
}

pub fn weighted_majority_label<'a, I>(
    labels: I,
    positive: &str,
    negative: &str,
    neutral: &str,
) -> String
where
    I: IntoIterator<Item = (&'a str, f64)>,
{
    let mut positive_weight = 0.0;
    let mut negative_weight = 0.0;
    let mut neutral_weight = 0.0;
    for (label, weight) in labels {
        match label {
            value if value == positive => positive_weight += weight,
            value if value == negative => negative_weight += weight,
            _ => neutral_weight += weight,
        }
    }
    if positive_weight > negative_weight && positive_weight >= neutral_weight {
        positive.to_string()
    } else if negative_weight > positive_weight && negative_weight >= neutral_weight {
        negative.to_string()
    } else {
        neutral.to_string()
    }
}
