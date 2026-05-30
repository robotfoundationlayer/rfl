# Example 01 — Cable insertion across three hand classes

> **Status**: worked example populated (2026-05-31). The three embodiment descriptors **validate against `schemas/embodiment-descriptor.schema.json`** (Draft 2020-12, verified); `skill.yaml` is a reference instance of the forthcoming `skill-isa.schema.json`; `run.py` invokes the reference CLI (`crates/rfl-cli`, in progress). The trace below is the design-level walkthrough.

One Skill ISA composition (`skill.yaml`) executes on three **structurally distinct** hand classes without change — the operational test of Principle 1 (embodiment-agnostic):

| Hand | Actuation | Contact sensing | Confirmation tier |
|---|---|---|---|
| Wonik Allegro (`allegro.yaml`) | tendon-driven 4-finger | distributed fingertip tactile | **manifold** |
| CMU LEAP (`leap.yaml`) | direct-drive 4-finger | fingertip force (coarser) | **manifold** |
| Pneumatic 6-finger (`pneumatic-6f.yaml`) | McKibben / bellows | **none** | **proxy** (force/position) |

The same `skill.yaml` retargets onto all three; only the embodiment descriptor changes. The pneumatic hand has no contact sensing at all, so every contact confirmation degrades to the force/position proxy — exercising the graceful-degradation path (`04` § Graceful degradation and the force/position proxy) end-to-end.

## Task

Pick up a flexible cable terminated by a rigid connector and insert the connector into a receptacle under force control. The task combines visual localization, an antipodal grasp on the connector, disturbance-free transport, axis alignment to sub-millimetre tolerance, and a force-controlled insertion that must tell a **seated** connector from a **jam**.

## Files

- `skill.yaml` — the embodiment-agnostic Skill ISA composition (no hand named)
- `embodiments/allegro.yaml`, `embodiments/leap.yaml`, `embodiments/pneumatic-6f.yaml` — the three target descriptors
- `run.py` — loads the skill + one embodiment and calls `retarget`

## Walkthrough — the skill, step by step

Each step names the chapter that governs it. The grasp state (`01` § Grasp state model) advances `free → held → … → free` across the sequence.

1. **`sense.locate(connector)` → `LetBind connector_t`.** A `Measurement(pose, uncertainty)` is bound into an `ObjectTarget` the grasp consumes — the observe→act L1 loop (`01` § Object reference). The bind is checked so the measured uncertainty does not exceed the grasp's target-resolution bound (`01` `LetBind` flow checks).

2. **`grasp.pinch(connector_t, 8 N)`.** `free → held`, `mode = pinch`, `closure = force`. `tactile_target = auto` expands to `normal_force ≥ ε at_least(2, antipodal)` (`04` § The `TactileTarget` type). The `force_budget` is clamped to `grip_force_max` and the connector's `max_contact_force`; `min_holding_force` is derived from the connector weight, mode, friction, and load direction (`02` GF1c). **Confirmation forks by hand**: Allegro/LEAP confirm via the manifold (`normal_force` at the opposing sites); the pneumatic hand, lacking `tactile_sensing`, confirms via the proxy — `grasp_width` converged to the connector cross-section **and** actuator force held over the confirm window (`04` § The force/position proxy), reported at `proxy` tier.

3. **`transport.move_to_pose(receptacle − 30 mm·z)`.** `held → held`. `max_acceleration` is clamped to the dynamic grasp-stability limit so the inertial load never exceeds holding capacity (`02` GF2c). The held connector is verified against the **grasp-continuity** envelope class throughout (`05` GC1): a securing set holds it at `≥ min_holding_force` at every sampled instant.

4. **`reach.align(receptacle, axes = [z])`** after a visual `sense.locate(receptacle)`. Only the insertion axis is constrained, so the residual orientation is resolved as the **minimum geodesic rotation** from the current orientation (`02` § Pose representation, CA2c) — deterministic, not arbitrary. This brings the connector within `force.insert_fit`'s start-capturable range.

5. **`force.insert_fit(receptacle_t, 15 N, all_of{effort_rise 12 N, reached 8 mm})`.** The `all_of` stop condition **is** the seating/jam discriminator (`01` § Stop / completion conditions): an effort rise *at* the expected depth is **seated**; the same rise *without* depth is a **jam**, never reported as success. The insertion reaction loads the grasp along the axial direction — bounded by the reaction-load limit so the connector does not slip in-grasp before it seats (`02` GF3c). The whole force profile is interval-sampled against the budget (`05` ENV4, the force/torque-trajectory class) — a mid-insertion spike is a violation. Because compliant search runs against contact dynamics, the *realized* trajectory is **Class 2-loose** (`02` RD2c); its canonical-action *generation* stays Class 2-strict. **Seating detection forks by hand**: Allegro/LEAP read the effort rise from fingertip force; the pneumatic hand reads it from chamber pressure (the force proxy), at `proxy` tier.

6. **`grasp.release` then `reach.retract(50 mm)`.** `held → free` under the shared break-contact clause (`01` § World-state model), then a terminal-postcondition retraction (`05`) with directional force monotonicity (a force *increase* during retreat signals a snag and aborts).

## How the one skill becomes three command streams

`retarget` emits a different canonical-action stream per hand from the *same* skill — the table makes the divergence concrete:

| Concern | Allegro | LEAP | Pneumatic 6-finger |
|---|---|---|---|
| `grasp.pinch` realization | tendon torques, 2 opposing fingertips | direct-drive joint torques | opposed compliant fingers around the palm frame |
| Contact confirmation | TactileManifold (`normal_force`, `shear`) | TactileManifold (`normal_force`) | force/position **proxy** (no tactile) |
| Confirmation fidelity tier | `manifold` | `manifold` | `proxy` |
| Insertion seating | fingertip force at depth | fingertip force at depth | chamber pressure at depth (proxy) |
| Compliance for search | `active` | `active` | `passive` (inherent) |
| Realized-execution class | 2-loose (compliant insert) | 2-loose | 2-loose |
| `max_acceleration` | clamped to its dynamic-stability limit | higher (direct-drive) | lower (soft, slow) |

The semantics are identical; the embodiment descriptor is the only thing that changed.

## Conformance envelope classes exercised (`05`)

- **Terminal-postcondition** — `reach.align`, `reach.retract` (endpoint at rest, no task contact).
- **Grasp-continuity (GC1)** — the connector held through `transport` and `force.insert_fit` (`≥ min_holding_force` every instant).
- **Force/torque-trajectory (ENV4)** — `force.insert_fit`'s force profile interval-sampled against the budget.
- **Tactile independence (`03` G4c / `04` TM6c)** — the pneumatic run passes `grasp.pinch`'s nominal hold test via the proxy with `tactile_sensing` undeclared.

## How to run

```bash
# From the repository root, once the rfl-cli reference implementation builds:
python3 examples/01-cable-insertion/run.py --embodiment allegro
python3 examples/01-cable-insertion/run.py --embodiment leap
python3 examples/01-cable-insertion/run.py --embodiment pneumatic-6f

# Equivalent direct CLI invocation:
cargo run -p rfl-cli -- retarget examples/01-cable-insertion/skill.yaml \
    --embodiment examples/01-cable-insertion/embodiments/allegro.yaml
```

Until `rfl-cli` builds, `run.py` prints the intended invocation; the skill and embodiment descriptors are complete and reviewable now.
