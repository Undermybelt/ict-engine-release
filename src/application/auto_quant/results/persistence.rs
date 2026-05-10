//! Persistence + ledger glue for the offline Auto-Quant prior-init path.
//!
//! Two ledger artifact kinds are emitted from this module:
//!
//! * `auto_quant_strategy_library_validated` — written by
//!   `persist_imported_library` after a manifest has been loaded,
//!   schema-validated, and copied into the symbol's state directory.
//!   Status defaults to `ready_for_prior_init` and the entry's
//!   `quality_score` is the count of `status=ok` strategies (clamped
//!   to `[0, 100]`).
//! * `auto_quant_prior_init_applied` — written by
//!   `persist_prior_init_outcome` after a prior-init has been computed
//!   (and possibly applied) against the trading network. The full
//!   before/after CPT row diff is captured for reversibility.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::state::{
    append_artifact_ledger_entry, artifact_state_path, save_state, ArtifactLedgerEntry,
};

use super::manifest::StrategyLibraryManifest;
use super::prior_init::AutoQuantPriorInitOutcome;

pub const STRATEGY_LIBRARY_FILE: &str = "auto_quant_strategy_library.json";
pub const PRIOR_INIT_HISTORY_FILE: &str = "auto_quant_prior_init_history.json";

pub const ARTIFACT_KIND_LIBRARY: &str = "auto_quant_strategy_library_validated";
pub const ARTIFACT_KIND_PRIOR_INIT: &str = "auto_quant_prior_init_applied";

pub const PRIOR_INIT_RULE_VERSION: &str = "auto-quant-prior-init-v1";
pub const LIBRARY_RULE_VERSION: &str = "auto-quant-strategy-library-v1";

/// Outcome of `persist_imported_library`: the on-disk path of the
/// canonicalised manifest and the unique `artifact_id` used to refer
/// to it from a downstream prior-init.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedLibrary {
    pub artifact_id: String,
    pub state_path: String,
    pub n_total_strategies: usize,
    pub n_ok: usize,
    pub n_error: usize,
    pub n_not_run: usize,
}

/// Outcome of `persist_prior_init_outcome`: the on-disk path of the
/// per-run outcome record and the ledger artifact id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedPriorInit {
    pub artifact_id: String,
    pub state_path: String,
    pub history_path: String,
    pub strategies_applied: usize,
    pub dry_run: bool,
}

/// Single immutable record persisted per prior-init invocation. Used
/// for after-the-fact audit and reversibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorInitHistoryEntry {
    pub run_id: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub library_artifact_id: String,
    pub library_state_path: String,
    pub dry_run: bool,
    pub outcome: AutoQuantPriorInitOutcome,
}

pub fn persist_imported_library(
    state_dir: &str,
    symbol: &str,
    manifest: &StrategyLibraryManifest,
    source_path: &str,
) -> Result<PersistedLibrary> {
    save_state(state_dir, symbol, STRATEGY_LIBRARY_FILE, manifest)?;
    let state_path = artifact_state_path(state_dir, symbol, STRATEGY_LIBRARY_FILE);

    let n_total = manifest.strategies.len();
    let n_ok = manifest
        .strategies
        .iter()
        .filter(|s| s.status == "ok")
        .count();
    let n_error = manifest
        .strategies
        .iter()
        .filter(|s| s.status == "error")
        .count();
    let n_not_run = manifest
        .strategies
        .iter()
        .filter(|s| s.status == "not_run")
        .count();

    let timestamp = Utc::now();
    let artifact_id = format!(
        "auto_quant_strategy_library_{}_{}",
        symbol,
        timestamp.format("%Y%m%dT%H%M%S%.9fZ")
    );

    let superseded_ids = mark_prior_libraries_superseded(state_dir, symbol, &artifact_id)?;
    let supersedes_artifact_id = superseded_ids.last().cloned();

    let review_reason = format!(
        "imported {} ok / {} error / {} not_run from {}",
        n_ok, n_error, n_not_run, source_path
    );

    append_artifact_ledger_entry(
        state_dir,
        symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact_id),
            artifact_kind: ARTIFACT_KIND_LIBRARY.to_string(),
            artifact_id: artifact_id.clone(),
            version: 1,
            generated_at: timestamp,
            symbol: symbol.to_string(),
            source_phase: "auto_quant_results_import".to_string(),
            source_run_id: Some(manifest.auto_quant_pinned_ref.clone()),
            path: state_path.clone(),
            status: if n_ok > 0 {
                "ready_for_prior_init".to_string()
            } else {
                "no_validated_strategies".to_string()
            },
            promote_candidate: n_ok > 0,
            actionable: n_ok > 0,
            decision_hint: if n_ok > 0 {
                "review_then_prior_init".to_string()
            } else {
                "skip".to_string()
            },
            review_reason,
            review_rule_version: LIBRARY_RULE_VERSION.to_string(),
            top_factor_name: None,
            top_factor_action: None,
            family_scores: BTreeMap::new(),
            supersedes_artifact_id,
            quality_score: n_ok.min(100) as i32,
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;

    Ok(PersistedLibrary {
        artifact_id,
        state_path,
        n_total_strategies: n_total,
        n_ok,
        n_error,
        n_not_run,
    })
}

