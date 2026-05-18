# ICT execution setup tree for policy layer

Status: experimental schema only

Purpose
- Compress ICT PDA/entry intuition into repo-compatible execution-policy features.
- Feed CatBoost/XGBoost style voting after belief/posterior qualification.
- Do not replace canonical timed PDA state machine.
- Do not treat setup flags as Bayesian hard evidence.

Non-goals
- Not a canonical PDA detector.
- Not a direct trade trigger by itself.
- Not a replacement for `src/ict/pda_state.rs`.
- Not a claim that 3-candle heuristics alone fully define ICT truth.

Placement in repo
- Canonical PDA truth remains:
  - `src/types.rs` timed PDA types
  - `src/ict/pda_state.rs` lifecycle/invalidation/inverse rules
- This tree belongs downstream as:
  - execution setup classifier
  - policy/voting feature layer
  - reflection/report explanation surface

Hard gates
1. `higher_tf_bias_match == 1`
2. `discount_premium_correct == 1`
3. `liquidity_context_valid == 1`
4. else -> `Observe`

Recommended feature families

Categorical
- `setup_family`
  - `order_block`
  - `fair_value_gap`
  - `inverse_fvg`
  - `breaker_block`
  - `mitigation_block`
  - `rejection_block`
  - `propulsion_block`
  - `liquidity_void`
  - `volume_imbalance`
  - `ote_confluence`
  - `silver_bullet`
  - `judas_swing`
  - `turtle_soup`
  - `none`
- `entry_style`
  - `limit_pullback`
  - `market_confirmation`
  - `stop_confirmation`
  - `observe`
- `risk_template`
  - `tight_external`
  - `structure_external`
  - `void_external`
  - `session_liquidity`
  - `observe_only`
- `setup_quality`
  - `high`
  - `medium`
  - `low`
- `signal_bar_pattern`
  - `none`
  - `displacement`
  - `reversal_reclaim`
  - `sweep_reject`
- `session_model`
  - `silver_bullet`
  - `judas`
  - `turtle_soup`
  - `standard`

Binary / one-hot
- `higher_tf_bias_match`
- `discount_premium_correct`
- `liquidity_swept`
- `signal_bar_present`
- `pda_signal_overlap`
- `ote_discount_zone`
- `silver_bullet_time`
- `judas_swing_flag`
- `turtle_soup_flag`
- `timed_pda_active_nearby`
- `timed_pda_inversed_nearby`
- `timed_pda_stale_nearby`

Numerical
- `pda_distance_bps`
- `pda_width_bps`
- `overlap_ratio`
- `displacement_strength`
- `sweep_depth_bps`
- `entry_price_offset_bps`
- `sl_distance_bps`
- `tp_rr_ratio`
- `timed_pda_mitigation_progress`

Repo-compatible policy semantics
- `timed_pda_active_nearby` should be derived from canonical timed PDA state, not from a second detector.
- `setup_family` may summarize extra heuristics, but must not override timed-PDA lifecycle truth.
- `session_model` is a setup classifier only.
- `signal_bar_present` and `pda_signal_overlap` are execution qualifiers, not posterior evidence.

Pseudo decision rules
1. If hard gate fails:
   - action: `Observe`
   - qualification: `disqualified`
   - entry_style: `observe`
2. If `setup_family in {order_block, breaker_block, fair_value_gap}` and `pda_signal_overlap == 1`:
   - prefer `market_confirmation`
   - upgrade `setup_quality`
3. If `setup_family in {order_block, fair_value_gap, mitigation_block}` and `signal_bar_present == 0`:
   - prefer `limit_pullback`
4. If `session_model in {silver_bullet, judas, turtle_soup}` and `liquidity_swept == 1`:
   - upgrade `setup_quality`
   - tighten `risk_template`
5. If `timed_pda_inversed_nearby == 1` and trade direction conflicts with that inverse:
   - downgrade to `observe`
6. If `timed_pda_stale_nearby == 1` and no fresh displacement:
   - downgrade to `observe`

Leaf output examples
- `Long_Limit_OB`
- `Short_Limit_FVG`
- `Long_Market_Confirmation`
- `Short_Market_Confirmation`
- `Wait_For_Pullback`
- `Observe`

Reflection/report intent
Reflection should explain:
- setup family
- whether overlap confirmed
- whether trade is pullback vs confirmation style
- whether timed PDA agreed, inversed, or stale
- risk template summary

Guardrails
- Keep canonical timed PDA ownership in `src/ict/pda_state.rs`.
- Keep BBN evidence ownership separate.
- Treat this tree as execution-policy metadata only.
- If a heuristic conflicts with timed PDA lifecycle, timed PDA wins.

Suggested next code surface
- Extend `PolicyFeatureVector`
- Extend sample CatBoost artifact schema
- Add optional reflection summary fields for setup-tree output
