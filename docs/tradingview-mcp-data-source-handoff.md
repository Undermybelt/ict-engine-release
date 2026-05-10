# TradingView MCP Data Source Handoff

## Goal

Make TradingView MCP a hot-pluggable, zero-config external data source for ict-engine backtests and live analysis while keeping existing defaults unchanged.

## User requirements

- Zero-config default remains usable without env vars.
- Consumers can opt into or out of TradingView MCP.
- Token-friendly provider/status outputs.
- No repo-root state pollution; runtime outputs stay in explicit state dirs or temp paths.
- No hidden debt: unsupported lanes fail clearly and fall back to existing providers where possible.
- Prioritize the user's desk needs: NQ/ES/YM/GC/CL reference ETFs, CFD/VIX overlays, and live/backtest candle generation.

## Current implementation status

- Python TradingView MCP now exposes `get_ohlcv` using Yahoo chart data and returns machine-readable bars.
- Python MCP normalizes existing TradingView-style preset symbols such as `NASDAQ:QQQ`, `CBOE:VIX`, CME futures aliases, `OANDA:XAUUSD`, and `TVC:USOIL` to Yahoo-compatible symbols.
- Python MCP accepts `4h` and aggregates from hourly bars so analyze-live can request its current multi-timeframe set.
- ict-engine has a shared TradingView MCP client module with HTTP compatibility and local stdio subprocess support.
- ict-engine fetch harness uses the shared client for `tradingview_mcp` OHLCV and hardened bar parsing.
- ict-engine fetch harness falls back to Yahoo options/volatility proxy when TradingView MCP options tools are unavailable.
- ict-engine live backend registration is wired through `LiveDataBackend::TradingViewMcp`.
- Provider-status and provider catalog now describe local stdio OHLCV as the zero-config ready path and only mark options enrichment as remote/optional degraded.

## TODO

- Commit only the files touched for this MCP integration, avoiding unrelated pre-existing ict-engine changes.

## Verification so far

- `uv run pytest` in TradingView MCP: 79 passed.
- Python service smoke: `fetch_ohlcv_bars('NASDAQ:QQQ','4h',1,'1mo')` returned success with 1 bar.
- MCP stdio smoke: initialize + `tools/call get_ohlcv` worked; FastMCP returned JSON in `content[0].text`.
- ict-engine `cargo check --manifest-path .../Cargo.toml`: passed.
- ict-engine `cargo test ... data_sources`: 24 passed.
- ict-engine `cargo test --test provider_neutral_cli provider_status_agent_hides_opt_in_profiles_without_selecting_one`: passed.
- ict-engine focused `cargo test ... tradingview_mcp`: passed.

## Opt-in knobs

- Default local stdio command first auto-detects `~/tradingview-mcp/tradingview-mcp` and runs `uv --directory <checkout> run tradingview-mcp`.
- If no local checkout is present, stdio falls back to `uvx --from tradingview-mcp-server tradingview-mcp`.
- Explicit override: set `ICT_ENGINE_TRADINGVIEW_MCP_CMD` and `ICT_ENGINE_TRADINGVIEW_MCP_ARGS`.
- Existing remote HTTP mode remains available via `ICT_ENGINE_TVREMIX_MCP_URL` and `ICT_ENGINE_TVREMIX_MCP_API_KEY`.
