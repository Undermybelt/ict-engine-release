//! Round 2 §3.2 smoke — read a state dir + symbol, build the
//! factor × regime × metric tensor, fit a Tucker core, persist
//! `factor_tucker_core.json` + ledger entry.
//!
//! Usage:
//! ```bash
//! cargo run --example round2_tucker_snapshot -- <state_dir> <symbol>
//! ```
//! Example:
//! ```bash
//! cargo run --example round2_tucker_snapshot -- state_autoresearch_smoke NQ
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use ict_engine::factor_lab::{
    build_factor_tucker_core_artifact, fit_tucker_core_from_state_dir,
    persist_factor_tucker_core_artifact,
};
use ict_engine::state::RunProvenance;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: {} <state_dir> <symbol>",
            args.first()
                .map(String::as_str)
                .unwrap_or("round2_tucker_snapshot")
        );
        return ExitCode::from(2);
    }
    let state_dir = PathBuf::from(&args[1]);
    let symbol = &args[2];

    match fit_tucker_core_from_state_dir(&state_dir, symbol) {
        Ok(Some((tucker, factor_labels, regime_labels, metric_labels))) => {
            println!(
                "fit tucker core: rank={:?} reconstruction_error={:.4}",
                tucker.rank_triplet, tucker.reconstruction_error
            );
            let provenance = RunProvenance {
                data_fingerprint: format!("round2-tucker-{}", symbol),
                ..RunProvenance::default()
            };
            let artifact = build_factor_tucker_core_artifact(
                symbol,
                tucker,
                factor_labels,
                regime_labels,
                metric_labels,
                provenance,
            );
            match persist_factor_tucker_core_artifact(
                &state_dir,
                &artifact,
                "round2_tucker_snapshot",
                None,
                "round2_smoke_replay",
            ) {
                Ok(()) => {
                    println!(
                        "persisted {} at {}/{}/factor_tucker_core.json",
                        artifact.artifact_id,
                        state_dir.display(),
                        symbol
                    );
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("persist failed: {err:#}");
                    ExitCode::from(1)
                }
            }
        }
        Ok(None) => {
            eprintln!(
                "skipped: {} has no factor regime_stats yet (run factor-autoresearch first)",
                symbol
            );
            ExitCode::from(3)
        }
        Err(err) => {
            eprintln!("tucker fit failed: {err:#}");
            ExitCode::from(1)
        }
    }
}
