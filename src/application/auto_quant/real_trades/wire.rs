//! Wire format for the real-trade outcomes JSONL artifact.
//!
//! Producer: `Auto-Quant/auto_quant_export_real_trades.py`.
//! Consumer: [`super::ingest::ingest_real_trades`].
//!
//! The artifact is **JSONL** (one JSON object per line). Each
//! [`RealTradeRecord`] carries enough context to build a fully-
//! qualified [`crate::state::FeedbackRecord`] without further
//! lookups against the ict-engine state directory.

use anyhow::{anyhow, bail, Result};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::application::backtest::trade_outcome_label_from_pnl;
use crate::application::decision_utils::normalize_entry_quality_label;
use crate::state::{FeedbackFactorUsage, FeedbackRecord, ModelProbabilitySnapshot};
use crate::types::{normalize_direction_label, normalize_regime_label, Direction};

/// Wire-schema version. Both exporter and ingest must agree.
pub const SCHEMA_VERSION: &str = "1.0";

/// One realised trade. The `factors_used` and
/// `model_probabilities_before_trade` fields are optional from the
/// exporter's perspective; missing values fall back to neutral
/// labels, which still produces a valid CPT row update.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealTradeRecord {
    pub schema_version: String,
    pub symbol: String,
    pub trade_id: String,
    pub strategy_name: String,
    #[serde(default)]
    pub strategy_mutation_id: String,
    #[serde(default)]
    pub auto_quant_run_id: String,
    pub open_ts_ms: i64,
    pub close_ts_ms: i64,
    pub direction: String,
    pub pnl: f64,
    #[serde(default)]
    pub realized_outcome: Option<String>,
    #[serde(default)]
    pub regime_at_entry: Option<String>,
    #[serde(default)]
    pub entry_signal: Option<String>,
    #[serde(default)]
    pub factors_used: Vec<RealTradeFactorUsage>,
    #[serde(default)]
    pub model_probabilities_before_trade: Option<RealTradeProbabilitySnapshot>,
    #[serde(default)]
    pub structural_feedback: Option<RealTradeStructuralFeedbackRefs>,
    #[serde(default)]
    pub regime_profit_branch_path: Option<String>,
    #[serde(default)]
    pub main_regime: Option<String>,
    #[serde(default)]
    pub sub_regime: Option<String>,
    #[serde(default)]
    pub sub_sub_regime_or_profit_factor: Option<String>,
    #[serde(default)]
    pub profit_factor: Option<String>,
}

/// Per-factor diagnostic captured at trade entry. Mirrors the
/// shape of [`FeedbackFactorUsage`] but uses string `direction`
/// to keep the wire format stable across language boundaries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealTradeFactorUsage {
    pub factor_name: String,
    pub category: String,
    pub direction: String,
    pub value: f64,
    pub confidence: f64,
    pub weighted_score: f64,
    pub uncertainty_contribution: f64,
}

