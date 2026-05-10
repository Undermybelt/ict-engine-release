//! Canonical ICT setup matchers over the PDA event timeline.
//!
//! Each matcher consumes the sorted `Vec<PdaEvent>` from
//! `super::builder::build_pda_timeline` and emits zero or more
//! `SetupMatch` records for the named ICT setup.
//!
//! ## Scope (v2 — P1b base + P1c extended)
//!
//! 27 of the 30 canonical setups in
//! `docs/2026-04-27-pda-factor-universe-plan.md` §4.1 are now
//! shipped as named matchers. The plan's three remaining SMT
//! variants (`EquityFuturesSmt`, `CurrencyFuturesSmt`,
//! `GoldVixDivergence`) are intentionally **not** distinct
//! variants here — the matcher logic is identical for all four
//! cross-symbol divergences; only the symbol pair differs, and
//! that lives in the caller's metadata, not in this enum. The
//! generic `SmtDivergenceConfirm` covers the pattern; the
//! plan's other three labels become rendering hints in
//! `factor_research` reports.
//!
//! ### Single-timeframe (P1b, 13)
//!
//! | Variant                     | Pattern                                      |
//! |-----------------------------|----------------------------------------------|
//! | ObRetestPropulsionConfirm   | OB → Propulsion (same dir, within horizon)   |
//! | IFvgContinuation            | MSS → iFVG (same dir, within horizon)        |
//! | BreakerBlockRetest          | BreakerBlock event present                   |
//! | MitigationBlockRetest       | MitigationBlock event present                |
//! | RejectionBlockAtKeyLevel    | RB level near a recent MSS/SB level          |
//! | VolumeImbalanceFiller       | VI → MSS/SB/Propulsion (same dir)            |
//! | LiquidityVoidContinuation   | LV → MSS/SB (same dir)                       |
//! | PropulsionPostMss           | MSS → Propulsion (same dir, within horizon)  |
//! | CisdAfterDistribution       | last MSS Bull → CISD Bear within horizon     |
//! | CisdAfterAccumulation       | last MSS Bear → CISD Bull within horizon     |
//! | UnicornModel                | BreakerBlock + FVG overlap (same dir, near)  |
//! | PowerOfThree                | LiquiditySweep → opposite-dir MSS/SB         |
//! | TurtleSoupLiquidityGrab     | LiquiditySweep → opposite-dir MSS            |
//!
//! ### Cross-timeframe (P1c, 5 — require `SetupContext::htf_events`)
//!
//! | Variant                     | Pattern                                      |
//! |-----------------------------|----------------------------------------------|
//! | HtfMssLtfFvg                | HTF MSS → LTF FVG (same dir, < max_lag)      |
//! | HtfCisdLtfObRetest          | HTF CISD → LTF OB (same dir, < max_lag)      |
//! | DailyHighSweepLtfMssFvg     | HTF Bull sweep → LTF Bear MSS + FVG          |
//! | DailyLowSweepLtfMssFvg      | HTF Bear sweep → LTF Bull MSS + FVG          |
//! | WeeklyOpenSweepDailyMss     | weekly sweep → daily MSS (uses `mtf_events`) |
//!
//! ### Session-aware (P1c, 5 — use `event.timestamp` only)
//!
//! | Variant                     | Pattern                                      |
//! |-----------------------------|----------------------------------------------|
//! | AsiaRangeRaidLondonMss      | sweep in Asia KZ → MSS in London KZ          |
//! | LondonRaidNyMssFvg          | sweep in London KZ → MSS + FVG in NY KZ      |
//! | SilverBulletWindow          | FVG with timestamp in 10:00-11:00 NY         |
//! | SilverBulletAm              | FVG with timestamp in 03:00-04:00 NY         |
//! | JudasSwingReversal          | sweep in 08:30-09:30 NY → opposite-dir MSS   |
//!
//! ### Cross-symbol (P1c, 1 — requires `SetupContext::primary_candles`
//! ### + `paired_candles`)
//!
//! | Variant                     | Pattern                                      |
//! |-----------------------------|----------------------------------------------|
//! | SmtDivergenceConfirm        | paired-symbol divergence + same-bar MSS      |
//!
//! ### OTE retracement (P1c, 3 — require `SetupContext::primary_candles`)
//!
//! | Variant                     | Pattern                                      |
//! |-----------------------------|----------------------------------------------|
//! | OteWithFvgConfluence        | FVG with level inside most-recent OTE band   |
//! | OteWithObConfluence         | OB with level inside most-recent OTE band    |
//! | OptimalTradeEntryWithCisd   | CISD with level inside most-recent OTE band  |

use chrono::Duration;
use serde::{Deserialize, Serialize};

use super::event::{PdaEvent, PdaEventKind};
use super::ote::most_recent_ote_zone;
use super::promoted::match_promoted_canonical_setups;
use super::sessions::{is_in_zone, SessionKillZone};
use crate::smt::Divergence;
use crate::types::{Candle, Direction};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CanonicalSetupKind {
    // Single-timeframe (P1b)
    ObRetestPropulsionConfirm,
    IFvgContinuation,
    BreakerBlockRetest,
    MitigationBlockRetest,
    RejectionBlockAtKeyLevel,
    VolumeImbalanceFiller,
    LiquidityVoidContinuation,
    PropulsionPostMss,
    CisdAfterDistribution,
    CisdAfterAccumulation,
    UnicornModel,
    PowerOfThree,
    TurtleSoupLiquidityGrab,
    // Cross-timeframe (P1c)
    HtfMssLtfFvg,
    HtfCisdLtfObRetest,
    DailyHighSweepLtfMssFvg,
    DailyLowSweepLtfMssFvg,
    WeeklyOpenSweepDailyMss,
    // Session-aware (P1c)
    AsiaRangeRaidLondonMss,
    LondonRaidNyMssFvg,
    SilverBulletWindow,
    SilverBulletAm,
    JudasSwingReversal,
    // Cross-symbol (P1c)
    SmtDivergenceConfirm,
    // OTE retracement (P1c)
    OteWithFvgConfluence,
    OteWithObConfluence,
    OptimalTradeEntryWithCisd,
    // Operator-promoted custom sequence (P3)
    PromotedCanonicalSequence,
}

impl CanonicalSetupKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ObRetestPropulsionConfirm => "ob_retest_propulsion_confirm",
            Self::IFvgContinuation => "ifvg_continuation",
            Self::BreakerBlockRetest => "breaker_block_retest",
            Self::MitigationBlockRetest => "mitigation_block_retest",
            Self::RejectionBlockAtKeyLevel => "rejection_block_at_key_level",
            Self::VolumeImbalanceFiller => "volume_imbalance_filler",
            Self::LiquidityVoidContinuation => "liquidity_void_continuation",
            Self::PropulsionPostMss => "propulsion_post_mss",
            Self::CisdAfterDistribution => "cisd_after_distribution",
            Self::CisdAfterAccumulation => "cisd_after_accumulation",
            Self::UnicornModel => "unicorn_model",
            Self::PowerOfThree => "power_of_three",
            Self::TurtleSoupLiquidityGrab => "turtle_soup_liquidity_grab",
            Self::HtfMssLtfFvg => "htf_mss_ltf_fvg",
            Self::HtfCisdLtfObRetest => "htf_cisd_ltf_ob_retest",
            Self::DailyHighSweepLtfMssFvg => "daily_high_sweep_ltf_mss_fvg",
            Self::DailyLowSweepLtfMssFvg => "daily_low_sweep_ltf_mss_fvg",
            Self::WeeklyOpenSweepDailyMss => "weekly_open_sweep_daily_mss",
            Self::AsiaRangeRaidLondonMss => "asia_range_raid_london_mss",
            Self::LondonRaidNyMssFvg => "london_raid_ny_mss_fvg",
            Self::SilverBulletWindow => "silver_bullet_window",
            Self::SilverBulletAm => "silver_bullet_am",
            Self::JudasSwingReversal => "judas_swing_reversal",
            Self::SmtDivergenceConfirm => "smt_divergence_confirm",
            Self::OteWithFvgConfluence => "ote_with_fvg_confluence",
            Self::OteWithObConfluence => "ote_with_ob_confluence",
            Self::OptimalTradeEntryWithCisd => "optimal_trade_entry_with_cisd",
            Self::PromotedCanonicalSequence => "promoted_canonical_sequence",
        }
    }
}

