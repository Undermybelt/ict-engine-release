# Provider Request Contract

`ict-engine` public provider orchestration is provider-neutral by default.

The CLI does not treat repo market packs as runtime truth.
Callers must provide explicit external request data through:

- `--request-json <path>`
- `--request-stdin`
- explicit CLI flags for simple cases

## Request schema

```json
{
  "market_key": "caller-defined-label",
  "primary_data_path": "/abs/or/relative/path/to/primary.json",
  "interval": "15m",
  "start": "2026-04-01T00:00:00Z",
  "end": "2026-04-27T00:00:00Z",
  "count": 200,
  "related_roles": ["etf_reference", "options_underlying"],
  "provider_preferences": {
    "etf_reference": "yfinance",
    "options_underlying": "tradingview_mcp"
  },
  "symbol_overrides": {
    "etf_reference": {
      "display_symbol": "SPY",
      "yfinance": "SPY"
    },
    "options_underlying": {
      "display_symbol": "AMEX:SPY",
      "tradingview_mcp": "AMEX:SPY"
    }
  },
  "options_volatility_proxy_symbol": "^VIX"
}
```

## Rules

- `market_key` is an opaque caller label. It is not a repo-owned market lookup key.
- Every requested role must have:
  - a provider in `provider_preferences`
  - a symbol spec in `symbol_overrides`
- `options_volatility_proxy_symbol` is optional and used only for `options.summary` fallback.
- `ibkr` requests should use full JSON so the caller can provide an explicit contract object.

## CLI shorthand

For simple providers, the CLI supports:

```bash
cargo run -- market-data-harness \
  --action plan \
  --market caller-request \
  --role etf_reference \
  --provider etf_reference=yfinance \
  --symbol-spec etf_reference=SPY
```

Shorthand is intentionally limited:

- `yfinance` and `tradingview_mcp` can reuse `role=symbol`
- `ibkr` must use full request JSON

## Failure mode

If a role is missing explicit provider or symbol data, the CLI now fails validation instead of consulting repo defaults.
