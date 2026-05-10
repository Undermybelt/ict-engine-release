# Auto-Quant → ict-engine BBN Prior Init (Phase 1)

Date: 2026-04-26
Status: shipped (commits a7347f6, 8d03a01, 9a2b26e, plus cross-library guard follow-up)
Phase: 1 of 3 (offline path)

## Context

The user asked: do Auto-Quant factors flow into the belief-network as evidence,
get assigned to nodes, post-settled, and update priors that drive live
recommendations? Audit answer:

- Stages D (pre-bayes filter), E (BBN inference), F (execution_tree), G
  (`apply_feedback_to_trade_outcome_network`), H (`recommended_command`) are
  internally wired in ict-engine and known-good.
- Stage C (factor research) is currently the internal Rust `factor_lab` —
  produces `FactorContribution[]` and `FeedbackRecord[]` from candles + Rust
  factor definitions.
- The Auto-Quant ↔ ict-engine handoff is **half-wired**: outbound payload is
  built (`auto_quant_factor_research_command`), but Auto-Quant *results*
  (per-strategy sharpe/dd/wr/pf) have **no consumer** in ict-engine. Adoption
  review only checks workspace readiness, not actual metrics.

## Architectural decision

Auto-Quant is **not** a Stage B pre-filter that re-enters Stage C. It is a
**replacement** for Stage C at a higher abstraction level (strategy, not
factor). Reasons:

1. Auto-Quant's FreqTrade Strategy = factor + entry/exit + position sizing.
   Strategy-level metrics (sharpe / dd / wr / pf) are **not** factor-level
   metrics (IC / IR / factor_ranking).
2. Re-running ict-engine's Rust `factor_lab` after Auto-Quant just spent
   compute validating the strategy is wasteful and loses information.
3. The user's strategy paradigm is ICT MMxM (manipulation/distribution
   pattern recognition), not parameter fitting. Backtest PnL is a legitimate
   signal source for prior calibration without overfitting concerns.

So Auto-Quant **takes over** the factor/strategy research role; the BBN
update channel (`apply_feedback_to_trade_outcome_network`) is **preserved**
and remains the **only** legitimate route to the posterior.

## Two channels into the BBN

| Channel | Source | When | Effect |
|---|---|---|---|
| **Prior init** (Phase 1 — this doc) | Auto-Quant validated backtest metrics | Once at strategy adoption | Seeds `trade_outcome` CPT with empirical-Bayes pseudo-counts |
| **Posterior update** (existing, preserved) | Real trades (dry_run / live) via `FeedbackRecord` | Continuously after each trade | `apply_feedback_to_trade_outcome_network` does the canonical update |

Phase 1 is the **prior init channel only**. Posterior updates from real
trades flow through the existing path, untouched.

## Phase 1 deliverables

### A. Auto-Quant — strategy metadata contract

`user_data/strategies_ibkr/<Name>.py` docstring must declare structured
fields parseable by `export_strategy_library.py`:

```
Strategy: MyBreakoutICT
Mutation_id: mb-001
Base_factor: ict_breakout_5m
Hypothesis: 5m breakout from manipulation phase ...
Expected_regime: expansion
Factors_used: bos, fvg, atr
Parent: _template
Status: active
```

`_template.py.example` is updated to include these as required template
sections.

### B. Auto-Quant — `run_ibkr.py` echoes metadata

For each strategy run, `run_ibkr.py` emits an extra line in the `---`
block:

```
auto_quant_meta:  {"mutation_id": "...", "factors_used": [...], ...}
```

ict-engine's parser keys on this prefix.

### C. Auto-Quant — `export_strategy_library.py`

New script. Scans `user_data/strategies_ibkr/` and the latest `run_ibkr.log`
to produce a manifest:

