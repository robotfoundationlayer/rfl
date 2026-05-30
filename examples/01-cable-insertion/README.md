# Example 01 — Cable insertion across three hand classes

> **Status**: skeleton. Full example targeted for v0.1 (2027 Q1).

A worked example demonstrating that the same Skill ISA composition executes correctly on three structurally distinct hand classes:

- Tendon-driven 4-finger anthropomorphic (e.g., Wonik Allegro)
- Direct-drive 4-finger (e.g., CMU LEAP)
- Pneumatic 6-finger (e.g., a McKibben-style hand)

## Task

Pick up a flexible cable terminated by a connector and insert it into a receptacle. The task combines:

- Compliant-object grasping (the cable is flexible)
- Force-controlled insertion (over-insertion damages pins)
- Visual servoing (the receptacle pose has sub-millimeter tolerance)

## Files (planned)

- `skill.yaml` — the high-level Skill ISA composition
- `embodiments/allegro.yaml` — Allegro Hand descriptor
- `embodiments/leap.yaml` — LEAP Hand descriptor
- `embodiments/pneumatic-6f.yaml` — generic pneumatic 6-finger descriptor
- `run.py` — execution driver (loads skill, loads embodiment, calls `retarget`, dispatches)

## How to run (once v0.1 ships)

```bash
# From repository root
cargo run -p rfl-cli -- retarget examples/01-cable-insertion/skill.yaml \
    --embodiment examples/01-cable-insertion/embodiments/allegro.yaml
```
