# Design: PincherX-100 existence proof (sim-first, Class-4)

Status: approved design, pre-implementation (2026-05-31; revised 2026-06-01 — the
headline task is **pick-and-place of a light object**, not reach-only: the 50 g payload
permits grasping a ≤ 50 g object, a far more compelling and richer demonstration).

The first **real-embodiment** existence proof for RFL: demonstrate that the retarget
contract is constructible against a real, low-end robot arm (Trossen PincherX 100), and
that Principle 1 (embodiment-agnostic), capability negotiation, and the graceful-degradation
proxy tier survive contact with real hardware — validated first in a Gazebo simulation,
then on the physical arm. This is the minimal realization of the README roadmap's v0.0.1
milestone ("single (VLA, embodiment) pair existence proof; measured integration cost
published") and lands on `spec/05`'s Class-4 "conformant simulator". A hardware-integration
project, distinct from the in-process Rust increments; it lives outside `crates/`.

## 1. Why this proof, and what it does / does not prove

The in-process reference implementation (12+ increments) proves the spec is internally
coherent, constructible, and adversarially verifiable — but against a **symbolic** driver
returning placeholder poses. The single biggest credibility leap available is to run the
*same* retarget engine against a **real** embodiment and measure the result.

**Proves:** the retarget contract is constructible against a real, independent driver; an
end-to-end skill → canonical-action → real-execution → telemetry loop closes;
**basic pick-and-place** (`grasp.pinch` → `transport.move_to_pose` → `grasp.release`) runs
on real hardware; the **grasp-force derivation** (`min_holding_force` for a light object)
holds on a real gripper; the **proxy fidelity tier** (no tactile → graceful degradation)
works on real hardware; capability negotiation rejects what the arm cannot do; a measured
integration cost.

**Does not prove:** dexterous / in-hand manipulation (the arm has a single parallel pinch,
no in-hand DOF); force-controlled interaction (`force.insert_fit` / `force.screw` etc. — no
wrist F/T sensing); tactile-confirmed grasps (no tactile → proxy only); nor that real VLAs
emit RFL Skill-ISA (the first proof hand-authors the skill).

**The low-end-ness is on-thesis.** A ~$600, 4-DOF, no-tactile arm doing the *basics*
(pick-and-place via the proxy tier) while the gate honestly *rejects* everything advanced
(in-hand, force-control, the seven other grasp modes, `transport.carry`) is strong evidence
that RFL spans embodiments from the baseline to the dexterous without the abstraction
leaking. The descriptor's *negative space* — what it does not declare — is part of the demo.

## 2. Verified hardware facts (Trossen PincherX 100, official page, 2026-05-31)

| Spec | Value | Consequence |
|---|---|---|
| DOF | 4 | No full 6-DOF orientation; top-down grasp + position, one orientation (pitch) |
| Payload | 50 g | A **light object (≤ 50 g)** is graspable; the cable/screw example objects are too heavy |
| Reach / Span | 300 mm / 600 mm | Small workspace; the pick and place zones must be in-workspace |
| Repeatability | 5 mm | Object + gripper opening sized to absorb it; task `position_tolerance` ≥ ~8 mm |
| Sensing | none listed (no F/T / tactile) | `grasp.pinch` runs at the **proxy** tier; no force-control tasks |
| Control | ROS 2 + MoveIt + Gazebo + Interbotix Python API | Driver target (incl. gripper open/close); Gazebo enables sim-first |
| Status | discontinued | User has one; confirm the installed SDK still runs it |

## 3. Approach: sim-first (Gazebo) → physical

The arm is not yet set up. Bringup (installing the Interbotix ROS 2 stack + Gazebo) is a
user task and the gating item, but it is **not on the critical path for writing the RFL
software** — the descriptor, driver, skills, and measurement are pure software, writable
without the hardware and validated against the Gazebo sim before the physical arm. A
Gazebo-sim proof is a legitimate Class-4 conformant-simulator result (`spec/05`
§ recursive simulator conformance). The physical-arm run is a follow-on: the same Interbotix
API backs both, so only the backend changes.

## 4. Location and components

A new top-level directory `hardware/pincherx-100/` (outside `crates/`, so it neither
collides with nor depends on the symbolic Rust workspace):

- `pincherx-100.yaml` — the embodiment descriptor.
- `skill-pickplace.yaml` — **headline**: locate → pinch → transport → release → retract.
- `skill-hover.yaml` / `skill-scan.yaml` — kinematic warm-ups (one standoff pose; a
  small-region raster) that smoke-test the driver and exercise the Σ generators +
  interval-invariant class without grasping.
- `rfl_interbotix_driver.py` — the **real Class-4 driver**: consumes the canonical-action
  JSONL, drives the Interbotix arm (Gazebo or physical) including **gripper open/close** for
  `grasp.pinch` / `grasp.release`, emits driver-interface telemetry.
- `measure.py` — the Class-4 measurement: realized vs commanded poses against tolerance,
  grasp success (object lifted + placed), and integration-cost capture (driver LOC, time).
- `README.md` — bringup instructions + how to run.

## 5. Data flow

1. `rfl-cli retarget hardware/pincherx-100/skill-pickplace.yaml --embodiment
   hardware/pincherx-100/pincherx-100.yaml` → canonical-action JSONL: a `sense.locate`
   (pose bound at runtime), a `grasp.pinch` (carrying the derived `min_holding_force`), a
   `transport.move_to_pose` (held; carrying the propagated floor, increment 10), a
   `grasp.release`, and a `reach.retract`.
2. `rfl_interbotix_driver.py` executes each message: for a pose target it calls the
   Interbotix Python API to move the end-effector (the stack does the 4-DOF IK); for
   `grasp.pinch` it closes the gripper, for `grasp.release` it opens it; it reads back the
   realized joint state → FK → realized pose, and emits an RFL **driver-interface** telemetry
   message + terminal status. `sense.locate`'s pose comes from the sim ground truth (or a
   fixed physical calibration) — perception source is out of RFL scope (`spec/04`). The pick
   and place frames are defined relative to the arm base by a known transform.
