# CLI help audit

Date: generated mechanically from live `cargo run -- --help` surfaces.

Verdict
- status: pass
- root `--version`: present
- audited subcommands: 22
- commands with missing option descriptions: 0

Method
- enumerate subcommands from root help
- run `<subcommand> --help` for each
- parse `Options:` block
- require every non-help option to have a description line or same-line description
- source of truth is live clap output, not source grep

Summary table

| Command | Options | Missing descriptions |
|---|---:|---:|
| `analyze` | 12 | 0 |
| `analyze-live` | 11 | 0 |
| `train` | 5 | 0 |
| `backtest` | 12 | 0 |
| `update` | 10 | 0 |
| `factor-research` | 15 | 0 |
| `factor-mutation-status` | 8 | 0 |
| `factor-autoresearch` | 18 | 0 |
| `factor-autoresearch-status` | 6 | 0 |
| `research-verdict` | 3 | 0 |
| `evidence-quality-breakdown` | 4 | 0 |
| `factor-backtest` | 12 | 0 |
| `clean-futures` | 5 | 0 |
| `futures-sop` | 4 | 0 |
| `expansion-sop` | 9 | 0 |
| `factor-pipeline-debug` | 11 | 0 |
| `workflow-status` | 15 | 0 |
| `pre-bayes-status` | 5 | 0 |
| `pre-bayes-diff` | 4 | 0 |
| `artifact-lineage` | 8 | 0 |
| `artifact-status` | 16 | 0 |
| `artifact-diff` | 5 | 0 |

Artifacts
- machine report: `support/docs/audits/help-audit.json`
- audit script: `support/scripts/help_audit.py`

Notes
- prior false positives came from naive parsing that assumed descriptions always render on separate wrapped lines
- current audit accepts both same-line and wrapped clap help text
