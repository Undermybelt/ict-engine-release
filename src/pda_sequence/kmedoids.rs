use anyhow::Result;

/// Outcome of `pam_cluster`. `labels[i]` is the cluster assignment for the
/// i-th sample; `medoids[c]` is the sample index that represents cluster `c`;
/// `silhouette` is the average silhouette score over all samples
/// (range: -1.0 .. 1.0, higher is better-separated).
#[derive(Debug, Clone, PartialEq)]
pub struct PamOutcome {
    pub labels: Vec<usize>,
    pub medoids: Vec<usize>,
    pub silhouette: f64,
}

const MAX_ITERATIONS: usize = 32;

/// Partitioning Around Medoids on a pre-computed square symmetric distance
/// matrix. Deterministic: initial medoids are the k samples with the
/// lowest total distance to all other samples (greedy seed), which is
/// reproducible and avoids needing an RNG.
///
/// Errors if `k == 0`, `k > n`, the matrix is not square, or the algorithm
/// cannot make progress on the first iteration (degenerate input).
pub fn pam_cluster(distance_matrix: &[Vec<f64>], k: usize) -> Result<PamOutcome> {
    let n = distance_matrix.len();
    if k == 0 {
        anyhow::bail!("k must be > 0");
    }
    if k > n {
        anyhow::bail!("k ({k}) must not exceed sample count ({n})");
    }
    for (i, row) in distance_matrix.iter().enumerate() {
        if row.len() != n {
            anyhow::bail!(
                "distance matrix must be square; row {i} has length {}",
                row.len()
            );
        }
    }

    // Greedy deterministic initialization.
    let mut medoids = initial_medoids(distance_matrix, k);
    let mut labels = assign_labels(distance_matrix, &medoids);

    for _ in 0..MAX_ITERATIONS {
        let new_medoids = update_medoids(distance_matrix, &labels, k);
        if new_medoids == medoids {
            break;
        }
        medoids = new_medoids;
        labels = assign_labels(distance_matrix, &medoids);
    }

    let silhouette = silhouette_score(distance_matrix, &labels, k);
    Ok(PamOutcome {
        labels,
        medoids,
        silhouette,
    })
}

fn initial_medoids(distance_matrix: &[Vec<f64>], k: usize) -> Vec<usize> {
    let n = distance_matrix.len();
    if n == 0 || k == 0 {
        return Vec::new();
    }
    // First medoid: most central (lowest total distance). Ties broken by
    // smallest index so the procedure is deterministic.
    let first_medoid = (0..n)
        .min_by(|a, b| {
            let sa: f64 = distance_matrix[*a].iter().copied().sum();
            let sb: f64 = distance_matrix[*b].iter().copied().sum();
            sa.partial_cmp(&sb)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.cmp(b))
        })
        .unwrap_or(0);

    let mut medoids = vec![first_medoid];
    // Subsequent medoids: k-medoids++-style farthest-point seeding —
    // pick the candidate whose minimum distance to any existing medoid is
    // largest. Deterministic: ties broken by smallest index.
    while medoids.len() < k {
        let next = (0..n).filter(|i| !medoids.contains(i)).max_by(|a, b| {
            let da = medoids
                .iter()
                .map(|m| distance_matrix[*a][*m])
                .fold(f64::INFINITY, f64::min);
            let db = medoids
                .iter()
                .map(|m| distance_matrix[*b][*m])
                .fold(f64::INFINITY, f64::min);
            da.partial_cmp(&db)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.cmp(a)) // prefer the smaller index on ties
        });
        match next {
            Some(idx) => medoids.push(idx),
            None => break,
        }
    }
    medoids
}

fn assign_labels(distance_matrix: &[Vec<f64>], medoids: &[usize]) -> Vec<usize> {
    (0..distance_matrix.len())
        .map(|sample| {
            medoids
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    distance_matrix[sample][**a]
                        .partial_cmp(&distance_matrix[sample][**b])
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(cluster_idx, _)| cluster_idx)
                .unwrap_or(0)
        })
        .collect()
}