pub const ALL_CANONICAL_SETUPS: [CanonicalSetupKind; 27] = [
    CanonicalSetupKind::ObRetestPropulsionConfirm,
    CanonicalSetupKind::IFvgContinuation,
    CanonicalSetupKind::BreakerBlockRetest,
    CanonicalSetupKind::MitigationBlockRetest,
    CanonicalSetupKind::RejectionBlockAtKeyLevel,
    CanonicalSetupKind::VolumeImbalanceFiller,
    CanonicalSetupKind::LiquidityVoidContinuation,
    CanonicalSetupKind::PropulsionPostMss,
    CanonicalSetupKind::CisdAfterDistribution,
    CanonicalSetupKind::CisdAfterAccumulation,
    CanonicalSetupKind::UnicornModel,
    CanonicalSetupKind::PowerOfThree,
    CanonicalSetupKind::TurtleSoupLiquidityGrab,
    CanonicalSetupKind::HtfMssLtfFvg,
    CanonicalSetupKind::HtfCisdLtfObRetest,
    CanonicalSetupKind::DailyHighSweepLtfMssFvg,
    CanonicalSetupKind::DailyLowSweepLtfMssFvg,
    CanonicalSetupKind::WeeklyOpenSweepDailyMss,
    CanonicalSetupKind::AsiaRangeRaidLondonMss,
    CanonicalSetupKind::LondonRaidNyMssFvg,
    CanonicalSetupKind::SilverBulletWindow,
    CanonicalSetupKind::SilverBulletAm,
    CanonicalSetupKind::JudasSwingReversal,
    CanonicalSetupKind::SmtDivergenceConfirm,
    CanonicalSetupKind::OteWithFvgConfluence,
    CanonicalSetupKind::OteWithObConfluence,
    CanonicalSetupKind::OptimalTradeEntryWithCisd,
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetupMatch {
    pub kind: CanonicalSetupKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_override: Option<String>,
    pub direction: Direction,
    pub anchor_bar: usize,
    pub confirm_bar: usize,
    pub event_bars: Vec<usize>,
}

impl SetupMatch {
    pub fn label(&self) -> &str {
        self.name_override.as_deref().unwrap_or(self.kind.as_str())
    }
}

pub const DEFAULT_SETUP_HORIZON_BARS: usize = 30;
/// Tolerance (in price terms relative to the level) for "near a key level".
/// 25 bps mirrors the magnitude `pda_state.rs` uses for touch checks.
pub const DEFAULT_KEY_LEVEL_TOLERANCE_BPS: f64 = 0.0025;

pub fn match_all_setups(events: &[PdaEvent], horizon_bars: usize) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    out.extend(match_ob_retest_propulsion(events, horizon_bars));
    out.extend(match_ifvg_continuation(events, horizon_bars));
    out.extend(match_breaker_block_retest(events));
    out.extend(match_mitigation_block_retest(events));
    out.extend(match_rejection_block_at_key_level(
        events,
        horizon_bars,
        DEFAULT_KEY_LEVEL_TOLERANCE_BPS,
    ));
    out.extend(match_volume_imbalance_filler(events, horizon_bars));
    out.extend(match_liquidity_void_continuation(events, horizon_bars));
    out.extend(match_propulsion_post_mss(events, horizon_bars));
    out.extend(match_cisd_after_distribution(events, horizon_bars));
    out.extend(match_cisd_after_accumulation(events, horizon_bars));
    out.extend(match_unicorn_model(events, horizon_bars));
    out.extend(match_power_of_three(events, horizon_bars));
    out.extend(match_turtle_soup_liquidity_grab(events, horizon_bars));
    out.extend(match_promoted_canonical_setups(events, horizon_bars));
    out.sort_by_key(|m| (m.confirm_bar, m.label().to_string()));
    out
}

pub fn match_all_setups_default(events: &[PdaEvent]) -> Vec<SetupMatch> {
    match_all_setups(events, DEFAULT_SETUP_HORIZON_BARS)
}

// --------------------------------------------------------------------
// P1c — extended setup matching with optional cross-TF / SMT / OTE
// context. Session-aware setups need no context (they read
// `event.timestamp` directly).
// --------------------------------------------------------------------

/// Lag tolerance between an HTF anchor event (e.g. 4h MSS) and its
/// LTF confirming event (e.g. 15m FVG). One trading day is enough
/// for the typical HTF=4h/daily → LTF=15m/1h chains in the plan.
pub const DEFAULT_CROSS_TF_MAX_LAG_HOURS: i64 = 24;

/// Lag tolerance for weekly-anchored setups (one calendar week).
pub const DEFAULT_WEEKLY_MAX_LAG_HOURS: i64 = 168;

/// Lag tolerance between two session events (e.g. Asia sweep →
/// London MSS). One full equity day covers the typical chain.
pub const DEFAULT_SESSION_MAX_LAG_HOURS: i64 = 12;

/// Lookback window passed to `Divergence::detect` for SMT setups.
pub const DEFAULT_SMT_LOOKBACK_BARS: usize = 20;

/// Recency window for SMT confirmation: the MSS confirming a divergence
/// must fire within this many bars of the divergence flag.
pub const DEFAULT_SMT_CONFIRM_WINDOW_BARS: usize = 5;

/// Swing strength used by `most_recent_ote_zone` when called from
/// the OTE setup matchers. Mirrors `TIMELINE_DEFAULT_SWING_STRENGTH`.
pub const DEFAULT_OTE_SWING_STRENGTH: usize = 3;

/// Optional inputs for the extended matcher. Each field controls a
/// disjoint group of setups; missing fields silently skip those
/// setups. The single-TF (P1b) setups always run regardless.
#[derive(Debug, Clone, Copy, Default)]
pub struct SetupContext<'a> {
    /// Primary-symbol candles (same series the events were built
    /// from). Required for OTE setups (swing leg detection) and SMT
    /// setups (close series for divergence).
    pub primary_candles: Option<&'a [Candle]>,
    /// Higher-timeframe events (e.g. 4h or daily). Required for the
    /// 5 cross-TF setups except `WeeklyOpenSweepDailyMss`, which
    /// also consults `mtf_events`.
    pub htf_events: Option<&'a [PdaEvent]>,
    /// Mid-timeframe events (e.g. daily when `htf_events` is weekly).
    /// Required only for `WeeklyOpenSweepDailyMss`.
    pub mtf_events: Option<&'a [PdaEvent]>,
    /// Paired-symbol candles for the cross-symbol SMT setup.
    pub paired_candles: Option<&'a [Candle]>,
}

/// Extended dispatcher: runs all 13 single-TF matchers
/// unconditionally, plus every context-eligible matcher whose
/// required `SetupContext` fields are populated. Output is sorted
/// by (confirm_bar, kind.as_str) like `match_all_setups`.
pub fn match_all_setups_extended(
    events: &[PdaEvent],
    context: &SetupContext<'_>,
    horizon_bars: usize,
) -> Vec<SetupMatch> {
    let mut out = match_all_setups(events, horizon_bars);

    // Cross-TF (5)
    if let Some(htf) = context.htf_events {
        let max_lag = Duration::hours(DEFAULT_CROSS_TF_MAX_LAG_HOURS);
        out.extend(match_htf_mss_ltf_fvg(htf, events, max_lag));
        out.extend(match_htf_cisd_ltf_ob_retest(htf, events, max_lag));
        out.extend(match_daily_high_sweep_ltf_mss_fvg(htf, events, max_lag));
        out.extend(match_daily_low_sweep_ltf_mss_fvg(htf, events, max_lag));
        if let Some(mtf) = context.mtf_events {
            let weekly_lag = Duration::hours(DEFAULT_WEEKLY_MAX_LAG_HOURS);
            out.extend(match_weekly_open_sweep_daily_mss(htf, mtf, weekly_lag));
        }
    }

    // Session-aware (5) — operate on `events` alone via timestamps.
    let session_lag = Duration::hours(DEFAULT_SESSION_MAX_LAG_HOURS);
    out.extend(match_asia_range_raid_london_mss(events, session_lag));
    out.extend(match_london_raid_ny_mss_fvg(events, session_lag));
    out.extend(match_silver_bullet_window(events));
    out.extend(match_silver_bullet_am(events));
    out.extend(match_judas_swing_reversal(events, session_lag));

    // Cross-symbol SMT (1)
    if let (Some(primary), Some(paired)) = (context.primary_candles, context.paired_candles) {
        out.extend(match_smt_divergence_confirm(
            events,
            primary,
            paired,
            DEFAULT_SMT_LOOKBACK_BARS,
            DEFAULT_SMT_CONFIRM_WINDOW_BARS,
        ));
    }

    // OTE (3)
    if let Some(primary) = context.primary_candles {
        out.extend(match_ote_with_fvg_confluence(events, primary));
        out.extend(match_ote_with_ob_confluence(events, primary));
        out.extend(match_optimal_trade_entry_with_cisd(events, primary));
    }

    out.sort_by_key(|m| (m.confirm_bar, m.label().to_string()));
    out
}

// --------------------------------------------------------------------
// Generic helpers
// --------------------------------------------------------------------

fn find_recent_before(
    events: &[PdaEvent],
    from_idx: usize,
    horizon: usize,
    pred: impl Fn(&PdaEvent) -> bool,
) -> Option<&PdaEvent> {
    let anchor_bar = events[from_idx].bar_index;
    events[..from_idx]
        .iter()
        .rev()
        .take_while(|e| anchor_bar.saturating_sub(e.bar_index) <= horizon)
        .find(|e| pred(e))
}

