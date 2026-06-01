#!/usr/bin/env bash
# End-to-end RFL demo — no hardware required.
#
# Runs the whole pipeline on the committed cable-insertion example:
#   validate -> retarget -> certify (live, via a mock driver) -> verify
#   -> sign -> verify (signed)
#
# The "mock driver" is a one-line stand-in for a real embodiment driver: it
# drains the canonical execute goals on stdin and replays a committed,
# conforming telemetry+status session. The certificate it produces is
# byte-identical to the --report replay path, demonstrating that the live
# (--driver) and replay (--report) certify paths agree.
#
#   scripts/demo.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SKILL="examples/01-cable-insertion/skill.yaml"
EMB="examples/01-cable-insertion/embodiments/allegro.yaml"
REPORT="$ROOT/examples/01-cable-insertion/driver-report.jsonl"

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

say "3. certify (live) — spawn a mock driver and check Class-3 obligations"
# A mock embodiment driver: drain the execute goals, replay a recorded session.
cat > "$WORK/mock-driver.sh" <<EOF
#!/bin/sh
cat > /dev/null
cat "$REPORT"
EOF
chmod +x "$WORK/mock-driver.sh"
"$RFL" certify --skill "$SKILL" --embodiment "$EMB" \
  --driver "$WORK/mock-driver.sh" --out "$WORK/certificate.json"

say "4. verify — re-check the certificate's integrity (unsigned)"
"$RFL" verify "$WORK/certificate.json"

say "5. sign — attach an ed25519 signature"
"$RFL" keygen "$WORK/signer.key" >/dev/null
"$RFL" sign --key "$WORK/signer.key" "$WORK/certificate.json" > "$WORK/certificate.signed.json"

say "6. verify (signed) — integrity + signer"
"$RFL" verify "$WORK/certificate.signed.json"

printf '\n\033[1;32mDemo complete — full validate→retarget→certify→verify→sign loop, no hardware.\033[0m\n'
