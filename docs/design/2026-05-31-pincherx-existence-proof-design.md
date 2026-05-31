# Design: PincherX-100 existence proof (sim-first, Class-4)

Status: approved design, pre-implementation (2026-05-31)

The first **real-embodiment** existence proof for RFL: demonstrate that the retarget
contract is constructible against a real, low-end robot arm (Trossen PincherX 100),
and that Principle 1 (embodiment-agnostic) and capability negotiation survive contact
with real hardware — validated first in a Gazebo simulation, then on the physical arm.
This is the minimal realization of the README roadmap's v0.0.1 milestone ("single
(VLA, embodiment) pair existence proof; measured integration cost published") and lands
on `spec/05`'s Class-4 "conformant simulator". This is a hardware-integration project,
distinct from the in-process Rust increments; it lives outside `crates/`.

## 1. Why this proof, and what it does / does not prove

The in-process reference implementation (12 increments) proves the spec is internally
coherent, constructible, and adversarially verifiable — but against a **symbolic**
driver returning placeholder poses. The single biggest credibility leap available is to
run the *same* retarget engine against a **real** embodiment and measure the result.

**Proves:** the retarget contract is constructible against a real, independent driver;
capability negotiation + graceful degradation work on real hardware; the descriptor
model captures a real (low-end, under-actuated, payload-less, no-tactile) embodiment;
an end-to-end skill → canonical-action → real-execution → telemetry loop closes; a
measured integration cost.

**Does not prove:** dexterous / tactile / in-hand richness (the PincherX cannot — 4 DOF,
50 g payload, simple parallel gripper, no force/tactile sensing); nor that real VLAs emit
RFL Skill-ISA (separable deeper question — the first proof hand-authors the skill).

**The low-end-ness is on-thesis.** If RFL handles a ~$600, 4-DOF, no-tactile arm via the
proxy / graceful-degradation path without the abstraction leaking, that is stronger
evidence for "neutral across embodiments" than a single high-end demo. The proof's
*negative space* — the descriptor declaring no grasp / force / tactile, and the retarget
gate rejecting those primitives — is itself the demonstration.

## 2. Verified hardware facts (Trossen PincherX 100, official page, 2026-05-31)

| Spec | Value | Consequence |
|---|---|---|
| DOF | 4 | No full 6-DOF orientation; position + one orientation (pitch) only |
| Payload | 50 g | Grasp / transport of any real object is off the table |
| Reach / Span | 300 mm / 600 mm | Scan region must be small (~80–120 mm), placed in-workspace |
| Repeatability | 5 mm | Task `position_tolerance` must be ≥ ~8 mm (Principle 1, honestly) |
| Sensing | none listed (no F/T / tactile) | Proxy fidelity tier; no grasp-force / force-control tasks |
| Control | ROS 2 + MoveIt + Gazebo + Interbotix Python API | Driver target; Gazebo enables sim-first |
| Status | discontinued | User has one; confirm the installed SDK still runs it |

These facts make `reach.scan` / `reach.hover` not merely the *best* first task but the
only physically sensible one (no payload, pure kinematic reach).

## 3. Approach: sim-first (Gazebo) → physical

The arm is not yet set up. Bringup (installing the Interbotix ROS 2 stack + Gazebo) is a
user task and the gating item, but it is **not on the critical path for writing the RFL
software** — the descriptor, driver, skill, and measurement are pure software, writable
without the hardware and validated against the Gazebo sim before the physical arm. A
Gazebo-sim proof is a legitimate Class-4 conformant-simulator result (`spec/05`
§ recursive simulator conformance). The physical-arm run is a follow-on: the same
Interbotix API backs both, so only the backend changes.

## 4. Location and components

A new top-level directory `hardware/pincherx-100/` (outside `crates/`, so it neither
collides with nor depends on the symbolic Rust workspace):

- `pincherx-100.yaml` — the embodiment descriptor.
- `skill-hover.yaml` (warm-up, one pose) and `skill-scan.yaml` (headline, small-region
  raster) — skills scoped to the arm's workspace and tolerances.
- `rfl_interbotix_driver.py` — the **real Class-4 driver**: consumes the canonical-action
  JSONL, drives the Interbotix arm (Gazebo or physical), emits driver-interface telemetry.
- `measure.py` — the Class-4 measurement: realized vs commanded Σ against tolerance, plus
  integration-cost capture (driver LOC, time).
- `README.md` — bringup instructions + how to run.

## 5. Data flow