/// Probabilities the model held just before the trade was opened.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealTradeProbabilitySnapshot {
    pub selected_direction: String,
    pub selected_probability: f64,
    pub long_score: f64,
    pub short_score: f64,
    pub win_prob_long: f64,
    pub win_prob_short: f64,
    pub uncertainty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealTradeStructuralFeedbackRefs {
    pub protocol_version: String,
    pub recommendation_id: String,
    pub recommended_at: String,
    pub node_id: String,
    pub branch_id: String,
    pub scenario_id: String,
    pub path_id: String,
    pub followed_path: bool,
    #[serde(default)]
    pub exit_reason: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl RealTradeRecord {
    /// Validate the record against the wire schema. Rejects unknown
    /// schema versions, NaN/inf, and impossible direction tokens.
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
        if self.trade_id.is_empty() {
            bail!("trade_id must not be empty");
        }
        if self.strategy_name.is_empty() {
            bail!("strategy_name must not be empty");
        }
        if !self.pnl.is_finite() {
            bail!("pnl must be finite, got {}", self.pnl);
        }
        // Direction must be parseable; we rely on normalize_* but
        // also reject obviously empty tokens up front.
        if self.direction.trim().is_empty() {
            bail!("direction must not be empty");
        }
        for (i, f) in self.factors_used.iter().enumerate() {
            f.validate()
                .map_err(|e| anyhow!("factors_used[{i}] invalid: {e}"))?;
        }
        if let Some(snap) = &self.model_probabilities_before_trade {
            snap.validate()?;
        }
        if let Some(structural_feedback) = &self.structural_feedback {
            structural_feedback.validate()?;
        }
        Ok(())
    }

    /// Convert this validated record into a [`FeedbackRecord`] ready
    /// for `apply_feedback_to_trade_outcome_network`.
    pub fn into_feedback_record(self, source: &str) -> FeedbackRecord {
        let record_level_branch_path = self.record_level_branch_path();
        let synthetic_structural_feedback = record_level_branch_path
            .as_ref()
            .map(|branch_path| self.synthetic_structural_feedback_refs(branch_path));
        let realized_outcome = self
            .realized_outcome
            .clone()
            .unwrap_or_else(|| trade_outcome_label_from_pnl(self.pnl));
        let regime_at_entry = normalize_regime_label(
            self.regime_at_entry
                .as_deref()
                .unwrap_or("manipulation_expansion"),
        );
        let _entry_quality =
            normalize_entry_quality_label(self.entry_signal.as_deref().unwrap_or("medium"));
        let mut factors_used = self
            .factors_used
            .into_iter()
            .map(|f| f.into_feedback_factor_usage())
            .collect::<Vec<_>>();
        if let Some(branch_path) = record_level_branch_path.as_ref() {
            push_branch_path_factor_usage(&mut factors_used, branch_path, &self.direction);
        }
        let model = match self.model_probabilities_before_trade {
            Some(snap) => snap.into_model_probability_snapshot(),
            None => ModelProbabilitySnapshot {
                selected_direction: normalize_direction_label(&self.direction),
                selected_probability: 0.0,
                long_score: 0.0,
                short_score: 0.0,
                win_prob_long: 0.0,
                win_prob_short: 0.0,
                uncertainty: 0.0,
            },
        };
        // Use close_ts_ms as the canonical "this trade ended" instant.
        let timestamp = ms_to_utc(self.close_ts_ms);
        FeedbackRecord {
            timestamp,
            symbol: self.symbol,
            source: source.to_string(),
            run_id: Some(self.auto_quant_run_id.clone()).filter(|s| !s.is_empty()),
            trade_id: Some(self.trade_id.clone()),
            prompt_version: None,
            factor_version: None,
            data_fingerprint: None,
            factors_used,
            model_probabilities_before_trade: model,
            realized_outcome,
            pnl: self.pnl,
            regime_at_entry,
            structural_feedback: self
                .structural_feedback
                .map(|refs| refs.into_structural_feedback_refs())
                .or(synthetic_structural_feedback),
            reflection_mismatch_tags: Vec::new(),
        }
    }

    fn record_level_branch_path(&self) -> Option<String> {
        self.regime_profit_branch_path
            .as_deref()
            .and_then(non_empty)
            .map(ToString::to_string)
            .or_else(|| {
                let main = self.main_regime.as_deref().and_then(non_empty)?;
                let sub = self.sub_regime.as_deref().and_then(non_empty)?;
                let sub_sub = self
                    .sub_sub_regime_or_profit_factor
                    .as_deref()
                    .and_then(non_empty)?;
                let profit = self.profit_factor.as_deref().and_then(non_empty)?;
                Some(format!("{main} -> {sub} -> {sub_sub} -> {profit}"))
            })
    }

    fn synthetic_structural_feedback_refs(
        &self,
        branch_path: &str,
    ) -> crate::state::StructuralFeedbackRefs {
        let path_segments = branch_path_segments(branch_path);
        let main = self
            .main_regime
            .as_deref()
            .and_then(non_empty)
            .or_else(|| path_segments.first().copied())
            .unwrap_or("unknown");
        let sub = self
            .sub_regime
            .as_deref()
            .and_then(non_empty)
            .or_else(|| path_segments.get(1).copied());
        let sub_sub = self
            .sub_sub_regime_or_profit_factor
            .as_deref()
            .and_then(non_empty)
            .or_else(|| path_segments.get(2).copied());
        let branch_id = sub
            .map(|sub| format!("{main} -> {sub}"))
            .unwrap_or_else(|| main.to_string());
        let scenario_id = sub_sub
            .map(|sub_sub| format!("{branch_id} -> {sub_sub}"))
            .unwrap_or_else(|| branch_id.clone());

        crate::state::StructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".to_string(),
            recommendation_id: format!(
                "auto-quant-real-trade-branch:{}:{}",
                self.symbol, self.trade_id
            ),
            recommended_at: ms_to_utc(self.open_ts_ms).to_rfc3339(),
            node_id: main.to_string(),
            branch_id,
            scenario_id,
            path_id: branch_path.to_string(),
            followed_path: true,
            exit_reason: self.realized_outcome.clone(),
            notes: Some("record_level_regime_profit_branch_path".to_string()),
        }
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn branch_path_segments(branch_path: &str) -> Vec<&str> {
    branch_path.split(" -> ").filter_map(non_empty).collect()
}

fn push_branch_path_factor_usage(
    factors_used: &mut Vec<FeedbackFactorUsage>,
    branch_path: &str,
    trade_direction: &str,
) {
    if factors_used.iter().any(|factor| {
        factor.category == "regime_profit_branch_path" && factor.factor_name == branch_path
    }) {
        return;
    }
    let direction = normalize_direction_label(trade_direction);
    let long_support = if direction == Direction::Bull {
        1.0
    } else {
        0.0
    };
    let short_support = if direction == Direction::Bear {
        1.0
    } else {
        0.0
    };
    factors_used.push(FeedbackFactorUsage {
        factor_name: branch_path.to_string(),
        category: "regime_profit_branch_path".to_string(),
        direction,
        value: 1.0,
        confidence: 1.0,
        weight: 1.0,
        long_support,
        short_support,
        uncertainty_contribution: 0.0,
    });
}

impl RealTradeFactorUsage {
    fn validate(&self) -> Result<()> {
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
            other => bail!("direction must be Bull|Bear|Neutral, got '{other}'"),
        }
    }

    fn into_feedback_factor_usage(self) -> FeedbackFactorUsage {
        let direction = match self.direction.as_str() {
            "Bull" => Direction::Bull,
            "Bear" => Direction::Bear,
            _ => Direction::Neutral,
        };
        let weight = self.weighted_score.abs();
        let long_support = if direction == Direction::Bull {
            self.weighted_score.max(0.0)
        } else {
            0.0
        };
        let short_support = if direction == Direction::Bear {
            self.weighted_score.abs()
        } else {
            0.0
        };
        FeedbackFactorUsage {
            factor_name: self.factor_name,
            category: self.category,
            direction,
            value: self.value,
            confidence: self.confidence,
            weight,
            long_support,
            short_support,
            uncertainty_contribution: self.uncertainty_contribution,
        }
    }
}

