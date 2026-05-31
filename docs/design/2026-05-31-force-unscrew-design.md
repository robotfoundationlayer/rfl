# Design: force.unscrew worked primitive (reverse fastening)

Status: approved design, pre-implementation (2026-05-31)

This is the eleventh reference-implementation increment. It adds the `force.unscrew`
primitive — reverse coupled rotation plus axial retreat to extract a threaded
fastener — completing the category-6 screw/unscrew pair. It reuses the GF4c
tool-mediated torque clamp (increment 7) and the entire E3 torque-trajectory
verification (increment 8) and requires no schema change: the spec design track
already typed `ForceUnscrewParams`. The specification under `spec/` is authoritative;
this document describes how the reference implementation realizes it.

## 1. Why this increment

`spec/01` § 6.5 `force.unscrew`: "Extract a threaded fastener by reverse coupled
rotation and axial retreat — overcoming the initial breakaway torque, turning out at
the thread pitch within a torque budget, and detecting full disengagement — then
handling the now-freed fastener." It is the reverse of `force.screw` (increment 6/7):
the same tool-mediated torque transmission and `thread_pitch` coupling, but a loosening
sense and a disengagement-on-completion handling instead of a seating one.

The Rust engine models `force.screw` (E1/E2/E3) but not `force.unscrew`. Adding it
broadens category-6 coverage and exercises the GF4c reaction-torque machinery and the
E3 torque-trajectory checker in a second primitive — verifying that those were built
generically, not screw-specifically.

## 2. The effort_drop completion (schema-respected)

`spec/01` § 6.5 lists `completion ∈ {disengagement (fully out), turns(n), torque_drop}`,
defaulting to `disengagement`. But the machine-readable schema records a **project
decision: the unscrew effort_drop (`disengagement` / `torque_drop`) variant is NOT
typed** — `ScrewStop` (shared by `force.screw` and `force.unscrew`) types only
`effort_rise(torque) | count(turns) | all_of{effort_rise(torque), reached(advance)}`.
`ForceUnscrewParams.completion` is therefore optional (defaults to `disengagement`).

Consequences for this increment:

- An **authored** `completion` must be a typed `ScrewStop` (same restriction as
  `force.screw`); the engine carries it as an opaque value and lowers it into a
  `Monitor`, exactly as `force.screw` does.
- The **default `disengagement`** (when `completion` is omitted) is represented at
  *lowering* time as a default disengagement `Monitor` in the canonical action. This is
  output-side: the driver-interface execute `Monitor` is the open floor, so emitting a
  `{"disengagement": true}` stop condition validates fine and does not touch the
  authored-skill schema.
- The worked example **omits `completion`** to exercise the disengagement default — no
  schema change, no un-typed authoring. (Parallel to `coverage_overlap:auto`, which
  stayed blocked pending a normative formula; here the authored effort_drop stays
  blocked pending the project decision, and the default is handled at lowering.)

## 3. Component 1 — rfl-core

All in one commit (the `lower` / `check_capability` matches have no catch-all, so a new
`Primitive` variant must land with both arms or it will not compile).

- **`ForceUnscrew` struct** (`skill_isa.rs`), mirroring `ForceScrew` (typed from
  `$defs/ForceUnscrewParams`): `thread_axis: Direction` (req), `torque_budget: Quantity`
  (req), `completion: Option<serde_yaml::Value>` (optional — the screw field is
  required), `thread_pitch: Option<Quantity>`, `tool_mediated: Option<serde_yaml::Value>`,
  `compliance: Option<Compliance>`, `grasp_handle: Option<GraspHandle>`,
  `on_disengagement: Option<serde_yaml::Value>` (the `{retain, drop_safe}` enum, default
  retain). No `axial_force_budget` (unscrew retreats, not seats).
- **`Primitive::ForceUnscrew`** + `#[serde(rename = "force.unscrew")]`.
- **`lower_force_unscrew`** (mirrors `lower_force_screw`):
  - `monitors`: the authored `completion` lowered into a `Monitor`, or — when omitted —
    a default `Monitor { stop_condition: {"disengagement": true} }`.
  - **GF4c reuse**: `tool_mediated` + a held tool → `grasp_force::reaction_torque_limit`
    clamps the loosening torque to the tool-grasp rotational capacity (the loosening
    torque loads the grasp exactly as the driving torque does — the identical clamp).
  - `force_profile = { "torque": <clamped>, "rotation_sense": "loosen", "on_disengagement":
    <retain|drop_safe, default retain> }`, plus `"coupling": {advance_per_turn}` when
    `thread_pitch` is set and `"tool_mediated"` when set — mirroring `force.screw` and
    adding the two unscrew-distinctive markers. `rotation_sense: "loosen"` (symbolic, v0)
    encodes the reverse sense without inventing physics, parallel to screw's symbolic
    markers.
  - `force_budget: None` (the budget is a torque); `target_pose = AxisRelative {
    thread_axis, "0 mm" }` (axial retreat symbolic via the coupling, as screw's advance
    is); `timing_mode: TimeScalable`.
