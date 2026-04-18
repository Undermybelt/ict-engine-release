/// IC (Information Coefficient) calculator
pub struct ICCalculator;

impl ICCalculator {
    /// Calculate mean IC between factor values and returns.
    pub fn calculate(factor_values: &[f64], returns: &[f64]) -> f64 {
        Self::rolling_ic(factor_values, returns, factor_values.len())
            .into_iter()
            .next()
            .unwrap_or(0.0)
    }

    pub fn rolling_ic(factor_values: &[f64], returns: &[f64], window: usize) -> Vec<f64> {
        if factor_values.len() != returns.len() || factor_values.len() < 2 {
            return Vec::new();
        }

        let bounded_window = window.clamp(2, factor_values.len());
        if bounded_window == factor_values.len() {
            return vec![Self::spearman(factor_values, returns)];
        }

        (bounded_window..=factor_values.len())
            .map(|end| {
                Self::spearman(
                    &factor_values[end - bounded_window..end],
                    &returns[end - bounded_window..end],
                )
            })
            .collect()
    }

    pub fn ir(ic_values: &[f64]) -> (f64, f64, f64) {
        if ic_values.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let mean = ic_values.iter().sum::<f64>() / ic_values.len() as f64;
        let variance = ic_values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / ic_values.len() as f64;
        let std = variance.sqrt();
        let ir = if std > 0.0 { mean / std } else { 0.0 };
        (mean, std, ir)
    }

    fn spearman(x: &[f64], y: &[f64]) -> f64 {
        if x.len() != y.len() || x.len() < 2 {
            return 0.0;
        }

        let x_rank = average_ranks(x);
        let y_rank = average_ranks(y);
        pearson(&x_rank, &y_rank)
    }
}

fn average_ranks(values: &[f64]) -> Vec<f64> {
    let mut indexed = values.iter().copied().enumerate().collect::<Vec<_>>();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut ranks = vec![0.0; values.len()];
    let mut start = 0usize;
    while start < indexed.len() {
        let mut end = start + 1;
        while end < indexed.len() && (indexed[end].1 - indexed[start].1).abs() <= 1e-12 {
            end += 1;
        }
        let average_rank = (start + end - 1) as f64 / 2.0;
        for idx in start..end {
            ranks[indexed[idx].0] = average_rank;
        }
        start = end;
    }
    ranks
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let x_mean = x.iter().sum::<f64>() / n;
    let y_mean = y.iter().sum::<f64>() / n;

    let mut xy = 0.0;
    let mut xx = 0.0;
    let mut yy = 0.0;
    for index in 0..x.len() {
        let dx = x[index] - x_mean;
        let dy = y[index] - y_mean;
        xy += dx * dy;
        xx += dx * dx;
        yy += dy * dy;
    }

    let denom = (xx * yy).sqrt();
    if denom > 0.0 {
        xy / denom
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ic_and_ir() {
        let factors = [1.0, 2.0, 3.0, 4.0, 5.0];
        let returns = [0.01, 0.02, 0.03, 0.05, 0.08];
        let rolling = ICCalculator::rolling_ic(&factors, &returns, 3);
        let (mean, _, ir) = ICCalculator::ir(&rolling);

        assert!(!rolling.is_empty());
        assert!(mean > 0.0);
        assert!(ir >= 0.0);
    }
}
