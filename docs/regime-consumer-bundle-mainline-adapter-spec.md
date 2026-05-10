# Regime Consumer Bundle Mainline Adapter Spec

Status: proposal only. No runtime behavior changes.

Goal: allow mainline consumers to optionally read `regime_consumer_bundle.json` when a user explicitly provides a path, while preserving zero-config defaults.

## Non-goals

- Do not auto-run sidecars from mainline commands.
- Do not change default `analyze`, BBN, path-ranker, or execution-tree behavior.
- Do not require Python sidecars for normal Rust runtime use.
- Do not treat missing/invalid bundle as fatal unless user selects strict mode.

## Proposed user-facing controls

CLI flags, if implemented later:

- `--regime-consumer-bundle <PATH>`
  - Optional path to `regime_consumer_bundle.json`.
  - Default: unset.
- `--regime-consumer-bundle-strict`
  - Optional strict mode.
  - Default: false.
  - If true, missing/invalid bundle fails the command.
  - If false, missing/invalid bundle is recorded as no-op evidence and runtime continues.

Config key, if implemented later:

```json
{
  "regime_consumer_bundle": {
    "path": "",
    "strict": false,
    "enabled": false
  }
}
```

Precedence:

1. CLI `--regime-consumer-bundle`
2. Config `regime_consumer_bundle.path`
3. Unset -> no-op

## Minimal bundle fields consumed

Source: `regime_consumer_bundle.json`.

Required for adapter to activate:

- `schema_version == "regime-consumer-bundle/v1"`
- `latest_decision`
- `consumer_hints`

Optional but preferred:

- `consumer_hints.execution_tree_hint`
- `consumer_hints.bbn_evidence_hint`
- `consumer_hints.path_ranker_context`
- `consumer_hints.user_vrp_nq_context`
- `missing_artifacts`

## Consumer mapping table

| Consumer | Bundle field | Proposed target | Notes |
|---|---|---|---|
| Execution tree | `consumer_hints.execution_tree_hint` | transition guard / accept regime branch | Values: `accept_regime`, `transition_guardrail`, `unknown_abstain`. |
| Execution tree | `latest_decision.trade_usable` | allow/guardrail switch | `false` should never force a trade; only guardrail/observe. |
| Execution tree | `latest_decision.abstain_reasons` | trace reasons | Append to trace, compact. |
| BBN | `consumer_hints.bbn_evidence_hint` | soft evidence nodes | Use as soft evidence only; missing keys neutral. |
| BBN | `latest_decision.decision_state` | regime decision state evidence | `single_label_99` strongest, `single_label_95` moderate, others neutral/guardrail. |
| Path ranker | `consumer_hints.path_ranker_context` | feature row enrichment | Add optional context columns; do not drop rows when absent. |
| Reports / human | `consumer_hints.user_vrp_nq_context` | human diagnostics | User-specific context: QQQ HV, NQ vs 200d, VIX3M, HV rank, VVIX/VIX. |

## Runtime behavior contract

Default unset:

- No file read.
- No side effects.
- Current behavior unchanged.

Bundle path set and valid:

- Load JSON once at command startup or consumer initialization.
- Validate schema/version.
- Extract only known fields.
- Unknown fields ignored.
- Add compact trace entry:
  - `regime_bundle_status=loaded`
  - `regime_bundle_path=<PATH>`
  - `regime_decision_state=<STATE>`

Bundle path set but missing/invalid, non-strict:

- Continue runtime.
- Add compact trace entry:
  - `regime_bundle_status=missing` or `invalid`
  - `regime_bundle_error=<short reason>`
- All mapped evidence remains neutral / absent.

Bundle path set but missing/invalid, strict:

- Return a clear error before main processing.
- No partial state mutation.

## Safety / no-pollution rules

- Adapter reads only the explicit bundle path.
- Adapter never writes sidecar outputs.
- Adapter never scans directories.
- Adapter never invokes Python scripts.
- Adapter never creates repo-root state.
- Adapter must be removable with no data migration.

## Suggested Rust shape if implemented later

Data type:

```rust
struct RegimeConsumerBundleAdapter {
    status: BundleStatus,
    latest_decision: Option<RegimeDecisionSummary>,
    consumer_hints: Option<RegimeConsumerHints>,
}
```

Load API:

```rust
RegimeConsumerBundleAdapter::load_optional(path: Option<&Path>, strict: bool) -> Result<Self>
```

No-op default:

```rust
RegimeConsumerBundleAdapter::disabled()
```

Implementation placement candidates:

- `src/application/regime/consumer_bundle_adapter.rs`
- or `src/application/orchestration/regime_bundle_adapter.rs`

Do not place sidecar-running logic in Rust runtime.

## Validation checklist for future implementation

- [ ] No flag/config set -> existing tests unchanged.
- [ ] Valid bundle -> trace includes loaded status and decision state.
- [ ] Missing bundle non-strict -> command succeeds with neutral evidence.
- [ ] Missing bundle strict -> command fails before state mutation.
- [ ] Invalid schema non-strict -> command succeeds with neutral evidence.
- [ ] Invalid schema strict -> command fails before state mutation.
- [ ] Execution tree sees only `accept_regime` / `transition_guardrail` / `unknown_abstain`.
- [ ] BBN receives soft evidence only.
- [ ] Path ranker enrichment is optional and row-preserving.