impl RealTradeProbabilitySnapshot {
    fn validate(&self) -> Result<()> {
        for (label, val) in [
            ("selected_probability", self.selected_probability),
            ("long_score", self.long_score),
            ("short_score", self.short_score),
            ("win_prob_long", self.win_prob_long),
            ("win_prob_short", self.win_prob_short),
            ("uncertainty", self.uncertainty),
        ] {
            if !val.is_finite() {
                bail!("model_probabilities_before_trade.{label} must be finite, got {val}");
            }
        }
        match self.selected_direction.as_str() {
            "Bull" | "Bear" | "Neutral" | "long" | "short" | "buy" | "sell" => Ok(()),
            other => bail!(
                "model_probabilities_before_trade.selected_direction must be Bull|Bear|Neutral, got '{other}'"
            ),
        }
    }

    fn into_model_probability_snapshot(self) -> ModelProbabilitySnapshot {
        ModelProbabilitySnapshot {
            selected_direction: normalize_direction_label(&self.selected_direction),
            selected_probability: self.selected_probability,
            long_score: self.long_score,
            short_score: self.short_score,
            win_prob_long: self.win_prob_long,
            win_prob_short: self.win_prob_short,
            uncertainty: self.uncertainty,
        }
    }
}

impl RealTradeStructuralFeedbackRefs {
    fn validate(&self) -> Result<()> {
        for (label, value) in [
            ("protocol_version", self.protocol_version.as_str()),
            ("recommendation_id", self.recommendation_id.as_str()),
            ("recommended_at", self.recommended_at.as_str()),
            ("node_id", self.node_id.as_str()),
            ("branch_id", self.branch_id.as_str()),
            ("scenario_id", self.scenario_id.as_str()),
            ("path_id", self.path_id.as_str()),
        ] {
            if value.trim().is_empty() {
                bail!("structural_feedback.{label} must not be empty");
            }
        }
        Ok(())
    }