- **`check_capability`** arm: the `force.unscrew` gate key (the descriptor must declare
  `force.unscrew`).
- **`envelope_class_for("unscrew") => ForceTrajectory`** (one line). This is the whole
  verification story: E3 already made the torque-trajectory checker read
  `force_profile.torque` and the `ReferenceDriver` echo it into `wrench.torque`, so
  unscrew's torque budget is verified and `OverTorque` rejects it — zero new checker or
  driver code.

## 4. Component 2 — worked example

`examples/03-screw-fasten/skill-unscrew.yaml`: grasp the driver (pinch, `estimated_mass`
so the GF4c clamp bites) → transport → `reach.align` → `force.unscrew` (`tool_mediated`,
`thread_axis`, `torque_budget: 2 N·m`, `thread_pitch`, `completion` omitted → the
disengagement default, `on_disengagement: retain`) → `reach.retract`. It reuses the
three existing embodiment descriptors. `force.unscrew` is added to each descriptor's
`capabilities.skills` (a uniform edit; `force.unscrew` is in `skill-isa`'s `PrimitiveId`,
so validate.py C1 stays green). The existing `screw_fasten` goldens are **unchanged** —
a descriptor capability addition does not alter the screw retarget output.

## 5. Component 3 — conformance

- `crates/rfl-conformance/tests/screw_fasten_unscrew.rs` (mirrors `screw_fasten.rs`):
  three per-hand insta goldens, generate-twice byte-equality, per-line boon execute-schema
  validation. The clamped loosening torque (allegro 0.2 / leap 0.15 / pneumatic 0.12 N·m
  for a 2 N·m budget, tool-mediated — identical to screw E2's `reaction_torque_limit`)
  rides the open Envelope floor.
- Two `envelope_conformance` tests reusing E3: nominal unscrew passes the
  ForceTrajectory (torque) check (echoed clamped torque ≤ budget); `FaultyDriver(OverTorque)`
  makes it fail.
- Class-1 check of `skill-unscrew.yaml` against `skill-isa.schema.json` via the one-off
  `uv run --with jsonschema` script (validate.py is example-01-only).

## 6. Conformance and goldens

`validate.py` (C1–C7) stays green: `force.unscrew` is already in the capability-enum
(C1), and the only descriptor edit is adding the (valid) `force.unscrew` capability key.
No schema change. New `screw_fasten_unscrew` goldens; `screw_fasten` (screw) goldens
unchanged; `driver_protocol` / `retarget_determinism` / `surface_scan*` untouched (no
unscrew).

## 7. Out of scope (deferred)

- The **authored `effort_drop` / `torque_drop`** completion — schema-blocked by the
  project decision; only the lowering-side disengagement default is realized.
- **`on_disengagement: drop_safe`** release motion — carried as a marker, not lowered
  into a `place`/`release` sub-action (the freed-fastener disposition mechanism is a
  later increment).
- **Coupled-motion decoupling detection** (advance-without-turn / turn-without-advance as
  a stripped-thread fault) — a driver / Class-4 concern.
- `force.cut` / `force.wipe` / `force.scrub` / `force.press_button` and the remaining
  category-6 primitives.

## 8. Sections to transcribe during implementation

- `spec/01-skill-isa.md` § 6.5 `force.unscrew` (the parameter table + postconditions) —
  already read for this design.
- `$defs/ForceUnscrewParams` + the shared `ScrewStop` (`schemas/skill-isa.schema.json`) —
  already read.
- `force.screw`'s `ForceScrew` struct, `lower_force_screw`, the `Primitive::ForceScrew`
  `lower` + `check_capability` arms, and `envelope_class_for` (`crates/rfl-core/src/{skill_isa,translation}.rs`,
  `crates/rfl-conformance/src/lib.rs`).
- `examples/03-screw-fasten/skill.yaml` + its descriptors, and `tests/screw_fasten.rs`
  + the `envelope_conformance` screw torque tests.