fn update_medoids(distance_matrix: &[Vec<f64>], labels: &[usize], k: usize) -> Vec<usize> {
    let n = distance_matrix.len();
    let mut new_medoids = vec![0usize; k];
    for (cluster_idx, medoid_slot) in new_medoids.iter_mut().enumerate().take(k) {
        let members: Vec<usize> = (0..n).filter(|i| labels[*i] == cluster_idx).collect();
        if members.is_empty() {
            // No reassignment possible — keep whatever the assignment step picked;
            // fall back to an arbitrary in-range index so downstream code stays sane.
            *medoid_slot = cluster_idx.min(n.saturating_sub(1));
            continue;
        }
        let best = members
            .iter()
            .copied()
            .min_by(|a, b| {
                let total_a: f64 = members
                    .iter()
                    .map(|other| distance_matrix[*a][*other])
                    .sum();
                let total_b: f64 = members
                    .iter()
                    .map(|other| distance_matrix[*b][*other])
                    .sum();
                total_a
                    .partial_cmp(&total_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        *medoid_slot = best;
    }
    new_medoids
}

fn silhouette_score(distance_matrix: &[Vec<f64>], labels: &[usize], k: usize) -> f64 {
    let n = distance_matrix.len();
    if k <= 1 || n <= 1 {
        return 0.0;
    }
    let mut total = 0.0;
    let mut count = 0usize;
    for i in 0..n {
        let own_cluster = labels[i];
        let own_members: Vec<usize> = (0..n)
            .filter(|j| *j != i && labels[*j] == own_cluster)
            .collect();
        if own_members.is_empty() {
            // Singleton cluster — silhouette is undefined; skip.
            continue;
        }
        let a: f64 = own_members
            .iter()
            .map(|j| distance_matrix[i][*j])
            .sum::<f64>()
            / own_members.len() as f64;

        let mut b = f64::INFINITY;
        for other_cluster in 0..k {
            if other_cluster == own_cluster {
                continue;
            }
            let members: Vec<usize> = (0..n).filter(|j| labels[*j] == other_cluster).collect();
            if members.is_empty() {
                continue;
            }
            let mean: f64 =
                members.iter().map(|j| distance_matrix[i][*j]).sum::<f64>() / members.len() as f64;
            if mean < b {
                b = mean;
            }
        }
        if !b.is_finite() {
            continue;
        }
        let denom = a.max(b);
        if denom > 0.0 {
            total += (b - a) / denom;
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn four_point_matrix() -> Vec<Vec<f64>> {
        // Two tight pairs: {0,1} and {2,3}; inter-cluster distance 5.0.
        vec![
            vec![0.0, 0.1, 5.0, 5.1],
            vec![0.1, 0.0, 5.1, 5.0],
            vec![5.0, 5.1, 0.0, 0.1],
            vec![5.1, 5.0, 0.1, 0.0],
        ]
    }

    #[test]
    fn rejects_zero_k() {
        assert!(pam_cluster(&four_point_matrix(), 0).is_err());
    }

    #[test]
    fn rejects_k_greater_than_n() {
        assert!(pam_cluster(&four_point_matrix(), 5).is_err());
    }

    #[test]
    fn separates_two_tight_clusters() {
        let outcome = pam_cluster(&four_point_matrix(), 2).unwrap();
        assert_eq!(outcome.medoids.len(), 2);
        assert_eq!(outcome.labels.len(), 4);
        // 0 & 1 must share a cluster; 2 & 3 must share a cluster.
        assert_eq!(outcome.labels[0], outcome.labels[1]);
        assert_eq!(outcome.labels[2], outcome.labels[3]);
        assert_ne!(outcome.labels[0], outcome.labels[2]);
        assert!(
            outcome.silhouette > 0.9,
            "tight clusters should yield high silhouette, got {}",
            outcome.silhouette
        );
    }

    #[test]
    fn k_equals_n_assigns_each_to_own_cluster() {
        let outcome = pam_cluster(&four_point_matrix(), 4).unwrap();
        let mut sorted = outcome.labels.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3]);
    }

    #[test]
    fn deterministic_across_reruns() {
        let m = four_point_matrix();
        let a = pam_cluster(&m, 2).unwrap();
        let b = pam_cluster(&m, 2).unwrap();
        assert_eq!(a, b);
    }
}
