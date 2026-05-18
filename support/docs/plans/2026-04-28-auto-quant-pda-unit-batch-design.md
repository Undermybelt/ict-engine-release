# Auto-Quant PDA Unit Batch Design

Date: 2026-04-28
Status: approved-for-implementation
Scope: Auto-Quant-centered unit batching for single-setup, single-symbol, single-timeframe iteration

Boundary note:
- This surface is now internal/experimental only.
- Consumer-facing orchestration should use `agent-material-*` generic protocol instead of ontology-driven PDA unit commands.

## Goal

Turn the user's PDA universe into Auto-Quant iteration units whose smallest runnable grain is:

- one setup sequence
- one symbol
- one timeframe
- one direction

The CLI must then batch these independent units and hand them to Auto-Quant in a form that supports parallel execution without polluting the managed workspace.

## Truth boundary

The repo's PDA/canonical-setup code is **reference only**. It is not the source of user intent.

The actual Auto-Quant iteration input must be a natural-language "completed oral strategy body" assembled by the agent from:

1. the user's PDA universe intent
2. the requested setup sequence
3. the execution scope (`symbol`, `timeframe`, `direction`)
4. the ranking objective:
   - win rate first
   - Sharpe second
   - return third

So the new surface must not force the iteration loop back through `structure_ict` as the primary unit of work.

## First-class primitive universe

The first iteration wave uses the 11 user-approved PDA primitives:

1. `order_block`
2. `fair_value_gap`
3. `inverse_fvg`
4. `breaker_block`
5. `mitigation_block`
6. `rejection_block`
7. `propulsion_block`
8. `liquidity_void`
9. `volume_imbalance`
10. `market_structure_shift`
11. `cisd`

Each primitive can be run in:

- `long`
- `short`

So the first base layer is 22 unit factors per `(symbol, timeframe)`.

## Batch model

The public CLI should expose a new Auto-Quant-centered batch surface that:

1. accepts explicit primitive selection
2. accepts explicit `combination_size`
3. accepts explicit `symbol`
4. accepts explicit `timeframe -> data path` mapping
5. accepts explicit direction set
6. creates isolated state dirs per unit
7. reuses one shared managed Auto-Quant checkout
8. persists one batch artifact plus one unit handoff artifact per unit

## Ordered sequence model

The user's universe plan is sequence-sensitive. So the new batch surface must support ordered setup sequences.

- `combination_size = 1`
  - single primitive unit
- `combination_size = 2`
  - ordered two-step sequence, e.g. `market_structure_shift -> fair_value_gap`
- future sizes can reuse the same structure

For `combination_size > 1`, generation should use ordered permutations over the selected primitive set. This keeps the CLI aligned with the universe-plan idea that temporal order matters.

## Output artifacts

### 1. Unit handoff payload

Each unit gets a persisted Auto-Quant handoff payload with:

- shared AQ workspace info
- isolated unit state dir
- explicit `symbol`
- explicit `timeframe`
- explicit `direction`
- ordered primitive sequence
- natural-language strategy brief
- evaluation priority (`win_rate > sharpe > return`)

### 2. Batch manifest

The batch manifest is the control-plane truth for the agent. It contains:

- `batch_id`
- `symbol`
- selected `timeframes`
- selected primitive universe
- `combination_size`
- `max_parallel`
- shared AQ workspace root
- per-unit job entries
- suggested parallel dispatch groups

## Agent-facing behavior

The new surface is for agent use, not end-user use.

So the manifest and unit payloads should optimize for:

- short, explicit unit labels
- actionable state-dir isolation
- explicit handoff artifact paths
- clear AQ execution priority
- no repo-owned market assumptions beyond what the caller passed explicitly

## Non-goals

- no automatic second-order promotion in the same change
- no in-binary Auto-Quant strategy-code generation
- no replacement of existing `factor-research` / `factor-autoresearch`
- no forced use of repo `canonical_setup` matchers as the primary iteration unit

## Immediate implementation shape

Add a new command:

- `auto-quant-pda-unit-batch`

This command will:

1. parse explicit primitive/timeframe/direction inputs
2. build unit specs
3. bootstrap/verify shared Auto-Quant workspace
4. create one isolated state dir per unit
5. create and persist one AQ handoff payload per unit
6. persist one batch manifest that groups those units for parallel execution

This gives the agent a real Auto-Quant-centered primitive for:

- base factor wave (`combination_size=1`)
- later ordered sequence wave (`combination_size=2+`)
