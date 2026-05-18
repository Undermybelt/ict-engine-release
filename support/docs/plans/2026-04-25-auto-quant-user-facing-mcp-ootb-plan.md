# Auto-Quant User-Facing MCP OOTB Plan

Date: 2026-04-25

Status
- planning document
- focused on user-facing MCP readiness and first-run ergonomics
- scoped to reducing user setup burden and token waste

## Goal

Make the Auto-Quant integration feel close to out-of-the-box for a new `ict-engine` user, even when their machine:

- has no local Auto-Quant checkout
- has no MCP registration for Auto-Quant
- has no local market data prepared

The user should not need to spend meaningful token budget teaching the agent:

- where Auto-Quant lives
- how to bootstrap it
- how to inspect whether it is healthy
- what to run next when it is missing prerequisites

## Problem statement

Current state is better than before, but still not yet "true OOTB":

1. `ict-engine` can now manage Auto-Quant as a dependency.
2. `factor-research` / `factor-autoresearch` can now hand off to Auto-Quant.
3. canonical handoff, review, decision, and next-step artifacts exist.

But a brand-new user still faces three setup gaps:

1. **Dependency gap**
   - Auto-Quant checkout may not exist.

2. **Execution-environment gap**
   - Python / `uv` / TA-Lib / FreqTrade assumptions may not hold.

3. **Data gap**
   - Auto-Quant's `user_data/data` may be empty, so execution is blocked until preparation.

If these gaps are surfaced only through free-form agent reasoning, the user pays in:

- extra token burn
- repeated steering
- brittle first-run experience

## Design principle

Do not make the user or their agent "discover the workflow by conversation".

Instead:

- encode the workflow in deterministic commands and machine-readable status
- let the CLI expose the current blocker
- let MCP/tooling read the same blocker
- only escalate to human reasoning when the machine-readable path is exhausted

## Desired OOTB contract

For a new user, this should be true:

1. `ict-engine` can tell whether Auto-Quant is missing.
2. `ict-engine` can bootstrap Auto-Quant automatically.
3. `ict-engine` can tell whether Auto-Quant is healthy.
4. `ict-engine` can tell whether Auto-Quant data is ready.
5. `ict-engine` can suggest the next concrete command without the agent inventing one.
6. `ict-engine` can expose all of the above to:
   - CLI users
   - agent users
   - future MCP clients

## Recommended framework

### Layer 1: deterministic local control plane

Keep all first-run logic in `ict-engine`, not in Auto-Quant prompts.

Required surfaces:

- `auto-quant-status`
- `auto-quant-bootstrap`
- `auto-quant-update`
- `auto-quant-adoption-review`
- `auto-quant-adoption-decision`

These already exist or are partially in place.

### Layer 2: OOTB first-run state machine

Add an explicit Auto-Quant readiness model with a small set of states:

- `missing_dependency`
- `dependency_unhealthy`
- `dependency_ready_data_missing`
- `dependency_ready_data_ready`
- `update_available`

Every user-facing surface should derive from this same state machine.

### Layer 3: zero-steering next-step emission

For each readiness state, `ict-engine` should emit:

- `recommended_next_command`
- `recommended_next_command_meta`
- `next_step`
- optional short human summary

This is the anti-token-waste rule:

The system should not require the agent to invent the next command when the state is already machine-known.

### Layer 4: optional MCP exposure

Do not make "MCP setup" a prerequisite for first-run usability.

Instead:

- the CLI remains the primary control plane
- MCP becomes a thin projection over the same deterministic command/state surfaces

That means:

- if MCP exists, great
- if MCP is absent, the user still gets the same first-run path through CLI commands

## What "MCP enough" means

The integration is user-facing-MCP-ready when all of the following are true:

1. A future MCP server does not need to know Auto-Quant internals.
2. A future MCP server can answer:
   - is bootstrap needed?
   - is the checkout healthy?
   - is data ready?
   - is an update available?
   - what command should run next?
