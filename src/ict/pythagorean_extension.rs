use serde::{Deserialize, Serialize};

use crate::math::geometry::{orthogonal_distance, segment_length, Point2};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct PythagoreanExtensionMetrics {
    pub trendline_distance: f64,
    pub orthogonal_extension: f64,
    pub normalized_overstretch: f64,
}

pub fn measure_pythagorean_extension(
    anchor_a: Point2,
    anchor_b: Point2,
    current: Point2,
) -> PythagoreanExtensionMetrics {
    let base = segment_length(anchor_a, anchor_b);
    let orthogonal_extension = orthogonal_distance(anchor_a, anchor_b, current);
    let trendline_distance = segment_length(anchor_b, current);
    let normalized_overstretch = if base <= f64::EPSILON {
        0.0
    } else {
        (orthogonal_extension / base).clamp(0.0, 1.0)
    };

    PythagoreanExtensionMetrics {
        trendline_distance,
        orthogonal_extension,
        normalized_overstretch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_pythagorean_extension_handles_aligned_points() {
        let metrics = measure_pythagorean_extension(
            Point2 { x: 0.0, y: 0.0 },
            Point2 { x: 10.0, y: 0.0 },
            Point2 { x: 12.0, y: 0.0 },
        );
        assert!(metrics.orthogonal_extension <= 1e-9);
        assert!(metrics.normalized_overstretch <= 1e-9);
        assert!((metrics.trendline_distance - 2.0).abs() < 1e-9);
    }

    #[test]
    fn measure_pythagorean_extension_handles_right_angle_extension() {
        let metrics = measure_pythagorean_extension(
            Point2 { x: 0.0, y: 0.0 },
            Point2 { x: 10.0, y: 0.0 },
            Point2 { x: 10.0, y: 5.0 },
        );
        assert!((metrics.orthogonal_extension - 5.0).abs() < 1e-9);
        assert!((metrics.normalized_overstretch - 0.5).abs() < 1e-9);
    }

    #[test]
    fn measure_pythagorean_extension_clamps_overstretch_for_extreme_dislocations() {
        let metrics = measure_pythagorean_extension(
            Point2 { x: 0.0, y: 0.0 },
            Point2 { x: 2.0, y: 0.0 },
            Point2 { x: 2.0, y: 20.0 },
        );
        assert_eq!(metrics.normalized_overstretch, 1.0);
    }
}
