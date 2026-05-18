//! Sprint 3 2.3 acceptance: HOSVD reconstruction + artifact persistence.

use ict_engine::factor_lab::{
    build_factor_tucker_core_artifact, fit_tucker_core, persist_factor_tucker_core_artifact,
    tucker_attribution_confidence_is_high, FACTOR_TUCKER_CORE_ARTIFACT_FILE,
    FACTOR_TUCKER_CORE_ARTIFACT_KIND,
};
use ict_engine::state::RunProvenance;
use ndarray::Array3;
use std::fs;
use tempfile::TempDir;

fn rank_one_tensor(nf: usize, nr: usize, nt: usize) -> Array3<f64> {
    let u: Vec<f64> = (0..nf).map(|i| (i + 1) as f64).collect();
    let v: Vec<f64> = (0..nr).map(|j| ((j + 1) as f64) * 0.5).collect();
    let w: Vec<f64> = (0..nt).map(|k| ((k + 1) as f64) * 0.25).collect();
    Array3::<f64>::from_shape_fn((nf, nr, nt), |(i, j, k)| u[i] * v[j] * w[k])
}

#[test]
fn rank_one_tensor_reconstructs_exactly() {
    let tensor = rank_one_tensor(4, 3, 2);
    let core = fit_tucker_core(&tensor, (1, 1, 1)).expect("fit");
    assert!(core.reconstruction_error < 1e-10);
    assert!(tucker_attribution_confidence_is_high(
        core.reconstruction_error
    ));
    assert_eq!(core.rank_triplet, (1, 1, 1));
}

#[test]
fn truncated_rank_incurs_bounded_error_on_mixed_tensor() {
    let base = rank_one_tensor(5, 4, 3);
    let noise = Array3::<f64>::from_shape_fn((5, 4, 3), |(i, j, k)| {
        (i as f64 - 2.0) * 0.05 + (j as f64 - 1.5) * 0.03 + (k as f64 - 1.0) * 0.02
    });
    let tensor = &base + &noise;
    let full = fit_tucker_core(&tensor, (5, 4, 3)).expect("full rank");
    let truncated = fit_tucker_core(&tensor, (2, 2, 2)).expect("truncated");
    assert!(full.reconstruction_error < 1e-8);
    assert!(truncated.reconstruction_error > full.reconstruction_error);
    // Truncated rank should still capture most of the variance — less than
    // 30% reconstruction error on a near-rank-1 tensor.
    assert!(truncated.reconstruction_error < 0.30);
}

#[test]
fn artifact_persists_with_lineage_metadata() {
    let tensor = rank_one_tensor(3, 2, 2);
    let core = fit_tucker_core(&tensor, (2, 2, 2)).expect("fit");
    let artifact = build_factor_tucker_core_artifact(
        "NQ",
        core,
        vec!["f0".to_string(), "f1".to_string(), "f2".to_string()],
        vec!["r0".to_string(), "r1".to_string()],
        vec!["t0".to_string(), "t1".to_string()],
        RunProvenance::default(),
    );
    let dir = TempDir::new().unwrap();
    persist_factor_tucker_core_artifact(dir.path(), &artifact, "analyze", None, "test").unwrap();

    let artifact_path = dir.path().join("NQ").join(FACTOR_TUCKER_CORE_ARTIFACT_FILE);
    assert!(artifact_path.exists());
    let raw = fs::read_to_string(&artifact_path).unwrap();
    assert!(raw.contains("\"factor_labels\""));
    assert!(raw.contains("\"timeframe_labels\""));
    assert!(raw.contains("\"tucker\""));

    let ledger = fs::read_to_string(
        dir.path()
            .join("NQ")
            .join(ict_engine::state::ARTIFACT_LEDGER_FILE),
    )
    .unwrap();
    assert!(ledger.contains(FACTOR_TUCKER_CORE_ARTIFACT_KIND));
    assert!(ledger.contains("attribution_confidence"));
    // Lineage artifact, not a gate.
    assert!(ledger.contains("\"promote_candidate\": false"));
}

#[test]
fn attribution_confidence_switches_on_error_band() {
    assert!(tucker_attribution_confidence_is_high(0.05));
    assert!(!tucker_attribution_confidence_is_high(0.50));
}
