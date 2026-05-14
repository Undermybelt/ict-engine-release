# freqtrade pattern intake — 2026-04-23

Source reviewed:
- local clone: `/Users/thrill3r/freqtrade`
- files read:
  - `README.md`
  - `support/docs/edge.md`
  - `support/docs/advanced-setup.md`
  - `support/docs/configuration.md`

## Verdict

Worth absorbing as configuration-discipline, dry-run/live separation, and risk-expectancy documentation patterns.

Not worth absorbing as a bot-shell or exchange-control product model.

`ict-engine` should learn from Freqtrade's:

- explicit config precedence rules
- mode separation between simulation and live paths
- operator-facing documentation for state/database isolation
- expectancy/risk/reward explanations written in practical terms

But it should reject:

- Telegram/UI/RPC-first bot operations as a core product surface
- exchange account control as a default repo concern
- strategy-file/config-file sprawl without stronger typed boundaries

## What the repo does well

### 1. It makes mode separation operationally explicit

Freqtrade is very clear that:

- dry-run is different from live
- persistence for dry-run and live should be isolated
- multiple concurrent instances need separate state stores

This is one of the strongest transferable ideas for `ict-engine`.

`ict-engine` is not a live trading bot, but it does already have multiple state modes in practice:

- isolated comparison runs
- shared-state autoresearch loops
- reusable smoke baselines
- disposable experiment directories

Freqtrade's docs are a good reminder that mode separation should be written as operator law, not left implicit.

The closest equivalent for `ict-engine` is:

- isolated state dirs for fair comparison
- shared state only for intentional iterative loops
- no casual mixing of experiment families in one state directory

### 2. It writes configuration precedence down clearly

Freqtrade explicitly documents:

- CLI overrides config
- config files override strategy defaults
- later config files override earlier ones

That is strong operator ergonomics.

`ict-engine` has fewer config layers, but the principle transfers well:

- user intent should override repo defaults
- explicit command arguments should override ambient defaults
- docs should say which surface is authoritative when two knobs overlap

This is especially useful where `ict-engine` already has overlapping decision surfaces such as:

- command flags vs derived workflow suggestions
- state-dir defaults vs explicit `--state-dir`
- output mode sugar vs explicit output-format settings

### 3. It treats persistence as part of runtime safety

The multiple-instance database guidance is good because it turns a subtle failure mode into a documented rule.

Translated to `ict-engine`, this suggests a similar documentation stance:

- state collision is a correctness issue, not just a tidiness issue
- mixed-purpose state dirs create false comparisons
- a reused state dir changes what later runs mean

Freqtrade's exact mechanism is SQL databases.
`ict-engine` uses repo-local state artifacts instead.
But the operator lesson is the same: persistence topology changes semantics.

### 4. It explains expectancy and risk in operator language

The `edge.md` material is useful not because `ict-engine` should import the full Edge module, but because it documents:

- win rate
- loss rate
- risk/reward ratio
- expectancy
- position sizing from allowed capital at risk

in a way an operator can actually use.

This is valuable for `ict-engine`, where many research surfaces are numerically rich but not always framed in compact operator language.

The best transfer is documentation style:

- define the metric
- give a worked example
- state what the metric is good for
- warn explicitly about historical overfitting

That would improve some current `ict-engine` scoring and gating docs.

### 5. It is disciplined about warnings and caveats

Freqtrade repeats important warnings in docs:

- dry-run before live
- exchange-specific caveats
- stoploss-on-exchange failure behavior
- historical results are not guarantees

This is valuable.

`ict-engine` benefits from the same tone in documentation:

- what a surface means
- what it does not mean
- what should never be inferred from it

This is already starting to happen in the new derived-surface contract docs, and Freqtrade reinforces that this is the right direction.

## What must not be copied into ict-engine

### 1. Do not import the bot-shell product shape

Freqtrade is fundamentally a trading bot with:

- exchange credentials
- trade execution
- Telegram/RPC control
- live operational controls

That is not the right default identity for `ict-engine`.

So the transfer should stay at:

- documentation discipline
- state isolation patterns
- risk explanation style

not the bot-shell feature set.

### 2. Do not let config sprawl outrun typed boundaries

Freqtrade has a large configuration surface because its product scope is large.

For `ict-engine`, indiscriminately copying that approach would be harmful.

The repo already has enough complexity that new knobs should remain:

- tightly scoped
- typed where possible
- justified by a real operator need

The lesson is not “add more config.”
The lesson is “if there is config, document override order and semantics mechanically.”

### 3. Do not turn historical expectancy into false certainty

Freqtrade's edge docs are useful partly because they explicitly warn that historical expectancy is not future guarantee.

`ict-engine` should absorb this warning discipline aggressively.

It should not:

- present score deltas as robust future edge
- treat autoresearch uplift as equivalent to production advantage
- let derived retrospectives sound more certain than the artifacts justify

## Best-fit migration targets inside ict-engine

### 1. State isolation and lifecycle docs

Best fit:

- `support/docs/state-directory-lifecycle.md`
- `support/docs/research-system-map.md`
- future operator docs for repeated research loops

Pattern to absorb:

- mode-specific state guidance
- separate stores for separate semantic runs
- explicit warning against mixed-purpose persistence

### 2. Config/override docs

Best fit:

- `README.md`
- `support/docs/first-run.md`
- any future environment/config reference docs

Pattern to absorb:

- explicit precedence rules
- examples of what overrides what
- warnings where override ambiguity matters

### 3. Scoring and gating explanation docs

Best fit:

- `support/docs/objective-scoring-map.md`
- `support/docs/research-gaps-and-paper-synthesis.md`
- future docs around execution quality / gating / risk

Pattern to absorb:

- metric definitions in plain language
- worked examples
- overfitting warnings
- “what this metric is not” sections

## Concrete recommendations for ict-engine docs

### Recommendation 1

Add a short explicit precedence note wherever command defaults and environment defaults overlap.

Example topics:

- `--state-dir` vs `ICT_ENGINE_STATE_DIR`
- output-format vs shorthand flags
- user-selected dataset vs previously recorded paths

Freqtrade is strong here because it writes the precedence, not just the knobs.

### Recommendation 2

Keep strengthening the “state-dir semantics” writing.

Good future phrasing would be:

- isolated state is for fair comparison
- shared state is for intentional cumulative loops
- reusing a state dir changes interpretation of later results

This is the strongest practical lesson from Freqtrade's multi-database guidance.

### Recommendation 3

When documenting any future risk or scoring surface, pair the formula with:

- a plain-language reading
- one worked example
- one warning about misuse

Freqtrade's edge docs are a good model for that style.

## Net effect on ict-engine

The right absorption is:

- **yes** to better mode/state separation docs
- **yes** to explicit override-precedence docs
- **yes** to practical risk/expectancy explanation style
- **yes** to stronger warning language around historical inference
- **no** to adopting the live-bot shell as core repo identity
- **no** to config-surface explosion without typed justification

## One-sentence takeaway

Freqtrade is useful to `ict-engine` mainly as a documentation example of how to make simulation/live boundaries, persistence isolation, and risk metrics legible to operators without pretending that historical tuning results are stronger than they are.
