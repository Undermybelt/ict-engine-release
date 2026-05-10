//! Wire format for Auto-Quant live factor signal envelopes.
//!
//! The matching producer is
//! `@/Users/thrill3r/Auto-Quant/auto_quant_live_signal_publisher.py`.
//! Both sides must agree on `SCHEMA_VERSION`. Any drift fails the
//! consumer parse loudly so the operator notices before evidence
//! flows into the BBN.

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

use crate::factor_lab::FactorContribution;
use crate::types::Direction;

/// Wire-schema version. Both publisher and consumer must agree.
pub const SCHEMA_VERSION: &str = "1.0";

/// Stream-key prefix on Redis. The full key is
/// `<prefix>:<lowercased_symbol>`.
pub const STREAM_KEY_PREFIX: &str = "auto_quant:factor_signals";

/// Single field of each XADD entry. Value is the JSON-encoded
/// `LiveFactorSignalEnvelope`.
pub const ENVELOPE_FIELD: &str = "payload";

/// Envelope carrying one or more factor contributions emitted by a
/// single Auto-Quant strategy on a single bar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveFactorSignalEnvelope {
    pub schema_version: String,
    pub symbol: String,
    pub timestamp_ms: i64,
    pub auto_quant_run_id: String,
    pub strategy_name: String,
    #[serde(default)]
    pub strategy_mutation_id: String,
    pub bar_close_ts_ms: i64,
    pub contributions: Vec<LiveFactorContribution>,
}

/// Wire-format twin of [`crate::factor_lab::FactorContribution`].
///
/// We do **not** reuse `FactorContribution` directly across the wire
/// because the Rust struct serialises `direction` as a Rust enum
/// variant and we want a stable string representation that the
/// Python publisher can produce without reflection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveFactorContribution {
    pub factor_name: String,
    pub category: String,
    pub direction: String,
    pub value: f64,
    pub confidence: f64,
    pub weighted_score: f64,
    pub uncertainty_contribution: f64,
    #[serde(default)]
    pub explanation: String,
}

impl LiveFactorSignalEnvelope {
    /// Parse + validate a UTF-8 JSON document.
    ///
    /// Returns a typed error including the failing field/value so
    /// the consumer can log a precise diagnostic before dropping the
    /// entry.
    pub fn from_json(raw: &str) -> Result<Self> {
        let env: Self =
            serde_json::from_str(raw).map_err(|e| anyhow!("failed to parse envelope: {e}"))?;
        env.validate()?;
        Ok(env)
    }

    /// Serialise to the canonical compact JSON form used on the wire.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| anyhow!("failed to serialise envelope: {e}"))
    }

    /// Reject envelopes that would silently corrupt downstream state.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            bail!(
                "unsupported schema_version '{}'; consumer expects '{}'",
                self.schema_version,
                SCHEMA_VERSION
            );
        }
        if self.symbol.is_empty() {
            bail!("symbol must not be empty");
        }
        if self.strategy_name.is_empty() {
            bail!("strategy_name must not be empty");
        }
        if self.contributions.is_empty() {
            bail!("contributions must contain at least one entry");
        }
        for (idx, c) in self.contributions.iter().enumerate() {
            c.validate()
                .map_err(|e| anyhow!("contribution[{idx}] invalid: {e}"))?;
        }
        Ok(())
    }
}

impl LiveFactorContribution {
    /// Reject NaN/inf and empty factor names; canonicalise direction
    /// to one of `"Bull" | "Bear" | "Neutral"`.
    pub fn validate(&self) -> Result<()> {
        if self.factor_name.is_empty() {
            bail!("factor_name must not be empty");
        }
        for (label, val) in [
            ("value", self.value),
            ("confidence", self.confidence),
            ("weighted_score", self.weighted_score),
            ("uncertainty_contribution", self.uncertainty_contribution),
        ] {
            if !val.is_finite() {
                bail!("{label} must be finite, got {val}");
            }
        }
        match self.direction.as_str() {
            "Bull" | "Bear" | "Neutral" => Ok(()),
            other => bail!("direction must be one of Bull|Bear|Neutral, got '{other}'"),
        }
    }

