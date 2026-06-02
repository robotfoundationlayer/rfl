#!/usr/bin/env bash
# Provisional ε from the stochastic reference simulator — no hardware required.
#
# The deterministic `rfl sim` has zero run-to-run variation (ε = 0). This sweeps
# `rfl sim --seed` over a declared variation model to generate varied traces, then
# runs `rfl measure` to produce a PROVISIONAL, sim-derived ε table.
#
# The σ live in the pending schemas/simulator-declaration.yaml `variation_model`
# and are DECLARED (illustrative), never physically measured. The committed
# normative schemas/epsilon-tolerances.yaml stays null; this output is provisional
# and printed to stdout, never written there.
#
#   scripts/provisional-epsilon-from-sim.sh [N_RUNS]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

RUNS="${1:-5}"
SKILL="examples/01-cable-insertion/skill.yaml"
EMB="examples/01-cable-insertion/embodiments/allegro.yaml"
DECL="schemas/simulator-declaration.yaml"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cargo build -q -p rfl-cli
RFL="$ROOT/target/debug/rfl"

ARGS=()
for ((s = 0; s < RUNS; s++)); do
  "$RFL" sim --skill "$SKILL" --embodiment "$EMB" --variation "$DECL" --seed "$s" > "$WORK/run$s.jsonl"
  ARGS+=(--run "$WORK/run$s.jsonl")
done

"$RFL" measure --skill "$SKILL" --embodiment "$EMB" "${ARGS[@]}"
