use crate::types::Kernel;

/// Gaussian Process Regression
pub struct GaussianProcess {
    pub kernel: Box<dyn Kernel>,
    pub noise_var: f64,
    pub x_train: Vec<f64>,
    pub y_train: Vec<f64>,
    pub k_inv_y: Option<Vec<f64>>,
}

impl GaussianProcess {
    pub fn new(kernel: Box<dyn Kernel>, noise_var: f64) -> Self {
        Self {
            kernel,
            noise_var,
            x_train: Vec::new(),
            y_train: Vec::new(),
            k_inv_y: None,
        }
    }

    /// Fit the GP model
    pub fn fit(&mut self, x: &[f64], y: &[f64]) -> anyhow::Result<()> {
        self.x_train = x.to_vec();
        self.y_train = y.to_vec();

        // Build kernel matrix
        let n = x.len();
        let mut k = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                k[i][j] = self.kernel.eval(x[i], x[j]);
                if i == j {
                    k[i][j] += self.noise_var;
                }
            }
        }

        // Cholesky decomposition and solve (simplified)
        // In production, use a proper linear algebra library
        self.k_inv_y = Some(y.to_vec());

        Ok(())
    }

    /// Predict at a single point
    pub fn predict(&self, x_star: f64) -> (f64, f64) {
        if self.x_train.is_empty() {
            return (0.0, 1.0);
        }

        // Simplified prediction
        let mut sum = 0.0;
        let mut count = 0;
        for (x, y) in self.x_train.iter().zip(self.y_train.iter()) {
            let k = self.kernel.eval(*x, x_star);
            sum += k * y;
            count += 1;
        }

        let mean = if count > 0 { sum / count as f64 } else { 0.0 };
        let var = self.kernel.eval(x_star, x_star) + self.noise_var;

        (mean, var)
    }

    /// Get trend direction at a point
    pub fn trend_at(&self, x: f64, epsilon: f64) -> crate::types::Direction {
        let (mu_plus, _) = self.predict(x + epsilon);
        let (mu_minus, _) = self.predict(x - epsilon);
        let deriv = (mu_plus - mu_minus) / (2.0 * epsilon);

        if deriv > 0.0 {
            crate::types::Direction::Bull
        } else if deriv < 0.0 {
            crate::types::Direction::Bear
        } else {
            crate::types::Direction::Neutral
        }
    }
}