3. `measure.py`: did the realized poses match the commanded poses within tolerance, was the
   object lifted and placed, and what was the measured integration cost. This closes the
   conformance loop on real hardware.

The driver is the real-hardware embodiment of the in-process `ReferenceDriver`: same
driver-interface contract, but the realized poses + grasp outcome come from a real arm.

## 6. The PincherX-100 descriptor

Per the verified facts and `spec/03` (frame model + capability manifest + limits):

- `id: pincherx-100`, `class: open-chain-4dof-arm`.
- `frames`: `control_frames: [ee_gripper]`; `role_defaults.control = ee_gripper`,
  `role_defaults.grasp = ee_gripper`, `role_defaults.sensor = ee_gripper`;
  `tool_axis: { ee_gripper: <confirm at bringup> }`.
- `capabilities.skills`: **`[grasp.pinch, transport, sense.locate]`** (plus the unkeyed
  `reach.*` baseline; `grasp.release` is presupposed by the declared `grasp.*`). `aux`:
  `tactile_sensing` **omitted** → `grasp.pinch`'s confirmation degrades to the **proxy tier**
  (position convergence + force-hold, `spec/04` § Graceful degradation). Deliberately **not**
  declared, so the gate rejects them: `in_hand.*`, `force.*`, the other seven grasp modes
  (`power` / `envelope` / `precision_tripod` / `lateral` / `pin` / `hook` / …),
  `transport.carry`, `sense.inspect`. That rejection set is the negative-space demo.
- `limits`: the reach baseline (`v/w/a_cartesian_max` at the PincherX's slow real values,
  `joint_velocity_ceiling`, `stop_time`, `tracking_bandwidth`) **and** the `grasp.pinch`
  limits the schema's M2 clause requires (`grip_force_max`, `v_grasp`, `payload_grasp_pinch`):
  `payload_grasp_pinch = 0.5 N` (the verified 50 g), `grip_force_max` from the gripper's
  Dynamixel-servo torque spec (a real value, ≥ the ~1 N `min_holding_force` of a 50 g object),
  `v_grasp` the gripper close speed.

Because the descriptor declares ≥ 1 capability key (`grasp.pinch` etc.), it satisfies the
schema's `capabilities.skills` `minItems: 1` — **no schema change is needed for this proof**
(superseding an earlier reach-only framing). The separate finding "the descriptor schema's
`minItems: 1` cannot express a pure-reach baseline-only embodiment" remains valid and is
noted as a deferred observation (§ 10), no longer on this proof's critical path.

