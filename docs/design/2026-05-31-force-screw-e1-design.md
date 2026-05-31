# Design: `force.screw` worked example + structural lowering (E1)

Status: approved design, pre-implementation (2026-05-31)

This is the sixth reference-implementation increment, and the first half of the
screw/fasten worked example. It adds `force.screw` as a working primitive
end-to-end — parse, retarget, conform — through a third worked example (drive a
screw with a held driver), exercising a new category-6 primitive and a new task
shape (force transmitted through a *held tool*). The tool-mediated reaction-torque
limit and the `thread_pitch` rotation↔advance coupling — the GF4c obligation — are
kept symbolic here and made numeric in E2. The specification under `spec/` is
authoritative; this document describes how the reference implementation realizes it.

## 1. Why this increment

The reference engine models only the ~9 primitives the first two examples exercise;
`force.screw` is the schema's primitive but is absent from the Rust `Primitive`
enum. Adding it broadens primitive coverage into a genuinely new task shape: unlike
`force.insert_fit` (a linear seating force on the held part itself), `force.screw`
transmits **torque** to a target *through a held tool* — the canonical action will
ultimately represent the driver as the force-transmission path, and the reaction
loads the *tool's* grasp (GF4c). E1 establishes the primitive and its structural
lowering so E2 can derive that reaction. It also completes the grasp-force arc from
increment 3: GF1c/GF2c/GF3c are done; GF4c (tool-mediated, E2) is the last, and it
presupposes `force.screw` existing.

## 2. Scope

In scope (E1):

- A new worked example `examples/03-screw-fasten/` (drive a screw with a held driver)
  + three self-contained embodiment descriptors with the `force.screw` capability.
- `force.screw` as a new `Primitive` (parse + structural lowering + capability gate).
- Class 2 conformance for the new example (golden + determinism + boon schema
  validation) + the `screw → force-trajectory` envelope-class mapping arm.

Deferred to E2 (GF4c — the numeric tool-mediated physics):

- The **tool-grasp reaction-torque limit**: the driving torque loads the *tool's*
  grasp; the reaction must not exceed its rotational holding capacity, or the driver
  slips in-grasp. A new grasp-force derivation (the torque counterpart of GF3c).
- The **`thread_pitch` coupling**: one turn ↔ `thread_pitch` of advance, expanded
  into the canonical action as a linked DOF; a decoupling (advance without turn, or
  vice versa) is a failure signal.
- The **torque-trajectory checker** (`|wrench.torque| ≤ torque_budget`, the ENV4
  torque generalization) + the `ReferenceDriver` torque echo + a screw
  envelope-conformance test.
- Rotational holding-capacity limits in the descriptors (E2 needs them; E1 does not).
- `force.unscrew` and the other category-6 primitives (no example exercises them).

## 3. The worked example (`examples/03-screw-fasten/`)

`skill.yaml` — objects `{driver, screw}`; sequence (all primitives already supported
except `force.screw`):

1. `let driver_t from sense.locate{target_ref: driver}`
2. `grasp.pinch{target: driver_t, force_budget: 6 N}` — a precision hold of the
   driver. Reusing `grasp.pinch` (rather than adding `grasp.power`) keeps E1 to a
   single new primitive; pinching a driver shank is an acceptable reference hold.
3. `transport.move_to_pose{target_pose: {frame: workpiece, offset: …}}` — bring the
   driver to the fastener.
4. `let screw_t from sense.locate{target_ref: screw}`
5. `reach.align{target_frame: screw, axes: [z]}` — tool axis to the thread axis.
6. `force.screw{grasp_handle: active, thread_axis: -z, torque_budget: 2 N·m,
   compliance: active, thread_pitch: 0.8 mm, tool_mediated: true, completion:
   all_of{effort_rise: 1.5 N·m, reached: {advance: 5 mm}}}` — drive to seated
   (torque rise *at* the expected advance), discriminating a seated screw from a
   cross-thread (the `all_of` rule, `spec/01` § Force-at-state).
7. `grasp.release{grasp_handle: active}`
8. `reach.retract{direction: -tool_axis, distance: 50 mm}`

`embodiments/{allegro,leap,pneumatic-6f}.yaml` — self-contained copies of the
cable descriptors (the surface-scan B1 pattern: the cable descriptors already
declare `grasp.pinch` / `transport` / `sense.locate` / the reach baseline) with
`force.screw` added to `capabilities.skills`. No torque or rotational-capacity
limits in E1 (E2 adds them for GF4c). `run.py` mirrors examples 01/02.

