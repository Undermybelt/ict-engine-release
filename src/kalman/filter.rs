use crate::types::{Direction, KalmanParams, KalmanState};
use ndarray::{Array1, Array2};

/// Standard Kalman Filter for price denoising
/// State: [price, velocity]
/// Observation: [price]
pub struct KalmanFilter {
    pub params: KalmanParams,
    pub state: KalmanState,
}

impl KalmanFilter {
    /// Initialize Kalman Filter
    pub fn new(
        initial_price: f64,
        process_noise_price: f64,
        process_noise_vel: f64,
        measurement_noise: f64,
    ) -> Self {
        // State transition matrix F
        let f = Array2::from_shape_vec((2, 2), vec![1.0, 1.0, 0.0, 1.0]).unwrap();

        // Observation matrix H
        let h = Array2::from_shape_vec((1, 2), vec![1.0, 0.0]).unwrap();

        // Process noise covariance Q
        let q = Array2::from_shape_vec(
            (2, 2),
            vec![process_noise_price, 0.0, 0.0, process_noise_vel],
        )
        .unwrap();

        // Measurement noise covariance R
        let r = Array2::from_shape_vec((1, 1), vec![measurement_noise]).unwrap();

        // Initial state
        let x = Array1::from_vec(vec![initial_price, 0.0]);
        let p = Array2::from_shape_vec((2, 2), vec![1.0, 0.0, 0.0, 1.0]).unwrap();

        Self {
            params: KalmanParams { f, h, q, r },
            state: KalmanState { x, p },
        }
    }

    /// Predict step
    /// x_pred = F * x
    /// P_pred = F * P * F^T + Q
    pub fn predict(&mut self) {
        // x_pred = F * x
        self.state.x = self.params.f.dot(&self.state.x);

        // P_pred = F * P * F^T + Q
        let fp = self.params.f.dot(&self.state.p);
        let fpf = fp.dot(&self.params.f.t());
        self.state.p = fpf + &self.params.q;
    }

    /// Update step with measurement
    /// K = P_pred * H^T * (H * P_pred * H^T + R)^{-1}
    /// x = x_pred + K * (z - H * x_pred)
    /// P = (I - K * H) * P_pred
    pub fn update(&mut self, measurement: f64) {
        let z = Array1::from_vec(vec![measurement]);

        // Innovation: y = z - H * x_pred
        let hx = self.params.h.dot(&self.state.x);
        let y = &z - &hx;

        // Innovation covariance: S = H * P_pred * H^T + R
        let hp = self.params.h.dot(&self.state.p);
        let hph = hp.dot(&self.params.h.t());
        let s = hph + &self.params.r;

        // Kalman gain: K = P_pred * H^T * S^{-1}
        let pht = self.state.p.dot(&self.params.h.t());
        let s_inv = self.pseudo_inverse(&s);
        let k = pht.dot(&s_inv);

        // State update: x = x_pred + K * y
        self.state.x = &self.state.x + k.dot(&y);

        // Covariance update: P = (I - K * H) * P_pred
        let kh = k.dot(&self.params.h);
        let i = Array2::<f64>::eye(2);
        let ikh = &i - &kh;
        self.state.p = ikh.dot(&self.state.p);
    }

    /// Complete step: predict + update
    /// Returns (denoised_price, velocity, uncertainty)
    pub fn step(&mut self, measurement: f64) -> (f64, f64, f64) {
        self.predict();
        self.update(measurement);

        let denoised_price = self.state.x[0];
        let velocity = self.state.x[1];
        let uncertainty = self.state.p[[0, 0]].sqrt();

        (denoised_price, velocity, uncertainty)
    }

    /// Smooth entire price series
    pub fn smooth_series(&mut self, prices: &[f64]) -> Vec<(f64, f64, f64)> {
        let mut results = Vec::with_capacity(prices.len());

        for &price in prices {
            results.push(self.step(price));
        }

        results
    }

    /// Get current trend direction
    pub fn trend_direction(&self) -> Direction {
        let velocity = self.state.x[1];
        if velocity > 0.0 {
            Direction::Bull
        } else if velocity < 0.0 {
            Direction::Bear
        } else {
            Direction::Neutral
        }
    }

    /// Get current denoised price
    pub fn current_price(&self) -> f64 {
        self.state.x[0]
    }

    /// Get current velocity
    pub fn current_velocity(&self) -> f64 {
        self.state.x[1]
    }

    /// Pseudo-inverse for 1x1 matrix (simplified)
    pub(crate) fn pseudo_inverse(&self, m: &Array2<f64>) -> Array2<f64> {
        if m.shape() == [1, 1] {
            let val = if m[[0, 0]].abs() > 1e-10 {
                1.0 / m[[0, 0]]
            } else {
                0.0
            };
            Array2::from_shape_vec((1, 1), vec![val]).unwrap()
        } else {
            // For larger matrices, use a more sophisticated approach
            // This is a simplified version
            let det = m[[0, 0]] * m[[1, 1]] - m[[0, 1]] * m[[1, 0]];
            if det.abs() < 1e-10 {
                Array2::zeros(m.raw_dim())
            } else {
                let inv_det = 1.0 / det;
                Array2::from_shape_vec(
                    (2, 2),
                    vec![
                        m[[1, 1]] * inv_det,
                        -m[[0, 1]] * inv_det,
                        -m[[1, 0]] * inv_det,
                        m[[0, 0]] * inv_det,
                    ],
                )
                .unwrap()
            }
        }
    }
}