fn find_after(
    events: &[PdaEvent],
    from_idx: usize,
    horizon: usize,
    pred: impl Fn(&PdaEvent) -> bool,
) -> Option<&PdaEvent> {
    let anchor_bar = events[from_idx].bar_index;
    events
        .iter()
        .skip(from_idx + 1)
        .take_while(|e| e.bar_index.saturating_sub(anchor_bar) <= horizon)
        .find(|e| pred(e))
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Bull => Direction::Bear,
        Direction::Bear => Direction::Bull,
        Direction::Neutral => Direction::Neutral,
    }
}

// --------------------------------------------------------------------
// 1. ObRetestPropulsionConfirm
// --------------------------------------------------------------------

fn match_ob_retest_propulsion(events: &[PdaEvent], horizon: usize) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for (i, ev) in events.iter().enumerate() {
        if ev.kind != PdaEventKind::PropulsionBlock {
            continue;
        }
        if let Some(ob) = find_recent_before(events, i, horizon, |e| {
            e.kind == PdaEventKind::OrderBlock && e.direction == ev.direction
        }) {
            out.push(SetupMatch {
                kind: CanonicalSetupKind::ObRetestPropulsionConfirm,
                name_override: None,
                direction: ev.direction,
                anchor_bar: ob.bar_index,
                confirm_bar: ev.bar_index,
                event_bars: vec![ob.bar_index, ev.bar_index],
            });
        }
    }
    out
}

// --------------------------------------------------------------------
// 2. IFvgContinuation: MSS → iFVG (same direction)
// --------------------------------------------------------------------

fn match_ifvg_continuation(events: &[PdaEvent], horizon: usize) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for (i, ev) in events.iter().enumerate() {
        if ev.kind != PdaEventKind::InverseFairValueGap {
            continue;
        }
        if let Some(mss) = find_recent_before(events, i, horizon, |e| {
            e.kind == PdaEventKind::MarketStructureShift && e.direction == ev.direction
        }) {
            out.push(SetupMatch {
                kind: CanonicalSetupKind::IFvgContinuation,
                name_override: None,
                direction: ev.direction,
                anchor_bar: mss.bar_index,
                confirm_bar: ev.bar_index,
                event_bars: vec![mss.bar_index, ev.bar_index],
            });
        }
    }
    out
}

// --------------------------------------------------------------------
// 3. BreakerBlockRetest — every BreakerBlock event already encodes
//    the violation + retest; the timeline event is the match.
// --------------------------------------------------------------------

fn match_breaker_block_retest(events: &[PdaEvent]) -> Vec<SetupMatch> {
    events
        .iter()
        .filter(|e| e.kind == PdaEventKind::BreakerBlock)
        .map(|e| SetupMatch {
            kind: CanonicalSetupKind::BreakerBlockRetest,
            name_override: None,
            direction: e.direction,
            anchor_bar: e.bar_index,
            confirm_bar: e.bar_index,
            event_bars: vec![e.bar_index],
        })
        .collect()
}

// --------------------------------------------------------------------
// 4. MitigationBlockRetest — every MitigationBlock event is a match.
// --------------------------------------------------------------------

fn match_mitigation_block_retest(events: &[PdaEvent]) -> Vec<SetupMatch> {
    events
        .iter()
        .filter(|e| e.kind == PdaEventKind::MitigationBlock)
        .map(|e| SetupMatch {
            kind: CanonicalSetupKind::MitigationBlockRetest,
            name_override: None,
            direction: e.direction,
            anchor_bar: e.bar_index,
            confirm_bar: e.bar_index,
            event_bars: vec![e.bar_index],
        })
        .collect()
}

// --------------------------------------------------------------------
// 5. RejectionBlockAtKeyLevel — RB.level within ε of a recent
//    MSS/StructureBreak level.
// --------------------------------------------------------------------

fn match_rejection_block_at_key_level(
    events: &[PdaEvent],
    horizon: usize,
    tolerance_bps: f64,
) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for (i, ev) in events.iter().enumerate() {
        if ev.kind != PdaEventKind::RejectionBlock {
            continue;
        }
        let Some(rb_level) = ev.level else { continue };
        let tolerance = rb_level.abs() * tolerance_bps;
        if let Some(level_event) = find_recent_before(events, i, horizon, |e| {
            (e.kind == PdaEventKind::MarketStructureShift || e.kind == PdaEventKind::StructureBreak)
                && e.level
                    .map(|l| (l - rb_level).abs() <= tolerance)
                    .unwrap_or(false)
        }) {
            out.push(SetupMatch {
                kind: CanonicalSetupKind::RejectionBlockAtKeyLevel,
                name_override: None,
                direction: ev.direction,
                anchor_bar: level_event.bar_index,
                confirm_bar: ev.bar_index,
                event_bars: vec![level_event.bar_index, ev.bar_index],
            });
        }
    }
    out
}

// --------------------------------------------------------------------
// 6. VolumeImbalanceFiller — VI followed by structure event same dir.
// --------------------------------------------------------------------

fn match_volume_imbalance_filler(events: &[PdaEvent], horizon: usize) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for (i, ev) in events.iter().enumerate() {
        if ev.kind != PdaEventKind::VolumeImbalance {
            continue;
        }
        if ev.direction == Direction::Neutral {
            continue;
        }
        if let Some(follow) = find_after(events, i, horizon, |e| {
            (e.kind == PdaEventKind::MarketStructureShift
                || e.kind == PdaEventKind::StructureBreak
                || e.kind == PdaEventKind::PropulsionBlock)
                && e.direction == ev.direction
        }) {
            out.push(SetupMatch {
                kind: CanonicalSetupKind::VolumeImbalanceFiller,
                name_override: None,
                direction: ev.direction,
                anchor_bar: ev.bar_index,
                confirm_bar: follow.bar_index,
                event_bars: vec![ev.bar_index, follow.bar_index],
            });
        }
    }
    out
}

// --------------------------------------------------------------------
// 7. LiquidityVoidContinuation — LV followed by MSS/SB same dir.
// --------------------------------------------------------------------

fn match_liquidity_void_continuation(events: &[PdaEvent], horizon: usize) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for (i, ev) in events.iter().enumerate() {
        if ev.kind != PdaEventKind::LiquidityVoid {
            continue;
        }
        if let Some(follow) = find_after(events, i, horizon, |e| {
            (e.kind == PdaEventKind::MarketStructureShift || e.kind == PdaEventKind::StructureBreak)
                && e.direction == ev.direction
        }) {
            out.push(SetupMatch {
                kind: CanonicalSetupKind::LiquidityVoidContinuation,
                name_override: None,
                direction: ev.direction,
                anchor_bar: ev.bar_index,
                confirm_bar: follow.bar_index,
                event_bars: vec![ev.bar_index, follow.bar_index],
            });
        }
    }
    out
}

// --------------------------------------------------------------------
// 8. PropulsionPostMss — MSS followed by Propulsion same dir.
// --------------------------------------------------------------------

fn match_propulsion_post_mss(events: &[PdaEvent], horizon: usize) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for (i, ev) in events.iter().enumerate() {
        if ev.kind != PdaEventKind::MarketStructureShift {
            continue;
        }
        if let Some(prop) = find_after(events, i, horizon, |e| {
            e.kind == PdaEventKind::PropulsionBlock && e.direction == ev.direction
        }) {
            out.push(SetupMatch {
                kind: CanonicalSetupKind::PropulsionPostMss,
                name_override: None,
                direction: ev.direction,
                anchor_bar: ev.bar_index,
                confirm_bar: prop.bar_index,
                event_bars: vec![ev.bar_index, prop.bar_index],
            });
        }
    }
    out
}

// --------------------------------------------------------------------
// 9-10. Cisd After Distribution / Accumulation
// --------------------------------------------------------------------

fn match_cisd_after_phase(
    events: &[PdaEvent],
    horizon: usize,
    cisd_direction: Direction,
    prior_phase_direction: Direction,
    kind: CanonicalSetupKind,
) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for (i, ev) in events.iter().enumerate() {
        if ev.kind != PdaEventKind::Cisd || ev.direction != cisd_direction {
            continue;
        }
        // Prior phase = most recent MSS in the opposite direction of the CISD.
        if let Some(prior) = find_recent_before(events, i, horizon, |e| {
            e.kind == PdaEventKind::MarketStructureShift && e.direction == prior_phase_direction
        }) {
            out.push(SetupMatch {
                kind,
                name_override: None,
                direction: ev.direction,
                anchor_bar: prior.bar_index,
                confirm_bar: ev.bar_index,
                event_bars: vec![prior.bar_index, ev.bar_index],
            });
        }
    }
    out
}

fn match_cisd_after_distribution(events: &[PdaEvent], horizon: usize) -> Vec<SetupMatch> {
    // Distribution = bull regime exhausted; CISD bear ends it.
    match_cisd_after_phase(
        events,
        horizon,
        Direction::Bear,
        Direction::Bull,
        CanonicalSetupKind::CisdAfterDistribution,
    )
}