```
{
  "manifest_version": "1.0",
  "exported_at": "2026-04-26T...",
  "auto_quant_repo_url": "...",
  "auto_quant_pinned_ref": "<git rev>",
  "config_path": "config.ibkr.json",
  "timeframe": "5m",
  "strategies": [
    {
      "name": "MyBreakoutICT",
      "file_path": "user_data/strategies_ibkr/MyBreakoutICT.py",
      "metadata": {
        "mutation_id": "mb-001",
        "base_factor": "ict_breakout_5m",
        "hypothesis": "...",
        "expected_regime": "expansion",
        "factors_used": ["bos", "fvg", "atr"],
        "parent": "_template",
        "status": "active"
      },
      "validation_metrics": {
        "sharpe": 1.42,
        "sortino": 2.13,
        "calmar": 4.50,
        "total_profit_pct": 12.3,
        "max_drawdown_pct": -3.2,
        "trade_count": 87,
        "win_rate_pct": 54.5,
        "profit_factor": 1.85
      },
      "per_pair_metrics": {
        "SPY/USD": { "sharpe": 1.5, "trade_count": 50, "win_rate_pct": 56.0 },
        "QQQ/USD": { ... }
      }
    }
  ]
}
```

Output path: `<state_dir>/<symbol>/auto_quant_strategy_library.json` (or
explicit `--output`).

### D. ict-engine — results module

`src/application/auto_quant/results/`:

- `mod.rs` — exports
- `log_parser.rs` — parses Auto-Quant `run_ibkr.log` (canonical `---`
  block format, `auto_quant_meta:` prefix)
- `strategy_library.rs` — loads `strategy_library.json`, validates schema
- `prior_init.rs` — Beta-Binomial empirical-Bayes prior overlay

### E. ict-engine — prior init math

For each validated strategy:

```
n      = trade_count
n_win  = round(n * win_rate_pct / 100)
n_loss = n - n_win
n_be   = 0   (FreqTrade reports breakeven as either win or loss)

Apply tempered pseudo-counts to trade_outcome CPT entry conditioned on the
parent_config that corresponds to "entry_quality = ready":

  k = 0.5   (temper factor; backtest counts not equivalent to real trades)
  cpt_entry[win]    = normalize( alpha_w_old + k * n_win )
  cpt_entry[loss]   = normalize( alpha_l_old + k * n_loss )
  cpt_entry[other]  = unchanged
```

Where `alpha_*` are computed by treating existing CPT entries as a
Dirichlet with strength `prior_strength` (configurable, default 4.0).

This is `CPTUpdater` semantics extended with bulk pseudo-counts. Implement
as a new method `CPTUpdater::add_pseudo_counts(network, parent_config,
counts, temper)` rather than reusing `update_from_trade` per virtual
observation (cleaner + faster).

### F. ict-engine — ledger

New artifact_kinds:

- `auto_quant_strategy_library_validated` — written when
  `auto-quant-results-import` succeeds
- `auto_quant_prior_init_applied` — written when prior init is applied;
  records (strategy_name, parent_config, before_cpt, after_cpt, k,
  prior_strength) for full provenance + reversibility

Library state machine (as shipped):

```
ready_for_prior_init        ── new manifest with n_ok > 0
no_validated_strategies     ── new manifest with n_ok == 0
superseded                  ── flipped to this when a *newer*
                              ready_for_prior_init manifest is imported
```

Re-import always creates a new ledger entry; the new entry's
`supersedes_artifact_id` records the most recent prior. The operator
never sees two `ready_for_prior_init` libraries simultaneously, so a
prior-init resolved-by-recency cannot accidentally pick a stale one.

Prior-init state machine (as shipped):

```
applied             ── non-dry-run with strategies_applied > 0
no_op               ── non-dry-run with strategies_applied == 0
dry_run_preview     ── any --dry-run invocation
```

### F'. Apply guard (replaces the loose "idempotent" property)

The math is **not** idempotent under repeated apply — each invocation
adds tempered pseudo-counts to the trade_outcome row. Two ledger-level
guards together prevent the silent double-count footgun:

1. **Same-library guard.** A second non-dry-run apply against a
   `library_artifact_id` that already has an `applied` ledger entry is
   refused:

   > library 'auto_quant_strategy_library_NQ_…' has already been
   > applied via 'auto_quant_prior_init_NQ_…'. Re-applying would
   > silently double the tempered pseudo-counts. Roll back the BBN
   > snapshot (`rm <state_dir>/<symbol>/bbn_network.json`) and pass
   > `--force` to override, or re-run `--dry-run` to inspect the diff.

