# Place family breadth — `put_down` / `stack` / `insert_loose` / `orient` / `hand_to` / `discard`

**Status**: design-complete, 2026-06-01. Implementation track wave 3 (primitive breadth, new
`place` category).

## Goal

Add the entire `place` category (`spec/01` § 5, 6 primitives) — the first primitive of a new
category the Translation Layer lowers. Every `place` primitive is the same shape: an object is
**held** through a placement motion, then released under a controlled, verified condition. By
`spec/05` ENV1 the whole category is the **grasp-continuity** class (the held leg, GC1 base
continuity — `place.put_down` is named the canonical base-continuity primitive), and the
controlled release ends the held state.

## Lowering

A shared `lower_place_action` helper: emits `force_profile.min_holding_force` (the GC1 held-leg
floor, from the read-only-then-cleared `ctx.held`), a `placement: <kind>` marker, and
`break_contact: true` (the controlled release, the shared safety clause of `spec/01` § 5);
`target_pose = Ref{<where>}` (symbolic v0 placement pose); no `grasp_stability` (no new grasp).
**After the action, `ctx.held` is cleared** — the placement completes by releasing the object.

| primitive | `<where>` ref | extra marker | suffix |
|---|---|---|---|
| put_down | `target_surface` | — | `put_down` |
| stack | `support_object` (required) | `alignment` | `stack` |
| insert_loose | `container` (required) | — | `insert_loose` |
| orient | `target_surface` | `required_orientation` | `orient` |
| hand_to | `handover` | `weight_transfer` / `max_interaction_force` | `hand_to` |
| discard | `discard_zone` (required) | `max_drop_height` | `discard` |

All suffixes map to `GraspContinuity` in `envelope_class_for`.

## Capability gates

put_down / stack / insert_loose / orient / discard key on their dotted ids. **`place.hand_to` is a
conjunctive gate** (`spec/03` § 318/445 SAF2c): it requires the `place.hand_to` skill **and** the
`human_collaboration_safety` aux capability — an embodiment must never hand an object to a human
without force-limited safety. Mirrors the `force.cut` + `tool_safety` gate. Add
`Aux.human_collaboration_safety` + `has_human_collaboration_safety()`.

## Worked examples + tests

- `skill-place.yaml` (locate → pinch → put_down): uniform 3-hand goldens for the base case.
- `translation.rs` unit tests for stack / insert_loose / orient / discard (placement marker +
  `ctx.held` cleared) and the `hand_to` conjunctive gate: allegro (declares
  `human_collaboration_safety`) lowers it; leap (declares `place.hand_to` but not the safety aux)
  is rejected `capability_absent: human_collaboration_safety` — the SAF2c human gate.

Descriptors gain the six `place.*` skills (all three) and `human_collaboration_safety` (allegro
only, with its `compliance` already declared per SAF4c).

## Deferred / blocked

- **HAZ3 human-handover bench** (release-on-weight-transfer, `max_interaction_force`, compliant
  yielding) — class 4, physical instrumented bench. Out of scope (standing Class-4 deferral).
- `surface_bound` rejection for `place.*` (a pinned object can't be placed) — a class-1 refinement
  of the STB3 walk; `place.*` not added to `is_free_transport` in v0 (noted, deferred).
- Supported-state / recursive-stack / contained predicates (world-state geometry) — deferred
  (perception-derived geometry, Principle 4).