## 4. The new primitive (`rfl-core::skill_isa`)

Add `ForceScrew` to the `Primitive` enum (`#[serde(rename = "force.screw")]`) and a
param struct modelling the schema's `ForceScrewParams` subset the example uses:

```
pub struct ForceScrew {
    pub thread_axis: Direction,        // = serde_yaml::Value (signed axis / vector / ref)
    pub torque_budget: Quantity,       // "2 N·m"
    pub completion: serde_yaml::Value, // ScrewStop, carried structurally (like insert_fit.stop_condition)
    #[serde(default)] pub thread_pitch: Option<Quantity>,
    #[serde(default)] pub tool_mediated: Option<serde_yaml::Value>, // bool | auto, carried
    #[serde(default)] pub compliance: Option<Compliance>,
    #[serde(default)] pub grasp_handle: Option<GraspHandle>,
}
```

Required per the schema: `thread_axis`, `torque_budget`, `completion`. The rest are
optional and carried opaquely (E1 does not interpret `thread_pitch`/`tool_mediated`).
**Exhaustive-match discipline (project rule):** `lower` and `check_capability` have
no catch-all, so the new variant and its two match arms must land in the *same*
commit as the enum change.

## 5. The lowering (`rfl-core::translation`)

- `check_capability`: `Primitive::ForceScrew(_) → "force.screw"` gate key (the
  descriptors declare it).
- `lower_force_screw`: mirrors `lower_force_insert_fit`, torque-shaped —
  - `completion` → one `Monitor` (`yaml_to_json`, the `ScrewStop` carried as-is);
  - `safety_envelope.force_profile = { "torque": torque_budget, "thread_pitch": …,
    "tool_mediated": … }` — the torque is the force-trajectory envelope's torque
    case; `thread_pitch` / `tool_mediated` are **symbolic markers** (E2 turns them
    into a numeric reaction limit + coupled DOF);
  - `compliance` set from the param;
  - `force_budget: None` (the budget is a torque, carried in `force_profile`; the
    tool-mediated reaction that would bound a grip force is E2);
  - `target_pose = AxisRelative{direction: thread_axis, distance: "0 mm"}` — drive
    along the thread axis; the advance is governed by the completion + the
    (symbolic) coupling, so the distance is a v0 placeholder;
  - `timing_mode: TimeScalable` (contact-bearing, like insert_fit).

Determinism (RD1c): `torque_budget` / `thread_pitch` are carried as their authored
strings (`"2 N·m"`, `"0.8 mm"`); no float emitted.

## 6. Envelope-class mapping

Add `"screw" => Some(EnvelopeClass::ForceTrajectory)` to `envelope_class_for`
(`force.*` → force/torque-trajectory, ENV1). This is forward-looking and does not
affect the C2 `envelope_conformance` tests, which run on the cable example (no screw
action). The numeric torque-trajectory check that would read `force_profile.torque`
and a driver-emitted `wrench.torque` is E2 (the current force-trajectory checker
reads `force_budget`, which is `None` for screw → it would pass vacuously, so E1
adds no misleading check).

## 7. Conformance

A new `crates/rfl-conformance/tests/screw_fasten.rs` (mirrors `surface_scan.rs`):
three insta goldens (one per hand — same skill, three descriptors), a generate-twice
byte-equality check, and per-line boon validation of the emitted `execute` stream
against `driver-interface.schema.json` (the new `force.screw` canonical action
passes the open `canonical_action` floor; the `force_profile.torque` content rides
the open `Envelope` floor). `validate.py` (C1–C7), the 48 rfl-core tests, and the
existing conformance suites (`retarget_determinism`, `surface_scan`,
`driver_protocol`, `envelope_conformance`) stay unchanged and green. No `spec/` or
`schemas/` change — `force.screw` is already typed in `skill-isa.schema.json`.

## 8. Sections to transcribe during implementation

- `schemas/skill-isa.schema.json` `$defs/ForceScrewParams` + `$defs/ScrewStop` (the
  exact field set + the ScrewStop members — already read for this design: required
  `thread_axis` / `torque_budget` / `completion`; ScrewStop = `effort_rise(torque) |
  count(turns) | all_of{effort_rise(torque), reached(advance)}`).
- `spec/01-skill-isa.md` § 6.4 `force.screw` (intent + parameter table) and § Stop /
  completion conditions (`ScrewStop`, the force-at-state `all_of` rule).
- The existing `lower_force_insert_fit` (`translation.rs`) and `surface_scan.rs`
  (the conformance-test shape) as the templates to mirror.
