# ict-engine compact routing

仅走低 token 接口。

规则：
1. 先用 scripts/compact_router.py。
2. 只调用这些命令：
   - next-action
   - research-compact
   - market-fork-status
   - pre-bayes-compact
   - artifact-gate-compact
   - factor-pipeline-debug --compact
3. 不直接读取 workflow_snapshot.json、research_runs.json、artifact_ledger.json 全量内容，除非 compact 结果不足。
4. 不展开 prompt pack，不回放全历史。
5. 先拿 compact 结果，再决定是否需要更重命令。

示例：
python3 scripts/compact_router.py next-action NQ /tmp/ict-market-fork-check
python3 scripts/compact_router.py research-compact NQ /tmp/ict-market-fork-check
python3 scripts/compact_router.py market-fork-status NQ /tmp/ict-market-fork-check
python3 scripts/compact_router.py pre-bayes-compact NQ /tmp/ict-market-fork-check
python3 scripts/compact_router.py artifact-gate-compact NQ /tmp/ict-market-fork-check

若仍需 sample 级诊断：
cargo run -- factor-pipeline-debug --symbol NQ --data <path> --factor structure_ict --objective expansion_manipulation --compact
