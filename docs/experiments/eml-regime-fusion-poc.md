# EML regime fusion PoC verdict

Status: rejected

Scope
- Tested a small EML-inspired non-linear fusion around regime handling for factor-lab backtests.
- Initial variant affected regime classification more broadly.
- Follow-up variant narrowed usage to a manipulation/expansion-local multiplier gate.

Files touched during experiment
- `src/factors/regime_conditional.rs`
- `src/factor_lab/backtest.rs`
- `src/factor_lab/engine.rs`
- `src/factor_lab/factor_definition.rs`
- `tests/eml_poc.rs`

Evaluation setup
- Walk-forward backtest compared baseline vs EML experiment branch before rollback
- Symbols tested: NQ, ES
- Comparison method: same backtest, only EML path changed
- Primary decision rule: reject if reversal-oriented quality does not improve and portfolio metrics do not improve

Observed outcome
- Total return: no improvement
- Sharpe: no improvement
- Win rate: no improvement
- The experiment mainly reallocated more samples/trades into `ManipulationExpansion`
- Follow-up gate version still failed to improve reversal behavior materially

Interpretation
- The EML surface behaved more like an expansion selector than a reversal discriminator.
- This conflicts with the intended use: distinguish liquidity-sweep reversal vs secondary expansion.
- Given unchanged portfolio metrics and failed reversal objective, the added non-linearity is not justified.

Decision
- Roll back runtime EML multiplier usage.
- Remove the dormant backtest EML toggle/path as well.
- Keep only the rejected PoC tests around `src/factors/regime_conditional.rs` and this note as experiment record.

Future revisit bar
Only revisit if new inputs are available that directly encode:
- actual liquidity sweep rejection strength
- post-sweep structural displacement quality
- explicit reversal labels or stronger outcome proxies

Do not revisit merely by retuning `exp/ln` parameters on current proxy inputs.