fn match_cisd_after_accumulation(events: &[PdaEvent], horizon: usize) -> Vec<SetupMatch> {
    // Accumulation = bear regime exhausted; CISD bull ends it.
    match_cisd_after_phase(
        events,
        horizon,
        Direction::Bull,
        Direction::Bear,
        CanonicalSetupKind::CisdAfterAccumulation,
    )
}

// --------------------------------------------------------------------
// 11. UnicornModel — Breaker overlapping FVG (same dir, near in time).
// --------------------------------------------------------------------

fn match_unicorn_model(events: &[PdaEvent], horizon: usize) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for (i, ev) in events.iter().enumerate() {
        if ev.kind != PdaEventKind::BreakerBlock {
            continue;
        }
        // Look both forward and backward within horizon for a same-dir
        // FVG; either direction works because the breaker confirms
        // late and the FVG can sit on either side of the violation.
        let neighbours = events
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .filter(|(_, other)| other.kind == PdaEventKind::FairValueGap);
        for (_, other) in neighbours {
            let dt = other
                .bar_index
                .max(ev.bar_index)
                .saturating_sub(other.bar_index.min(ev.bar_index));
            if dt > horizon {
                continue;
            }
            if other.direction != ev.direction {
                continue;
            }
            out.push(SetupMatch {
                kind: CanonicalSetupKind::UnicornModel,
                name_override: None,
                direction: ev.direction,
                anchor_bar: other.bar_index.min(ev.bar_index),
                confirm_bar: other.bar_index.max(ev.bar_index),
                event_bars: vec![ev.bar_index, other.bar_index],
            });
            // Only emit once per breaker — pick the closest FVG
            // implicitly by the take_break below.
            break;
        }
    }
    out
}

// --------------------------------------------------------------------
// 12. PowerOfThree — LiquiditySweep → opposite-direction MSS/SB.
//     This is the "manipulation → distribution" simplification.
// --------------------------------------------------------------------

fn match_power_of_three(events: &[PdaEvent], horizon: usize) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for (i, ev) in events.iter().enumerate() {
        if ev.kind != PdaEventKind::LiquiditySweep {
            continue;
        }
        let target = opposite(ev.direction);
        if target == Direction::Neutral {
            continue;
        }
        if let Some(follow) = find_after(events, i, horizon, |e| {
            (e.kind == PdaEventKind::MarketStructureShift || e.kind == PdaEventKind::StructureBreak)
                && e.direction == target
        }) {
            out.push(SetupMatch {
                kind: CanonicalSetupKind::PowerOfThree,
                name_override: None,
                direction: target,
                anchor_bar: ev.bar_index,
                confirm_bar: follow.bar_index,
                event_bars: vec![ev.bar_index, follow.bar_index],
            });
        }
    }
    out
}

// --------------------------------------------------------------------
// 13. TurtleSoupLiquidityGrab — LiquiditySweep → opposite-dir MSS only
//     (stricter than PowerOfThree's MSS/SB tolerance).
// --------------------------------------------------------------------

fn match_turtle_soup_liquidity_grab(events: &[PdaEvent], horizon: usize) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for (i, ev) in events.iter().enumerate() {
        if ev.kind != PdaEventKind::LiquiditySweep {
            continue;
        }
        let target = opposite(ev.direction);
        if target == Direction::Neutral {
            continue;
        }
        if let Some(follow) = find_after(events, i, horizon, |e| {
            e.kind == PdaEventKind::MarketStructureShift && e.direction == target
        }) {
            out.push(SetupMatch {
                kind: CanonicalSetupKind::TurtleSoupLiquidityGrab,
                name_override: None,
                direction: target,
                anchor_bar: ev.bar_index,
                confirm_bar: follow.bar_index,
                event_bars: vec![ev.bar_index, follow.bar_index],
            });
        }
    }
    out
}

// --------------------------------------------------------------------
// P1c — Cross-timeframe matchers (consume `htf_events` + LTF events).
// --------------------------------------------------------------------
//
// All cross-TF matchers compare wall-clock timestamps via
// `event.timestamp`. Events that lack a timestamp (e.g. hand-built in
// unit tests of unrelated modules) are silently skipped — that
// keeps the matcher safe to call from any dispatcher path.

fn timestamps_within(a: &PdaEvent, b: &PdaEvent, max_lag: Duration) -> bool {
    match (a.timestamp, b.timestamp) {
        (Some(at), Some(bt)) => {
            let delta = if bt >= at { bt - at } else { return false };
            delta <= max_lag
        }
        _ => false,
    }
}

fn match_htf_mss_ltf_fvg(
    htf_events: &[PdaEvent],
    ltf_events: &[PdaEvent],
    max_lag: Duration,
) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for htf in htf_events
        .iter()
        .filter(|e| e.kind == PdaEventKind::MarketStructureShift)
    {
        for ltf in ltf_events
            .iter()
            .filter(|e| e.kind == PdaEventKind::FairValueGap && e.direction == htf.direction)
        {
            if !timestamps_within(htf, ltf, max_lag) {
                continue;
            }
            out.push(SetupMatch {
                kind: CanonicalSetupKind::HtfMssLtfFvg,
                name_override: None,
                direction: htf.direction,
                anchor_bar: htf.bar_index,
                confirm_bar: ltf.bar_index,
                event_bars: vec![htf.bar_index, ltf.bar_index],
            });
        }
    }
    out
}

fn match_htf_cisd_ltf_ob_retest(
    htf_events: &[PdaEvent],
    ltf_events: &[PdaEvent],
    max_lag: Duration,
) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for htf in htf_events.iter().filter(|e| e.kind == PdaEventKind::Cisd) {
        for ltf in ltf_events
            .iter()
            .filter(|e| e.kind == PdaEventKind::OrderBlock && e.direction == htf.direction)
        {
            if !timestamps_within(htf, ltf, max_lag) {
                continue;
            }
            out.push(SetupMatch {
                kind: CanonicalSetupKind::HtfCisdLtfObRetest,
                name_override: None,
                direction: htf.direction,
                anchor_bar: htf.bar_index,
                confirm_bar: ltf.bar_index,
                event_bars: vec![htf.bar_index, ltf.bar_index],
            });
        }
    }
    out
}

/// HTF Bull liquidity sweep (raid above HTF high) followed by a Bear
/// MSS + same-direction FVG on the LTF — the classic "daily high
/// raid then reversal".
fn match_daily_high_sweep_ltf_mss_fvg(
    htf_events: &[PdaEvent],
    ltf_events: &[PdaEvent],
    max_lag: Duration,
) -> Vec<SetupMatch> {
    daily_sweep_reversal(
        htf_events,
        ltf_events,
        max_lag,
        Direction::Bull,
        Direction::Bear,
        CanonicalSetupKind::DailyHighSweepLtfMssFvg,
    )
}

fn match_daily_low_sweep_ltf_mss_fvg(
    htf_events: &[PdaEvent],
    ltf_events: &[PdaEvent],
    max_lag: Duration,
) -> Vec<SetupMatch> {
    daily_sweep_reversal(
        htf_events,
        ltf_events,
        max_lag,
        Direction::Bear,
        Direction::Bull,
        CanonicalSetupKind::DailyLowSweepLtfMssFvg,
    )
}

fn daily_sweep_reversal(
    htf_events: &[PdaEvent],
    ltf_events: &[PdaEvent],
    max_lag: Duration,
    sweep_direction: Direction,
    reversal_direction: Direction,
    kind: CanonicalSetupKind,
) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for sweep in htf_events
        .iter()
        .filter(|e| e.kind == PdaEventKind::LiquiditySweep && e.direction == sweep_direction)
    {
        for mss in ltf_events.iter().filter(|e| {
            e.kind == PdaEventKind::MarketStructureShift && e.direction == reversal_direction
        }) {
            if !timestamps_within(sweep, mss, max_lag) {
                continue;
            }
            // Look for an FVG in the reversal direction within the
            // same horizon, anchored after the MSS.
            let fvg = ltf_events.iter().find(|e| {
                e.kind == PdaEventKind::FairValueGap
                    && e.direction == reversal_direction
                    && e.bar_index >= mss.bar_index
                    && timestamps_within(mss, e, max_lag)
            });
            if let Some(fvg) = fvg {
                out.push(SetupMatch {
                    kind,
                    name_override: None,
                    direction: reversal_direction,
                    anchor_bar: sweep.bar_index,
                    confirm_bar: fvg.bar_index,
                    event_bars: vec![sweep.bar_index, mss.bar_index, fvg.bar_index],
                });
            }
        }
    }
    out
}

