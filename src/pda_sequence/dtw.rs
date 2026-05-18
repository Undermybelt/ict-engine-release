use super::token::{pda_token_cost, PdaToken};

/// Outcome of `dtw_distance` / `dtw_alignment`: the optimal DTW score plus
/// the alignment path, expressed as `(index_in_a, index_in_b)` pairs
/// ordered from start to end. Path is always non-empty for non-empty inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct DtwAlignment {
    pub distance: f64,
    pub path: Vec<(usize, usize)>,
}

/// Classic DTW: min-cost monotonic alignment under the three-move step
/// pattern (match / skip-a / skip-b). Time O(m * n), space O(m * n) —
/// fine for PDA sequences (usually < 50 tokens per session).
///
/// Returns `None` when either input is empty because an empty sequence
/// has no defined alignment; callers should handle empty inputs as a
/// caller-side data-quality issue rather than a cluster-level edge case.
pub fn dtw_alignment<F>(a: &[PdaToken], b: &[PdaToken], cost: F) -> Option<DtwAlignment>
where
    F: Fn(&PdaToken, &PdaToken) -> f64,
{
    if a.is_empty() || b.is_empty() {
        return None;
    }

    let m = a.len();
    let n = b.len();
    let mut cost_matrix = vec![vec![f64::INFINITY; n + 1]; m + 1];
    cost_matrix[0][0] = 0.0;

    for i in 1..=m {
        for j in 1..=n {
            let step_cost = cost(&a[i - 1], &b[j - 1]);
            let prev = cost_matrix[i - 1][j - 1]
                .min(cost_matrix[i - 1][j])
                .min(cost_matrix[i][j - 1]);
            cost_matrix[i][j] = step_cost + prev;
        }
    }

    let distance = cost_matrix[m][n];

    // Backtrack path from (m, n) to (1, 1). Emit 0-indexed (i-1, j-1) pairs.
    let mut path = Vec::with_capacity(m + n);
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        path.push((i - 1, j - 1));
        if i == 1 && j == 1 {
            break;
        }
        let diag = cost_matrix[i - 1][j - 1];
        let up = cost_matrix[i - 1][j];
        let left = cost_matrix[i][j - 1];
        if diag <= up && diag <= left {
            i -= 1;
            j -= 1;
        } else if up <= left {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    path.reverse();

    Some(DtwAlignment { distance, path })
}

/// Shortcut when only the scalar distance is needed.
pub fn dtw_distance(a: &[PdaToken], b: &[PdaToken]) -> Option<f64> {
    dtw_alignment(a, b, pda_token_cost).map(|alignment| alignment.distance)
}

/// Pairwise DTW distance matrix for a list of sequences, indexed as
/// `matrix[i][j] = dtw_distance(sequences[i], sequences[j])`. Diagonal is 0.
/// Symmetric since the default cost is symmetric.
pub fn dtw_distance_matrix(sequences: &[Vec<PdaToken>]) -> Vec<Vec<f64>> {
    let n = sequences.len();
    let mut matrix = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d = dtw_distance(&sequences[i], &sequences[j]).unwrap_or(f64::INFINITY);
            matrix[i][j] = d;
            matrix[j][i] = d;
        }
    }
    matrix
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pda_sequence::token::PdaTokenKind;

    fn tok(kind: PdaTokenKind, bar: usize) -> PdaToken {
        PdaToken::new(kind, bar)
    }

    #[test]
    fn identical_sequences_have_zero_distance() {
        let a = vec![
            tok(PdaTokenKind::FairValueGap, 0),
            tok(PdaTokenKind::OrderBlock, 1),
            tok(PdaTokenKind::LiquiditySweep, 2),
        ];
        let d = dtw_distance(&a, &a).unwrap();
        assert!(d.abs() < 1e-9, "identity must be zero, got {d}");
    }

    #[test]
    fn empty_sequence_returns_none() {
        let a: Vec<PdaToken> = vec![];
        let b = vec![tok(PdaTokenKind::OrderBlock, 0)];
        assert!(dtw_distance(&a, &b).is_none());
        assert!(dtw_distance(&b, &a).is_none());
    }

    #[test]
    fn different_kinds_increase_distance() {
        let a = vec![
            tok(PdaTokenKind::FairValueGap, 0),
            tok(PdaTokenKind::OrderBlock, 1),
        ];
        let b = vec![
            tok(PdaTokenKind::LiquiditySweep, 0),
            tok(PdaTokenKind::StructureBreak, 1),
        ];
        let d_same = dtw_distance(&a, &a).unwrap();
        let d_diff = dtw_distance(&a, &b).unwrap();
        assert!(d_diff > d_same, "mismatched kinds must exceed identity");
    }

    #[test]
    fn missing_token_warping_costs_less_than_full_mismatch() {
        // Sequence a: FVG -> OB -> Sweep
        // Sequence b: FVG -> Sweep   (missing OB — should warp with one stretch)
        let a = vec![
            tok(PdaTokenKind::FairValueGap, 0),
            tok(PdaTokenKind::OrderBlock, 1),
            tok(PdaTokenKind::LiquiditySweep, 2),
        ];
        let b = vec![
            tok(PdaTokenKind::FairValueGap, 0),
            tok(PdaTokenKind::LiquiditySweep, 1),
        ];
        let d_warp = dtw_distance(&a, &b).unwrap();

        // A fully-mismatched pair as reference upper bound.
        let c = vec![
            tok(PdaTokenKind::Cisd, 0),
            tok(PdaTokenKind::RejectionBlock, 1),
            tok(PdaTokenKind::PropulsionBlock, 2),
        ];
        let d_full_mismatch = dtw_distance(&a, &c).unwrap();
        assert!(
            d_warp < d_full_mismatch,
            "warped alignment ({d_warp}) must beat full mismatch ({d_full_mismatch})"
        );
    }

    #[test]
    fn alignment_path_endpoints_are_anchored() {
        let a = vec![
            tok(PdaTokenKind::FairValueGap, 0),
            tok(PdaTokenKind::OrderBlock, 1),
            tok(PdaTokenKind::LiquiditySweep, 2),
        ];
        let b = vec![
            tok(PdaTokenKind::FairValueGap, 0),
            tok(PdaTokenKind::LiquiditySweep, 1),
        ];
        let alignment = dtw_alignment(&a, &b, pda_token_cost).unwrap();
        assert_eq!(alignment.path.first().copied(), Some((0, 0)));
        assert_eq!(
            alignment.path.last().copied(),
            Some((a.len() - 1, b.len() - 1))
        );
    }

    #[test]
    fn distance_matrix_is_symmetric_with_zero_diagonal() {
        let seqs = vec![
            vec![
                tok(PdaTokenKind::FairValueGap, 0),
                tok(PdaTokenKind::OrderBlock, 1),
            ],
            vec![
                tok(PdaTokenKind::LiquiditySweep, 0),
                tok(PdaTokenKind::Cisd, 1),
            ],
            vec![
                tok(PdaTokenKind::FairValueGap, 0),
                tok(PdaTokenKind::Cisd, 1),
            ],
        ];
        let m = dtw_distance_matrix(&seqs);
        for (i, row) in m.iter().enumerate().take(seqs.len()) {
            assert!(row[i].abs() < 1e-9, "diagonal must be zero at {i}");
            for (j, value) in row.iter().enumerate().take(seqs.len()).skip(i + 1) {
                assert!((*value - m[j][i]).abs() < 1e-9, "asymmetric at ({i},{j})");
            }
        }
    }
}