1. `rfl-cli retarget hardware/pincherx-100/skill-scan.yaml --embodiment
   hardware/pincherx-100/pincherx-100.yaml` → canonical-action JSONL (the Σ poses for the
   PincherX descriptor's nominal sensor FOV; concrete numbers in the region frame).
2. `rfl_interbotix_driver.py` parses each `execute` message's `target_pose`. For a
   `reach.scan` SweepPath it iterates the Σ poses; for `reach.hover` it holds the standoff
   pose. For each pose it calls the Interbotix Python API ("move the end-effector to this
   Cartesian pose" — the stack does the 4-DOF IK), reads back the realized joint state →
   FK → realized end-effector pose, and emits an RFL **driver-interface** telemetry message
   (`realized_pose`) plus a terminal status. The region frame is defined relative to the
   arm base by a known transform (the sim places the region at a fixed offset; the physical
   arm calibrates it).
3. `measure.py` compares realized poses vs commanded Σ: every σ visited within the task's
   `position_tolerance` (the `reach.scan` C1 / interval-invariant conformance, on real
   hardware), and records the measured integration cost.

The driver is the real-hardware embodiment of the in-process `ReferenceDriver`: same
driver-interface contract, but the realized poses come from a real arm, not a placeholder.

## 6. The PincherX-100 descriptor

Per the verified facts and `spec/03` (frame model + capability manifest + limits):

- `id: pincherx-100`, `class: open-chain-4dof-arm`.
- `frames`: `control_frames: [ee_gripper]`; `role_defaults.control = ee_gripper`,
  `role_defaults.sensor = ee_gripper` (no camera — the tool tip is treated as the "sensor
  frame", honoring only `reach.scan`'s kinematic contract; perception is out of RFL scope);
  `tool_axis: { ee_gripper: <confirm at bringup> }`.
- `capabilities.skills`: **`reach.*` only** (the relevant `reach.approach` / `reach.align`
  / `reach.hover` / `reach.scan` / `reach.retract`). Deliberately **no** `grasp.*`,
  `in_hand.*`, `force.*`, `transport.carry`, or `sense.inspect` — so the retarget capability
  gate rejects them (the negotiation demonstration). `aux.tactile_sensing` omitted (→ proxy
  tier where relevant).
- `limits`: the reach baseline (`v/w/a_cartesian_max` at the PincherX's slow real values,
  `joint_velocity_ceiling`, `stop_time`, `tracking_bandwidth`). No grasp / force limits.
- `sensors`: a nominal `ee_gripper` FOV so `reach.scan`'s Σ is computed (small-region
  assumption); `bore_axis`, `fov`, `working_range` set to plausible values for the proof.

The skill (not the descriptor) sets `position_tolerance ≥ 8 mm` to honor the 5 mm
repeatability — Principle 1 made concrete: the same scan skill retargets onto a coarse arm
by respecting its declared limits.

## 7. Honest caveats (the substance of the proof)

- **4-DOF → yaw not independently controllable.** Σ's fixed-roll convention is
  unachievable; the realized orientation's yaw is whatever the IK yields. The measurement
  judges **position** (the 4-DOF arm can hit it) and accepts the **orientation residual**
  under `spec/02`'s under-constrained-orientation rule — i.e. "the descriptor declares
  limited orientation DOF, RFL tolerates the residual", demonstrated on real hardware.
- **Frame calibration.** Σ carries concrete poses in the region frame; the driver maps the
  region frame to the arm base. The sim defines this transform explicitly; the physical arm
  calibrates it. This is the "concrete poses + where is the region?" reality the symbolic
  reference implementation elided (its runtime poses stayed symbolic).
- **Small region.** 300 mm reach / 600 mm span → the scan region is ~80–120 mm, placed
  within the workspace.
- **Not a flagship demo.** The value is the binary leap (symbolic → real-hardware-verified)
  + the embodiment-agnostic evidence at the low end + a measured integration cost — not
  dexterity.

## 8. First task

Warm-up: `reach.hover` (one standoff pose — smoke-test the driver + the interval-invariant
3-sample telemetry on real hardware). Headline: `reach.scan` (small-region raster —
exercises the Σ generators + per-pose conformance on real hardware). Same descriptor and
driver for both; only the skill differs.

## 9. Division of labor

- **(User, parallel, now)** Bringup: install the Interbotix ROS 2 stack + Gazebo; confirm
  the Python API moves the (sim) arm's end-effector to a Cartesian pose (hello-world).
- **(Implementer, no hardware needed)** Write the descriptor + driver + skills + measurement
  (pure software; only running needs the sim/arm).
- **(Together)** Run the driver against the Gazebo sim → measure → iterate → physical arm.

## 10. Out of scope (deferred)

- Real-VLA integration — the first proof hand-authors the skill; the proof is "RFL → real
  embodiment", not "real-VLA → RFL → real embodiment".
- Grasp / force-control tasks — physically impossible on this arm (50 g, no tactile/F-T).
- Full physical-arm calibration — after the sim proof.

## 11. Confirm during implementation (transcribe from source, not memory)

- Whether `rfl-cli retarget` accepts arbitrary skill / descriptor file paths (vs example
  stems); if not, a thin Python wrapper that calls `rfl_core::translation::retarget`, or a
  small CLI extension.
- The exact Interbotix Python API for the installed version (the manipulator class and the
  set-end-effector-pose method + its 4-DOF argument set), confirmed at bringup.
- The exact `driver-interface.schema.json` `execute` / `telemetry` / `status` fields the
  Python driver must emit (read the schema; reuse the in-process `driver.rs` shapes as the
  reference for field names).
- `embodiment-descriptor.schema.json` for the PincherX descriptor (Class-1 validate it via
  the one-off `uv run --with jsonschema` script, as for the other examples).