/// Flip every prior `auto_quant_strategy_library_validated` ledger
/// entry whose status is `ready_for_prior_init` to `superseded` and
/// link them to `new_artifact_id`. Called from
/// `persist_imported_library` *before* the new entry is appended,
/// so the operator can never run prior-init against an obsolete
/// manifest. Returns the list of superseded artifact ids so the new
/// entry can record the most recent one in `supersedes_artifact_id`.
pub fn mark_prior_libraries_superseded(
    state_dir: &str,
    symbol: &str,
    new_artifact_id: &str,
) -> Result<Vec<String>> {
    let mut ledger: Vec<ArtifactLedgerEntry> =
        crate::state::load_state_or_default(state_dir, symbol, crate::state::ARTIFACT_LEDGER_FILE)?;
    let mut superseded: Vec<String> = Vec::new();
    let now = Utc::now();
    for entry in ledger.iter_mut() {
        if entry.artifact_kind == ARTIFACT_KIND_LIBRARY && entry.status == "ready_for_prior_init" {
            entry.status = "superseded".to_string();
            entry.actionable = false;
            entry.promote_candidate = false;
            entry.decision_hint = "superseded_by_newer_library".to_string();
            entry.regraded_at = Some(now);
            entry.consumption_regrade_status = Some("superseded".to_string());
            entry.consumption_regrade_reason = Some(format!(
                "superseded by newer library import {}",
                new_artifact_id
            ));
            superseded.push(entry.artifact_id.clone());
        }
    }
    if !superseded.is_empty() {
        save_state(
            state_dir,
            symbol,
            crate::state::ARTIFACT_LEDGER_FILE,
            &ledger,
        )?;
    }
    Ok(superseded)
}

/// Look up an existing `auto_quant_prior_init_applied` ledger entry
/// whose `source_run_id` matches `library_artifact_id` AND whose
/// `status == "applied"` (i.e. the BBN snapshot has actually been
/// mutated against this library). Used by the prior-init command to
/// refuse a second non-dry-run apply against the same library.
pub fn find_existing_apply_for_library(
    state_dir: &str,
    symbol: &str,
    library_artifact_id: &str,
) -> Result<Option<String>> {
    let ledger: Vec<ArtifactLedgerEntry> =
        crate::state::load_state_or_default(state_dir, symbol, crate::state::ARTIFACT_LEDGER_FILE)?;
    Ok(ledger
        .into_iter()
        .find(|entry| {
            entry.artifact_kind == ARTIFACT_KIND_PRIOR_INIT
                && entry.status == "applied"
                && entry.source_run_id.as_deref() == Some(library_artifact_id)
        })
        .map(|entry| entry.artifact_id))
}