2. **Cross-library guard.** A non-dry-run apply against library v2
   when v1 was previously applied (and v1 has since been
   auto-superseded by the v2 import) is refused:

   > BBN already carries an Auto-Quant prior init from library
   > 'auto_quant_strategy_library_NQ_…' (apply
   > 'auto_quant_prior_init_NQ_…'); current request targets library
   > 'auto_quant_strategy_library_NQ_…'. Re-applying without rollback
   > would stack two pseudo-count layers on the same trade_outcome
   > row. Roll back the BBN snapshot
   > (`rm <state_dir>/<symbol>/bbn_network.json` and re-run import +
   > prior-init) or pass `--force` to deliberately stack.

Both `--dry-run` (always allowed; emits a `dry_run_preview` ledger
entry without touching `bbn_network.json`) and `--force` (writes a
fresh `applied` entry on top of the existing one) bypass these
guards, so the operator has read-only inspection and explicit-stack
escape hatches.

Manual rollback recipe (no dedicated subcommand yet — deferred to
Phase 2): delete the `bbn_network.json` snapshot, then re-run
`ict-engine auto-quant-prior-init …`. The fresh build_trading_network
seed is the single source of truth.

### G. ict-engine — CLI commands

```
ict-engine auto-quant-results-import \
    --symbol NQ \
    --state-dir state \
    --library state/NQ/auto_quant_strategy_library.json \
    [--log run_ibkr.log]              # optional cross-check

ict-engine auto-quant-prior-init \
    --symbol NQ \
    --state-dir state \
    [--library <path>]                # default: state/NQ/auto_quant_strategy_library.json
    [--strategies <name>,<name>]      # comma-delimited; default: every status=ok strategy
    [--temper 0.5]
    [--prior-strength 4.0]
    [--parent-config 0,0,0]
    [--dry-run]
    [--force]                         # override the single-apply guard
```

`--dry-run` writes the proposed CPT diff to stdout (and a
`dry_run_preview` ledger entry) without touching `bbn_network.json`.
The library entry is resolved via the most recent
`auto_quant_strategy_library_validated` ledger entry; lineage is
captured by `source_run_id` on the prior-init entry.

### H. Tests

Shipped:

- Unit: manifest schema rejects missing/unsupported version, malformed JSON.
- Unit: prior-init math — Beta-Binomial conjugate posterior on a
  balanced row, empirical-dominance under high prior strength,
  zero-temper no-op, parent-config arity validation, strategy-filter
  scoping.
- Unit: log parser — `---` block split, aggregate metrics,
  per-pair metrics, error-status capture, `auto_quant_meta:` JSON
  capture.
- Unit: cross-check — clean mirror, numeric drift detection,
  asymmetric coverage (manifest_only / log_only).
- Unit: persistence — library + ledger round-trip, re-import flips
  prior to `superseded`, dry-run leaves library untouched, apply guard
  recognises only `applied` (not `dry_run_preview`).
- Unit (command_entry): import + dry-run leaves BBN untouched, apply
  writes BBN, missing library yields a directed error, second apply
  against the same library is refused with an `already been applied`
  error, second apply against a *different* library after a prior
  apply is refused with a `BBN already carries an Auto-Quant prior
  init` cross-library error, `--force` overrides both guards,
  `--log` runs the cross-check informationally.

Out of scope here (deferred to Phase 2):

- A `cargo test --test ...` integration crate that drives the full
  binary end-to-end (the in-process unit tests + the manual smoke
  `./target/debug/ict-engine auto-quant-...` cover the same paths
  today).

## Out of scope (Phase 2 / 3)

- **Phase 2**: PyO3 adapter that calls Python strategy modules to emit live
  signals as `FactorContribution[]` for Stage D consumption.
- **Phase 3**: Real-trade `FeedbackRecord` ingestion through dry_run / live
  OMS feeding `apply_feedback_to_trade_outcome_network`.

## Risks

| Risk | Mitigation |
|---|---|
| Backtest pseudo-counts swamp prior | `temper` factor (default 0.5) + cap on max delta per CPT entry |
| Schema drift between Auto-Quant export + ict-engine import | `manifest_version` field + strict serde |
| Multiple library imports clobber each other | Ledger supersession + `prior_init_applied` artifact records before/after |
| Auto-Quant changes its `run_ibkr.log` format | Parser keys on stable `---` + `auto_quant_meta:` prefix; integration test catches drift |
