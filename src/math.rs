pub fn wilder_smooth(values: &[f64], period: usize) -> Vec<f64> {
    if values.len() < period {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(values.len());

    let first: f64 = values[..period].iter().sum();
    result.push(first / period as f64);

    for value in values.iter().skip(period) {
        let prev = result.last().unwrap();
        let smoothed = (prev * (period - 1) as f64 + *value) / period as f64;
        result.push(smoothed);
    }

    result
}

pub fn sma(values: &[f64], period: usize) -> Vec<f64> {
    if values.len() < period {
        return Vec::new();
    }

    values
        .windows(period)
        .map(|w| w.iter().sum::<f64>() / period as f64)
        .collect()
}

pub fn ema(values: &[f64], period: usize) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }

    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut result = Vec::with_capacity(values.len());

    if values.len() >= period {
        let first_sma: f64 = values[..period].iter().sum::<f64>() / period as f64;
        result.push(first_sma);

        for value in values.iter().skip(period) {
            let previous = *result.last().unwrap();
            let next = (*value - previous) * multiplier + previous;
            result.push(next);
        }
    }

    result
}

pub fn std_dev(values: &[f64], mean: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let variance: f64 =
        values.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}
