//! Tucker factor tensor decomposition via HOSVD.
//!
//! Input: factor × regime × timeframe tensor `T` of shape `[nf, nr, nt]`.
//! Output: core tensor `G` of shape `[rf, rr, rt]` + three loading matrices
//! (`U_f`, `U_r`, `U_t`). Reconstruction is `T ≈ G ×_1 U_f ×_2 U_r ×_3 U_t`.
//!
//! Implementation is HOSVD (Higher-Order SVD / Tucker-1): unfold the tensor
//! along each mode, compute the Gram matrix, run symmetric Jacobi
//! eigendecomposition, and keep the top-k eigenvectors as the mode's loading
//! matrix. Core tensor is the tensor multiplied by each loading's transpose.
//!
//! We deliberately avoid `ndarray-linalg` (which pulls in BLAS/LAPACK) — our
//! tensors are small (factors ≤ 20, regimes ≤ 6, timeframes ≤ 5), so pure
//! Rust Jacobi is plenty fast and keeps the dependency surface flat.

use ndarray::{Array2, Array3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuckerCore {
    pub core: Vec<Vec<Vec<f64>>>,
    pub factor_loadings: Vec<Vec<f64>>,
    pub regime_loadings: Vec<Vec<f64>>,
    pub timeframe_loadings: Vec<Vec<f64>>,
    pub reconstruction_error: f64,
    pub rank_triplet: (usize, usize, usize),
    pub input_shape: (usize, usize, usize),
}

/// Fit a Tucker core via HOSVD. Returns None when any requested rank is zero
/// or exceeds the corresponding input dimension. Reconstruction error is the
/// Frobenius norm of `T - T̂` over the Frobenius norm of `T`, so it is
/// dimensionless and comparable across runs.
pub fn fit_tucker_core(tensor: &Array3<f64>, ranks: (usize, usize, usize)) -> Option<TuckerCore> {
    let shape = tensor.dim();
    let (nf, nr, nt) = shape;
    let (rf, rr, rt) = ranks;
    if rf == 0 || rr == 0 || rt == 0 {
        return None;
    }
    if rf > nf || rr > nr || rt > nt {
        return None;
    }
    if nf == 0 || nr == 0 || nt == 0 {
        return None;
    }

    let u_f = mode_n_left_singular_vectors(tensor, 0, rf);
    let u_r = mode_n_left_singular_vectors(tensor, 1, rr);
    let u_t = mode_n_left_singular_vectors(tensor, 2, rt);

    let core = mode_product_all(tensor, &u_f, &u_r, &u_t);
    let reconstructed = reconstruct_from_core(&core, &u_f, &u_r, &u_t, shape);

    let frob_tensor = tensor.iter().map(|v| v * v).sum::<f64>().sqrt();
    let diff_frob = tensor
        .iter()
        .zip(reconstructed.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt();
    let reconstruction_error = if frob_tensor > 0.0 {
        diff_frob / frob_tensor
    } else {
        0.0
    };

    Some(TuckerCore {
        core: array3_to_nested(&core),
        factor_loadings: array2_to_nested(&u_f),
        regime_loadings: array2_to_nested(&u_r),
        timeframe_loadings: array2_to_nested(&u_t),
        reconstruction_error,
        rank_triplet: ranks,
        input_shape: shape,
    })
}

fn array3_to_nested(tensor: &Array3<f64>) -> Vec<Vec<Vec<f64>>> {
    let (a, b, c) = tensor.dim();
    (0..a)
        .map(|i| {
            (0..b)
                .map(|j| (0..c).map(|k| tensor[[i, j, k]]).collect())
                .collect()
        })
        .collect()
}

fn array2_to_nested(mat: &Array2<f64>) -> Vec<Vec<f64>> {
    let (rows, cols) = mat.dim();
    (0..rows)
        .map(|i| (0..cols).map(|j| mat[[i, j]]).collect())
        .collect()
}

// Unfold a 3-tensor along mode `n` into a 2D matrix. The standard Kolda
// unfolding: row index = mode-n index; column index = enumerates all other
// modes in fastest-to-slowest order.
fn unfold(tensor: &Array3<f64>, mode: usize) -> Array2<f64> {
    let (nf, nr, nt) = tensor.dim();
    match mode {
        0 => {
            let mut out = Array2::<f64>::zeros((nf, nr * nt));
            for i in 0..nf {
                for j in 0..nr {
                    for k in 0..nt {
                        out[[i, j + k * nr]] = tensor[[i, j, k]];
                    }
                }
            }
            out
        }
        1 => {
            let mut out = Array2::<f64>::zeros((nr, nf * nt));
            for j in 0..nr {
                for i in 0..nf {
                    for k in 0..nt {
                        out[[j, i + k * nf]] = tensor[[i, j, k]];
                    }
                }
            }
            out
        }
        2 => {
            let mut out = Array2::<f64>::zeros((nt, nf * nr));
            for k in 0..nt {
                for i in 0..nf {
                    for j in 0..nr {
                        out[[k, i + j * nf]] = tensor[[i, j, k]];
                    }
                }
            }
            out
        }
        _ => unreachable!("3-tensor has modes 0/1/2 only"),
    }
}

// Top-`rank` left singular vectors of the mode-n unfolding. Computed via
// eigendecomposition of M * M^T — valid because left singular vectors are
// the eigenvectors of the Gram matrix, with singular values = sqrt(eigen).
fn mode_n_left_singular_vectors(tensor: &Array3<f64>, mode: usize, rank: usize) -> Array2<f64> {
    let unfolded = unfold(tensor, mode);
    let gram = unfolded.dot(&unfolded.t());
    let (eigenvalues, eigenvectors) = jacobi_eigendecomposition(&gram);
    let mut order: Vec<usize> = (0..eigenvalues.len()).collect();
    order.sort_by(|a, b| {
        eigenvalues[*b]
            .partial_cmp(&eigenvalues[*a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let rows = gram.dim().0;
    let mut loadings = Array2::<f64>::zeros((rows, rank));
    for (col, idx) in order.iter().take(rank).enumerate() {
        for row in 0..rows {
            loadings[[row, col]] = eigenvectors[[row, *idx]];
        }
    }
    loadings
}

// Mode product T ×_n U^T along each of the three modes. Returns the core
// tensor of shape `(rf, rr, rt)`.
fn mode_product_all(
    tensor: &Array3<f64>,
    u_f: &Array2<f64>,
    u_r: &Array2<f64>,
    u_t: &Array2<f64>,
) -> Array3<f64> {
    let (nf, nr, nt) = tensor.dim();
    let rf = u_f.dim().1;
    let rr = u_r.dim().1;
    let rt = u_t.dim().1;

    // Stage 1: mode-0 product, shape (rf, nr, nt)
    let mut stage1 = Array3::<f64>::zeros((rf, nr, nt));
    for a in 0..rf {
        for j in 0..nr {
            for k in 0..nt {
                let mut acc = 0.0;
                for i in 0..nf {
                    acc += u_f[[i, a]] * tensor[[i, j, k]];
                }
                stage1[[a, j, k]] = acc;
            }
        }
    }

    // Stage 2: mode-1 product, shape (rf, rr, nt)
    let mut stage2 = Array3::<f64>::zeros((rf, rr, nt));
    for a in 0..rf {
        for b in 0..rr {
            for k in 0..nt {
                let mut acc = 0.0;
                for j in 0..nr {
                    acc += u_r[[j, b]] * stage1[[a, j, k]];
                }
                stage2[[a, b, k]] = acc;
            }
        }
    }

    // Stage 3: mode-2 product, shape (rf, rr, rt)
    let mut core = Array3::<f64>::zeros((rf, rr, rt));
    for a in 0..rf {
        for b in 0..rr {
            for c in 0..rt {
                let mut acc = 0.0;
                for k in 0..nt {
                    acc += u_t[[k, c]] * stage2[[a, b, k]];
                }
                core[[a, b, c]] = acc;
            }
        }
    }

    core
}

// Reconstruct T̂ = G ×_1 U_f ×_2 U_r ×_3 U_t (no transposes; loadings are
// column-orthogonal so this is the forward Tucker composition).
fn reconstruct_from_core(
    core: &Array3<f64>,
    u_f: &Array2<f64>,
    u_r: &Array2<f64>,
    u_t: &Array2<f64>,
    shape: (usize, usize, usize),
) -> Array3<f64> {
    let (nf, nr, nt) = shape;
    let (rf, rr, rt) = core.dim();

    // Reverse of mode_product_all: multiply core by loadings (not transposes).
    let mut stage1 = Array3::<f64>::zeros((nf, rr, rt));
    for i in 0..nf {
        for b in 0..rr {
            for c in 0..rt {
                let mut acc = 0.0;
                for a in 0..rf {
                    acc += u_f[[i, a]] * core[[a, b, c]];
                }
                stage1[[i, b, c]] = acc;
            }
        }
    }

    let mut stage2 = Array3::<f64>::zeros((nf, nr, rt));
    for i in 0..nf {
        for j in 0..nr {
            for c in 0..rt {
                let mut acc = 0.0;
                for b in 0..rr {
                    acc += u_r[[j, b]] * stage1[[i, b, c]];
                }
                stage2[[i, j, c]] = acc;
            }
        }
    }

    let mut out = Array3::<f64>::zeros((nf, nr, nt));
    for i in 0..nf {
        for j in 0..nr {
            for k in 0..nt {
                let mut acc = 0.0;
                for c in 0..rt {
                    acc += u_t[[k, c]] * stage2[[i, j, c]];
                }
                out[[i, j, k]] = acc;
            }
        }
    }
    out
}

// Symmetric Jacobi eigendecomposition. Returns `(eigenvalues, eigenvectors)`
// where each column of `eigenvectors` is the eigenvector for the matching
// eigenvalue. Matrix is assumed symmetric; nonsymmetric inputs are silently
// symmetrized by the caller (the Gram matrix is symmetric by construction).
fn jacobi_eigendecomposition(matrix: &Array2<f64>) -> (Vec<f64>, Array2<f64>) {
    let n = matrix.dim().0;
    debug_assert_eq!(matrix.dim().0, matrix.dim().1);

    let mut a = matrix.clone();
    let mut v = Array2::<f64>::eye(n);
    const MAX_SWEEPS: usize = 50;
    const TOL: f64 = 1e-12;

    for _ in 0..MAX_SWEEPS {
        let mut off = 0.0_f64;
        for p in 0..n {
            for q in (p + 1)..n {
                off += a[[p, q]].abs();
            }
        }
        if off < TOL {
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                let apq = a[[p, q]];
                if apq.abs() < TOL {
                    continue;
                }
                let app = a[[p, p]];
                let aqq = a[[q, q]];
                let theta = (aqq - app) / (2.0 * apq);
                let t = if theta >= 0.0 {
                    1.0 / (theta + (1.0 + theta * theta).sqrt())
                } else {
                    1.0 / (theta - (1.0 + theta * theta).sqrt())
                };
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = t * c;
                a[[p, p]] = app - t * apq;
                a[[q, q]] = aqq + t * apq;
                a[[p, q]] = 0.0;
                a[[q, p]] = 0.0;
                for r in 0..n {
                    if r != p && r != q {
                        let arp = a[[r, p]];
                        let arq = a[[r, q]];
                        a[[r, p]] = c * arp - s * arq;
                        a[[p, r]] = a[[r, p]];
                        a[[r, q]] = s * arp + c * arq;
                        a[[q, r]] = a[[r, q]];
                    }
                    let vrp = v[[r, p]];
                    let vrq = v[[r, q]];
                    v[[r, p]] = c * vrp - s * vrq;
                    v[[r, q]] = s * vrp + c * vrq;
                }
            }
        }
    }

    let eigenvalues: Vec<f64> = (0..n).map(|i| a[[i, i]]).collect();
    (eigenvalues, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rank_one_tensor(nf: usize, nr: usize, nt: usize) -> Array3<f64> {
        let mut tensor = Array3::<f64>::zeros((nf, nr, nt));
        // Outer product of three vectors — tensor is guaranteed rank-1.
        let u: Vec<f64> = (0..nf).map(|i| (i + 1) as f64).collect();
        let v: Vec<f64> = (0..nr).map(|j| ((j + 1) as f64) * 0.5).collect();
        let w: Vec<f64> = (0..nt).map(|k| ((k + 1) as f64) * 0.25).collect();
        for i in 0..nf {
            for j in 0..nr {
                for k in 0..nt {
                    tensor[[i, j, k]] = u[i] * v[j] * w[k];
                }
            }
        }
        tensor
    }

    #[test]
    fn rank_one_tensor_reconstructs_exactly() {
        let tensor = rank_one_tensor(4, 3, 2);
        let core = fit_tucker_core(&tensor, (1, 1, 1)).expect("rank-1 fit");
        assert!(
            core.reconstruction_error < 1e-10,
            "error={}",
            core.reconstruction_error
        );
    }

    #[test]
    fn full_rank_reconstructs_exactly() {
        let tensor = rank_one_tensor(3, 3, 3)
            + Array3::<f64>::from_shape_fn((3, 3, 3), |(i, j, k)| (i + 2 * j + 3 * k) as f64);
        let core = fit_tucker_core(&tensor, (3, 3, 3)).expect("full-rank fit");
        assert!(
            core.reconstruction_error < 1e-8,
            "error={}",
            core.reconstruction_error
        );
    }

    #[test]
    fn rejects_zero_rank() {
        let tensor = rank_one_tensor(2, 2, 2);
        assert!(fit_tucker_core(&tensor, (0, 1, 1)).is_none());
        assert!(fit_tucker_core(&tensor, (1, 0, 1)).is_none());
        assert!(fit_tucker_core(&tensor, (1, 1, 0)).is_none());
    }

    #[test]
    fn rejects_rank_exceeding_dimension() {
        let tensor = rank_one_tensor(2, 2, 2);
        assert!(fit_tucker_core(&tensor, (3, 1, 1)).is_none());
    }

    #[test]
    fn core_has_requested_shape() {
        let tensor = rank_one_tensor(5, 4, 3);
        let core = fit_tucker_core(&tensor, (2, 2, 2)).expect("fit");
        assert_eq!(core.rank_triplet, (2, 2, 2));
        assert_eq!(core.core.len(), 2);
        assert_eq!(core.core[0].len(), 2);
        assert_eq!(core.core[0][0].len(), 2);
        assert_eq!(core.factor_loadings.len(), 5);
        assert_eq!(core.factor_loadings[0].len(), 2);
        assert_eq!(core.regime_loadings.len(), 4);
        assert_eq!(core.timeframe_loadings.len(), 3);
    }

    #[test]
    fn jacobi_recovers_eigenvalues_of_diagonal() {
        let diag = Array2::<f64>::from_diag(&ndarray::arr1(&[3.0, 1.0, 2.0]));
        let (eigenvalues, _) = jacobi_eigendecomposition(&diag);
        let mut sorted: Vec<f64> = eigenvalues.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((sorted[0] - 1.0).abs() < 1e-10);
        assert!((sorted[1] - 2.0).abs() < 1e-10);
        assert!((sorted[2] - 3.0).abs() < 1e-10);
    }
}
