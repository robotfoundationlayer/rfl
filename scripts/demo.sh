#!/usr/bin/env bash
# End-to-end RFL demo — no hardware required.
#
# Runs the whole pipeline on the committed cable-insertion example:
#   validate -> retarget -> certify (live, via a mock driver) -> verify
#   -> sign -> verify (signed)
#
# The "driver" spawned by `rfl certify --driver` is `rfl sim`, the reference
# simulator: it drains the canonical execute goals on stdin and emits a freshly
# generated, conformant telemetry+status session (no pre-recorded report) — the
# supply-side reference driving the demand-side certify, all in software.
#
#   scripts/demo.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SKILL="examples/01-cable-insertion/skill.yaml"
EMB="examples/01-cable-insertion/embodiments/allegro.yaml"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

say() { printf '\n\033[1m== %s\033[0m\n' "$1"; }

say "Build the rfl CLI"
cargo build -q -p rfl-cli
RFL="$ROOT/target/debug/rfl"

say "1. validate — parse + Class-1 composition checks"
"$RFL" validate "$SKILL"

say "2. retarget — lower onto the Allegro hand (first goal shown)"
"$RFL" retarget "$SKILL" --embodiment "$EMB" | head -1
echo "   ($("$RFL" retarget "$SKILL" --embodiment "$EMB" | wc -l | tr -d ' ') canonical execute goals total)"

say "3. certify (live) — spawn the reference simulator and check Class-3 obligations"
# The driver is `rfl sim`: drain the execute goals on stdin, then generate a
# fresh conformant session for this skill+embodiment (no pre-recorded report).
cat > "$WORK/sim-driver.sh" <<EOF
#!/bin/sh
cat > /dev/null
exec "$RFL" sim --skill "$ROOT/$SKILL" --embodiment "$ROOT/$EMB"
EOF
chmod +x "$WORK/sim-driver.sh"
"$RFL" certify --skill "$SKILL" --embodiment "$EMB" \
  --driver "$WORK/sim-driver.sh" --out "$WORK/certificate.json"

say "4. verify — re-check the certificate's integrity (unsigned)"
"$RFL" verify "$WORK/certificate.json"

say "5. sign — attach an ed25519 signature"
"$RFL" keygen "$WORK/signer.key" >/dev/null
"$RFL" sign --key "$WORK/signer.key" "$WORK/certificate.json" > "$WORK/certificate.signed.json"

say "6. verify (signed) — integrity + signer"
"$RFL" verify "$WORK/certificate.signed.json"

printf '\n\033[1;32mDemo complete — full validate→retarget→certify→verify→sign loop, no hardware.\033[0m\n'
