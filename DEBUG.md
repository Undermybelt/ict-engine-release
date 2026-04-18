# DEBUG

## Task
Recover `src/main.rs` drift/truncation before wiring production-stage logic_family auto-injection.

## Reproduction
- `read_file` showed current `src/main.rs` only ~2000 lines.
- Earlier compile/test positions referenced lines >17000 and >21000.
- `git diff -- src/main.rs` revealed a huge deletion from around line 1998 onward.
- Tail of current file ends mid-match arm at `pre_bayes_status_command`, proving truncation.

## Evidence
- `wc -l src/main.rs` -> 2000
- `git diff` shows nearly the entire lower body deleted after line ~1998
- patch/lint reported `unclosed delimiter`
- therefore current working tree `src/main.rs` is inconsistent/truncated, not safe for further direct patching

## Root cause hypothesis
- Earlier patch/edit against `src/main.rs` landed while file was already in drifted state or truncation occurred during prior edit cycle.
- Any further production-path wiring into `main.rs` must wait until file is restored to a consistent version.

## Recovery rule
1. Do not continue feature work inside current truncated `main.rs`.
2. Restore `src/main.rs` from repo HEAD or other known-good source.
3. Re-run focused tests.
4. Only then resume production-stage logic_family auto-injection.

## Current status
- BBN typed logic fields, overlay loader, CPT loader, and inference tests are good.
- Production-stage injection into `main.rs` remains blocked pending file recovery.