/// Library-agnostic counterpart to `find_existing_apply_for_library`:
/// returns the most recent `auto_quant_prior_init_applied` entry
/// (regardless of source library) whose `status == "applied"`.
/// Returns `(apply_artifact_id, library_artifact_id)` so the caller
/// can build a precise cross-library double-apply error message.
///
/// This covers the gap where the operator imports v1, applies v1,
/// then imports v2 (which auto-supersedes v1) and applies v2: the
/// per-library guard sees no prior apply for v2's id and would let
/// the second mutation stack on top of v1's still-live effect.
pub fn find_any_active_prior_init_apply(
    state_dir: &str,
    symbol: &str,
) -> Result<Option<(String, Option<String>)>> {
    let ledger: Vec<ArtifactLedgerEntry> =
        crate::state::load_state_or_default(state_dir, symbol, crate::state::ARTIFACT_LEDGER_FILE)?;
    Ok(ledger
        .into_iter()
        .rev()
        .find(|entry| entry.artifact_kind == ARTIFACT_KIND_PRIOR_INIT && entry.status == "applied")
        .map(|entry| (entry.artifact_id, entry.source_run_id)))
}

pub fn persist_prior_init_outcome(
    state_dir: &str,
    symbol: &str,
    outcome: &AutoQuantPriorInitOutcome,
    library_artifact_id: &str,
    library_state_path: &str,
    dry_run: bool,
) -> Result<PersistedPriorInit> {
    let timestamp = Utc::now();
    let run_id = format!(
        "auto_quant_prior_init_{}_{}",
        symbol,
        timestamp.format("%Y%m%dT%H%M%S%.9fZ")
    );
    let artifact_id = run_id.clone();
    let outcome_filename = format!("{}.json", run_id);

    save_state(state_dir, symbol, &outcome_filename, outcome)
        .with_context(|| format!("persisting prior-init outcome '{}'", outcome_filename))?;
    let state_path = artifact_state_path(state_dir, symbol, &outcome_filename);

    let mut history: Vec<PriorInitHistoryEntry> =
        crate::state::load_state_or_default(state_dir, symbol, PRIOR_INIT_HISTORY_FILE)?;
    history.push(PriorInitHistoryEntry {
        run_id: run_id.clone(),
        timestamp,
        library_artifact_id: library_artifact_id.to_string(),
        library_state_path: library_state_path.to_string(),
        dry_run,
        outcome: outcome.clone(),
    });
    save_state(state_dir, symbol, PRIOR_INIT_HISTORY_FILE, &history)?;
    let history_path = artifact_state_path(state_dir, symbol, PRIOR_INIT_HISTORY_FILE);

    let strategies_applied = outcome.strategies_applied.len();
    let strategies_skipped = outcome.strategies_skipped.len();

    let review_reason = format!(
        "prior_init applied={} skipped={} dry_run={} parent_config={:?}",
        strategies_applied, strategies_skipped, dry_run, outcome.parent_config
    );

    append_artifact_ledger_entry(
        state_dir,
        symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact_id),
            artifact_kind: ARTIFACT_KIND_PRIOR_INIT.to_string(),
            artifact_id: artifact_id.clone(),
            version: 1,
            generated_at: timestamp,
            symbol: symbol.to_string(),
            source_phase: "auto_quant_prior_init".to_string(),
            source_run_id: Some(library_artifact_id.to_string()),
            path: state_path.clone(),
            status: if dry_run {
                "dry_run_preview".to_string()
            } else if strategies_applied > 0 {
                "applied".to_string()
            } else {
                "no_op".to_string()
            },
            promote_candidate: false,
            actionable: false,
            decision_hint: if dry_run {
                "review_then_apply".to_string()
            } else {
                "applied".to_string()
            },
            review_reason,
            review_rule_version: PRIOR_INIT_RULE_VERSION.to_string(),
            top_factor_name: None,
            top_factor_action: None,
            family_scores: BTreeMap::new(),
            supersedes_artifact_id: None,
            quality_score: strategies_applied.min(100) as i32,
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;

    Ok(PersistedPriorInit {
        artifact_id,
        state_path,
        history_path,
        strategies_applied,
        dry_run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::auto_quant::results::manifest::{
        StrategyLibraryEntry, StrategyLibraryMetadata, StrategyLibraryValidationMetrics,
    };
    use crate::state::ARTIFACT_LEDGER_FILE;

    fn manifest_with_two_ok_one_error() -> StrategyLibraryManifest {
        let mk_entry = |name: &str, status: &str| StrategyLibraryEntry {
            name: name.to_string(),
            file_path: format!("user_data/strategies_ibkr/{name}.py"),
            metadata: StrategyLibraryMetadata {
                strategy: name.to_string(),
                mutation_id: format!("mut-{name}"),
                ..Default::default()
            },
            status: status.to_string(),
            validation_metrics: Some(StrategyLibraryValidationMetrics {
                trade_count: 100,
                win_rate_pct: 60.0,
                ..Default::default()
            }),
            ..Default::default()
        };

        StrategyLibraryManifest {
            manifest_version: "1.0".to_string(),
            auto_quant_pinned_ref: "abc1234".to_string(),
            strategies: vec![
                mk_entry("Alpha", "ok"),
                mk_entry("Beta", "ok"),
                mk_entry("Gamma", "error"),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn persist_library_writes_state_and_ledger() {
        let temp = tempfile::tempdir().unwrap();
        let manifest = manifest_with_two_ok_one_error();
        let result = persist_imported_library(
            temp.path().to_str().unwrap(),
            "NQ",
            &manifest,
            "/tmp/strategy_library.json",
        )
        .unwrap();

        assert_eq!(result.n_total_strategies, 3);
        assert_eq!(result.n_ok, 2);
        assert_eq!(result.n_error, 1);
        assert_eq!(result.n_not_run, 0);
        assert!(std::path::Path::new(&result.state_path).exists());

        let ledger =
            std::fs::read_to_string(temp.path().join("NQ").join(ARTIFACT_LEDGER_FILE)).unwrap();
        assert!(ledger.contains(ARTIFACT_KIND_LIBRARY));
        assert!(ledger.contains(LIBRARY_RULE_VERSION));
        assert!(ledger.contains("ready_for_prior_init"));
    }

    #[test]
    fn persist_prior_init_writes_history_and_consumes_library() {
        let temp = tempfile::tempdir().unwrap();
        let manifest = manifest_with_two_ok_one_error();
        let lib = persist_imported_library(
            temp.path().to_str().unwrap(),
            "NQ",
            &manifest,
            "/tmp/strategy_library.json",
        )
        .unwrap();

        let outcome = AutoQuantPriorInitOutcome {
            parent_config: vec![0, 0, 0],
            initial_probs: vec![1.0 / 3.0; 3],
            final_probs: vec![0.5, 0.05, 0.45],
            strategies_applied: vec![Default::default(), Default::default()],
            strategies_skipped: vec![("Gamma".into(), "status=error".into())],
            temper: 0.5,
            prior_strength: 4.0,
            bbn_entropy_reduction: 0.125,
            bbn_log_loss_delta: 0.75,
            bbn_contradiction_lift: 0.5,
            evidence_value_gate_passed: true,
        };

        let pi = persist_prior_init_outcome(
            temp.path().to_str().unwrap(),
            "NQ",
            &outcome,
            &lib.artifact_id,
            &lib.state_path,
            false,
        )
        .unwrap();

        assert_eq!(pi.strategies_applied, 2);
        assert!(!pi.dry_run);
        assert!(std::path::Path::new(&pi.state_path).exists());
        assert!(std::path::Path::new(&pi.history_path).exists());
        let persisted: AutoQuantPriorInitOutcome =
            serde_json::from_str(&std::fs::read_to_string(&pi.state_path).unwrap()).unwrap();
        assert_eq!(persisted.bbn_entropy_reduction, 0.125);
        assert_eq!(persisted.bbn_log_loss_delta, 0.75);
        assert_eq!(persisted.bbn_contradiction_lift, 0.5);
        assert!(persisted.evidence_value_gate_passed);
        let history: Vec<PriorInitHistoryEntry> =
            serde_json::from_str(&std::fs::read_to_string(&pi.history_path).unwrap()).unwrap();
        assert_eq!(history[0].outcome.bbn_entropy_reduction, 0.125);
        assert_eq!(history[0].outcome.bbn_log_loss_delta, 0.75);

        let ledger =
            std::fs::read_to_string(temp.path().join("NQ").join(ARTIFACT_LEDGER_FILE)).unwrap();
        assert!(ledger.contains(ARTIFACT_KIND_PRIOR_INIT));
        assert!(ledger.contains(PRIOR_INIT_RULE_VERSION));
        // Lineage captured via source_run_id pointing back at the library.
        assert!(ledger.contains(&lib.artifact_id));
        // Library remains in ready_for_prior_init (re-runnable with different
        // temper / parent_config), not flipped to a consumed terminal state.
        assert!(ledger.contains("ready_for_prior_init"));
    }

    #[test]
    fn persist_prior_init_dry_run_records_dry_run_status(/* and leaves library untouched */) {
        let temp = tempfile::tempdir().unwrap();
        let manifest = manifest_with_two_ok_one_error();
        let lib = persist_imported_library(
            temp.path().to_str().unwrap(),
            "NQ",
            &manifest,
            "/tmp/strategy_library.json",
        )
        .unwrap();

        let outcome = AutoQuantPriorInitOutcome {
            parent_config: vec![0, 0, 0],
            initial_probs: vec![1.0 / 3.0; 3],
            final_probs: vec![0.5, 0.0, 0.5],
            strategies_applied: vec![Default::default()],
            strategies_skipped: vec![],
            temper: 0.5,
            prior_strength: 4.0,
            ..Default::default()
        };

        let pi = persist_prior_init_outcome(
            temp.path().to_str().unwrap(),
            "NQ",
            &outcome,
            &lib.artifact_id,
            &lib.state_path,
            true, // dry_run
        )
        .unwrap();

        assert!(pi.dry_run);
        let ledger: Vec<ArtifactLedgerEntry> =
            crate::state::load_state(temp.path(), "NQ", ARTIFACT_LEDGER_FILE).unwrap();
        let prior_init_entry = ledger
            .iter()
            .find(|e| e.artifact_kind == ARTIFACT_KIND_PRIOR_INIT)
            .expect("prior init ledger entry");
        assert_eq!(prior_init_entry.status, "dry_run_preview");
        assert_eq!(
            prior_init_entry.source_run_id.as_deref(),
            Some(lib.artifact_id.as_str())
        );

        let library_entry = ledger
            .iter()
            .find(|e| e.artifact_kind == ARTIFACT_KIND_LIBRARY)
            .expect("library ledger entry");
        // Library entry is untouched by the dry run (no consumed_* fields set).
        assert!(library_entry.consumed_by_update_run_id.is_none());
        assert!(library_entry.consumed_at.is_none());
    }

    #[test]
    fn second_import_supersedes_prior_ready_library() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let manifest = manifest_with_two_ok_one_error();

        let first =
            persist_imported_library(state_dir, "NQ", &manifest, "/tmp/first.json").unwrap();
        // Same manifest, simulated as a re-export.
        let second =
            persist_imported_library(state_dir, "NQ", &manifest, "/tmp/second.json").unwrap();

        let ledger: Vec<ArtifactLedgerEntry> =
            crate::state::load_state(temp.path(), "NQ", ARTIFACT_LEDGER_FILE).unwrap();

        let first_entry = ledger
            .iter()
            .find(|e| e.artifact_id == first.artifact_id)
            .expect("first library entry");
        assert_eq!(first_entry.status, "superseded");
        assert!(!first_entry.actionable);
        assert_eq!(first_entry.decision_hint, "superseded_by_newer_library");

        let second_entry = ledger
            .iter()
            .find(|e| e.artifact_id == second.artifact_id)
            .expect("second library entry");
        assert_eq!(second_entry.status, "ready_for_prior_init");
        assert_eq!(
            second_entry.supersedes_artifact_id.as_deref(),
            Some(first.artifact_id.as_str())
        );
    }

    #[test]
    fn no_op_supersession_when_no_prior_library_exists() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let superseded = mark_prior_libraries_superseded(state_dir, "NQ", "fresh-id").unwrap();
        assert!(superseded.is_empty());
    }

    #[test]
    fn find_existing_apply_returns_only_applied_entries() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let manifest = manifest_with_two_ok_one_error();
        let lib = persist_imported_library(state_dir, "NQ", &manifest, "/tmp/lib.json").unwrap();

        // No prior init yet → None.
        assert!(
            find_existing_apply_for_library(state_dir, "NQ", &lib.artifact_id)
                .unwrap()
                .is_none()
        );

        // Dry-run does NOT count as applied.
        let dry_outcome = AutoQuantPriorInitOutcome {
            parent_config: vec![0, 0, 0],
            initial_probs: vec![1.0 / 3.0; 3],
            final_probs: vec![0.5, 0.0, 0.5],
            strategies_applied: vec![Default::default()],
            strategies_skipped: vec![],
            temper: 0.5,
            prior_strength: 4.0,
            ..Default::default()
        };
        persist_prior_init_outcome(
            state_dir,
            "NQ",
            &dry_outcome,
            &lib.artifact_id,
            &lib.state_path,
            true, // dry_run
        )
        .unwrap();
        assert!(
            find_existing_apply_for_library(state_dir, "NQ", &lib.artifact_id)
                .unwrap()
                .is_none(),
            "dry_run_preview must not register as applied"
        );

        // Real apply DOES register.
        let apply_outcome = AutoQuantPriorInitOutcome {
            parent_config: vec![0, 0, 0],
            initial_probs: vec![1.0 / 3.0; 3],
            final_probs: vec![0.5, 0.0, 0.5],
            strategies_applied: vec![Default::default()],
            strategies_skipped: vec![],
            temper: 0.5,
            prior_strength: 4.0,
            ..Default::default()
        };
        let applied = persist_prior_init_outcome(
            state_dir,
            "NQ",
            &apply_outcome,
            &lib.artifact_id,
            &lib.state_path,
            false, // real apply
        )
        .unwrap();
        let found = find_existing_apply_for_library(state_dir, "NQ", &lib.artifact_id).unwrap();
        assert_eq!(found.as_deref(), Some(applied.artifact_id.as_str()));
    }

    #[test]
    fn find_any_active_apply_spans_library_boundaries() {
        let temp = tempfile::tempdir().unwrap();
        let state_dir = temp.path().to_str().unwrap();
        let manifest = manifest_with_two_ok_one_error();

        // Import v1, apply v1.
        let v1 = persist_imported_library(state_dir, "NQ", &manifest, "/tmp/v1.json").unwrap();
        let outcome = AutoQuantPriorInitOutcome {
            parent_config: vec![0, 0, 0],
            initial_probs: vec![1.0 / 3.0; 3],
            final_probs: vec![0.5, 0.0, 0.5],
            strategies_applied: vec![Default::default()],
            strategies_skipped: vec![],
            temper: 0.5,
            prior_strength: 4.0,
            ..Default::default()
        };
        let v1_apply = persist_prior_init_outcome(
            state_dir,
            "NQ",
            &outcome,
            &v1.artifact_id,
            &v1.state_path,
            false,
        )
        .unwrap();

        // Import v2 (auto-supersedes v1).
        let v2 = persist_imported_library(state_dir, "NQ", &manifest, "/tmp/v2.json").unwrap();

        // Per-library guard misses v1's apply when asked about v2.
        assert!(
            find_existing_apply_for_library(state_dir, "NQ", &v2.artifact_id)
                .unwrap()
                .is_none()
        );

        // Cross-library guard catches it: an applied prior_init exists,
        // it points back to v1's library_artifact_id.
        let any = find_any_active_prior_init_apply(state_dir, "NQ").unwrap();
        let (apply_id, lib_id) = any.expect("expected active apply");
        assert_eq!(apply_id, v1_apply.artifact_id);
        assert_eq!(lib_id.as_deref(), Some(v1.artifact_id.as_str()));
        assert_ne!(lib_id.as_deref(), Some(v2.artifact_id.as_str()));
    }
}
