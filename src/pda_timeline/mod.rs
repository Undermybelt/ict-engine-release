//! Unified PDA event timeline + adjacency matrices.
//!
//! This module is **separate** from `crate::pda_sequence`:
//! - `pda_sequence` is the DTW / clustering pipeline keyed on
//!   `PdaTokenKind` (a 7-variant enum frozen at v1 to keep the
//!   clustering fixture stable).
//! - `pda_timeline` owns its own richer 13-variant `PdaEventKind`
//!   and is consumed by canonical-setup matchers and the
//!   factor_research evidence layer in P1b.
//!
//! Public surface:
//! - [`PdaEvent`], [`PdaEventKind`], [`ALL_EVENT_KINDS`]
//! - [`build_pda_timeline`] — assemble a sorted `Vec<PdaEvent>` from
//!   `(candles, atr)` using all 13 detectors with module defaults.
//! - [`compute_cooccurrence_matrix`], [`compute_precedence_matrix`],
//!   [`EventMatrix`], [`MatrixKind`].

pub mod builder;
pub mod event;
pub mod generated_promoted_setups;
pub mod matrices;
pub mod ote;
pub mod promoted;
pub mod sessions;
pub mod setups;

pub use builder::{
    assert_timeline_bars_valid, build_pda_timeline, TIMELINE_DEFAULT_CISD_MIN_STRENGTH,
    TIMELINE_DEFAULT_LIQUIDITY_POOL_ATR_MULT, TIMELINE_DEFAULT_LIQUIDITY_POOL_MIN_TOUCHES,
    TIMELINE_DEFAULT_LIQUIDITY_SWEEP_RETURN_BARS, TIMELINE_DEFAULT_RB_BODY_WICK_RATIO,
    TIMELINE_DEFAULT_RB_MIN_RANGE_ATR, TIMELINE_DEFAULT_SWING_STRENGTH,
};
pub use event::{PdaEvent, PdaEventKind, ALL_EVENT_KINDS};
pub use matrices::{
    compute_cooccurrence_matrix, compute_precedence_matrix, EventMatrix, MatrixKind,
};
pub use ote::{most_recent_ote_zone, OteZone, OTE_HIGH, OTE_LOW};
pub use promoted::{
    append_promoted_canonical_setup, build_promoted_canonical_setup_spec,
    embedded_promoted_canonical_setups, load_promoted_canonical_setup_manifest,
    match_promoted_canonical_setups, parse_promoted_sequence_label, repo_root_from_manifest_dir,
    PromotedCanonicalSetupManifest, PromotedCanonicalSetupSpec,
    PROMOTED_CANONICAL_SETUPS_CONFIG_FILE, PROMOTED_CANONICAL_SETUPS_GENERATED_FILE,
};
pub use sessions::{classify_session_zones, is_in_zone, SessionKillZone};
pub use setups::{
    match_all_setups, match_all_setups_default, match_all_setups_extended, CanonicalSetupKind,
    SetupContext, SetupMatch, ALL_CANONICAL_SETUPS, DEFAULT_CROSS_TF_MAX_LAG_HOURS,
    DEFAULT_KEY_LEVEL_TOLERANCE_BPS, DEFAULT_OTE_SWING_STRENGTH, DEFAULT_SESSION_MAX_LAG_HOURS,
    DEFAULT_SETUP_HORIZON_BARS, DEFAULT_SMT_CONFIRM_WINDOW_BARS, DEFAULT_SMT_LOOKBACK_BARS,
    DEFAULT_WEEKLY_MAX_LAG_HOURS,
};
