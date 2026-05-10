"""IBKR live data bridge for ict-engine and Auto-Quant.

Single producer (`bridge.py`) holds the IBKR connection and fans out market
data via Redis Streams. Multiple consumers (Auto-Quant strategies, ict-engine
research scripts) read via `consumer.IbkrConsumer`. All IBKR I/O is
localhost-only.

See `docs/2026-04-26-ibkr-live-data-bridge-plan.md` for the full design.
"""
