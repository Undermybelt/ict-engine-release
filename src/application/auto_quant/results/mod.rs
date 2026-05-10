//! Auto-Quant results ingestion (offline path / Phase 1).
//!
//! Consumes the cross-repo handoff artifacts produced by Auto-Quant's
//! `export_strategy_library.py` (canonical) and / or `run_ibkr.log`
//! (redundant cross-check) and converts them into:
//!
//! 1. A persisted `strategy_library_validated` ledger artifact
//!    (provenance + metrics, no BBN side effect).
//! 2. A `prior_init_applied` ledger artifact whose application step
//!    seeds the trading network's `trade_outcome` CPT with
//!    Beta-Binomial / Dirichlet-tempered pseudo-counts derived from
//!    the strategy's empirical win-rate.
//!
//! This module **never** writes to the BBN posterior — that path is
//! reserved for `apply_feedback_to_trade_outcome_network` driven by
//! real-trade `FeedbackRecord`s. See
//! `docs/2026-04-26-auto-quant-bbn-prior-init-plan.md` for the
//! Phase 1 contract.

mod log_parser;
mod manifest;
mod persistence;
mod prior_init;

pub use log_parser::{
    cross_check_manifest_against_log, parse_run_ibkr_log, ManifestLogCrossCheck,
    ManifestLogMismatch, RunIbkrLogBlock,
};
pub use manifest::{
    load_strategy_library_manifest, StrategyLibraryEntry, StrategyLibraryEntryStatus,
    StrategyLibraryManifest, StrategyLibraryMetadata, StrategyLibraryValidationError,
    StrategyLibraryValidationMetrics, MANIFEST_SUPPORTED_VERSIONS,
};
pub use persistence::{
    find_any_active_prior_init_apply, find_existing_apply_for_library,
    mark_prior_libraries_superseded, persist_imported_library, persist_prior_init_outcome,
    PersistedLibrary, PersistedPriorInit, PriorInitHistoryEntry, ARTIFACT_KIND_LIBRARY,
    ARTIFACT_KIND_PRIOR_INIT, LIBRARY_RULE_VERSION, PRIOR_INIT_HISTORY_FILE,
    PRIOR_INIT_RULE_VERSION, STRATEGY_LIBRARY_FILE,
};
pub use prior_init::{
    apply_strategy_library_prior_init, AutoQuantPriorInitInput, AutoQuantPriorInitOutcome,
    AutoQuantPriorInitStrategyEffect, DEFAULT_DEFAULT_PARENT_CONFIG, DEFAULT_PRIOR_STRENGTH,
    DEFAULT_TEMPER,
};
