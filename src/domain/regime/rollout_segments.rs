//! Rollout evaluation in short/medium/long segments.
//!
//! The Well's VRMSE methodology evaluates rollout loss on discrete windows —
//! (6:12), (13:30) — so that short- vs medium-horizon behaviour can diverge
//! without hiding in a single aggregate. We port that split to MECE recovery:
//! the accuracy over bars `[0, 30%)`, `[30%, 80%)`, `[80%, 100%]` and the
//! slope of `execution_readiness` across each window are tracked
//! independently. A single "good" aggregate accuracy no longer passes the
//! gate if any segment collapses.

use serde::{Deserialize, Serialize};

/// Lower bound for the short-horizon segment. Keeps parity with the main
/// `MECE_RECOVERY_ACCURACY_GATE` so short-horizon recovery has to clear the
/// same bar as the aggregate.
pub const MECE_SEGMENT_SHORT_FLOOR: f64 = 0.95;

/// Lower bound for the medium-horizon segment. Relaxed from short because
/// the search is smaller over mid-range windows; still strict enough that
/// regime recovery degradation is caught early.
pub const MECE_SEGMENT_MEDIUM_FLOOR: f64 = 0.85;

/// Lower bound for the long-horizon segment. Represents the worst-case
/// tolerable decay — below this the recovery surface is unreliable for
/// trailing execution plans.
pub const MECE_SEGMENT_LONG_FLOOR: f64 = 0.75;

/// Maximum allowed negative drift of `execution_readiness` per bar within a
/// segment. A more negative slope means readiness is collapsing faster than
/// the gate tolerates.
pub const MECE_SEGMENT_DRIFT_FLOOR: f64 = -0.03;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RolloutSegment {
    pub horizon_bars: (usize, usize),
    pub accuracy: f64,
    pub execution_readiness_mean: f64,
    /// Slope of execution_readiness over the segment (per bar). Negative =
    /// readiness decaying within the window. Zero when window has < 2 bars.
    pub execution_readiness_drift: f64,
    pub sample_count: usize,
}

pub fn compute_rollout_segments<F>(
    segments_bounds: &[(usize, usize)],
    predicted_correct: &[bool],
    readiness_per_bar: &[f64],
    mut label_samples: F,
) -> Vec<RolloutSegment>
where
    F: FnMut(&(usize, usize)) -> (usize, usize),
{
    let mut out = Vec::with_capacity(segments_bounds.len());
    for bounds in segments_bounds {
        let (start, end) = label_samples(bounds);
        if end <= start || end > predicted_correct.len() {
            out.push(RolloutSegment {
                horizon_bars: *bounds,
                accuracy: 0.0,
                execution_readiness_mean: 0.0,
                execution_readiness_drift: 0.0,
                sample_count: 0,
            });
            continue;
        }
        let window = &predicted_correct[start..end];
        let readiness_window = &readiness_per_bar[start..end];
        let correct = window.iter().filter(|flag| **flag).count();
        let total = window.len();
        let accuracy = if total == 0 {
            0.0
        } else {
            correct as f64 / total as f64
        };
        let mean = readiness_window.iter().sum::<f64>() / total.max(1) as f64;
        let drift = linear_slope(readiness_window);
        out.push(RolloutSegment {
            horizon_bars: *bounds,
            accuracy,
            execution_readiness_mean: mean,
            execution_readiness_drift: drift,
            sample_count: total,
        });
    }
    out
}

/// Convenience wrapper: partition `[0, total)` into short / medium / long
/// windows at 30% / 80% cutoffs. Mirrors the Well's tiered evaluation but
/// scaled to arbitrary series length so tiny fixtures still exercise all
/// three segments.
pub fn default_segment_bounds(total: usize) -> Vec<(usize, usize)> {
    if total == 0 {
        return Vec::new();
    }
    let s1 = (total as f64 * 0.3).round() as usize;
    let s2 = (total as f64 * 0.8).round() as usize;
    let s1 = s1.max(1).min(total);
    let s2 = s2.max(s1 + 1).min(total);
    vec![(0, s1), (s1, s2), (s2, total)]
}

/// Hard-gate verdict: `"promote"` when every segment clears its floor and no
/// segment drifts below `MECE_SEGMENT_DRIFT_FLOOR`, else `"blocked"`.
/// Returned as `&'static str` to mirror the rest of the gate family.
pub fn classify_mece_recovery_segments_gate(segments: &[RolloutSegment]) -> &'static str {
    if segments.len() < 3 {
        return "blocked";
    }
    let floors = [
        MECE_SEGMENT_SHORT_FLOOR,
        MECE_SEGMENT_MEDIUM_FLOOR,
        MECE_SEGMENT_LONG_FLOOR,
    ];
    for (segment, floor) in segments.iter().zip(floors.iter()) {
        if segment.sample_count == 0 {
            return "blocked";
        }
        if segment.accuracy < *floor {
            return "blocked";
        }
        if segment.execution_readiness_drift < MECE_SEGMENT_DRIFT_FLOOR {
            return "blocked";
        }
    }
    "promote"
}