    /// Convert to the in-engine [`FactorContribution`] shape used by
    /// Stage D consumers. `category` is preserved verbatim since
    /// `FactorContribution` stores it as `String` already.
    pub fn into_factor_contribution(self) -> FactorContribution {
        FactorContribution {
            factor_name: self.factor_name,
            category: self.category,
            direction: parse_direction(&self.direction),
            value: self.value,
            confidence: self.confidence,
            weighted_score: self.weighted_score,
            uncertainty_contribution: self.uncertainty_contribution,
            explanation: self.explanation,
        }
    }
}

fn parse_direction(s: &str) -> Direction {
    match s {
        "Bull" => Direction::Bull,
        "Bear" => Direction::Bear,
        _ => Direction::Neutral,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_contrib() -> LiveFactorContribution {
        LiveFactorContribution {
            factor_name: "ict_breakout_5m".into(),
            category: "breakout".into(),
            direction: "Bull".into(),
            value: 0.42,
            confidence: 0.71,
            weighted_score: 0.30,
            uncertainty_contribution: 0.08,
            explanation: "BOS confirmed".into(),
        }
    }

    fn good_env() -> LiveFactorSignalEnvelope {
        LiveFactorSignalEnvelope {
            schema_version: SCHEMA_VERSION.into(),
            symbol: "NQ".into(),
            timestamp_ms: 1_745_678_901_234,
            auto_quant_run_id: "live:NQ:MyBreakoutICT:20260426T120000Z".into(),
            strategy_name: "MyBreakoutICT".into(),
            strategy_mutation_id: "mb-001".into(),
            bar_close_ts_ms: 1_745_678_900_000,
            contributions: vec![good_contrib()],
        }
    }

    #[test]
    fn round_trip_canonical_envelope() {
        let e = good_env();
        let json = e.to_json().unwrap();
        let back = LiveFactorSignalEnvelope::from_json(&json).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let mut e = good_env();
        e.schema_version = "9.9".into();
        let err = e.validate().unwrap_err().to_string();
        assert!(err.contains("unsupported schema_version"), "got {err}");
    }

    #[test]
    fn rejects_empty_contributions() {
        let mut e = good_env();
        e.contributions.clear();
        let err = e.validate().unwrap_err().to_string();
        assert!(err.contains("at least one entry"), "got {err}");
    }

    #[test]
    fn rejects_nan_in_contribution() {
        let mut e = good_env();
        e.contributions[0].confidence = f64::NAN;
        let err = e.validate().unwrap_err().to_string();
        assert!(err.contains("must be finite"), "got {err}");
    }

    #[test]
    fn rejects_inf_in_contribution() {
        let mut e = good_env();
        e.contributions[0].weighted_score = f64::INFINITY;
        let err = e.validate().unwrap_err().to_string();
        assert!(err.contains("must be finite"), "got {err}");
    }

    #[test]
    fn rejects_empty_factor_name() {
        let mut e = good_env();
        e.contributions[0].factor_name.clear();
        let err = e.validate().unwrap_err().to_string();
        assert!(err.contains("factor_name"), "got {err}");
    }

    #[test]
    fn rejects_unknown_direction() {
        let mut e = good_env();
        e.contributions[0].direction = "sideways".into();
        let err = e.validate().unwrap_err().to_string();
        assert!(err.contains("direction"), "got {err}");
    }

    #[test]
    fn into_factor_contribution_preserves_fields() {
        let c = good_contrib();
        let fc = c.clone().into_factor_contribution();
        assert_eq!(fc.factor_name, c.factor_name);
        assert_eq!(fc.category, c.category);
        assert_eq!(fc.direction, Direction::Bull);
        assert_eq!(fc.confidence, c.confidence);
    }

    #[test]
    fn into_factor_contribution_neutralises_unknown_direction() {
        let mut c = good_contrib();
        c.direction = "anything-else".into();
        let fc = c.into_factor_contribution();
        assert_eq!(fc.direction, Direction::Neutral);
    }
}