The skill (not the descriptor) sets `position_tolerance ≥ 8 mm` to honor the 5 mm
repeatability; the graspable object is light (≤ 50 g) and sized, with the gripper opening,
to absorb that repeatability.

## 7. Honest caveats (the substance of the proof)

- **Light object, top-down grasp.** ≤ 50 g; the 4-DOF arm grasps top-down (gripper pointing
  down). Yaw is not independently controllable, so the realized grasp/place orientation's
  yaw is IK-determined; the measurement judges **position** + accepts the **orientation
  residual** under `spec/02`'s under-constrained-orientation rule.
- **Proxy-tier grasp.** No tactile → `grasp.pinch` is confirmed by position convergence +
  force-hold (the proxy tier), not tactile sites — the spec's graceful-degradation path,
  exercised on real hardware.
- **Frame calibration.** The locate/pick/place frames map to the arm base by a known
  transform (sim: a fixed offset; physical: a calibration). This is the "concrete poses +
  where is the object?" reality the symbolic reference implementation elided.
- **Small workspace, repeatability-sized object.** Pick and place zones in the ~300 mm reach;
  object + gripper opening sized so a ≤ 8 mm placement error still grasps/places.
- **Not a flagship demo.** The value is the binary leap (symbolic → real-hardware-verified) +
  the embodiment-agnostic evidence at the low end + a measured integration cost — not
  dexterity.

## 8. Task sequence

1. `reach.hover` (one standoff pose) — smoke-test the driver + the interval-invariant
   3-sample telemetry on real hardware.
2. `reach.scan` (small-region raster) — exercise the Σ generators + per-pose conformance,
   no grasping.
3. **`skill-pickplace`** (headline) — `sense.locate` → `grasp.pinch` → `transport.move_to_pose`
   → `grasp.release` → `reach.retract` of a ≤ 50 g object; exercises grasp + transport + the
   grasp-force floor + the proxy tier + the negative-space gate.

Same descriptor and driver throughout; only the skill differs (the driver gains gripper
control for step 3).

## 9. Division of labor

- **(User, parallel, now)** Bringup: install the Interbotix ROS 2 stack + Gazebo; confirm
  the Python API moves the (sim) arm's end-effector to a Cartesian pose and opens/closes the
  gripper (hello-world).
- **(Implementer, no hardware needed)** Write the descriptor + driver + skills + measurement
  (pure software; only running needs the sim/arm).
- **(Together)** Run the driver against the Gazebo sim → measure → iterate → physical arm.

## 10. Out of scope (deferred)

- **Force-controlled interaction** (`force.*`) and **in-hand manipulation** (`in_hand.*`) —
  physically impossible on this arm (no F/T sensing; single parallel pinch).
- A proper **`place.put_down`** primitive (lower-onto-surface + release) — the engine has no
  `place.*` yet; the proof uses `transport.move_to_pose` + `grasp.release` instead, which are
  implemented.
- **Real-VLA integration** — the first proof hand-authors the skill ("RFL → real embodiment",
  not "real-VLA → RFL → real embodiment").
- **The baseline-only schema finding** — relaxing the descriptor schema's `minItems: 1` to
  express a pure-reach embodiment is a valid, on-thesis spec improvement, but not needed here
  (this descriptor declares `grasp.pinch`); deferred as its own small spec/schema change.
- **Full physical-arm calibration** — after the sim proof.

## 11. Confirm during implementation (transcribe from source, not memory)

- Whether `rfl-cli retarget` accepts arbitrary skill / descriptor file paths (vs example
  stems); if not, a thin Python wrapper calling `rfl_core::translation::retarget`, or a CLI
  extension.
- The exact Interbotix Python API for the installed version: the manipulator class, the
  set-end-effector-pose method (+ its 4-DOF argument set), and the **gripper open/close**
  calls — confirmed at bringup.
- The exact `driver-interface.schema.json` `execute` / `telemetry` / `status` fields the
  Python driver must emit (read the schema; reuse the in-process `driver.rs` shapes as the
  field-name reference, re-implemented in Python).
- `embodiment-descriptor.schema.json` for the PincherX descriptor — including the M2 clause
  that `grasp.pinch` requires (`grip_force_max` / `v_grasp` / `payload_grasp_pinch`) — Class-1
  validated via the one-off `uv run --with jsonschema` script.