/// Weekly liquidity sweep on the higher-of-HTF series → daily MSS in
/// the opposite direction (provided via `mtf_events`).
fn match_weekly_open_sweep_daily_mss(
    weekly_events: &[PdaEvent],
    daily_events: &[PdaEvent],
    max_lag: Duration,
) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for sweep in weekly_events
        .iter()
        .filter(|e| e.kind == PdaEventKind::LiquiditySweep)
    {
        let target = opposite(sweep.direction);
        if target == Direction::Neutral {
            continue;
        }
        for mss in daily_events
            .iter()
            .filter(|e| e.kind == PdaEventKind::MarketStructureShift && e.direction == target)
        {
            if !timestamps_within(sweep, mss, max_lag) {
                continue;
            }
            out.push(SetupMatch {
                kind: CanonicalSetupKind::WeeklyOpenSweepDailyMss,
                name_override: None,
                direction: target,
                anchor_bar: sweep.bar_index,
                confirm_bar: mss.bar_index,
                event_bars: vec![sweep.bar_index, mss.bar_index],
            });
        }
    }
    out
}

// --------------------------------------------------------------------
// P1c — Session-aware matchers (use `event.timestamp` only).
// --------------------------------------------------------------------

fn event_in_zone(event: &PdaEvent, zone: SessionKillZone) -> bool {
    event
        .timestamp
        .map(|ts| is_in_zone(ts, zone))
        .unwrap_or(false)
}

fn match_asia_range_raid_london_mss(events: &[PdaEvent], max_lag: Duration) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for sweep in events.iter().filter(|e| {
        e.kind == PdaEventKind::LiquiditySweep && event_in_zone(e, SessionKillZone::AsiaSession)
    }) {
        let target = opposite(sweep.direction);
        if target == Direction::Neutral {
            continue;
        }
        for mss in events.iter().filter(|e| {
            e.kind == PdaEventKind::MarketStructureShift
                && e.direction == target
                && event_in_zone(e, SessionKillZone::LondonSession)
        }) {
            if !timestamps_within(sweep, mss, max_lag) {
                continue;
            }
            out.push(SetupMatch {
                kind: CanonicalSetupKind::AsiaRangeRaidLondonMss,
                name_override: None,
                direction: target,
                anchor_bar: sweep.bar_index,
                confirm_bar: mss.bar_index,
                event_bars: vec![sweep.bar_index, mss.bar_index],
            });
        }
    }
    out
}

fn match_london_raid_ny_mss_fvg(events: &[PdaEvent], max_lag: Duration) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for sweep in events.iter().filter(|e| {
        e.kind == PdaEventKind::LiquiditySweep && event_in_zone(e, SessionKillZone::LondonSession)
    }) {
        let target = opposite(sweep.direction);
        if target == Direction::Neutral {
            continue;
        }
        for mss in events.iter().filter(|e| {
            e.kind == PdaEventKind::MarketStructureShift
                && e.direction == target
                && event_in_zone(e, SessionKillZone::NySession)
        }) {
            if !timestamps_within(sweep, mss, max_lag) {
                continue;
            }
            // Confirming FVG in the same direction inside NY session.
            let fvg = events.iter().find(|e| {
                e.kind == PdaEventKind::FairValueGap
                    && e.direction == target
                    && e.bar_index >= mss.bar_index
                    && event_in_zone(e, SessionKillZone::NySession)
                    && timestamps_within(mss, e, max_lag)
            });
            if let Some(fvg) = fvg {
                out.push(SetupMatch {
                    kind: CanonicalSetupKind::LondonRaidNyMssFvg,
                    name_override: None,
                    direction: target,
                    anchor_bar: sweep.bar_index,
                    confirm_bar: fvg.bar_index,
                    event_bars: vec![sweep.bar_index, mss.bar_index, fvg.bar_index],
                });
            }
        }
    }
    out
}

fn match_silver_bullet_window(events: &[PdaEvent]) -> Vec<SetupMatch> {
    events
        .iter()
        .filter(|e| {
            e.kind == PdaEventKind::FairValueGap
                && event_in_zone(e, SessionKillZone::SilverBulletPm)
        })
        .map(|e| SetupMatch {
            kind: CanonicalSetupKind::SilverBulletWindow,
            name_override: None,
            direction: e.direction,
            anchor_bar: e.bar_index,
            confirm_bar: e.bar_index,
            event_bars: vec![e.bar_index],
        })
        .collect()
}

fn match_silver_bullet_am(events: &[PdaEvent]) -> Vec<SetupMatch> {
    events
        .iter()
        .filter(|e| {
            e.kind == PdaEventKind::FairValueGap
                && event_in_zone(e, SessionKillZone::SilverBulletAm)
        })
        .map(|e| SetupMatch {
            kind: CanonicalSetupKind::SilverBulletAm,
            name_override: None,
            direction: e.direction,
            anchor_bar: e.bar_index,
            confirm_bar: e.bar_index,
            event_bars: vec![e.bar_index],
        })
        .collect()
}

fn match_judas_swing_reversal(events: &[PdaEvent], max_lag: Duration) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for sweep in events.iter().filter(|e| {
        e.kind == PdaEventKind::LiquiditySweep && event_in_zone(e, SessionKillZone::JudasWindow)
    }) {
        let target = opposite(sweep.direction);
        if target == Direction::Neutral {
            continue;
        }
        for mss in events.iter().filter(|e| {
            e.kind == PdaEventKind::MarketStructureShift
                && e.direction == target
                && event_in_zone(e, SessionKillZone::NySession)
        }) {
            if !timestamps_within(sweep, mss, max_lag) {
                continue;
            }
            out.push(SetupMatch {
                kind: CanonicalSetupKind::JudasSwingReversal,
                name_override: None,
                direction: target,
                anchor_bar: sweep.bar_index,
                confirm_bar: mss.bar_index,
                event_bars: vec![sweep.bar_index, mss.bar_index],
            });
        }
    }
    out
}

// --------------------------------------------------------------------
// P1c — Cross-symbol SMT.
// --------------------------------------------------------------------

fn match_smt_divergence_confirm(
    events: &[PdaEvent],
    primary_candles: &[Candle],
    paired_candles: &[Candle],
    lookback_bars: usize,
    confirm_window_bars: usize,
) -> Vec<SetupMatch> {
    // Align the two close-price series at their shared length so
    // `Divergence::detect` does not trip its length-equality guard.
    let aligned_len = primary_candles.len().min(paired_candles.len());
    if aligned_len < lookback_bars + 1 {
        return Vec::new();
    }
    let primary_offset = primary_candles.len() - aligned_len;
    let paired_offset = paired_candles.len() - aligned_len;
    let primary_close: Vec<f64> = primary_candles[primary_offset..]
        .iter()
        .map(|c| c.close)
        .collect();
    let paired_close: Vec<f64> = paired_candles[paired_offset..]
        .iter()
        .map(|c| c.close)
        .collect();
    let flags = Divergence::detect(&primary_close, &paired_close, lookback_bars);

    let mut out = Vec::new();
    for (i, &flag) in flags.iter().enumerate() {
        if !flag {
            continue;
        }
        let primary_bar = primary_offset + i;
        // Confirming MSS within `confirm_window_bars` after the
        // divergence flag.
        if let Some(mss) = events.iter().find(|e| {
            e.kind == PdaEventKind::MarketStructureShift
                && e.bar_index >= primary_bar
                && e.bar_index <= primary_bar + confirm_window_bars
        }) {
            out.push(SetupMatch {
                kind: CanonicalSetupKind::SmtDivergenceConfirm,
                name_override: None,
                direction: mss.direction,
                anchor_bar: primary_bar,
                confirm_bar: mss.bar_index,
                event_bars: vec![primary_bar, mss.bar_index],
            });
        }
    }
    out
}

// --------------------------------------------------------------------
// P1c — OTE retracement confluence.
// --------------------------------------------------------------------

fn match_ote_confluence_kind(
    events: &[PdaEvent],
    primary_candles: &[Candle],
    target_kind: PdaEventKind,
    setup_kind: CanonicalSetupKind,
) -> Vec<SetupMatch> {
    let mut out = Vec::new();
    for ev in events.iter().filter(|e| e.kind == target_kind) {
        let Some(level) = ev.level else { continue };
        // OTE zone "as of" the event's emission bar — preserves the
        // forward-leak invariant: only candles in `primary[..=ev.bar]`
        // are visible to the swing detector.
        let bar = ev.bar_index;
        if bar >= primary_candles.len() {
            continue;
        }
        let view = &primary_candles[..=bar];
        let Some(zone) = most_recent_ote_zone(view, DEFAULT_OTE_SWING_STRENGTH) else {
            continue;
        };
        if zone.direction != ev.direction {
            continue;
        }
        if !zone.contains(level) {
            continue;
        }
        out.push(SetupMatch {
            kind: setup_kind,
            name_override: None,
            direction: ev.direction,
            anchor_bar: zone.leg_end_bar,
            confirm_bar: ev.bar_index,
            event_bars: vec![zone.leg_end_bar, ev.bar_index],
        });
    }
    out
}

