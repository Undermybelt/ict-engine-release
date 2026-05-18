//! Auto-Quant real-trade outcome ingestion (Phase 3).
//!
//! See `support/docs/2026-04-26-auto-quant-real-trades-plan.md` for the full
//! design. This module turns a JSONL artifact produced by
//! `Auto-Quant/auto_quant_export_real_trades.py` into a batch of
//! `FeedbackRecord`s and feeds them through the existing
//! `apply_feedback_to_trade_outcome_network` path so the
//! `trade_outcome` CPT learns from realised P&L.

pub mod ingest;
pub mod wire;

pub use ingest::{
    ingest_real_trades, IngestRealTradesInput, IngestRealTradesOutcome, ARTIFACT_KIND_REAL_TRADES,
    REAL_TRADES_RULE_VERSION,
};
pub use wire::{RealTradeRecord, SCHEMA_VERSION};
