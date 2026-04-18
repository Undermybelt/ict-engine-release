use crate::types::KalmanParams;
use ndarray::{Array1, Array2};

/// Rauch-Tung-Striebel (RTS) Smoother
/// Performs backward smoothing on Kalman filter results
pub struct RTSSmoother;

impl RTSSmoother {
    /// Smooth the entire sequence
    /// Takes forward filter results and applies backward smoothing
    pub fn smooth(
        filtered_states: &[(Array1<f64>, Array2<f64>)],
        predicted_states: &[(Array1<f64>, Array2<f64>)],
        params: &KalmanParams,
    ) -> Vec<(Array1<f64>, Array2<f64>)> {
        if filtered_states.is_empty() {
            return Vec::new();
        }

        let t = filtered_states.len();
        let mut smoothed = Vec::with_capacity(t);

        // Initialize with the last filtered state
        smoothed.push(filtered_states[t - 1].clone());

        // Backward pass
        for t_idx in (0..t - 1).rev() {
            let (ref x_filt, ref p_filt) = filtered_states[t_idx];
            let (ref x_pred_next, ref p_pred_next) = predicted_states[t_idx + 1];
            let (ref x_smooth_next, ref p_smooth_next) = smoothed[t - 1 - t_idx];

            // G_t = P_filt[t] * F^T * (P_pred[t+1])^{-1}
            let g_t = Self::compute_gain(p_filt, &params.f, p_pred_next);

            // x_smooth[t] = x_filt[t] + G_t * (x_smooth[t+1] - x_pred[t+1])
            let innovation = x_smooth_next - x_pred_next;
            let x_smooth = x_filt + g_t.dot(&innovation);

            // P_smooth[t] = P_filt[t] + G_t * (P_smooth[t+1] - P_pred[t+1]) * G_t^T
            let p_innovation = p_smooth_next - p_pred_next;
            let gp = g_t.dot(&p_innovation);
            let gpgt = gp.dot(&g_t.t());
            let p_smooth = p_filt + &gpgt;

            smoothed.push((x_smooth, p_smooth));
        }

        // Reverse to get chronological order
        smoothed.reverse();
        smoothed
    }

    /// Compute smoothing gain
    fn compute_gain(
        p_filt: &Array2<f64>,
        f: &Array2<f64>,
        p_pred_next: &Array2<f64>,
    ) -> Array2<f64> {
        // G = P_filt * F^T * P_pred^{-1}
        let pft = p_filt.dot(&f.t());
        let p_pred_inv = Self::pseudo_inverse(p_pred_next);
        pft.dot(&p_pred_inv)
    }

    /// Simplified pseudo-inverse for 2x2 matrices
    fn pseudo_inverse(m: &Array2<f64>) -> Array2<f64> {
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
