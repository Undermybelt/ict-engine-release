# Environment Variables

`ict-engine` supports a small set of environment variables that affect CLI behavior and runtime defaults.

| Variable | Default | Used by | Purpose | Example |
| --- | --- | --- | --- | --- |
| `ICT_ENGINE_STATE_DIR` | `state` | Most CLI commands with `--state-dir` | Overrides the default state directory when `--state-dir` is omitted. | `export ICT_ENGINE_STATE_DIR=/tmp/ict-engine-state` |
| `ICT_ENGINE_STAGED_ORCHESTRATION` | unset | staged orchestration | Enables staged orchestration flow. | `export ICT_ENGINE_STAGED_ORCHESTRATION=1` |
| `ICT_ENGINE_BELIEF_PRIMARY` | unset | belief engine registry | Selects the primary belief engine implementation. | `export ICT_ENGINE_BELIEF_PRIMARY=loopy` |
| `ICT_ENGINE_FAMILY_HISTORY_WINDOW` | built-in config default | family history windowing | Adjusts the family-history lookback window used in summaries and decisions. | `export ICT_ENGINE_FAMILY_HISTORY_WINDOW=12` |
| `ICT_ENGINE_TOMAC_ROOT` | auto-discovered | futures cleaning / SOP commands | Sets the TOMAC data root when `--root` is omitted. | `export ICT_ENGINE_TOMAC_ROOT=/data/tomac` |
| `ICT_EXECUTION_FOCUS` | unset | workflow/analyze reporting | Enables execution-focus reporting surfaces where wired. | `export ICT_EXECUTION_FOCUS=1` |
| `HOME` | OS-provided | path discovery | Used indirectly for home-relative path discovery. | `echo "$HOME"` |

Preferred order of precedence:

1. Explicit CLI flag
2. Environment variable
3. Built-in default
