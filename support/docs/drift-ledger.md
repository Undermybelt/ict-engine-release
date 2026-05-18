# Drift ledger

## 2026-04-15 ensemble-voting assessment
- symptom: new architecture suggestion risks landing directly in large CLI/workflow surfaces
- root cause: repo already has strong typed state/reporting artifacts, but extension seams for posterior audit and execution voting are not yet formalized
- repair order that worked:
  1. inspect routing + repo structure
  2. inspect belief posterior, reflection bundle, workflow snapshot, artifact ledger surfaces
  3. constrain change surface to docs first
  4. define recommended insertion points before code changes
- permanent guardrail: ensemble execution work should land as typed domain/application/state artifacts first, then narrow CLI/helper wiring
