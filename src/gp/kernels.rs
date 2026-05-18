use crate::types::Kernel;

/// RBF (Radial Basis Function) Kernel
pub struct RBFKernel {
    pub length_scale: f64,
    pub variance: f64,
}

impl Kernel for RBFKernel {
    fn eval(&self, x1: f64, x2: f64) -> f64 {
        let d = x1 - x2;
        self.variance * (-0.5 * d * d / (self.length_scale * self.length_scale)).exp()
    }
}

/// Matérn Kernel
pub struct MaternKernel {
    pub length_scale: f64,
    pub variance: f64,
    pub nu: f64,
}

impl Kernel for MaternKernel {
    fn eval(&self, x1: f64, x2: f64) -> f64 {
        let r = (x1 - x2).abs();
        let s3r = (3.0_f64).sqrt() * r / self.length_scale;

        match self.nu {
            0.5 => self.variance * (-r / self.length_scale).exp(),
            1.5 => self.variance * (1.0 + s3r) * (-s3r).exp(),
            2.5 => {
                let s5r = (5.0_f64).sqrt() * r / self.length_scale;
                self.variance
                    * (1.0 + s5r + 5.0 * r * r / (3.0 * self.length_scale * self.length_scale))
                    * (-s5r).exp()
            }
            _ => self.variance * (-0.5 * r * r / (self.length_scale * self.length_scale)).exp(),
        }
    }
}
