#!/usr/bin/env bash
# RFL demo for the SBIR presentation video.
# One Skill ISA description, certified on two opposite hands — no hardware.
#   retarget -> certify (live mock sim) -> sign -> verify   (rigid Allegro)
#   then the SAME skill on a soft pneumatic hand            (cross-embodiment = "general-purpose")
#
# Pacing is tuned for screen capture. Override delays for a fast correctness run:
#   TYPE_DELAY=0 GAP=0 bash scripts/demo-video.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
RFL="$ROOT/target/debug/rfl"
[ -x "$RFL" ] || cargo build -q -p rfl-cli

SKILL="examples/01-cable-insertion/skill.yaml"
A="examples/01-cable-insertion/embodiments/allegro.yaml"       # rigid multi-finger
B="examples/01-cable-insertion/embodiments/pneumatic-6f.yaml"  # soft pneumatic
WORK="$(mktemp -d)"; trap 'rm -rf "$WORK"' EXIT

TYPE_DELAY="${TYPE_DELAY:-0.018}"
GAP="${GAP:-1.0}"
type_cmd(){ printf '\033[1;32m$ \033[0m'; local s="$1" i; for ((i=0;i<${#s};i++)); do printf '\033[1m%s\033[0m' "${s:$i:1}"; sleep "$TYPE_DELAY"; done; printf '\n'; sleep 0.35; }
note(){ printf '\n\033[1;36m# %s\033[0m\n' "$1"; sleep 0.6; }
gap(){ sleep "$(echo "$GAP * ${1:-1}" | bc -l 2>/dev/null || echo "$GAP")"; }

# reference simulators (supply side): drain execute goals on stdin, emit a fresh conformant session
mk_sim(){ cat > "$2" <<EOF
#!/bin/sh
cat > /dev/null
exec "$RFL" sim --skill "$ROOT/$SKILL" --embodiment "$ROOT/$1"
EOF
chmod +x "$2"; }
mk_sim "$A" "$WORK/sim-A.sh"; mk_sim "$B" "$WORK/sim-B.sh"

clear 2>/dev/null || true
note "RFL: one skill description, certified on two opposite hands. No hardware."
gap

note "Embodiment A — Wonik Allegro (rigid multi-finger hand)"
type_cmd "rfl retarget skill.yaml --embodiment allegro.yaml | head -1"
"$RFL" retarget "$SKILL" --embodiment "$A" | head -1
echo "   ($("$RFL" retarget "$SKILL" --embodiment "$A" | wc -l | tr -d ' ') canonical execute goals)"
gap

type_cmd "rfl certify --skill skill.yaml --embodiment allegro.yaml --driver ./sim --out certA.json"
"$RFL" certify --skill "$SKILL" --embodiment "$A" --driver "$WORK/sim-A.sh" --out "$WORK/certA.json"
gap

type_cmd "rfl keygen signer.key  &&  rfl sign --key signer.key certA.json > certA.signed.json"
"$RFL" keygen "$WORK/signer.key" >/dev/null
"$RFL" sign --key "$WORK/signer.key" "$WORK/certA.json" > "$WORK/certA.signed.json"
type_cmd "rfl verify certA.signed.json"
"$RFL" verify "$WORK/certA.signed.json"
gap 1.5

note "SAME skill — Embodiment B: 6-finger PNEUMATIC SOFT hand (opposite grasp principle)"
type_cmd "rfl certify --skill skill.yaml --embodiment pneumatic-6f.yaml --driver ./sim --out certB.json"
"$RFL" certify --skill "$SKILL" --embodiment "$B" --driver "$WORK/sim-B.sh" --out "$WORK/certB.json"
gap
type_cmd "rfl verify certB.json"
"$RFL" verify "$WORK/certB.json"
gap 1.5

printf '\n\033[1;32m  Same Skill ISA -> certified on a RIGID hand AND a SOFT pneumatic hand.\033[0m\n'
printf '\033[1;32m  Cross-embodiment reproducibility = a checkable definition of "general-purpose".\033[0m\n\n'
gap
