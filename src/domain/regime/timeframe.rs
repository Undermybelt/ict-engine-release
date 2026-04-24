#[derive(Debug, Clone)]
pub struct TimeframeAlignment {
    pub aligned: bool,
    pub score: f64,
    pub evidence: Vec<String>,
}

pub fn regime_direction(label: &str) -> &'static str {
    match label {
        "trend_impulse" | "trend_decay" => "trend",
        "range_calm" | "range_choppy" => "range",
        _ => "unknown",
    }
}

pub fn timeframe_alignment(higher: &str, lower: &str) -> TimeframeAlignment {
    let higher_direction = regime_direction(higher);
    let lower_direction = regime_direction(lower);
    let aligned = higher_direction == lower_direction && higher_direction != "unknown";
    TimeframeAlignment {
        aligned,
        score: if aligned { 1.0 } else { 0.0 },
        evidence: vec![
            format!("higher={higher}"),
            format!("lower={lower}"),
            format!("higher_direction={higher_direction}"),
            format!("lower_direction={lower_direction}"),
        ],
    }
}