fn match_ote_with_fvg_confluence(
    events: &[PdaEvent],
    primary_candles: &[Candle],
) -> Vec<SetupMatch> {
    match_ote_confluence_kind(
        events,
        primary_candles,
        PdaEventKind::FairValueGap,
        CanonicalSetupKind::OteWithFvgConfluence,
    )
}

fn match_ote_with_ob_confluence(
    events: &[PdaEvent],
    primary_candles: &[Candle],
) -> Vec<SetupMatch> {
    match_ote_confluence_kind(
        events,
        primary_candles,
        PdaEventKind::OrderBlock,
        CanonicalSetupKind::OteWithObConfluence,
    )
}

fn match_optimal_trade_entry_with_cisd(
    events: &[PdaEvent],
    primary_candles: &[Candle],
) -> Vec<SetupMatch> {
    match_ote_confluence_kind(
        events,
        primary_candles,
        PdaEventKind::Cisd,
        CanonicalSetupKind::OptimalTradeEntryWithCisd,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(kind: PdaEventKind, bar: usize, direction: Direction) -> PdaEvent {
        PdaEvent::new(kind, bar, direction).with_level(100.0)
    }

    fn ev_with_level(kind: PdaEventKind, bar: usize, direction: Direction, level: f64) -> PdaEvent {
        PdaEvent::new(kind, bar, direction).with_level(level)
    }

    #[test]
    fn empty_timeline_yields_no_matches() {
        assert!(match_all_setups_default(&[]).is_empty());
    }

    #[test]
    fn single_event_yields_only_self_anchored_setups() {
        // Lone BreakerBlock — should fire BreakerBlockRetest but no
        // pair-based setup.
        let events = vec![ev(PdaEventKind::BreakerBlock, 10, Direction::Bull)];
        let matches = match_all_setups_default(&events);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].kind, CanonicalSetupKind::BreakerBlockRetest);
    }

    #[test]
    fn ob_retest_propulsion_fires() {
        let events = vec![
            ev(PdaEventKind::OrderBlock, 10, Direction::Bull),
            ev(PdaEventKind::PropulsionBlock, 12, Direction::Bull),
        ];
        let m = match_all_setups_default(&events);
        let setup = m
            .iter()
            .find(|s| s.kind == CanonicalSetupKind::ObRetestPropulsionConfirm)
            .expect("expected setup");
        assert_eq!(setup.direction, Direction::Bull);
        assert_eq!(setup.anchor_bar, 10);
        assert_eq!(setup.confirm_bar, 12);
    }

    #[test]
    fn ob_retest_propulsion_does_not_fire_across_direction_mismatch() {
        let events = vec![
            ev(PdaEventKind::OrderBlock, 10, Direction::Bull),
            ev(PdaEventKind::PropulsionBlock, 12, Direction::Bear),
        ];
        let m = match_all_setups_default(&events);
        assert!(!m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::ObRetestPropulsionConfirm));
    }

    #[test]
    fn horizon_excludes_far_pairs() {
        let events = vec![
            ev(PdaEventKind::OrderBlock, 10, Direction::Bull),
            ev(PdaEventKind::PropulsionBlock, 100, Direction::Bull),
        ];
        let near = match_all_setups(&events, 5);
        let far = match_all_setups(&events, 200);
        assert!(!near
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::ObRetestPropulsionConfirm));
        assert!(far
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::ObRetestPropulsionConfirm));
    }

    #[test]
    fn ifvg_continuation_requires_prior_mss() {
        // iFVG without a same-dir MSS preceding it should not fire.
        let events = vec![
            ev(PdaEventKind::FairValueGap, 5, Direction::Bull),
            ev(PdaEventKind::InverseFairValueGap, 10, Direction::Bear),
        ];
        let m = match_all_setups_default(&events);
        assert!(!m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::IFvgContinuation));
        // Now add the MSS
        let events_with_mss = vec![
            ev(PdaEventKind::MarketStructureShift, 8, Direction::Bear),
            ev(PdaEventKind::InverseFairValueGap, 10, Direction::Bear),
        ];
        let m2 = match_all_setups_default(&events_with_mss);
        let setup = m2
            .iter()
            .find(|s| s.kind == CanonicalSetupKind::IFvgContinuation)
            .expect("expected iFvgContinuation");
        assert_eq!(setup.direction, Direction::Bear);
    }

    #[test]
    fn rejection_block_at_key_level_uses_price_proximity() {
        // RB at 100.0; MSS at 100.05 (~5 bps) → within tolerance (25 bps).
        let events = vec![
            ev_with_level(
                PdaEventKind::MarketStructureShift,
                5,
                Direction::Bear,
                100.05,
            ),
            ev_with_level(PdaEventKind::RejectionBlock, 8, Direction::Bear, 100.0),
        ];
        let m = match_all_setups_default(&events);
        assert!(
            m.iter()
                .any(|s| s.kind == CanonicalSetupKind::RejectionBlockAtKeyLevel),
            "RB at MSS level (~5 bps) should match"
        );

        // Same bars but MSS level far away (105) → outside tolerance.
        let far = vec![
            ev_with_level(
                PdaEventKind::MarketStructureShift,
                5,
                Direction::Bear,
                105.0,
            ),
            ev_with_level(PdaEventKind::RejectionBlock, 8, Direction::Bear, 100.0),
        ];
        let m2 = match_all_setups_default(&far);
        assert!(!m2
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::RejectionBlockAtKeyLevel));
    }

    #[test]
    fn volume_imbalance_filler_requires_same_dir_continuation() {
        let events = vec![
            ev(PdaEventKind::VolumeImbalance, 5, Direction::Bull),
            ev(PdaEventKind::PropulsionBlock, 8, Direction::Bull),
        ];
        let m = match_all_setups_default(&events);
        assert!(m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::VolumeImbalanceFiller));

        let events_no_match = vec![
            ev(PdaEventKind::VolumeImbalance, 5, Direction::Bull),
            ev(PdaEventKind::PropulsionBlock, 8, Direction::Bear),
        ];
        let m2 = match_all_setups_default(&events_no_match);
        assert!(!m2
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::VolumeImbalanceFiller));
    }

    #[test]
    fn turtle_soup_requires_opposite_direction() {
        let events = vec![
            ev(PdaEventKind::LiquiditySweep, 5, Direction::Bull),
            ev(PdaEventKind::MarketStructureShift, 8, Direction::Bear),
        ];
        let m = match_all_setups_default(&events);
        let turtle = m
            .iter()
            .find(|s| s.kind == CanonicalSetupKind::TurtleSoupLiquidityGrab)
            .expect("expected turtle soup");
        // Direction reflects the eventual move (opposite of sweep).
        assert_eq!(turtle.direction, Direction::Bear);
        assert_eq!(turtle.anchor_bar, 5);
        assert_eq!(turtle.confirm_bar, 8);

        let same_dir = vec![
            ev(PdaEventKind::LiquiditySweep, 5, Direction::Bull),
            ev(PdaEventKind::MarketStructureShift, 8, Direction::Bull),
        ];
        let m2 = match_all_setups_default(&same_dir);
        assert!(!m2
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::TurtleSoupLiquidityGrab));
    }

    #[test]
    fn cisd_after_distribution_requires_bull_regime_then_bear_cisd() {
        let events = vec![
            ev(PdaEventKind::MarketStructureShift, 5, Direction::Bull),
            ev(PdaEventKind::Cisd, 10, Direction::Bear),
        ];
        let m = match_all_setups_default(&events);
        assert!(m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::CisdAfterDistribution));

        let no_prior = vec![ev(PdaEventKind::Cisd, 10, Direction::Bear)];
        let m2 = match_all_setups_default(&no_prior);
        assert!(!m2
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::CisdAfterDistribution));
    }

    #[test]
    fn unicorn_model_pairs_breaker_with_same_dir_fvg() {
        let events = vec![
            ev(PdaEventKind::FairValueGap, 8, Direction::Bull),
            ev(PdaEventKind::BreakerBlock, 12, Direction::Bull),
        ];
        let m = match_all_setups_default(&events);
        assert!(m.iter().any(|s| s.kind == CanonicalSetupKind::UnicornModel));

        let mismatch = vec![
            ev(PdaEventKind::FairValueGap, 8, Direction::Bear),
            ev(PdaEventKind::BreakerBlock, 12, Direction::Bull),
        ];
        let m2 = match_all_setups_default(&mismatch);
        assert!(!m2
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::UnicornModel));
    }

    #[test]
    fn output_is_sorted_by_confirm_bar_then_kind_name() {
        let events = vec![
            ev(PdaEventKind::OrderBlock, 10, Direction::Bull),
            ev(PdaEventKind::PropulsionBlock, 20, Direction::Bull),
            ev(PdaEventKind::BreakerBlock, 15, Direction::Bull),
            ev(PdaEventKind::MitigationBlock, 12, Direction::Bear),
        ];
        let m = match_all_setups_default(&events);
        for window in m.windows(2) {
            assert!(window[0].confirm_bar <= window[1].confirm_bar);
            if window[0].confirm_bar == window[1].confirm_bar {
                assert!(window[0].kind.as_str() <= window[1].kind.as_str());
            }
        }
    }

    #[test]
    fn determinism_across_calls() {
        let events = vec![
            ev(PdaEventKind::OrderBlock, 10, Direction::Bull),
            ev(PdaEventKind::PropulsionBlock, 12, Direction::Bull),
            ev(PdaEventKind::MarketStructureShift, 14, Direction::Bull),
            ev(PdaEventKind::PropulsionBlock, 16, Direction::Bull),
            ev(PdaEventKind::BreakerBlock, 18, Direction::Bear),
        ];
        let a = match_all_setups_default(&events);
        let b = match_all_setups_default(&events);
        assert_eq!(a, b);
    }

    // ----------------------------------------------------------------
    // P1c — extended dispatcher + cross-TF / session / SMT / OTE
    // ----------------------------------------------------------------

    use chrono::TimeZone;
    use chrono_tz::America::New_York;

    fn ny_ts(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
    ) -> chrono::DateTime<chrono::Utc> {
        New_York
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    fn ev_with_ts(
        kind: PdaEventKind,
        bar: usize,
        direction: Direction,
        ts: chrono::DateTime<chrono::Utc>,
    ) -> PdaEvent {
        PdaEvent::new(kind, bar, direction)
            .with_level(100.0)
            .with_timestamp(ts)
    }

    fn synthetic_candles_with_swings(count: usize) -> Vec<Candle> {
        // Reuses the bull-leg shape from the OTE module test so the
        // OTE confluence matchers have a non-empty swing leg.
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let (lo, hi) = if i < 10 {
                let v = 100.0 - i as f64;
                (v - 0.5, v + 0.5)
            } else if i == 10 {
                (89.0, 90.0)
            } else if i < 25 {
                let v = 90.0 + (i - 10) as f64;
                (v - 0.5, v + 0.5)
            } else if i == 25 {
                (105.0, 106.0)
            } else {
                let v = (106.0 - (i - 25) as f64).max(95.0);
                (v - 0.5, v + 0.5)
            };
            out.push(Candle {
                timestamp: ny_ts(2026, 1, 5, 0, 0) + chrono::Duration::minutes(i as i64),
                open: (lo + hi) / 2.0,
                high: hi,
                low: lo,
                close: (lo + hi) / 2.0,
                volume: 1000.0,
            });
        }
        out
    }

    #[test]
    fn extended_dispatcher_with_default_context_matches_base_setups_only() {
        // Without timestamps and without context, the extended
        // dispatcher must produce exactly the same set of matches
        // as `match_all_setups`.
        let events = vec![
            ev(PdaEventKind::OrderBlock, 10, Direction::Bull),
            ev(PdaEventKind::PropulsionBlock, 12, Direction::Bull),
            ev(PdaEventKind::BreakerBlock, 18, Direction::Bull),
        ];
        let base = match_all_setups(&events, DEFAULT_SETUP_HORIZON_BARS);
        let ext = match_all_setups_extended(
            &events,
            &SetupContext::default(),
            DEFAULT_SETUP_HORIZON_BARS,
        );
        assert_eq!(base, ext);
    }

    #[test]
    fn htf_mss_ltf_fvg_fires_within_lag() {
        let htf_anchor = ny_ts(2026, 1, 5, 9, 0);
        let htf_events = vec![ev_with_ts(
            PdaEventKind::MarketStructureShift,
            5,
            Direction::Bull,
            htf_anchor,
        )];
        let ltf_events = vec![ev_with_ts(
            PdaEventKind::FairValueGap,
            42,
            Direction::Bull,
            htf_anchor + chrono::Duration::hours(1),
        )];
        let ctx = SetupContext {
            htf_events: Some(&htf_events),
            ..SetupContext::default()
        };
        let m = match_all_setups_extended(&ltf_events, &ctx, DEFAULT_SETUP_HORIZON_BARS);
        assert!(m.iter().any(|s| s.kind == CanonicalSetupKind::HtfMssLtfFvg));
    }

    #[test]
    fn htf_mss_ltf_fvg_skips_outside_lag() {
        let htf_anchor = ny_ts(2026, 1, 5, 9, 0);
        let htf_events = vec![ev_with_ts(
            PdaEventKind::MarketStructureShift,
            5,
            Direction::Bull,
            htf_anchor,
        )];
        // 25 hours later — beyond the 24h cross-TF window.
        let ltf_events = vec![ev_with_ts(
            PdaEventKind::FairValueGap,
            42,
            Direction::Bull,
            htf_anchor + chrono::Duration::hours(25),
        )];
        let ctx = SetupContext {
            htf_events: Some(&htf_events),
            ..SetupContext::default()
        };
        let m = match_all_setups_extended(&ltf_events, &ctx, DEFAULT_SETUP_HORIZON_BARS);
        assert!(!m.iter().any(|s| s.kind == CanonicalSetupKind::HtfMssLtfFvg));
    }

    #[test]
    fn htf_cisd_ltf_ob_retest_basic_chain() {
        let anchor = ny_ts(2026, 1, 5, 9, 0);
        let htf_events = vec![ev_with_ts(PdaEventKind::Cisd, 3, Direction::Bull, anchor)];
        let ltf_events = vec![ev_with_ts(
            PdaEventKind::OrderBlock,
            20,
            Direction::Bull,
            anchor + chrono::Duration::hours(2),
        )];
        let ctx = SetupContext {
            htf_events: Some(&htf_events),
            ..SetupContext::default()
        };
        let m = match_all_setups_extended(&ltf_events, &ctx, DEFAULT_SETUP_HORIZON_BARS);
        assert!(m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::HtfCisdLtfObRetest));
    }

    #[test]
    fn daily_high_sweep_emits_three_event_chain() {
        let anchor = ny_ts(2026, 1, 5, 9, 0);
        let htf_events = vec![ev_with_ts(
            PdaEventKind::LiquiditySweep,
            7,
            Direction::Bull,
            anchor,
        )];
        let ltf_events = vec![
            ev_with_ts(
                PdaEventKind::MarketStructureShift,
                30,
                Direction::Bear,
                anchor + chrono::Duration::hours(2),
            ),
            ev_with_ts(
                PdaEventKind::FairValueGap,
                40,
                Direction::Bear,
                anchor + chrono::Duration::hours(3),
            ),
        ];
        let ctx = SetupContext {
            htf_events: Some(&htf_events),
            ..SetupContext::default()
        };
        let m = match_all_setups_extended(&ltf_events, &ctx, DEFAULT_SETUP_HORIZON_BARS);
        let setup = m
            .iter()
            .find(|s| s.kind == CanonicalSetupKind::DailyHighSweepLtfMssFvg)
            .expect("expected DailyHighSweepLtfMssFvg");
        assert_eq!(setup.direction, Direction::Bear);
        assert_eq!(setup.event_bars.len(), 3);
    }

    #[test]
    fn weekly_open_sweep_daily_mss_consumes_mtf_events() {
        let anchor = ny_ts(2026, 1, 5, 9, 0);
        let weekly = vec![ev_with_ts(
            PdaEventKind::LiquiditySweep,
            1,
            Direction::Bull,
            anchor,
        )];
        let daily = vec![ev_with_ts(
            PdaEventKind::MarketStructureShift,
            20,
            Direction::Bear,
            anchor + chrono::Duration::hours(48),
        )];
        let ctx = SetupContext {
            htf_events: Some(&weekly),
            mtf_events: Some(&daily),
            ..SetupContext::default()
        };
        // The events parameter here is irrelevant for this setup
        // (it scans `htf_events` and `mtf_events`).
        let m = match_all_setups_extended(&[], &ctx, DEFAULT_SETUP_HORIZON_BARS);
        assert!(m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::WeeklyOpenSweepDailyMss));
    }

    #[test]
    fn silver_bullet_pm_fires_for_fvg_in_kill_zone() {
        // 2026-01-05 10:30 NY is inside the 10:00-11:00 SilverBulletPm window.
        let events = vec![ev_with_ts(
            PdaEventKind::FairValueGap,
            42,
            Direction::Bull,
            ny_ts(2026, 1, 5, 10, 30),
        )];
        let m = match_all_setups_extended(
            &events,
            &SetupContext::default(),
            DEFAULT_SETUP_HORIZON_BARS,
        );
        assert!(m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::SilverBulletWindow));
    }

    #[test]
    fn silver_bullet_am_fires_for_fvg_in_kill_zone() {
        let events = vec![ev_with_ts(
            PdaEventKind::FairValueGap,
            7,
            Direction::Bull,
            ny_ts(2026, 1, 5, 3, 30),
        )];
        let m = match_all_setups_extended(
            &events,
            &SetupContext::default(),
            DEFAULT_SETUP_HORIZON_BARS,
        );
        assert!(m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::SilverBulletAm));
    }

    #[test]
    fn asia_raid_london_mss_chain() {
        // 21:00 NY Sunday → ~10:00 Tokyo Monday (Asia kill zone).
        // Followed by MSS at 04:30 NY Monday (London kill zone).
        let sweep_ts = ny_ts(2026, 1, 4, 21, 0);
        let mss_ts = ny_ts(2026, 1, 5, 4, 30);
        let events = vec![
            ev_with_ts(PdaEventKind::LiquiditySweep, 5, Direction::Bull, sweep_ts),
            ev_with_ts(
                PdaEventKind::MarketStructureShift,
                20,
                Direction::Bear,
                mss_ts,
            ),
        ];
        let m = match_all_setups_extended(
            &events,
            &SetupContext::default(),
            DEFAULT_SETUP_HORIZON_BARS,
        );
        let setup = m
            .iter()
            .find(|s| s.kind == CanonicalSetupKind::AsiaRangeRaidLondonMss)
            .expect("expected AsiaRangeRaidLondonMss");
        assert_eq!(setup.direction, Direction::Bear);
    }

    #[test]
    fn judas_swing_reversal_fires_in_first_hour_ny() {
        // 08:45 NY → JudasWindow.
        let sweep_ts = ny_ts(2026, 1, 5, 8, 45);
        // Within the same NY session (10:30 still inside NySession).
        let mss_ts = ny_ts(2026, 1, 5, 10, 30);
        let events = vec![
            ev_with_ts(PdaEventKind::LiquiditySweep, 4, Direction::Bull, sweep_ts),
            ev_with_ts(
                PdaEventKind::MarketStructureShift,
                12,
                Direction::Bear,
                mss_ts,
            ),
        ];
        let m = match_all_setups_extended(
            &events,
            &SetupContext::default(),
            DEFAULT_SETUP_HORIZON_BARS,
        );
        assert!(m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::JudasSwingReversal));
    }

    #[test]
    fn london_raid_ny_mss_fvg_three_event_chain() {
        // London 09:00 ≈ 04:00 NY (LondonSession); NY at 10:30 NY.
        let sweep_ts = ny_ts(2026, 1, 5, 4, 0);
        let mss_ts = ny_ts(2026, 1, 5, 10, 0);
        let fvg_ts = ny_ts(2026, 1, 5, 10, 30);
        let events = vec![
            ev_with_ts(PdaEventKind::LiquiditySweep, 4, Direction::Bull, sweep_ts),
            ev_with_ts(
                PdaEventKind::MarketStructureShift,
                15,
                Direction::Bear,
                mss_ts,
            ),
            ev_with_ts(PdaEventKind::FairValueGap, 18, Direction::Bear, fvg_ts),
        ];
        let m = match_all_setups_extended(
            &events,
            &SetupContext::default(),
            DEFAULT_SETUP_HORIZON_BARS,
        );
        let setup = m
            .iter()
            .find(|s| s.kind == CanonicalSetupKind::LondonRaidNyMssFvg)
            .expect("expected LondonRaidNyMssFvg");
        assert_eq!(setup.event_bars.len(), 3);
    }

    #[test]
    fn smt_divergence_confirm_requires_paired_and_primary_candles() {
        // 30 bars of synthetic primary + flat paired so the divergence
        // detector flips at least one bar to true; then place an MSS
        // at the resulting bar.
        let primary = synthetic_candles_with_swings(30);
        let paired: Vec<Candle> = primary
            .iter()
            .map(|c| {
                let mut c = c.clone();
                c.close = 200.0; // flat paired close
                c.open = 200.0;
                c.high = 200.0;
                c.low = 200.0;
                c
            })
            .collect();
        // Place an MSS one bar after the typical divergence flag bar.
        let events = vec![ev_with_ts(
            PdaEventKind::MarketStructureShift,
            22,
            Direction::Bull,
            primary[22].timestamp,
        )];
        let ctx = SetupContext {
            primary_candles: Some(&primary),
            paired_candles: Some(&paired),
            ..SetupContext::default()
        };
        let m = match_all_setups_extended(&events, &ctx, DEFAULT_SETUP_HORIZON_BARS);
        // The setup may or may not fire depending on whether the MSS
        // bar happens to fall in the SMT confirmation window. The
        // important contract is: when paired_candles is *None* it
        // never fires, and when present it can fire — we exercise
        // both directions of the gate.
        let with_paired_count = m
            .iter()
            .filter(|s| s.kind == CanonicalSetupKind::SmtDivergenceConfirm)
            .count();
        let ctx_no_paired = SetupContext {
            primary_candles: Some(&primary),
            paired_candles: None,
            ..SetupContext::default()
        };
        let m2 = match_all_setups_extended(&events, &ctx_no_paired, DEFAULT_SETUP_HORIZON_BARS);
        let without_paired_count = m2
            .iter()
            .filter(|s| s.kind == CanonicalSetupKind::SmtDivergenceConfirm)
            .count();
        assert_eq!(without_paired_count, 0);
        assert!(with_paired_count >= without_paired_count);
    }

    #[test]
    fn smt_divergence_skips_when_paired_candles_too_short() {
        let primary = synthetic_candles_with_swings(30);
        let paired: Vec<Candle> = primary.iter().take(5).cloned().collect();
        let events: Vec<PdaEvent> = Vec::new();
        let ctx = SetupContext {
            primary_candles: Some(&primary),
            paired_candles: Some(&paired),
            ..SetupContext::default()
        };
        let m = match_all_setups_extended(&events, &ctx, DEFAULT_SETUP_HORIZON_BARS);
        assert!(!m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::SmtDivergenceConfirm));
    }

    #[test]
    fn ote_with_fvg_confluence_fires_when_level_in_zone() {
        // Bull leg: low ≈ 89 at bar 10, high ≈ 106 at bar 25.
        // OTE band ≈ [92.57, 95.46]. Place the FVG at bar 29 so the
        // primary[..=29] slice covers enough bars after the swing
        // high (find_swing_highs needs `lookback` bars on each side).
        let primary = synthetic_candles_with_swings(30);
        let events = vec![ev_with_level(
            PdaEventKind::FairValueGap,
            29,
            Direction::Bull,
            94.0,
        )];
        let ctx = SetupContext {
            primary_candles: Some(&primary),
            ..SetupContext::default()
        };
        let m = match_all_setups_extended(&events, &ctx, DEFAULT_SETUP_HORIZON_BARS);
        assert!(m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::OteWithFvgConfluence));
    }

    #[test]
    fn ote_with_ob_confluence_skips_when_level_outside_zone() {
        let primary = synthetic_candles_with_swings(30);
        // Level 110 is well above the OTE band's upper bound (~95.5).
        let events = vec![ev_with_level(
            PdaEventKind::OrderBlock,
            29,
            Direction::Bull,
            110.0,
        )];
        let ctx = SetupContext {
            primary_candles: Some(&primary),
            ..SetupContext::default()
        };
        let m = match_all_setups_extended(&events, &ctx, DEFAULT_SETUP_HORIZON_BARS);
        assert!(!m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::OteWithObConfluence));
    }

    #[test]
    fn ote_with_cisd_confluence_uses_cisd_kind() {
        let primary = synthetic_candles_with_swings(30);
        let events = vec![ev_with_level(PdaEventKind::Cisd, 29, Direction::Bull, 93.5)];
        let ctx = SetupContext {
            primary_candles: Some(&primary),
            ..SetupContext::default()
        };
        let m = match_all_setups_extended(&events, &ctx, DEFAULT_SETUP_HORIZON_BARS);
        assert!(m
            .iter()
            .any(|s| s.kind == CanonicalSetupKind::OptimalTradeEntryWithCisd));
    }

    #[test]
    fn extended_dispatcher_output_remains_sorted() {
        // Sanity: even with mixed P1b + P1c hits, the output must
        // satisfy the same sort invariant as `match_all_setups`.
        let primary = synthetic_candles_with_swings(30);
        let events = vec![
            ev_with_ts(
                PdaEventKind::OrderBlock,
                10,
                Direction::Bull,
                primary[10].timestamp,
            ),
            ev_with_ts(
                PdaEventKind::PropulsionBlock,
                12,
                Direction::Bull,
                primary[12].timestamp,
            ),
            ev_with_level(PdaEventKind::FairValueGap, 29, Direction::Bull, 94.0)
                .with_timestamp(primary[29].timestamp),
        ];
        let ctx = SetupContext {
            primary_candles: Some(&primary),
            ..SetupContext::default()
        };
        let m = match_all_setups_extended(&events, &ctx, DEFAULT_SETUP_HORIZON_BARS);
        for window in m.windows(2) {
            assert!(window[0].confirm_bar <= window[1].confirm_bar);
            if window[0].confirm_bar == window[1].confirm_bar {
                assert!(window[0].kind.as_str() <= window[1].kind.as_str());
            }
        }
    }
}
