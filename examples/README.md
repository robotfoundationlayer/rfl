# Examples

Worked examples that exercise the full Skill ISA → Translation Layer → Driver Interface chain across structurally distinct embodiments.

## Examples

| # | Example | Embodiments exercised | Status |
|---|---|---|---|
| 01 | `cable-insertion/` | Tendon-driven 4-finger (e.g., Allegro), direct-drive 4-finger (e.g., LEAP), pneumatic 6-finger | populated (skill + 3 descriptors + walkthrough) |
| 02 | `surface-scan/` | Same three | populated (`reach.scan` raster + spiral sweep, `sense.inspect`; `skill.yaml` + `skill-spiral.yaml` + 3 descriptors) |
| 03 | `screw-fasten/` | Same three | populated (grasp + transport + `force.screw` / `force.unscrew`; `skill.yaml` + `skill-unscrew.yaml` + 3 descriptors) |

Each example contains:

- `README.md` — what the example demonstrates and how to run it
- `skill.yaml` — the high-level Skill ISA composition
- `embodiments/*.yaml` — embodiment descriptors for each target hand
- `run.py` (or `run.sh`) — the entry script
