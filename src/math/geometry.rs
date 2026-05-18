#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

pub fn segment_length(a: Point2, b: Point2) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

pub fn orthogonal_distance(anchor_a: Point2, anchor_b: Point2, point: Point2) -> f64 {
    let base = segment_length(anchor_a, anchor_b);
    if base <= f64::EPSILON {
        return segment_length(anchor_a, point);
    }

    let cross = ((anchor_b.x - anchor_a.x) * (anchor_a.y - point.y)
        - (anchor_a.x - point.x) * (anchor_b.y - anchor_a.y))
        .abs();
    cross / base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_length_matches_euclidean_distance() {
        let a = Point2 { x: 0.0, y: 0.0 };
        let b = Point2 { x: 3.0, y: 4.0 };
        assert!((segment_length(a, b) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn orthogonal_distance_handles_aligned_points() {
        let a = Point2 { x: 0.0, y: 0.0 };
        let b = Point2 { x: 10.0, y: 0.0 };
        let point = Point2 { x: 4.0, y: 0.0 };
        assert!(orthogonal_distance(a, b, point) <= 1e-9);
    }

    #[test]
    fn orthogonal_distance_handles_right_angle_projection() {
        let a = Point2 { x: 0.0, y: 0.0 };
        let b = Point2 { x: 10.0, y: 0.0 };
        let point = Point2 { x: 5.0, y: 6.0 };
        assert!((orthogonal_distance(a, b, point) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn orthogonal_distance_degrades_to_point_distance_for_degenerate_segment() {
        let a = Point2 { x: 2.0, y: 3.0 };
        let point = Point2 { x: 5.0, y: 7.0 };
        assert!((orthogonal_distance(a, a, point) - 5.0).abs() < 1e-9);
    }
}
