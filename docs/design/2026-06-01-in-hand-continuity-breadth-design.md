# In-hand continuity breadth — `rotate` / `translate` / `roll` / `slide`

**Status**: design-complete, 2026-06-01. Implementation track wave 2 (primitive breadth).

## Goal

Add the four remaining `in_hand` manipulation primitives (`spec/01` § 3.1 / 3.2 / 3.4 / 3.6). v0
already lowers `in_hand.flip` (3.7, continuity-suspending), `in_hand.regrasp` (3.3, GC3),
`in_hand.pivot` (3.5, GC4). These four are the **plain continuity-preserving** in-hand operations:
they reposition the object within the grasp while keeping grasp identity and stability class, so
they carry only the GC1 securing floor — no new obligation dimension, no `grasp_stability` (no new
grasp to confirm — GC2 is vacuous), like `in_hand.pivot` minus the released-DOF / GC4 marker.

## Lowering

All four share a `lower_in_hand_manip` helper that emits, from the read-only `ctx.held`:
`force_profile.min_holding_force` (the GC1 floor, the held object's weight × k), a
`manipulation: <kind>` marker, `target_pose = AxisRelative{axis, distance}`, and **no**
`grasp_stability` (continuity preserved, identity unchanged). Per-primitive:

| primitive | axis | distance | extra |
|---|---|---|---|
| in_hand.rotate | `axis` | `0 mm` (position fixed) | — |
| in_hand.translate | `direction` | `distance` | — |
| in_hand.roll | `roll_axis` | `0 mm` | — |
| in_hand.slide | `slide_direction` | `0 mm` | `stop_condition` → a Monitor |

`angle` (rotate / roll) is carried symbolic (like pivot's). Suffixes: `rotate` / `translate` /
`roll` / `slide`, all `GraspContinuity` in `envelope_class_for`.

## Capability + descriptors

Each keys on its dotted identifier (`in_hand.rotate` / `in_hand.translate` / `in_hand.roll` /
`in_hand.slide`), consistent with the existing `in_hand.flip/regrasp/pivot` arms. Added to all
three descriptors (which already declare the other in_hand primitives) → uniform 3-hand goldens.

## Deferred

- **DOF-admissibility** (rotate/roll reject `form_held` / `rotation_constrained` axes; translate/
  slide reject `form_held`) — this is wave 8's class-1 `Skill::validate` check, now unblocked
  because the `rotation_constrained` (tripod) and `form_held` (hook) modes exist. NOT in this wave.
- `keep_position` / `keep_orientation` drift bounds, `regrip_policy` gaiting, `max_*_velocity`
  in-hand clamps — carried/ignored in v0 (symbolic).