fn linear_slope(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let n = values.len() as f64;
    let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
    let sum_y: f64 = values.iter().sum();
    let mean_x = sum_x / n;
    let mean_y = sum_y / n;
    let mut num = 0.0;
    let mut den = 0.0;
    for (i, y) in values.iter().enumerate() {
        let dx = i as f64 - mean_x;
        num += dx * (y - mean_y);
        den += dx * dx;
    }
    if den > 0.0 {
        num / den
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bounds_partition_total_into_three_windows() {
        let bounds = default_segment_bounds(100);
        assert_eq!(bounds.len(), 3);
        assert_eq!(bounds[0].0, 0);
        assert_eq!(bounds[2].1, 100);
        assert!(bounds[0].1 <= bounds[1].0);
        assert!(bounds[1].1 <= bounds[2].0 + 1);
    }

    #[test]
    fn default_bounds_handle_tiny_series() {
        let bounds = default_segment_bounds(5);
        assert_eq!(bounds.len(), 3);
        assert_eq!(bounds[0].0, 0);
        assert_eq!(bounds.last().unwrap().1, 5);
    }

    #[test]
    fn linear_slope_detects_decay() {
        let decaying = vec![0.9, 0.7, 0.5, 0.3];
        assert!(linear_slope(&decaying) < 0.0);
        let rising = vec![0.1, 0.4, 0.8];
        assert!(linear_slope(&rising) > 0.0);
        assert_eq!(linear_slope(&[0.5]), 0.0);
    }

    #[test]
    fn segments_gate_promotes_when_all_floors_are_cleared() {
        let segments = vec![
            RolloutSegment {
                horizon_bars: (0, 30),
                accuracy: 0.97,
                execution_readiness_mean: 0.8,
                execution_readiness_drift: 0.0,
                sample_count: 30,
            },
            RolloutSegment {
                horizon_bars: (30, 80),
                accuracy: 0.88,
                execution_readiness_mean: 0.75,
                execution_readiness_drift: -0.01,
                sample_count: 50,
            },
            RolloutSegment {
                horizon_bars: (80, 100),
                accuracy: 0.80,
                execution_readiness_mean: 0.65,
                execution_readiness_drift: -0.02,
                sample_count: 20,
            },
        ];
        assert_eq!(classify_mece_recovery_segments_gate(&segments), "promote");
    }

    #[test]
    fn segments_gate_blocks_when_any_segment_misses_floor() {
        let segments = vec![
            RolloutSegment {
                accuracy: 0.97,
                execution_readiness_drift: 0.0,
                sample_count: 30,
                ..Default::default()
            },
            RolloutSegment {
                accuracy: 0.60, // below medium floor
                execution_readiness_drift: 0.0,
                sample_count: 50,
                ..Default::default()
            },
            RolloutSegment {
                accuracy: 0.80,
                execution_readiness_drift: 0.0,
                sample_count: 20,
                ..Default::default()
            },
        ];
        assert_eq!(classify_mece_recovery_segments_gate(&segments), "blocked");
    }

    #[test]
    fn segments_gate_blocks_when_drift_exceeds_floor() {
        let segments = vec![
            RolloutSegment {
                accuracy: 0.99,
                execution_readiness_drift: -0.05, // below drift floor
                sample_count: 30,
                ..Default::default()
            },
            RolloutSegment {
                accuracy: 0.90,
                execution_readiness_drift: 0.0,
                sample_count: 50,
                ..Default::default()
            },
            RolloutSegment {
                accuracy: 0.80,
                execution_readiness_drift: 0.0,
                sample_count: 20,
                ..Default::default()
            },
        ];
        assert_eq!(classify_mece_recovery_segments_gate(&segments), "blocked");
    }

    #[test]
    fn compute_rollout_segments_splits_and_measures() {
        let predicted: Vec<bool> = (0..10).map(|i| i < 8).collect();
        let readiness: Vec<f64> = (0..10).map(|i| 0.9 - (i as f64) * 0.05).collect();
        let bounds = default_segment_bounds(10);
        let segments = compute_rollout_segments(&bounds, &predicted, &readiness, |(start, end)| {
            (*start, *end)
        });
        assert_eq!(segments.len(), 3);
        for segment in &segments {
            assert!(segment.sample_count > 0);
        }
        assert!(segments[0].execution_readiness_drift < 0.0);
    }
}
