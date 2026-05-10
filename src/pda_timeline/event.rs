//! Unified PDA event vocabulary used by the timeline assembler.
//!
//! `PdaEventKind` is intentionally **separate** from
//! `crate::pda_sequence::token::PdaTokenKind`. Adding kinds to
//! `PdaTokenKind` is documented as a breaking change to the
//! clustering fixture (it shifts the DTW cost surface). The timeline
//! consumer (canonical-setup matchers, co-occurrence / precedence
//! matrices) needs richer granularity, so it owns its own enum.
//!
//! Each variant maps to exactly one detector in `crate::ict::*`.
//!
//! `bar_index` on `PdaEvent` is the **emission bar** — the latest
//! candle required for the detector to confirm the event. Forward-
//! only consumers can filter on `event.bar_index <= current_bar`
//! without touching detector internals.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::Direction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PdaEventKind {
    FairValueGap,
    InverseFairValueGap,
    OrderBlock,
    BreakerBlock,
    MitigationBlock,
    PropulsionBlock,
    RejectionBlock,
    LiquiditySweep,
    LiquidityVoid,
    StructureBreak,
    MarketStructureShift,
    Cisd,
    VolumeImbalance,
}

impl PdaEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FairValueGap => "fair_value_gap",
            Self::InverseFairValueGap => "inverse_fair_value_gap",
            Self::OrderBlock => "order_block",
            Self::BreakerBlock => "breaker_block",
            Self::MitigationBlock => "mitigation_block",
            Self::PropulsionBlock => "propulsion_block",
            Self::RejectionBlock => "rejection_block",
            Self::LiquiditySweep => "liquidity_sweep",
            Self::LiquidityVoid => "liquidity_void",
            Self::StructureBreak => "structure_break",
            Self::MarketStructureShift => "market_structure_shift",
            Self::Cisd => "cisd",
            Self::VolumeImbalance => "volume_imbalance",
        }
    }
}

/// Canonical iteration order over all event kinds. Matrix layouts
/// produced by `crate::pda_timeline::matrices` use this order, so
/// downstream code can index them deterministically.
pub const ALL_EVENT_KINDS: [PdaEventKind; 13] = [
    PdaEventKind::FairValueGap,
    PdaEventKind::InverseFairValueGap,
    PdaEventKind::OrderBlock,
    PdaEventKind::BreakerBlock,
    PdaEventKind::MitigationBlock,
    PdaEventKind::PropulsionBlock,
    PdaEventKind::RejectionBlock,
    PdaEventKind::LiquiditySweep,
    PdaEventKind::LiquidityVoid,
    PdaEventKind::StructureBreak,
    PdaEventKind::MarketStructureShift,
    PdaEventKind::Cisd,
    PdaEventKind::VolumeImbalance,
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdaEvent {
    pub kind: PdaEventKind,
    /// Latest candle required to confirm this event.
    pub bar_index: usize,
    pub direction: Direction,
    /// Representative price level (e.g. FVG midpoint, OB midpoint,
    /// sweep pool price). `None` only when the event has no natural
    /// price anchor (currently never — every emitted event provides
    /// a level).
    pub level: Option<f64>,
    /// Wall-clock timestamp of the emission bar. `None` for events
    /// constructed by hand in unit tests; the production builder
    /// (`super::builder::build_pda_timeline`) always populates this
    /// from `candles[bar_index].timestamp`. Cross-timeframe and
    /// session-aware setup matchers require this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

impl PdaEvent {
    pub fn new(kind: PdaEventKind, bar_index: usize, direction: Direction) -> Self {
        Self {
            kind,
            bar_index,
            direction,
            level: None,
            timestamp: None,
        }
    }

    pub fn with_level(mut self, level: f64) -> Self {
        self.level = Some(level);
        self
    }

    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_event_kinds_is_complete_and_unique() {
        // Sanity: ALL_EVENT_KINDS must contain each variant exactly
        // once. Using `as_str` as a hashable surrogate.
        let mut names: Vec<&'static str> = ALL_EVENT_KINDS.iter().map(|k| k.as_str()).collect();
        names.sort_unstable();
        let len_before = names.len();
        names.dedup();
        assert_eq!(
            names.len(),
            len_before,
            "ALL_EVENT_KINDS must not contain duplicates"
        );
        assert_eq!(len_before, 13);
    }

    #[test]
    fn event_round_trips_through_serde() {
        let event =
            PdaEvent::new(PdaEventKind::FairValueGap, 12, Direction::Bull).with_level(101.5);
        let json = serde_json::to_string(&event).unwrap();
        let back: PdaEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, back);
    }
}
