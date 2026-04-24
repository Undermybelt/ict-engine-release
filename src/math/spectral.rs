use std::f64::consts::PI;

#[derive(Debug, Clone, Copy)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };

    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn norm_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    pub fn norm(self) -> f64 {
        self.norm_sq().sqrt()
    }

    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    fn add(self, other: Self) -> Self {
        Self {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }

    fn mul(self, other: Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
}

fn next_power_of_two(n: usize) -> usize {
    if n <= 1 {
        return 1;
    }
    let mut p = 1;
    while p < n {
        p <<= 1;
    }
    p
}

// In-place radix-2 Cooley-Tukey FFT. Requires `buffer.len()` to be a power of 2.
// Keeps the codebase dependency-free; at the window sizes ict-engine feeds in
// (<= 1024 samples), the O(n log n) cost is negligible and the iterative
// implementation avoids the stack depth of the recursive form.
fn fft_radix2_in_place(buffer: &mut [Complex]) {
    let n = buffer.len();
    if n <= 1 {
        return;
    }
    debug_assert!(
        n.is_power_of_two(),
        "fft_radix2_in_place requires power-of-2 length"
    );

    // Bit-reversal permutation.
    let bits = n.trailing_zeros();
    for i in 0..n {
        let mut j = 0usize;
        for bit in 0..bits {
            if (i >> bit) & 1 == 1 {
                j |= 1 << (bits - 1 - bit);
            }
        }
        if j > i {
            buffer.swap(i, j);
        }
    }

    let mut size = 2;
    while size <= n {
        let half = size / 2;
        let angle_step = -2.0 * PI / size as f64;
        let w_step = Complex::new(angle_step.cos(), angle_step.sin());
        let mut chunk_start = 0;
        while chunk_start < n {
            let mut w = Complex::new(1.0, 0.0);
            for k in 0..half {
                let t = w.mul(buffer[chunk_start + k + half]);
                let u = buffer[chunk_start + k];
                buffer[chunk_start + k] = u.add(t);
                buffer[chunk_start + k + half] = u.sub(t);
                w = w.mul(w_step);
            }
            chunk_start += size;
        }
        size <<= 1;
    }
}

// One-sided real FFT. Input is demeaned and zero-padded to the next power of 2
// so the estimator is not distorted by a DC bias. Output has `padded/2 + 1`
// complex bins (index 0 = DC, index padded/2 = Nyquist).
pub fn rfft_one_sided(samples: &[f64]) -> Vec<Complex> {
    if samples.is_empty() {
        return Vec::new();
    }
    let padded_len = next_power_of_two(samples.len());
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let mut buffer: Vec<Complex> = (0..padded_len)
        .map(|i| {
            if i < samples.len() {
                Complex::new(samples[i] - mean, 0.0)
            } else {
                Complex::ZERO
            }
        })
        .collect();
    fft_radix2_in_place(&mut buffer);
    buffer.truncate(padded_len / 2 + 1);
    buffer
}

// Soft-thresholding: sign(x) * max(|x| - lambda, 0). The Well's AFNO uses this
// in the frequency domain to prune low-energy bins; here we apply it to each
// bin's magnitude while preserving phase so downstream phase-alignment reads
// remain meaningful on surviving bins.
pub fn softshrink_bins(bins: &mut [Complex], lambda: f64) {
    if lambda <= 0.0 {
        return;
    }
    for bin in bins.iter_mut() {
        let magnitude = bin.norm();
        if magnitude <= lambda {
            *bin = Complex::ZERO;
        } else {
            let scale = (magnitude - lambda) / magnitude;
            bin.re *= scale;
            bin.im *= scale;
        }
    }
}

pub fn power_spectrum(bins: &[Complex]) -> Vec<f64> {
    bins.iter().map(|bin| bin.norm_sq()).collect()
}

#[derive(Debug, Clone, Copy)]
pub struct DominantMode {
    pub bin_index: usize,
    pub energy: f64,
    pub period_bars: f64,
    pub phase: f64,
}

// Pick the largest non-DC bin. `padded_len` is the FFT input length (power of
// 2) needed to recover period_bars; without it the caller would need to
// replicate the padding math.
pub fn dominant_mode(bins: &[Complex], padded_len: usize) -> Option<DominantMode> {
    if bins.len() <= 1 || padded_len == 0 {
        return None;
    }
    let mut best_index = 0usize;
    let mut best_energy = -1.0_f64;
    for (index, bin) in bins.iter().enumerate().skip(1) {
        let energy = bin.norm_sq();
        if energy > best_energy {
            best_energy = energy;
            best_index = index;
        }
    }
    if best_energy <= 0.0 {
        return None;
    }
    Some(DominantMode {
        bin_index: best_index,
        energy: best_energy,
        period_bars: padded_len as f64 / best_index as f64,
        phase: bins[best_index].arg(),
    })
}

// Shannon entropy of the normalized power spectrum, normalized to [0, 1]
// against the uniform spectrum upper bound log(n_bins). 0 = single-mode,
// 1 = white (useless for rhythmic execution).
pub fn normalized_spectral_entropy(bins: &[Complex]) -> f64 {
    if bins.len() <= 1 {
        return 0.0;
    }
    let energies: Vec<f64> = bins.iter().skip(1).map(|bin| bin.norm_sq()).collect();
    let total: f64 = energies.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let n = energies.len() as f64;
    let max_entropy = n.ln();
    if max_entropy <= 0.0 {
        return 0.0;
    }
    let mut entropy = 0.0_f64;
    for energy in energies {
        if energy <= 0.0 {
            continue;
        }
        let p = energy / total;
        entropy -= p * p.ln();
    }
    (entropy / max_entropy).clamp(0.0, 1.0)
}

// Fraction of non-DC energy concentrated in the dominant bin — tells us how
// "rhythmic" the series is. 1.0 = pure sinusoid, ~1/n = white noise.
pub fn dominant_energy_ratio(bins: &[Complex]) -> f64 {
    if bins.len() <= 1 {
        return 0.0;
    }
    let total: f64 = bins.iter().skip(1).map(|bin| bin.norm_sq()).sum();
    if total <= 0.0 {
        return 0.0;
    }
    let dominant_energy = bins
        .iter()
        .skip(1)
        .map(|bin| bin.norm_sq())
        .fold(0.0_f64, f64::max);
    dominant_energy / total
}

// Cosine of the phase the dominant mode would have at the end of the window.
// Value in [-1, 1]. +1 = dominant mode's "peak phase" aligned with the most
// recent sample (expect near-term mean reversion); -1 = anti-aligned.
pub fn dominant_phase_alignment(
    dominant: DominantMode,
    samples_len: usize,
    padded_len: usize,
) -> f64 {
    if samples_len == 0 || padded_len == 0 {
        return 0.0;
    }
    let t_last = (samples_len as f64 - 1.0).max(0.0);
    let angle = 2.0 * PI * dominant.bin_index as f64 * t_last / padded_len as f64 + dominant.phase;
    angle.cos().clamp(-1.0, 1.0)
}

// Energy retained after a softshrink-style threshold normalized against total.
// `high_freq_noise_ratio = 1 - (retained / total)` gives "what fraction we
// pruned as noise." Returns 0 when total is zero.
pub fn high_frequency_noise_ratio(bins: &[Complex], pruned_bins: &[Complex]) -> f64 {
    let total: f64 = bins.iter().map(|bin| bin.norm_sq()).sum();
    if total <= 0.0 {
        return 0.0;
    }
    let retained: f64 = pruned_bins.iter().map(|bin| bin.norm_sq()).sum();
    (1.0 - retained / total).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn pure_sine_collapses_spectrum_to_one_bin() {
        // 8 periods of a unit sine over 128 samples → FFT should place almost
        // all energy at bin 8. Dominant ratio near 1.0, entropy near 0.
        let n = 128usize;
        let cycles = 8.0_f64;
        let samples: Vec<f64> = (0..n)
            .map(|i| (2.0 * PI * cycles * i as f64 / n as f64).sin())
            .collect();
        let bins = rfft_one_sided(&samples);
        let padded = next_power_of_two(n);
        let ratio = dominant_energy_ratio(&bins);
        let entropy = normalized_spectral_entropy(&bins);
        let dominant = dominant_mode(&bins, padded).expect("sine has a dominant mode");

        assert!(ratio > 0.95, "dominant_energy_ratio={ratio}");
        assert!(entropy < 0.15, "entropy={entropy}");
        assert_eq!(dominant.bin_index, 8);
        assert!(
            approx_eq(dominant.period_bars, 16.0, 1e-6),
            "period={}",
            dominant.period_bars
        );
    }

    #[test]
    fn white_noise_has_high_entropy_and_low_dominance() {
        // Deterministic LCG noise → expect roughly uniform spectrum.
        let n = 256usize;
        let mut state: u64 = 0xD1B54A32D192ED03;
        let samples: Vec<f64> = (0..n)
            .map(|_| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((state >> 33) as f64) / (u32::MAX as f64) * 2.0 - 1.0
            })
            .collect();
        let bins = rfft_one_sided(&samples);
        let ratio = dominant_energy_ratio(&bins);
        let entropy = normalized_spectral_entropy(&bins);
        assert!(entropy > 0.75, "entropy={entropy}");
        assert!(ratio < 0.2, "dominant_energy_ratio={ratio}");
    }

    #[test]
    fn softshrink_zeroes_small_bins_and_shrinks_larger_ones() {
        let mut bins = vec![
            Complex::new(0.05, 0.0),
            Complex::new(0.3, 0.4), // magnitude 0.5
            Complex::new(0.0, 0.01),
            Complex::new(-1.0, 0.0),
        ];
        softshrink_bins(&mut bins, 0.1);
        assert_eq!(bins[0].norm(), 0.0);
        assert_eq!(bins[2].norm(), 0.0);
        assert!(approx_eq(bins[1].norm(), 0.4, 1e-9));
        assert!(approx_eq(bins[3].norm(), 0.9, 1e-9));
    }

    #[test]
    fn high_frequency_noise_ratio_matches_softshrink_reduction() {
        let samples: Vec<f64> = (0..64)
            .map(|i| {
                (2.0 * PI * i as f64 / 16.0).sin() * 1.0 + (2.0 * PI * i as f64 / 4.0).sin() * 0.1
            })
            .collect();
        let full = rfft_one_sided(&samples);
        let mut pruned = full.clone();
        softshrink_bins(&mut pruned, 0.5);
        let noise_ratio = high_frequency_noise_ratio(&full, &pruned);
        assert!(noise_ratio > 0.0 && noise_ratio < 1.0);
    }

    #[test]
    fn dominant_phase_alignment_stays_in_unit_range() {
        let samples: Vec<f64> = (0..64)
            .map(|i| (2.0 * PI * i as f64 / 16.0).sin())
            .collect();
        let bins = rfft_one_sided(&samples);
        let padded = next_power_of_two(samples.len());
        let dominant = dominant_mode(&bins, padded).expect("sine has dominant mode");
        let alignment = dominant_phase_alignment(dominant, samples.len(), padded);
        assert!((-1.0..=1.0).contains(&alignment));
    }

    #[test]
    fn empty_input_returns_empty_spectrum() {
        let bins = rfft_one_sided(&[]);
        assert!(bins.is_empty());
        assert_eq!(dominant_energy_ratio(&bins), 0.0);
        assert_eq!(normalized_spectral_entropy(&bins), 0.0);
        assert!(dominant_mode(&bins, 0).is_none());
    }

    #[test]
    fn constant_series_has_zero_ac_energy() {
        let samples = vec![100.0_f64; 64];
        let bins = rfft_one_sided(&samples);
        // All non-DC bins should be numerically zero after demeaning.
        assert!(dominant_energy_ratio(&bins) <= 1e-10);
        assert_eq!(normalized_spectral_entropy(&bins), 0.0);
    }
}
