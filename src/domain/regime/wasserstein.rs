use anyhow::Result;
use std::collections::BTreeMap;

pub const HYBRID_REGIME_LABELS: [&str; 4] =
    ["trend_impulse", "trend_decay", "range_calm", "range_choppy"];

#[derive(Debug, Clone)]
pub struct WassersteinClassification {
    pub label: String,
    pub distance: f64,
    pub membership: BTreeMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct WassersteinClassifier {
    prototypes: Vec<(&'static str, Vec<f64>)>,
}

impl Default for WassersteinClassifier {
    fn default() -> Self {
        Self {
            prototypes: vec![
                ("trend_impulse", vec![0.85, 0.75, 0.70, 0.80]),
                ("trend_decay", vec![0.55, 0.35, 0.60, 0.45]),
                ("range_calm", vec![0.10, 0.15, 0.12, 0.08]),
                ("range_choppy", vec![0.30, 0.75, 0.25, 0.70]),
            ],
        }
    }
}

pub fn wasserstein_1d(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    let mut left = a.to_vec();
    let mut right = b.to_vec();
    left.sort_by(f64::total_cmp);
    right.sort_by(f64::total_cmp);
    Some(
        left.iter()
            .zip(right.iter())
            .map(|(x, y)| (x - y).abs())
            .sum::<f64>()
            / left.len() as f64,
    )
}

impl WassersteinClassifier {
    pub fn classify(&self, features: &[f64]) -> Result<WassersteinClassification> {
        if features.is_empty() {
            anyhow::bail!("feature vector must not be empty");
        }
        let mut distances = Vec::with_capacity(self.prototypes.len());
        for (label, prototype) in &self.prototypes {
            let distance = wasserstein_1d(features, prototype).ok_or_else(|| {
                anyhow::anyhow!(
                    "feature length {} does not match prototype length {}",
                    features.len(),
                    prototype.len()
                )
            })?;
            distances.push(((*label).to_string(), distance));
        }

        let mut membership = BTreeMap::new();
        let denom: f64 = distances
            .iter()
            .map(|(_, distance)| 1.0 / (distance + 1e-6))
            .sum();
        for (label, distance) in &distances {
            membership.insert(label.clone(), (1.0 / (distance + 1e-6)) / denom.max(1e-12));
        }

        let (label, distance) = distances
            .into_iter()
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .ok_or_else(|| anyhow::anyhow!("missing Wasserstein prototypes"))?;

        Ok(WassersteinClassification {
            label,
            distance,
            membership,
        })
    }
}
