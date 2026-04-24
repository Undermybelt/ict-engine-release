#!/usr/bin/env bash
# Round 2 §3.5 — round-trip smoke replay.
# Reads a real state dir + symbol, runs the Tucker snapshot example, and
# asserts the three artifacts our Round 1/2 work writes end up on disk with
# the expected fields. Exit non-zero if any assertion fails.
#
# Usage:
#   bash scripts/round2_smoke_replay.sh [state_dir] [symbol]
#
# Defaults to state_autoresearch_smoke / NQ (smallest real fixture in the repo).

set -euo pipefail

STATE_DIR="${1:-state_autoresearch_smoke}"
SYMBOL="${2:-NQ}"

if [[ ! -d "${STATE_DIR}/${SYMBOL}" ]]; then
  echo "FAIL: ${STATE_DIR}/${SYMBOL} does not exist" >&2
  exit 2
fi

say() { printf '[round2-smoke] %s\n' "$*"; }

assert_json_has() {
  local file="$1"
  local key="$2"
  if ! grep -q "\"${key}\"" "${file}"; then
    echo "FAIL: ${file} missing key \"${key}\"" >&2
    exit 1
  fi
}

say "state_dir=${STATE_DIR} symbol=${SYMBOL}"

# --- Step 1: run Tucker snapshot example -----------------------------------
say "cargo run --example round2_tucker_snapshot -- ${STATE_DIR} ${SYMBOL}"
if ! cargo run --quiet --example round2_tucker_snapshot -- "${STATE_DIR}" "${SYMBOL}"; then
  echo "FAIL: tucker snapshot example returned non-zero" >&2
  exit 1
fi

# --- Step 2: assert tucker artifact landed ---------------------------------
TUCKER_PATH="${STATE_DIR}/${SYMBOL}/factor_tucker_core.json"
if [[ ! -f "${TUCKER_PATH}" ]]; then
  echo "FAIL: ${TUCKER_PATH} not written" >&2
  exit 1
fi
say "tucker artifact present: ${TUCKER_PATH}"
assert_json_has "${TUCKER_PATH}" "tucker"
assert_json_has "${TUCKER_PATH}" "factor_labels"
assert_json_has "${TUCKER_PATH}" "regime_labels"
assert_json_has "${TUCKER_PATH}" "reconstruction_error"

# --- Step 3: assert ledger entry kind --------------------------------------
LEDGER="${STATE_DIR}/${SYMBOL}/artifact_ledger.json"
if [[ ! -f "${LEDGER}" ]]; then
  echo "FAIL: ${LEDGER} not present" >&2
  exit 1
fi
if ! grep -q "factor_tucker_core" "${LEDGER}"; then
  echo "FAIL: ${LEDGER} missing factor_tucker_core kind" >&2
  exit 1
fi
say "ledger has factor_tucker_core entry"

# --- Step 4: optional — assert execution_artifact has spectral_metrics ------
EXEC_ART="${STATE_DIR}/${SYMBOL}/execution_artifact.json"
if [[ -f "${EXEC_ART}" ]]; then
  if grep -q "spectral_metrics" "${EXEC_ART}"; then
    say "execution_artifact.json carries spectral_metrics ✓"
  else
    say "execution_artifact.json present but no spectral_metrics (legacy v1 from before Round 1)"
  fi
else
  say "execution_artifact.json absent — rerun analyze to populate (non-fatal)"
fi

# --- Step 5: optional — assert mece_recovery_artifact carries round2 fields -
MECE_ART="${STATE_DIR}/${SYMBOL}/mece_recovery_artifact.json"
if [[ -f "${MECE_ART}" ]]; then
  if grep -q "sparsity_ratio" "${MECE_ART}" && grep -q "segments" "${MECE_ART}"; then
    say "mece_recovery_artifact.json carries sparsity_ratio + segments ✓"
  else
    say "mece_recovery_artifact.json present but missing round2 fields (legacy v1)"
  fi
else
  say "mece_recovery_artifact.json absent — rerun recovery to populate (non-fatal)"
fi

say "PASS"
