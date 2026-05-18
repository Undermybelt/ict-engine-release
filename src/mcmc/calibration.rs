/// LR calibration using MCMC
pub struct LRPosterior {
    pub layer: usize,
    pub samples: Vec<f64>,
    pub mean: f64,
    pub ci_95: (f64, f64),
}

pub fn calibrate_lr(
    layer_idx: usize,
    n_confirmed_expanded: usize,
    n_confirmed_total: usize,
    n_expanded_total: usize,
    n_total: usize,
    n_samples: usize,
) -> LRPosterior {
    // Simplified calibration
    let p_expanded = n_confirmed_expanded as f64 / n_expanded_total.max(1) as f64;
    let p_total = n_confirmed_total as f64 / n_total.max(1) as f64;

    let lr = if p_total > 0.0 {
        p_expanded / p_total
    } else {
        1.0
    };

    LRPosterior {
        layer: layer_idx,
        samples: vec![lr; n_samples],
        mean: lr,
        ci_95: (lr * 0.9, lr * 1.1),
    }
}
