# timed-pda recovery compact

目标：恢复 ict-engine 中 timed PDA 相关模块到一致可编译状态，再继续真实 NQ 边际评估。

当前真实阻断：
1. src/types.rs 缺 timed PDA 基础类型：
   - PriceLevelBand
   - PdaLifecycleState
   - PdaInvalidationRule
   - PdaInverseMode
   - PdaConceptKind
   - PdaStateTransition
   - TimedPdaState
2. src/factor_lab/factor_definition.rs 缺较新版 role/phase 能力：
   - FactorUsagePhase
   - FactorSignal.ensure_phase(...)
3. 导出/实现层漂移：
   - src/ict/mod.rs 未导出 pda_state
   - src/bbn/mod.rs 未导出 summarize_timed_pda_states
   - src/bbn/evidence.rs 旧版，无 timed PDA summary / EvidenceBinding
   - src/bbn/trading/update.rs 旧版，无 trade_evidence_with_timed_pda_summary
   - src/main.rs 为新旧混合态，已依赖上列缺失符号

现已确认的正确方向：
- 先修基础类型层，再修 factor role/phase，再修 bbn/ict 导出实现层，最后再修 main.rs 调用面。
- 否则会反复出现“上一层缺符号”。

推荐修复顺序：
1. src/types.rs
   - 补全部 timed PDA 类型
   - 若已有 ICTStructureSummary，保留 timed_pda_states: Vec<TimedPdaState>
2. src/factor_lab/factor_definition.rs
   - 审查当前版本是否已有 FactorRole / FactorUsagePhase / ensure_phase
   - 若缺，补齐最小可用版，与 bbn/evidence.rs 对齐
3. src/ict/mod.rs
   - pub mod pda_state;
   - pub use pda_state::*;
4. src/bbn/evidence.rs
   - 恢复 summarize_timed_pda_states
   - 恢复 EvidenceBinding / validate_factor_evidence_binding / tests
5. src/bbn/mod.rs
   - 导出 summarize_timed_pda_states
6. src/bbn/trading/update.rs
   - 恢复 trade_evidence_with_timed_pda_summary
   - entry_quality 使用 network 的真实状态名：high/medium/low
7. src/main.rs
   - 最后统一修正调用面
   - 确保 build_pre_bayes_evidence_filter 新签名全修
   - ensure workflow/pre-bayes status 输出 timed PDA 五字段

当前 compact 状态：
- pda_state.rs 本身较新，含概念专属规则表 pda_rule_spec。
- main.rs 头部已部分修到新版，但仍因上游模块缺失而报错。
- compact_router 也因上述编译错误无法给 next-action。

验证标准：
1. cargo check
2. cargo test
3. 再跑真实 NQ：
   cargo run -- factor-pipeline-debug --symbol NQ --data /Users/thrill3r/Downloads/Tomac/ict-cleaned-mtf/cleaned-15m/nq.continuous-15m.json --factor structure_ict --objective expansion_manipulation

进入下一轮时，先读此文件，再按顺序修，不要先乱 patch main.rs。