    fn into_structural_feedback_refs(self) -> crate::state::StructuralFeedbackRefs {
        crate::state::StructuralFeedbackRefs {
            protocol_version: self.protocol_version,
            recommendation_id: self.recommendation_id,
            recommended_at: self.recommended_at,
            node_id: self.node_id,
            branch_id: self.branch_id,
            scenario_id: self.scenario_id,
            path_id: self.path_id,
            followed_path: self.followed_path,
            exit_reason: self.exit_reason,
            notes: self.notes,
        }
    }
}

fn ms_to_utc(ms: i64) -> chrono::DateTime<Utc> {
    Utc.timestamp_millis_opt(ms)
        .single()
        .unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_record() -> RealTradeRecord {
        RealTradeRecord {
            schema_version: SCHEMA_VERSION.into(),
            symbol: "NQ".into(),
            trade_id: "t-1".into(),
            strategy_name: "S".into(),
            strategy_mutation_id: "m-1".into(),
            auto_quant_run_id: "run-1".into(),
            open_ts_ms: 1_745_423_100_000,
            close_ts_ms: 1_745_427_900_000,
            direction: "Bull".into(),
            pnl: 0.0123,
            realized_outcome: Some("win".into()),
            regime_at_entry: Some("expansion".into()),
            entry_signal: Some("strong_buy".into()),
            factors_used: vec![RealTradeFactorUsage {
                factor_name: "f1".into(),
                category: "c".into(),
                direction: "Bull".into(),
                value: 0.4,
                confidence: 0.7,
                weighted_score: 0.3,
                uncertainty_contribution: 0.05,
            }],
            model_probabilities_before_trade: Some(RealTradeProbabilitySnapshot {
                selected_direction: "Bull".into(),
                selected_probability: 0.62,
                long_score: 0.3,
                short_score: -0.05,
                win_prob_long: 0.66,
                win_prob_short: 0.42,
                uncertainty: 0.18,
            }),
            structural_feedback: None,
            regime_profit_branch_path: None,
            main_regime: None,
            sub_regime: None,
            sub_sub_regime_or_profit_factor: None,
            profit_factor: None,
        }
    }

    #[test]
    fn validates_canonical_record() {
        good_record().validate().unwrap();
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let mut r = good_record();
        r.schema_version = "9.9".into();
        assert!(r
            .validate()
            .unwrap_err()
            .to_string()
            .contains("unsupported schema_version"));
    }

    #[test]
    fn rejects_nan_pnl() {
        let mut r = good_record();
        r.pnl = f64::NAN;
        assert!(r.validate().unwrap_err().to_string().contains("pnl"));
    }

    #[test]
    fn rejects_empty_trade_id() {
        let mut r = good_record();
        r.trade_id.clear();
        assert!(r.validate().unwrap_err().to_string().contains("trade_id"));
    }

    #[test]
    fn rejects_bad_factor_direction() {
        let mut r = good_record();
        r.factors_used[0].direction = "sideways".into();
        assert!(r.validate().unwrap_err().to_string().contains("direction"));
    }

    #[test]
    fn into_feedback_record_uses_close_ts_as_timestamp() {
        let r = good_record();
        let close_ts = r.close_ts_ms;
        let fr = r.into_feedback_record("auto_quant_real_trades");
        assert_eq!(fr.timestamp.timestamp_millis(), close_ts);
        assert_eq!(fr.symbol, "NQ");
        assert_eq!(fr.realized_outcome, "win");
        assert_eq!(fr.factors_used.len(), 1);
        assert_eq!(fr.run_id.as_deref(), Some("run-1"));
        assert_eq!(fr.trade_id.as_deref(), Some("t-1"));
    }

    #[test]
    fn into_feedback_record_derives_outcome_label_when_missing() {
        let mut r = good_record();
        r.realized_outcome = None;
        r.pnl = -0.05;
        let fr = r.into_feedback_record("test");
        assert_eq!(fr.realized_outcome, "loss");
    }

    #[test]
    fn into_feedback_record_drops_empty_run_id() {
        let mut r = good_record();
        r.auto_quant_run_id.clear();
        let fr = r.into_feedback_record("test");
        assert!(fr.run_id.is_none());
    }

    #[test]
    fn into_feedback_record_neutral_when_no_probability_snapshot() {
        let mut r = good_record();
        r.model_probabilities_before_trade = None;
        let fr = r.into_feedback_record("test");
        assert_eq!(
            fr.model_probabilities_before_trade.selected_probability,
            0.0
        );
    }

    #[test]
    fn into_feedback_record_preserves_structural_feedback_refs_when_present() {
        let mut r = good_record();
        r.structural_feedback = Some(RealTradeStructuralFeedbackRefs {
            protocol_version: "structural-feedback-v1".into(),
            recommendation_id: "structural-feedback:NQ:node:path".into(),
            recommended_at: "2026-05-07T09:56:50Z".into(),
            node_id: "node-1".into(),
            branch_id: "branch-1".into(),
            scenario_id: "scenario-1".into(),
            path_id: "path-1".into(),
            followed_path: true,
            exit_reason: Some("target_hit".into()),
            notes: Some("matched from helper".into()),
        });
        let fr = r.into_feedback_record("test");
        let refs = fr.structural_feedback.expect("structural refs");
        assert_eq!(refs.path_id, "path-1");
        assert!(refs.followed_path);
        assert_eq!(refs.exit_reason.as_deref(), Some("target_hit"));
    }

    #[test]
    fn record_level_regime_profit_branch_path_becomes_structural_feedback_and_factor_usage() {
        let branch_path = "Crisis -> ExtremeStress -> NQFlushRebound -> NQRootAdaptiveCostCrisisRepairV3:crisis_flush_rebound_h72";
        let raw = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "symbol": "NQ",
            "trade_id": "t-branch-1",
            "strategy_name": "NQRootAdaptiveCostCrisisRepairV3_Crisis",
            "strategy_mutation_id": "m-branch-1",
            "auto_quant_run_id": "run-branch-1",
            "open_ts_ms": 1745423100000_i64,
            "close_ts_ms": 1745427900000_i64,
            "direction": "Bull",
            "pnl": 0.0123,
            "realized_outcome": "win",
            "regime_at_entry": "Crisis",
            "entry_signal": "strong_buy",
            "regime_profit_branch_path": branch_path,
            "main_regime": "Crisis",
            "sub_regime": "ExtremeStress",
            "sub_sub_regime_or_profit_factor": "NQFlushRebound",
            "profit_factor": "NQRootAdaptiveCostCrisisRepairV3:crisis_flush_rebound_h72",
            "factors_used": []
        })
        .to_string();
        let record: RealTradeRecord = serde_json::from_str(&raw).unwrap();

        let feedback = record.into_feedback_record("auto_quant_real_trades");

        let refs = feedback
            .structural_feedback
            .expect("record-level branch path should synthesize structural feedback refs");
        assert_eq!(refs.path_id, branch_path);
        assert_eq!(refs.node_id, "Crisis");
        assert_eq!(refs.branch_id, "Crisis -> ExtremeStress");
        assert!(refs.followed_path);
        assert!(feedback.factors_used.iter().any(|factor| {
            factor.category == "regime_profit_branch_path" && factor.factor_name == branch_path
        }));
    }

    #[test]
    fn explicit_regime_profit_branch_path_recovers_structural_segments_without_split_fields() {
        let branch_path = "Bear -> BearMarketDrawdown -> NQHighVixOversoldRebound -> NQRootAdaptiveCostCrisisRepairV3:bear_oversold_high_vix_rebound_h72";
        let raw = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "symbol": "NQ",
            "trade_id": "t-branch-2",
            "strategy_name": "NQRootAdaptiveCostCrisisRepairV3_Bear",
            "strategy_mutation_id": "m-branch-2",
            "auto_quant_run_id": "run-branch-2",
            "open_ts_ms": 1745423100000_i64,
            "close_ts_ms": 1745427900000_i64,
            "direction": "Bear",
            "pnl": 0.0081,
            "realized_outcome": "win",
            "regime_at_entry": "Bear",
            "entry_signal": "strong_sell",
            "regime_profit_branch_path": branch_path,
            "factors_used": []
        })
        .to_string();
        let record: RealTradeRecord = serde_json::from_str(&raw).unwrap();

        let feedback = record.into_feedback_record("auto_quant_real_trades");

        let refs = feedback
            .structural_feedback
            .expect("record-level branch path should synthesize structural feedback refs");
        assert_eq!(refs.node_id, "Bear");
        assert_eq!(refs.branch_id, "Bear -> BearMarketDrawdown");
        assert_eq!(
            refs.scenario_id,
            "Bear -> BearMarketDrawdown -> NQHighVixOversoldRebound"
        );
        assert_eq!(refs.path_id, branch_path);
    }
}
