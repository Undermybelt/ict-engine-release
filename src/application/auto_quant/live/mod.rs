//! Auto-Quant live factor signals (Phase 2).
//!
//! See `docs/2026-04-26-auto-quant-live-signals-plan.md` for the full
//! design. This module is the ict-engine consumer: it reads
//! `LiveFactorSignalEnvelope`s from a Redis stream populated by
//! `Auto-Quant/auto_quant_live_signal_publisher.py`, validates them
//! against the wire schema, and appends them to a JSONL state file
//! that downstream Stage D consumers can read.

pub mod consumer;
pub mod persistence;
pub mod wire;

pub use consumer::{
    consume_live_signals, ConsumeLiveSignalsInput, ConsumeLiveSignalsOutcome, RealRedisSource,
    StreamEntry, StreamSource,
};
pub use persistence::{cursor_path, jsonl_path, read_cursor, write_cursor};
pub use wire::{
    LiveFactorContribution, LiveFactorSignalEnvelope, ENVELOPE_FIELD, SCHEMA_VERSION,
    STREAM_KEY_PREFIX,
};
