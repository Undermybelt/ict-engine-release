# Demo data

Small synthetic candle data for first-run smoke tests.

Try:

```bash
cargo run -- factor-pipeline-debug \
  --symbol DEMO \
  --data examples/demo/demo-15m.json \
  --factor structure_ict \
  --objective expansion_manipulation
```

The file is intentionally tiny and synthetic; use it only to verify that the CLI runs after clone.