3. The MCP answers are derived from the same local state as the CLI.
4. Removing MCP still leaves the user with a usable OOTB path.

## What not to do

### 1. Do not make MCP the only path

If first-run success depends on external MCP registration, the system is not OOTB.

### 2. Do not hide blockers in prose only

If the user must read a long agent message to know "run prepare.py", the design is wasting tokens.

### 3. Do not require conversational setup discovery

The agent should not need to ask:

- "Where is Auto-Quant?"
- "How do I clone it?"
- "Which script should I run?"

Those must be embedded in deterministic surfaces.

### 4. Do not assume Python/FreqTrade readiness

Dependency checkout is not the same as executable environment.

Health must cover:

- repo present
- expected files present
- executable environment sanity (eventually)
- data readiness

## Concrete plan

### Phase A: make readiness first-class in `ict-engine`

Add one canonical Auto-Quant readiness surface that returns:

- dependency status
- data readiness
- update availability
- explicit next command

Potential command:

- `ict-engine auto-quant-status --agent`

Desired output fields:

- `status`
- `healthy`
- `bootstrap_needed`
- `data_ready`
- `update_available`
- `recommended_next_command`
- `next_step`

### Phase B: connect readiness to research entrypoints

When a user runs:

- `factor-research --backend auto-quant`
- `factor-autoresearch --backend auto-quant`

the result should already contain:

- current readiness state
- next command
- canonical artifact path

This mostly exists now, but should be normalized around the single readiness model instead of ad hoc field combinations.

### Phase C: add an Auto-Quant first-run helper surface

Recommended new command:

- `ict-engine auto-quant-setup`

What it should do:

1. bootstrap dependency if missing
2. verify checkout health
3. detect whether execution dependencies are missing
4. detect whether data is missing
5. emit exactly one consolidated report

It should not silently launch long experiments.

### Phase D: future MCP wrapper

Once the CLI readiness model is stable, expose a tiny MCP server or MCP tool adapter that simply forwards:

- status
- bootstrap
- update
- review
- decision

The MCP layer should be transport only, not business logic.

## Suggested command framework

### Existing commands to keep

- `auto-quant-status`
- `auto-quant-bootstrap`
- `auto-quant-update`
- `auto-quant-adoption-review`
- `auto-quant-adoption-decision`

### Recommended new commands

- `auto-quant-setup`
- `auto-quant-health`
- optionally `auto-quant-status --agent|--human`

### Recommended future flags

- `--agent`
- `--human`
- `--output-format json|compact|agent|human`

The goal is consistency with the rest of `ict-engine`, not one-off UX.

## Agent guidance rules

Agent users should not have to infer workflow.

Required rule:

- if Auto-Quant readiness is blocked, always surface the deterministic next command first

Examples:

- missing checkout -> `ict-engine auto-quant-bootstrap --state-dir ...`
- unhealthy checkout -> `ict-engine auto-quant-update --state-dir ...`
- missing data -> `uv run .../prepare.py`

Only after that should the system include explanatory prose.

## Cross-host expectations

OOTB does not mean "works with zero prerequisites on every OS".
It means:

- the system can identify what is missing
- the system can say what to do next
- the system does not require hidden maintainer-local knowledge

For another host, acceptable first-run outcomes are:

1. full success
2. deterministic blocked-with-next-command

Unacceptable outcome:

- vague conversational confusion

## Acceptance criteria

This line is done when a fresh user can:

1. clone `ict-engine`
2. run one Auto-Quant-related command
3. get one of:
   - working state
   - exact next action
4. do so without spending tokens teaching the agent the setup model

## Immediate next implementation target

Build a unified Auto-Quant readiness state surface and use it in:

- `auto-quant-status`
- `factor-research --backend auto-quant`
- `factor-autoresearch --backend auto-quant`
- `auto-quant-adoption-review`

That is the smallest step that materially improves user-facing MCP/OOTB readiness without creating new structural debt.
