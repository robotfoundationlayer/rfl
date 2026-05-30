# Examples

Worked examples that exercise the full Skill ISA → Translation Layer → Driver Interface chain across structurally distinct embodiments.

## Planned examples

| # | Example | Embodiments exercised | Status |
|---|---|---|---|
| 01 | `cable-insertion/` | Tendon-driven 4-finger (e.g., Allegro), direct-drive 4-finger (e.g., LEAP), pneumatic 6-finger | populated (skill + 3 descriptors + walkthrough) |
| 02 | `pick-and-place/` | Same three + collaborative arm | planned |
| 03 | `bimanual-handoff/` | Two arms with different end-effector classes | planned |

Each example contains:

- `README.md` — what the example demonstrates and how to run it
- `skill.yaml` — the high-level Skill ISA composition
- `embodiments/*.yaml` — embodiment descriptors for each target hand
- `run.py` (or `run.sh`) — the entry script
