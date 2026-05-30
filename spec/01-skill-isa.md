# Skill ISA — Specification (v0.1 skeleton)

> **Status**: skeleton expanded with provisional 50-primitive enumeration and BNF refinement (2026-05-30). Per-primitive semantics, parameters, edge cases, and conformance tests are the **next design phase**; the current text fixes the surface structure that subsequent passes will fill in.

## Scope

This chapter defines:

1. The fifty primitive manipulations, organized into seven categories
2. The compositional algebra that combines primitives into multi-step manipulations
3. The JSON schema (`schemas/skill-isa.schema.json`) against which a Skill ISA file is validated

## Provisional primitive enumeration (50 total)

> Naming convention: `category.primitive` in lowercase snake_case. Reserved names listed here cannot be re-used by extensions (see `06-extension-registry.md`).

### Category 1 — `reach` (6 primitives)

Pre-grasp positioning. Brings the end-effector into a target pose without contact-forming intent.

| ID | Provisional name | Intent |
|---|---|---|
| 1.1 | `reach.to_pose` | Move the controlled frame to an absolute reference-frame pose |
| 1.2 | `reach.approach` | Approach a target object along its surface normal at a configurable standoff distance |
| 1.3 | `reach.align` | Align end-effector orientation with a target frame (axis-by-axis) |
| 1.4 | `reach.retract` | Retreat the controlled frame along a direction (default −tool axis), breaking incidental contact |
| 1.5 | `reach.hover` | Maintain a pose at a configurable standoff above a target |
| 1.6 | `reach.scan` | Sweep a configurable region with end-effector or sensor frame for perception purposes |

### Category 2 — `grasp` (10 primitives)

Object acquisition. All primitives commit to a contact pattern; force-controlled completion is handled by the Translation Layer per embodiment.

| ID | Provisional name | Intent |
|---|---|---|
| 2.1 | `grasp.pinch` | Two-opposing-point pinch (thumb-and-index in anthropomorphic hands; equivalent on non-anthropomorphic) |
| 2.2 | `grasp.power` | Whole-volume enclosure of a graspable object |
| 2.3 | `grasp.hook` | Hook-shape closure for handle / loop / strap targets |
| 2.4 | `grasp.precision_tripod` | Three-finger opposition for small-object stability |
| 2.5 | `grasp.lateral` | Side grasp (key-grip style) for thin / flat objects |
| 2.6 | `grasp.platform` | Open-palm support for items resting on the end-effector |
| 2.7 | `grasp.pin` | Single-finger pin against an opposing surface |
| 2.8 | `grasp.envelope` | Soft-body or cage enclosure (for pneumatic / underactuated hands) |
| 2.9 | `grasp.adjust` | Modify an established grasp without re-acquiring |
| 2.10 | `grasp.release` | Open contact and clear the object envelope |

### Category 3 — `in_hand` (7 primitives)

In-hand manipulation that preserves the grasp identity.

| ID | Provisional name | Intent |
|---|---|---|
| 3.1 | `in_hand.rotate` | Rotate the object about an in-hand axis |
| 3.2 | `in_hand.translate` | Translate the object within the grasp envelope |
| 3.3 | `in_hand.regrasp` | Switch from one stable grasp to another without releasing |
| 3.4 | `in_hand.roll` | Continuous rolling about the longitudinal axis (typical for cylindrical objects) |
| 3.5 | `in_hand.pivot` | Pivot the object about a single contact point |
| 3.6 | `in_hand.slide` | Controlled slip of the object along one finger surface |
| 3.7 | `in_hand.flip` | 180° reorientation requiring momentary release |

### Category 4 — `transport` (6 primitives)

Whole-body relocation of the grasped object.

| ID | Provisional name | Intent |
|---|---|---|
| 4.1 | `transport.move_to_pose` | Move grasped object to an absolute task-frame target pose |
| 4.2 | `transport.follow_trajectory` | Track a parameterized trajectory through the workspace |
| 4.3 | `transport.handoff` | Transfer the grasped object to a partner end-effector |
| 4.4 | `transport.carry` | Maintain grasp stability under perturbation during translation |
| 4.5 | `transport.lift` | Vertical lift with anti-slip force monitoring |
| 4.6 | `transport.lower` | Vertical lower with controlled deceleration |

### Category 5 — `place` (6 primitives)

Object placement and release.

| ID | Provisional name | Intent |
|---|---|---|
| 5.1 | `place.put_down` | Place object on a target surface with controlled release |
| 5.2 | `place.stack` | Place object on top of an existing stack with alignment verification |
| 5.3 | `place.insert_loose` | Insert into a container with clearance (no fit tolerance) |
| 5.4 | `place.orient` | Place with a required orientation (label-up, port-out, etc.) |
| 5.5 | `place.hand_to` | Hand-over to a human (controlled release on contact / weight transfer) |
| 5.6 | `place.discard` | Drop or release without precise target pose |

### Category 6 — `force` (10 primitives)

Force-controlled interaction. The largest category because force semantics are where embodiment heterogeneity is most consequential.

| ID | Provisional name | Intent |
|---|---|---|
| 6.1 | `force.insert_fit` | Tolerance-fit insertion (peg-in-hole class, connector mating) |
| 6.2 | `force.push` | Apply a directional force without object displacement above threshold |
| 6.3 | `force.pull` | Apply tensile force (cable extraction, drawer opening) |
| 6.4 | `force.screw` | Rotational insertion with torque budget |
| 6.5 | `force.unscrew` | Reverse-rotational extraction |
| 6.6 | `force.press_button` | Sub-millimeter displacement with force threshold detection |
| 6.7 | `force.cut` | Tool-mediated separation with continuous shear |
| 6.8 | `force.wipe` | Tangential contact maintenance over a surface |
| 6.9 | `force.scrub` | Oscillating tangential contact with normal-force regulation |
| 6.10 | `force.snap_engage` | Bistable mechanism engagement (clip, latch, snap-fit) |

### Category 7 — `sense` (5 primitives)

Sensing-only primitives. No object state change; output is a measurement or state assertion.

| ID | Provisional name | Intent |
|---|---|---|
| 7.1 | `sense.probe` | Single-point tactile probe at a target pose |
| 7.2 | `sense.inspect` | Visual or sensor-mediated state observation |
| 7.3 | `sense.weigh` | End-effector-mediated mass estimation of a grasped object |
| 7.4 | `sense.locate` | Pose estimation of a referenced object |
| 7.5 | `sense.verify` | Predicate check on an external state (e.g., is the connector seated?) |

### Total count check

| Category | Count |
|---|---|
| `reach` | 6 |
| `grasp` | 10 |
| `in_hand` | 7 |
| `transport` | 6 |
| `place` | 6 |
| `force` | 10 |
| `sense` | 5 |
| **Total** | **50** ✓ |

## Compositional algebra — BNF (v0.1 candidate)

```bnf
Skill           ::= Composition

Composition     ::= Primitive
                  | Sequence
                  | Parallel
                  | Reactive
                  | Repeat
                  | Branch
                  | LetBind
                  | Extension

Primitive       ::= PrimitiveId "(" ParamList? ")"
PrimitiveId     ::= Category "." Identifier            (* core primitive *)
                  | "ext" "." Identifier "." Identifier (* extension primitive *)
Category        ::= "reach" | "grasp" | "in_hand" | "transport"
                  | "place" | "force" | "sense"

Sequence        ::= "sequence" "(" Composition { "," Composition } ")"
Parallel        ::= "parallel" "(" Composition { "," Composition } ")"
Reactive        ::= "reactive" "(" Composition "," "until" "(" Predicate ")" ")"
Repeat          ::= "repeat" "(" Composition "," Count ")"
Branch          ::= "branch" "(" Predicate ","
                                  "then" "(" Composition ")" ","
                                  "else" "(" Composition ")" ")"

LetBind         ::= "let" "(" Identifier ":=" Expression ")"
                    "in" "(" Composition ")"           (* hoist a planner-derived
                                                          value for reuse *)

Extension       ::= "ext" "." Identifier "." Identifier "(" ParamList? ")"

ParamList       ::= Param { "," Param }
Param           ::= Identifier ":" Value
Value           ::= Literal | Identifier | Reference | Auto

Predicate       ::= "tactile_contact" "(" ContactSpec ")"
                  | "pose_reached" "(" Tolerance ")"
                  | "force_exceeds" "(" ThresholdN ")"
                  | "object_present" "(" ObjectRef ")"
                  | "elapsed" "(" Duration ")"
                  | "user_defined" "(" PredicateRef ")"  (* extension hook *)

Reference       ::= "&" Identifier                      (* let-binding ref *)
Auto            ::= "auto"                               (* planner-derived *)
Count           ::= Integer | "until_satisfied"
Tolerance       ::= "mm" | "deg" | { Number ":" Unit }
ThresholdN      ::= Number  "N"
Duration        ::= Number ("ms" | "s")
```

### Notes on the BNF

1. **`Reactive` until-predicate semantics** are subtle: concurrent envelope violation must abort the inner `Composition` deterministically. Full specification deferred to a dedicated subsection in the next pass.
2. **`LetBind`** is included to allow a planner-derived pose (e.g., the result of `sense.locate`) to flow into subsequent primitives without re-derivation. This is what enables a single Skill ISA file to express "find the cable end, then grasp it, then insert" as a single composition. It is also the canonical way a perception-derived `SurfaceTarget` (point + outward normal) reaches `reach.approach`: `let (t := sense.locate(...)) in (reach.approach(target: &t, standoff: 50mm))`. RFL does not define how `t` is perceived — only how it flows once resolved.
3. **`Predicate` extensibility via `user_defined`** is the escape valve: domain-specific predicates can ship as extensions without modifying the core grammar.
4. **`Auto` value** lets the spec author defer choice to the Translation Layer's planner (e.g., a `grasp_pose: auto` parameter delegates pose selection).

## Per-primitive semantic specification

> **Status**: this section is filled category-by-category as each primitive reaches v0.1 freeze-ready text. A primitive that appears in the enumeration above but not here is still skeleton-only. The enumeration table is the index; this section is the normative semantics.
>
> Each primitive is specified through a fixed seven-field rubric: **Intent**, **Parameters**, **Preconditions**, **Postconditions**, **Safety envelope**, **Failure modes**, **Conformance test sketch**. All quantitative bounds are expressed relative to embodiment-declared descriptor fields (`embodiment.*`), never as absolute constants, in service of Principle 1 (embodiment-agnostic).

### Category 1 — `reach`

`reach` primitives bring a controlled frame into a target pose **without forming task contact**. The absence of intended contact is what separates `reach` from `grasp` and `force`; it is enforced in both the postconditions (no contact formed) and the safety envelope (external force bounded by the embodiment's collision-detection threshold). All six `reach` primitives are defined relative to `reach.to_pose`, the base free-space motion, so that composition and retargeting logic is shared rather than re-derived per primitive.

#### 1.1 `reach.to_pose`

**Intent.** Move the embodiment's controlled frame to an absolute target pose expressed in a named reference frame, terminating at rest, without forming task contact.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target_pose` | `Pose6D` | — (required) | m / rad | must admit ≥ 1 IK solution for `controlled_frame` |
| `frame` | `FrameRef` | `task` | — | calibrated frame in `embodiment.known_frames` |
| `controlled_frame` | `FrameRef` | `embodiment.default_control_frame` | — | a declared control frame on the kinematic tree |
| `position_tolerance` | `Length` | `2` | mm | `> 0`; `≥` embodiment positioning floor |
| `orientation_tolerance` | `Angle` | `2` | deg | `> 0`; geodesic angle on SO(3) |
| `max_velocity` | `Velocity \| auto` | `auto` | m/s | clamped to `embodiment.limits.v_cartesian_max` |
| `clearance` | `Length` | `0` | mm | `≥ 0`; minimum margin to the static collision model |
| `contact_response` | `{abort, stop, comply}` | `abort` | — | reaction to unplanned contact |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `frame` and `controlled_frame` are resolvable and `calibration_valid`.
- `target_pose` lies within `embodiment.workspace(controlled_frame)` (at least one IK solution exists).
- `target_pose`, inflated by `clearance`, is collision-free against the static environment model.
- `controlled_frame` is not bound to an active force-control loop owned by another primitive.

**Postconditions (on `success`).**
- `pose(controlled_frame, frame)` is within `(position_tolerance, orientation_tolerance)` of `target_pose`.
- The embodiment is at rest: `‖cartesian_velocity(controlled_frame)‖ ≈ 0`.
- No task contact was formed; world-model object poses are invariant.

**Safety envelope (holds throughout execution).**
- `‖cartesian_velocity(controlled_frame)‖ ≤ min(max_velocity, embodiment.limits.v_cartesian_max)`.
- `‖cartesian_acceleration(controlled_frame)‖ ≤ embodiment.limits.a_cartesian_max`.
- `joint_velocity ≤ embodiment.limits.joint_velocity_ceiling` (descriptor-inherited).
- `min_clearance(controlled_frame, static_model) ≥ clearance`.
- `external_force(controlled_frame) ≤ contact_abort_threshold` (default `auto` → embodiment collision-detection threshold); a breach triggers `contact_response`.
- On any breach: decelerate to rest within `embodiment.limits.stop_time`, ending in a safe state.

**Failure modes (detection → invariant the conformance suite hooks).**

| Mode | Detection | Invariant |
|---|---|---|
| `unreachable` | pre-motion IK / workspace check | no motion may occur |
| `no_collision_free_path` | planner returns no path within `clearance` | no motion past the last safe configuration |
| `unexpected_contact` | runtime `external_force` > threshold | react per `contact_response`; an abort ends at rest |
| `pose_not_reached` | post-motion pose vs tolerance | MUST NOT report `success` when out of tolerance |
| `timeout` | wall clock vs `timeout` | clean halt at rest |
| `envelope_violation` | runtime velocity / accel / clearance monitor | abort + safe state |

**Conformance test sketch.**
- **C1 — nominal.** Command `reach.to_pose(target_pose = P)` for a bench-calibrated reachable `P`. PASS iff `result == success` ∧ externally measured (bench metrology, not self-report) `controlled_frame` pose is within `(position_tolerance, orientation_tolerance)` of `P` ∧ terminal velocity ≈ 0 ∧ no telemetry sample exceeded the velocity cap.
- **C2 — negative / unreachable.** Command a `P` 1 m beyond the declared workspace. PASS iff `result == unreachable` ∧ `controlled_frame` displacement `< ε` (reject-without-attempt).
- **C3 — safety / contact.** Insert a calibrated compliant obstacle into the planned path *after* planning. PASS iff `result == unexpected_contact` ∧ peak external force `≤ threshold + transient_margin` ∧ at-rest within `stop_time`.

#### 1.2 `reach.approach`

**Intent.** Move the controlled frame to a standoff pose offset from a target surface point along that surface's outward normal, oriented so the controlled frame's tool axis points toward the surface, terminating at rest without contact.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target` | `SurfaceTarget` | — (required) | — | resolvable point `p` + outward unit normal `n` in a calibrated frame |
| `standoff` | `Length` | — (required) | mm | `> 0`; terminal distance from surface along `n` |
| `controlled_frame` | `FrameRef` | `embodiment.default_control_frame` | — | declared control frame |
| `approach_axis` | `Axis` | `embodiment.default_tool_axis` | — | controlled-frame axis aligned anti-parallel to `n` |
| `position_tolerance` | `Length` | `2` | mm | `> 0` |
| `orientation_tolerance` | `Angle` | `2` | deg | `> 0`; geodesic on SO(3) |
| `max_velocity` | `Velocity \| auto` | `auto` | m/s | clamped to `embodiment.limits.v_cartesian_max` |
| `clearance` | `Length` | `0` | mm | `≥ 0`; margin to static model **excluding `target`** |
| `contact_response` | `{abort, stop, comply}` | `abort` | — | reaction to contact before standoff |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Derived pose.** `approach` is defined as `reach.to_pose(target_pose = derive(p, n, standoff), …)` where `derive` yields position `p + standoff·n` and an orientation placing `approach_axis` anti-parallel to `n`. The remaining rotational DOF about `n` is left free unless constrained by a composed `reach.align`. All `reach.to_pose` semantics are inherited except where overridden below.

**Preconditions.**
- `target.point` and `target.normal` are resolvable in a `calibration_valid` frame, with estimated uncertainty `≤ embodiment.perception.pose_uncertainty_bound` (or a caller-supplied bound).
- The derived standoff pose admits ≥ 1 IK solution for `controlled_frame`.
- The straight-line corridor from the current pose to the standoff pose, inflated by `clearance`, is collision-free against the static model **excluding `target`**.
- `standoff > 0` (the terminal pose does not contact `target`).

**Postconditions (on `success`).**
- `controlled_frame` position within `position_tolerance` of `p + standoff·n`.
- `approach_axis` anti-parallel to `n` within `orientation_tolerance`.
- Embodiment at rest; no task contact formed.
- Distance from the `controlled_frame` tool point to the `target` surface ≈ `standoff` (within `position_tolerance`).

**Safety envelope (holds throughout execution).**
- All `reach.to_pose` envelope clauses (velocity, acceleration, joint-velocity, stop-time).
- `min_clearance(controlled_frame, static_model \ target) ≥ clearance`.
- **No overshoot:** `distance(controlled_frame, target.surface) ≥ standoff − overshoot_margin` throughout (default `overshoot_margin = position_tolerance`); a breach is an envelope violation.
- `external_force(controlled_frame) ≤ contact_abort_threshold`; a breach triggers `contact_response` (contact before standoff implies a perception error and must degrade safely).

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `target_underdetermined` | `point`/`normal` unresolved or uncertainty over bound | no motion may occur |
| `unreachable` | IK on the derived standoff pose | no motion |
| `no_collision_free_path` | planner finds no corridor within `clearance` | no motion past the last safe config |
| `unexpected_contact` | `external_force` > threshold before standoff | react per `contact_response`; an abort ends at rest |
| `overshoot` | distance < `standoff − overshoot_margin` | abort + safe state |
| `pose_not_reached` | post-motion pose / standoff distance vs tolerance | MUST NOT report `success` |
| `timeout` | wall clock vs `timeout` | clean halt at rest |

**Conformance test sketch.**
- **C1 — nominal standoff.** Provide a bench surface with a metrology-known point and normal; command `reach.approach(target, standoff = 50 mm)`. PASS iff `result == success` ∧ externally measured tool-point-to-surface distance is within `position_tolerance` of 50 mm ∧ `approach_axis` is anti-parallel to `n` within `orientation_tolerance` ∧ no telemetry sample recorded contact (external force below threshold throughout).
- **C2 — perception-error / early contact.** Place the real surface 10 mm *nearer* than the supplied `point` (inject a perception error so the true surface lies inside the standoff). PASS iff `result == unexpected_contact` ∧ peak external force `≤ threshold + transient_margin` ∧ at-rest within `stop_time`. (Verifies safe degradation under the dominant real-world failure mode for approach.)

#### 1.3 `reach.align`

**Intent.** Rotate the controlled frame so that a selected set of its basis axes becomes parallel to the same-named axes of a target frame, optionally holding position fixed, terminating at rest without contact.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target_frame` | `FrameRef` | — (required) | — | calibrated reference orientation |
| `controlled_frame` | `FrameRef` | `embodiment.default_control_frame` | — | declared control frame |
| `axes` | `set<{x,y,z}> \| all` | `all` | — | controlled-frame axes to bring parallel to `target_frame` |
| `orientation_tolerance` | `Angle` | `2` | deg | `> 0`; per-axis geodesic angle |
| `hold_position` | `bool` | `true` | — | keep `controlled_frame` position fixed during rotation |
| `position_tolerance` | `Length` | `2` | mm | drift bound while rotating (when `hold_position`) |
| `max_angular_velocity` | `AngularVelocity \| auto` | `auto` | rad/s | clamped to `embodiment.limits.w_cartesian_max` |
| `clearance` | `Length` | `0` | mm | `≥ 0`; margin to static model over the swept volume |
| `contact_response` | `{abort, stop, comply}` | `abort` | — | reaction to unplanned contact |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Constraint cardinality.** Selecting ≥ 2 axes fully determines orientation (the third is implied). Selecting exactly 1 axis leaves a 1-DOF residual rotation about that axis. The residual is resolved deterministically as the **minimum geodesic rotation from the current orientation** that satisfies the axis constraint; this rule is binding because `retarget` must be byte-for-byte reproducible (`02-translation-layer.md` § Retargeting). Selecting 0 axes is rejected at validation.

**Preconditions.**
- `target_frame` and `controlled_frame` are resolvable and `calibration_valid`.
- The constrained orientation (at the held position, if `hold_position`) admits ≥ 1 IK solution.
- The rotation's swept volume, inflated by `clearance`, is collision-free against the static model.

**Postconditions (on `success`).**
- For each `a ∈ axes`: `controlled_frame.a` is parallel to `target_frame.a` within `orientation_tolerance`.
- If `hold_position`: `controlled_frame` position is unchanged within `position_tolerance`.
- Embodiment at rest; no task contact formed.

**Safety envelope (holds throughout execution).**
- `‖angular_velocity(controlled_frame)‖ ≤ min(max_angular_velocity, embodiment.limits.w_cartesian_max)`.
- All link extremities respect `embodiment.limits.v_cartesian_max` (in-place rotation still translates links).
- `min_clearance(swept_volume, static_model) ≥ clearance`.
- If `hold_position`: `‖position(controlled_frame) − position₀‖ ≤ position_tolerance` throughout.
- `external_force(controlled_frame) ≤ contact_abort_threshold`; a breach triggers `contact_response`.
- On any breach: decelerate to rest within `embodiment.limits.stop_time`, ending in a safe state.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `target_underdetermined` | `target_frame` unresolved | no motion may occur |
| `empty_axis_set` | `axes` resolves to ∅ | rejected at validation; no motion |
| `unreachable` | IK on the constrained orientation at the held position | no motion |
| `no_collision_free_path` | swept-volume collision check fails | no motion past the last safe config |
| `position_drift` | `hold_position` and drift > `position_tolerance` | abort + safe state |
| `unexpected_contact` | `external_force` > threshold | react per `contact_response` |
| `orientation_not_reached` | post-motion per-axis angle vs tolerance | MUST NOT report `success` |
| `timeout` | wall clock vs `timeout` | clean halt at rest |

**Conformance test sketch.**
- **C1 — single-axis alignment.** Command `reach.align(target_frame, axes = {z})` from a perturbed orientation. PASS iff `result == success` ∧ externally measured angle between `controlled_frame.z` and `target_frame.z` `≤ orientation_tolerance` ∧ (with `hold_position`) position drift `≤ position_tolerance` ∧ at rest. The residual rotation about `z` is unconstrained and not scored, but the resolved value MUST be reproducible across identical re-runs (determinism check).
- **C2 — full orientation.** Command `axes = all`. PASS iff every per-axis angle to `target_frame` `≤ orientation_tolerance` ∧ position drift within tolerance ∧ at rest.

#### 1.4 `reach.retract`

**Intent.** Move the controlled frame a specified distance along a retreat direction (by default, the reverse of its own tool axis), breaking any incidental contact and clearing the object envelope, terminating at rest. `retract` is the dual of `reach.approach`: where `approach` is permitted no contact, `retract` is permitted to *start* in contact and must leave it.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `distance` | `Length` | — (required) | mm | `> 0`; retreat travel along `retract_axis` |
| `controlled_frame` | `FrameRef` | `embodiment.default_control_frame` | — | declared control frame |
| `retract_axis` | `SignedAxis` | `−embodiment.default_tool_axis` | — | retreat direction |
| `frame` | `FrameRef` | `controlled_frame` | — | frame the axis is expressed in (default: move along own tool axis) |
| `break_contact` | `bool` | `true` | — | require any starting contact to be cleared |
| `position_tolerance` | `Length` | `2` | mm | `> 0` |
| `max_velocity` | `Velocity \| auto` | `auto` | m/s | clamped to `embodiment.limits.v_cartesian_max` |
| `clearance` | `Length` | `0` | mm | `≥ 0`; margin to the full static model (no target exclusion) |
| `contact_response` | `{abort, stop, comply}` | `abort` | — | reaction to a force *increase* during retreat |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `controlled_frame` is resolvable and `calibration_valid`.
- The retreat target pose (`start + distance · retract_axis`) admits ≥ 1 IK solution.
- The retreat corridor, inflated by `clearance`, is collision-free against the static model in the retreat direction.
- Starting contact is permitted (no precondition forbidding it), unlike `reach.to_pose`/`reach.approach`.

**Postconditions (on `success`).**
- `controlled_frame` displaced by `distance` along `retract_axis` from the start pose, within `position_tolerance`.
- Embodiment at rest.
- If `break_contact`: no task contact remains; `external_force(controlled_frame) ≈ 0`.
- World-model object poses are invariant (retract carries nothing; carrying is `transport`).

**Safety envelope (holds throughout execution).**
- `‖cartesian_velocity(controlled_frame)‖ ≤ min(max_velocity, embodiment.limits.v_cartesian_max)`; acceleration / joint-velocity per `reach.to_pose`.
- `min_clearance(controlled_frame, static_model) ≥ clearance`.
- **Directional force monotonicity:** `external_force` along the retreat direction is non-increasing beyond `force_noise_margin`; an *increase* signals an obstacle behind the frame and triggers `contact_response`. (Force *opposing* retreat — the surface being left — is expected to decay to zero and is not a violation.)
- On any breach: decelerate to rest within `embodiment.limits.stop_time`, ending in a safe state.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `unreachable` | IK on the retreat target pose | no motion |
| `blocked_path` | clearance violation in the retreat direction | no motion past the last safe config |
| `unexpected_contact` | `external_force` increases along retreat beyond margin | react per `contact_response`; an abort ends at rest |
| `contact_not_cleared` | `break_contact` set, residual force at end | MUST NOT report `success` |
| `pose_not_reached` | final displacement vs `distance` / tolerance | MUST NOT report `success` |
| `timeout` | wall clock vs `timeout` | clean halt at rest |

**Conformance test sketch.**
- **C1 — nominal retreat.** From a bench-known start pose, command `reach.retract(distance = 50 mm)` along `−tool_axis`. PASS iff `result == success` ∧ externally measured displacement is `50 mm` along the axis within `position_tolerance` ∧ at rest ∧ no residual external force.
- **C2 — break-contact without drag.** Start with the tool in light contact against an instrumented, free-standing surface; command retract. PASS iff `result == success` ∧ final external force `≈ 0` ∧ the surface's measured pose is unchanged (the retreat broke contact without dragging it) ∧ at rest.

#### 1.5 `reach.hover`

**Intent.** Maintain the controlled frame at a standoff pose offset from a target along the target's outward normal, actively rejecting disturbances over a bounded interval, without forming contact. Unlike `reach.approach` (a transient motion ending at rest), `hover` is a sustained station-keeping primitive.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target` | `SurfaceTarget \| FrameRef` | — (required) | — | reference to hover over; may be time-varying |
| `standoff` | `Length` | — (required) | mm | `> 0`; maintained distance from `target` along `n` |
| `controlled_frame` | `FrameRef` | `embodiment.default_control_frame` | — | declared control frame |
| `approach_axis` | `Axis` | `embodiment.default_tool_axis` | — | axis held anti-parallel to `n` |
| `station_tolerance` | `Length` | `2` | mm | allowed positional excursion from the hover setpoint |
| `orientation_tolerance` | `Angle` | `2` | deg | `> 0`; geodesic |
| `duration` | `Duration \| until` | `until` | s | hold time; `until` defers termination to an enclosing `reactive` predicate |
| `track_target` | `bool` | `false` | — | follow a moving target vs. hold a fixed setpoint |
| `settling_time` | `Duration \| auto` | `auto` | s | max time to return within `station_tolerance` after a disturbance |
| `max_velocity` | `Velocity \| auto` | `auto` | m/s | correction / tracking speed cap |
| `clearance` | `Length` | `0` | mm | `≥ 0`; margin to static model **excluding `target`** |
| `contact_response` | `{abort, stop, comply}` | `abort` | — | reaction to unplanned contact |

**Setpoint.** The hover setpoint is `S = p + standoff·n` (for a `FrameRef` target, `p` is the frame origin and `n` its outward/declared axis). With `track_target = false`, `S` is latched at `t₀`; with `track_target = true`, `S(t)` follows the target, valid only while the target's motion stays within `embodiment.limits.tracking_bandwidth`.

**Preconditions.**
- `target` is resolvable in a `calibration_valid` frame; uncertainty `≤` bound (as in `reach.approach`).
- The initial setpoint `S(t₀)` admits ≥ 1 IK solution and is reachable via a collision-free corridor (clearance applied, **excluding `target`**).
- If `track_target`: the anticipated target motion is within `embodiment.limits.tracking_bandwidth`.
- `standoff > 0`.

**Postconditions (on `success`).**
- The station invariant held for the entire interval: `‖pose(controlled_frame) − S(t)‖ ≤ station_tolerance` ∀ `t ∈ [t₀, t_end]` (position), and `approach_axis` anti-parallel to `n` within `orientation_tolerance` throughout.
- Termination was clean per the termination condition (`duration` elapsed, or the enclosing `reactive` predicate fired).
- No task contact was formed; embodiment at rest at `t_end` (unless control is handed to a composed successor).

**Safety envelope (holds throughout the interval).**
- `‖cartesian_velocity(controlled_frame)‖ ≤ min(max_velocity, embodiment.limits.v_cartesian_max)` during corrections/tracking.
- `min_clearance(controlled_frame, static_model \ target) ≥ clearance`.
- `external_force(controlled_frame) ≤ contact_abort_threshold`; a breach triggers `contact_response`.
- Disturbance recovery: after a bounded disturbance, station error returns `≤ station_tolerance` within `settling_time`; failure to recover is an envelope violation.
- On any breach: decelerate to rest within `embodiment.limits.stop_time`, ending in a safe state.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `target_underdetermined` | `target` unresolved or over uncertainty bound | no motion may occur |
| `unreachable` | IK on initial setpoint `S(t₀)` | no motion |
| `no_collision_free_path` | corridor to `S(t₀)` blocked | no motion past the last safe config |
| `track_lost` | target exits workspace or exceeds `tracking_bandwidth` mid-hover | abort + safe state; report interval held before loss |
| `station_exceeded` | error > `station_tolerance` beyond `settling_time` | abort + safe state |
| `unexpected_contact` | `external_force` > threshold | react per `contact_response` |
| `timeout` | watchdog while acquiring `S(t₀)` | clean halt at rest |

**Conformance test sketch.**
- **C1 — sustained hold.** Command `reach.hover(target, standoff = 50 mm, duration = 5 s)` over a fixed bench target. PASS iff `result == success` ∧ at **every** sampled instant over the 5 s (interval sampling, not endpoint) the externally measured station error `≤ station_tolerance` and orientation within `orientation_tolerance` ∧ no contact recorded ∧ clean termination at 5 s.
- **C2 — disturbance rejection.** During the hold, apply a calibrated lateral impulse. PASS iff station error returns `≤ station_tolerance` within `settling_time` (recover-and-continue), OR — if the impulse exceeds the envelope — the primitive aborts within `stop_time` to a safe state. Either outcome must be the deterministic function of the impulse magnitude.

#### 1.6 `reach.scan`

**Intent.** Sweep a declared sensor frame over a parameterized region at a maintained standoff, following a deterministic coverage pattern, so a perception system can observe the region — without forming contact or changing object state. The primitive's contract is purely kinematic (it guarantees the sweep poses were visited); interpretation of the sensed data is out of RFL scope.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `region` | `ScanRegion` | — (required) | — | volume / surface / path to cover, in a calibrated frame |
| `sensor_frame` | `FrameRef` | `embodiment.default_sensor_frame` | — | frame whose coverage matters (may equal the control frame) |
| `standoff` | `Length` | — (required) | mm | `> 0`; sensor-to-region distance maintained during the sweep |
| `pattern` | `{raster, spiral, arc, waypoints}` | `raster` | — | deterministic sweep pattern over `region` |
| `coverage_overlap` | `Ratio` | `auto` | — | overlap between passes; `auto` derives from `sensor_frame` FOV |
| `sensor_axis` | `Axis` | `embodiment.sensors[sensor_frame].bore_axis` | — | sensor axis pointed at the region |
| `dwell` | `Duration` | `0` | s | hold time at each sweep pose (sensor integration) |
| `position_tolerance` | `Length` | `2` | mm | per-sweep-pose positional tolerance |
| `orientation_tolerance` | `Angle` | `2` | deg | `> 0`; geodesic |
| `max_velocity` | `Velocity \| auto` | `auto` | m/s | sweep speed; also bounded by `embodiment.sensors[sensor_frame].max_sweep_rate` |
| `clearance` | `Length` | `0` | mm | `≥ 0`; margin to static model over the full sweep path, **excluding `region`** |
| `contact_response` | `{abort, stop, comply}` | `abort` | — | reaction to unplanned contact |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Sweep set.** From `(region, pattern, standoff, coverage_overlap, sensor FOV)` a **deterministic** ordered set of sweep poses `Σ = {σ₁ … σₖ}` is generated, each at `standoff` from the region surface with `sensor_axis` pointed at it. `Σ` is a pure function of the inputs (required for `retarget` determinism); the pattern generators are normatively specified (see Open issue).

**Preconditions.**
- `region` and `sensor_frame` are resolvable and `calibration_valid`.
- Every `σ ∈ Σ` admits ≥ 1 IK solution, and the path connecting them is collision-free at `clearance` (excluding `region`).
- `standoff > 0`; `embodiment.sensors[sensor_frame]` declares the FOV / `bore_axis` needed to generate `Σ` when `coverage_overlap = auto`.

**Postconditions (on `success`).**
- Every `σ ∈ Σ` was visited: `sensor_frame` passed within `(position_tolerance, orientation_tolerance)` of `σ` and held for `≥ dwell`.
- `standoff` was maintained throughout (sensor never closer than `standoff − position_tolerance` to `region`).
- No task contact was formed; world-model object poses are invariant.
- Embodiment at rest at the end of the sweep.

**Safety envelope (holds throughout execution).**
- `‖cartesian_velocity(sensor_frame)‖ ≤ min(max_velocity, embodiment.limits.v_cartesian_max, embodiment.sensors[sensor_frame].max_sweep_rate)`.
- `min_clearance(swept_path_volume, static_model \ region) ≥ clearance`.
- No overshoot toward `region`: `distance(sensor_frame, region.surface) ≥ standoff − position_tolerance` throughout.
- `external_force(sensor_frame) ≤ contact_abort_threshold`; a breach triggers `contact_response`.
- On any breach: decelerate to rest within `embodiment.limits.stop_time`, ending in a safe state.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `region_underdetermined` | `region`/`sensor_frame` unresolved, or FOV missing when `overlap = auto` | no motion may occur |
| `unreachable` | no `σ ∈ Σ` admits IK | no motion |
| `coverage_incomplete` | some but not all `σ ∈ Σ` reachable/visited | visit the reachable subset safely; report covered fraction in telemetry; result ≠ `success` |
| `no_collision_free_path` | swept-path collision check fails | no motion past the last safe config |
| `unexpected_contact` | `external_force` > threshold | react per `contact_response` |
| `pose_not_reached` | a `σ` visited but out of tolerance, or `dwell` not met | MUST NOT report `success` |
| `timeout` | wall clock vs `timeout` | clean halt at rest |

**Conformance test sketch.**
- **C1 — nominal raster coverage.** Command `reach.scan(region, pattern = raster, standoff = 100 mm)` over a metrology-known planar region. The conformance suite **recomputes `Σ` from the inputs** and PASSES iff `result == success` ∧ the externally measured `sensor_frame` trajectory passed within `(position_tolerance, orientation_tolerance)` of every `σ ∈ Σ` ∧ `dwell` satisfied at each ∧ standoff maintained ∧ no contact recorded.
- **C2 — deterministic sweep generation.** Issue identical inputs twice. PASS iff the generated `Σ` is byte-for-byte identical across runs and across two independent conformant implementations (cross-implementation determinism).

### Category 2 — `grasp`

`grasp` primitives form and commit a contact pattern on a target object. Unlike `reach`, contact is the *intent*. At the Skill ISA level a `grasp` primitive commits only the contact pattern and a force budget; the force-controlled completion (joint torques for tendon-driven hands, pneumatic pressure for bellows hands, jaw force for parallel-jaw grippers) is resolved per embodiment by the Translation Layer. `grasp` primitives are the first to carry a **capability requirement** (not every embodiment supports every grasp) and to integrate the **TactileManifold** for contact confirmation, with a graceful-degradation proxy for embodiments lacking tactile sensing (Principle 5).

#### 2.1 `grasp.pinch`

**Intent.** Form a stable two-opposing-point (antipodal) pinch on a target object, driving opposing contact regions together until force closure is achieved within a force budget, with contact confirmed through the tactile manifold. Force-controlled completion is resolved per embodiment by the Translation Layer.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target` | `ObjectTarget` | — (required) | — | geometry + pose + material/mass descriptor; perception-derived, supplied via `LetBind` |
| `force_budget` | `Force` | — (required) | N | `> 0`; clamped to `embodiment.limits.grip_force_max` and `target.max_contact_force` if declared |
| `grasp_pose` | `Pose6D \| auto` | `auto` | — | pinch contact-frame pose; `auto` = planner-derived from `target` |
| `grasp_width` | `Length \| auto` | `auto` | mm | pre-close opening; `auto` from `target` geometry |
| `controlled_frame` | `FrameRef` | `embodiment.default_grasp_frame` | — | declared grasp frame |
| `tactile_target` | `TactileTarget \| auto` | `auto` | — | contact-confirmation criterion in TactileManifold terms; `auto` = `normal_force ≥ ε` at ≥ 2 antipodal sites |
| `slip_response` | `{abort, retighten, hold}` | `retighten` | — | reaction to detected slip |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- *Capability:* `embodiment` declares `pinch_grasp` capability; `tactile_sensing` is **preferred** (if absent, confirmation degrades to a force/position proxy — see Failure modes).
- `target` resolvable in a `calibration_valid` frame; `target.estimated_mass ≤ embodiment.limits.payload_grasp_pinch`.
- `controlled_frame` is free (not already holding an object) and `target` lies within `embodiment.grasp_envelope(controlled_frame)` (this bounds the final approach; gross transit is `reach`'s responsibility).
- The derived `grasp_pose` admits ≥ 1 IK solution and an antipodal contact pair exists on `target` for the embodiment's opposing geometry.

**Postconditions (on `success`).**
- `object_held(target, mode = pinch)`: the object is held in a stable two-opposing-point pinch (force closure), grip force `≤ force_budget`.
- `tactile_target` satisfied (or its degraded proxy): contact confirmed at the opposing sites.
- `controlled_frame` now owns `target`; `end_effector_free = false`.

**Safety envelope (holds throughout execution).**
- `peak_grip_force ≤ force_budget · (1 + transient_margin)` (default `transient_margin = 0.2`, per `reach` convention).
- `closing_velocity ≤ embodiment.limits.v_grasp`.
- Crush protection: if `target` declares a fragility / `max_contact_force`, or the tactile manifold reports a deformation-rate indicator over threshold, abort before exceeding it.
- `min_clearance(controlled_frame, static_model \ target) ≥ clearance` during the final approach.
- On any breach: release to a safe state (open contact, no residual force) within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `target_underdetermined` | `target` unresolved / over uncertainty bound | no closing may occur |
| `unreachable` | IK on `grasp_pose` / no antipodal pair | no closing |
| `capability_absent` | `pinch_grasp` not declared | reject at validation; no attempt |
| `no_contact_confirmation` | closed but `tactile_target` (or proxy) unmet — e.g. missed / empty grasp | open to safe state; result ≠ `success` |
| `force_exceeded` | force saturates at budget without force closure | stop at budget; never exceed margin; result ≠ `success` |
| `slip` | tactile slip feature fires post-closure | apply `slip_response`; if unrecoverable, report `slip` |
| `crush_abort` | deformation-rate / `max_contact_force` exceeded | abort + release to safe state |
| `timeout` | wall clock vs `timeout` | open to safe state |

**Conformance test sketch.**
- **C1 — nominal grasp + hold.** Present a bench-fixtured graspable object within the grasp envelope; command `grasp.pinch(target, force_budget = 5 N)`. PASS iff `result == success` ∧ a post-grasp **hold test** (apply a calibrated perturbation below `force_budget`) retains the object ∧ externally measured peak grip force `≤ force_budget · (1 + transient_margin)` ∧ tactile (or proxy) confirmed contact at ≥ 2 opposing sites.
- **C2 — missed object / empty close.** Command `grasp.pinch` with no object at `grasp_pose`. PASS iff `result == no_contact_confirmation` (closing on nothing is detected, not falsely reported `success`) ∧ no force exceeded ∧ end-effector returns to a safe state.

#### 2.2 `grasp.power`

**Intent.** Form a stable whole-volume enclosure of a target object, distributing contact across many regions to maximize stability and payload within a force budget, with enclosure confirmed through the tactile manifold. Force-controlled completion is resolved per embodiment by the Translation Layer.

**Parameters.** (Shares the `grasp` core with `grasp.pinch`; deltas marked **†**.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target` | `ObjectTarget` | — (required) | — | geometry + pose + material/mass descriptor; via `LetBind` |
| `force_budget` | `Force` | — (required) | N | `> 0`; clamped to `embodiment.limits.grip_force_max` and `target.max_contact_force` if declared |
| `grasp_pose` | `Pose6D \| auto` | `auto` | — | enclosure contact-frame pose; `auto` = planner-derived |
| `controlled_frame` | `FrameRef` | `embodiment.default_grasp_frame` | — | declared grasp frame |
| `enclosure_completeness` **†** | `Ratio \| auto` | `auto` | — | required fraction of available contact sites that must engage; `auto` from `target` + embodiment geometry |
| `tactile_target` | `TactileTarget \| auto` | `auto` | — | `auto` = `normal_force ≥ ε` at ≥ `enclosure_completeness · N_sites` enclosure sites |
| `slip_response` | `{abort, retighten, hold}` | `retighten` | — | reaction to detected slip |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- *Capability:* `embodiment` declares `power_grasp`; `tactile_sensing` preferred (else proxy).
- `target.estimated_mass ≤ embodiment.limits.payload_grasp_power` (**†** typically higher than the pinch payload).
- **†** `target` characteristic size `≤ embodiment.limits.enclosure_span` (an object larger than the enclosure span cannot be power-grasped).
- `controlled_frame` free; `target` within `embodiment.grasp_envelope(controlled_frame)`.
- The derived `grasp_pose` admits ≥ 1 IK solution and an enclosing contact configuration exists for the embodiment's geometry.

**Postconditions (on `success`).**
- `object_held(target, mode = power)`: enclosed grasp with distributed contact (force closure), grip force `≤ force_budget`.
- `tactile_target` satisfied (or proxy): contact confirmed at ≥ the enclosure-completeness threshold.
- `controlled_frame` owns `target`; `end_effector_free = false`.

**Safety envelope (holds throughout execution).**
- `peak_total_force ≤ force_budget · (1 + transient_margin)` (aggregate over enclosure sites).
- Per-site force respects `target.max_contact_force` if declared (distributed contact is gentler per area, but the total can be high).
- `closing_velocity ≤ embodiment.limits.v_grasp`; crush protection per `target` fragility / deformation-rate.
- `min_clearance(controlled_frame, static_model \ target) ≥ clearance` during the final approach.
- On any breach: release to a safe state within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `target_underdetermined` | `target` unresolved / over uncertainty bound | no closing |
| `object_too_large` **†** | characteristic size > `enclosure_span` | reject at validation; no attempt |
| `unreachable` | IK on `grasp_pose` / no enclosing configuration | no closing |
| `capability_absent` | `power_grasp` not declared | reject; no attempt |
| `incomplete_enclosure` **†** | closed but `enclosure_completeness` threshold unmet | open to safe state; result ≠ `success` |
| `force_exceeded` | force saturates without force closure | stop at budget; result ≠ `success` |
| `slip` | tactile slip feature fires post-closure | apply `slip_response`; else report `slip` |
| `crush_abort` | per-site / aggregate force or deformation-rate exceeded | abort + release to safe state |
| `timeout` | wall clock vs `timeout` | open to safe state |

**Conformance test sketch.**
- **C1 — nominal power grasp + hold.** Present a bench-fixtured heavier/larger object within the enclosure span; command `grasp.power(target, force_budget = 30 N)`. PASS iff `result == success` ∧ a post-grasp **hold test** at a larger calibrated perturbation (power grasps must resist more than pinch) retains the object ∧ externally measured peak total force `≤ force_budget · (1 + transient_margin)` ∧ contact confirmed at ≥ the enclosure threshold.
- **C2 — oversize rejection.** Present an object whose characteristic size exceeds `enclosure_span`. PASS iff `result == object_too_large` ∧ no closing attempted ∧ safe state.

#### 2.3 `grasp.hook`

**Intent.** Engage a hookable feature (handle, loop, strap, or bar) on a target by threading the controlled frame into a hook configuration, achieving **form closure** that supports load along a primary load direction — without relying on opposing grip force. Unlike `grasp.pinch`/`grasp.power` (force closure), a hook grasp is directional.

**Parameters.** (Grasp core with hook-specific deltas marked **†**.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target` | `ObjectTarget` | — (required) | — | must expose a hookable feature; via `LetBind` |
| `hook_feature` **†** | `FeatureRef \| auto` | `auto` | — | the handle / loop / bar to engage; `auto` = planner-selected |
| `load_budget` **†** | `Force` | — (required) | N | `> 0`; load the engagement must support; replaces `force_budget` (no opposing squeeze) |
| `load_direction` **†** | `Direction \| auto` | `auto` | — | primary supported load direction; `auto` = anticipated load (often gravity) |
| `seating_force` | `Force \| auto` | `auto` | N | small force to fully seat the hook; bounded by `target.max_contact_force` |
| `grasp_pose` | `Pose6D \| auto` | `auto` | — | hook engagement pose |
| `controlled_frame` | `FrameRef` | `embodiment.default_grasp_frame` | — | declared grasp frame |
| `tactile_target` | `TactileTarget \| auto` | `auto` | — | `auto` = contact along the hook inner curve (full seating) |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- *Capability:* `embodiment` declares `hook_grasp`; `tactile_sensing` preferred (else proxy).
- `target` exposes a resolvable `hook_feature` (handle/loop/bar) with known geometry.
- `embodiment.limits.hook_load_capacity ≥ load_budget` (and the feature's load rating `≥ load_budget` if declared).
- The derived `grasp_pose` admits ≥ 1 IK solution and the hook can be threaded around / through the feature without collision.
- `controlled_frame` free.

**Postconditions (on `success`).**
- `object_held(target, mode = hook, closure = form, stable_directions = {load_direction})`: the hook is fully seated around `hook_feature`, supporting load along `load_direction`.
- **Directional stability (declared):** the grasp is stable under loads along `load_direction` but **may disengage under reverse or lateral loads** — this is recorded in the result so downstream `transport`/`in_hand` respect the stability envelope.
- Engagement confirmed (tactile inner-curve contact, or proxy).
- `controlled_frame` owns `target`; `end_effector_free = false`.

**Safety envelope (holds throughout execution).**
- Threading motion respects `min_clearance(controlled_frame, static_model \ target) ≥ clearance` and `embodiment.limits.v_grasp`.
- `seating_force ≤ target.max_contact_force` if declared (the hook seats, it does not crush the handle).
- Full-seating guard: a partially seated hook (engagement below the tactile / geometry threshold) must not be reported as held.
- On any breach: withdraw the hook to a safe, disengaged state within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `capability_absent` | `hook_grasp` not declared | reject at validation; no attempt |
| `no_hookable_feature` | `target` exposes no resolvable hookable feature | reject; no attempt |
| `load_rating_insufficient` | `hook_load_capacity` or feature rating < `load_budget` | reject; no attempt |
| `unreachable` | IK on `grasp_pose` / hook cannot be threaded | no engagement |
| `partial_engagement` | hook seated below threshold (slip risk) | withdraw to safe state; result ≠ `success` |
| `timeout` | wall clock vs `timeout` | withdraw to safe state |

**Conformance test sketch.**
- **C1 — nominal hook + directional hold.** Present a bench-fixtured handle/loop; command `grasp.hook(target, load_budget = 20 N, load_direction = down)`. PASS iff `result == success` ∧ a **directional hold test** — calibrated load applied along `load_direction` only — retains the object ∧ the result declares `stable_directions = {load_direction}` ∧ engagement confirmed along the inner curve.
- **C2 — partial-engagement rejection.** Present a feature too large to fully seat the hook. PASS iff `result == partial_engagement` (not falsely reported `success`) ∧ the hook withdraws to a safe, disengaged state.

#### 2.4 `grasp.precision_tripod`

**Intent.** Form a stable three-point (tripod) precision grasp on a small target, placing three opposing contact regions in a non-degenerate triangle so the grasp resists rotation about the grasp axis — providing orientation stability a two-point pinch cannot. Force closure; force-controlled completion resolved per embodiment by the Translation Layer.

**Parameters.** (Grasp core; tripod deltas marked **†**.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target` | `ObjectTarget` | — (required) | — | small object; via `LetBind` |
| `force_budget` | `Force` | — (required) | N | `> 0`; clamped to `grip_force_max` and `target.max_contact_force` if declared |
| `grasp_pose` | `Pose6D \| auto` | `auto` | — | tripod contact-frame pose; `auto` = planner-derived |
| `controlled_frame` | `FrameRef` | `embodiment.default_grasp_frame` | — | declared grasp frame |
| `tactile_target` | `TactileTarget \| auto` | `auto` | — | `auto` = `normal_force ≥ ε` at **3 non-collinear** sites **†** |
| `slip_response` | `{abort, retighten, hold}` | `retighten` | — | reaction to detected slip |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- *Capability:* `embodiment` declares `tripod_grasp` (≥ 3 independently positionable opposing contacts); `tactile_sensing` preferred (else proxy).
- `target.estimated_mass ≤ embodiment.limits.payload_grasp_tripod` and `target` characteristic size `≤ embodiment.limits.precision_object_size_max` **†** (tripod is a precision grasp for small objects).
- `controlled_frame` free; `target` within `embodiment.grasp_envelope(controlled_frame)`.
- The derived `grasp_pose` admits ≥ 1 IK solution and **three non-collinear contact points** exist on `target` **†**.

**Postconditions (on `success`).**
- `object_held(target, mode = tripod, closure = force, stable_directions = omnidirectional)`: held at three contact points forming a non-degenerate triangle (force closure), grip force `≤ force_budget`.
- **Rotation-constrained (declared):** the result carries `rotation_constrained = true` — the three-point triangle resists torque about the grasp axis, so `in_hand` primitives know the grasp constrains object rotation.
- `tactile_target` satisfied (or proxy) at 3 non-collinear sites; `controlled_frame` owns `target`; `end_effector_free = false`.

**Safety envelope (holds throughout execution).**
- `peak_grip_force ≤ force_budget · (1 + transient_margin)` (per-contact and aggregate); per-site `≤ target.max_contact_force` if declared.
- `closing_velocity ≤ embodiment.limits.v_grasp`; crush protection per `target` fragility / deformation-rate.
- `min_clearance(controlled_frame, static_model \ target) ≥ clearance` during the final approach.
- On any breach: release to a safe state within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `target_underdetermined` | `target` unresolved / over uncertainty bound | no closing |
| `capability_absent` | `tripod_grasp` not declared | reject; no attempt |
| `object_too_large` | size > `precision_object_size_max` | reject; no attempt |
| `degenerate_triangle` **†** | only collinear / coincident contacts achievable | open to safe state; result ≠ `success` |
| `unreachable` | IK on `grasp_pose` / no 3 non-collinear points | no closing |
| `no_contact_confirmation` | closed but < 3 sites confirmed | open to safe state; result ≠ `success` |
| `force_exceeded` | force saturates without force closure | stop at budget; result ≠ `success` |
| `slip` | tactile slip feature fires post-closure | apply `slip_response`; else report `slip` |
| `crush_abort` | per-site / aggregate force or deformation-rate exceeded | abort + release |
| `timeout` | wall clock vs `timeout` | open to safe state |

**Conformance test sketch.**
- **C1 — nominal tripod + rotation hold.** Present a bench-fixtured small object; command `grasp.precision_tripod(target, force_budget = 5 N)`. PASS iff `result == success` ∧ a hold test that includes a **calibrated torque perturbation about the grasp axis** (which a two-point pinch would fail) retains object position *and orientation* ∧ contact confirmed at 3 non-collinear sites ∧ peak force within budget.
- **C2 — degenerate-triangle rejection.** Present a target geometry admitting only collinear contacts. PASS iff `result == degenerate_triangle` (not falsely reported `success`) ∧ safe state.

#### 2.5 `grasp.lateral`

**Intent.** Form a lateral (key-grip) grasp on a thin or flat target, clamping opposing force across the object's thin dimension with at least one broad side contact — for objects too thin to tip-pinch or enclose. Force closure across the clamp direction; in-plane retention is friction-limited.

**Parameters.** (Grasp core; lateral deltas marked **†**.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target` | `ObjectTarget` | — (required) | — | thin / flat object; via `LetBind` |
| `force_budget` | `Force` | — (required) | N | `> 0`; clamped to `grip_force_max` and `target.max_contact_force` if declared |
| `grasp_edge` **†** | `FeatureRef \| auto` | `auto` | — | the thin dimension / face pair to clamp across; `auto` = planner-selected |
| `grasp_pose` | `Pose6D \| auto` | `auto` | — | clamp contact-frame pose |
| `controlled_frame` | `FrameRef` | `embodiment.default_grasp_frame` | — | declared grasp frame |
| `tactile_target` | `TactileTarget \| auto` | `auto` | — | `auto` = `normal_force ≥ ε` across the clamp (thin) dimension |
| `slip_response` | `{abort, retighten, hold}` | `retighten` | — | reaction to detected slip |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- *Capability:* `embodiment` declares `lateral_grasp` (the most broadly supported grasp — parallel-jaw embodiments qualify); `tactile_sensing` preferred (else proxy).
- `target` thickness `≤ embodiment.limits.lateral_grasp_max_thickness` **†**, with a broad face available to clamp.
- `target.estimated_mass ≤ embodiment.limits.payload_grasp_lateral`.
- `controlled_frame` free; `target` within `embodiment.grasp_envelope(controlled_frame)`.
- The derived `grasp_pose` admits ≥ 1 IK solution and an opposing contact across the thin dimension is feasible.

**Postconditions (on `success`).**
- `object_held(target, mode = lateral, closure = force, stable_directions = omnidirectional)`: clamped across the thin dimension, grip force `≤ force_budget`.
- **In-plane retention is friction-limited (declared):** retention against shear within the clamp plane depends on `force_budget` and `target` weight/friction — recorded so `transport` applies conservative acceleration for lateral grasps.
- `tactile_target` satisfied (or proxy) across the clamp dimension; `controlled_frame` owns `target`; `end_effector_free = false`.

**Safety envelope (holds throughout execution).**
- `peak_grip_force ≤ force_budget · (1 + transient_margin)`; per-site `≤ target.max_contact_force` if declared.
- Bend / crease protection: thin objects are bend-prone; if `target` declares a bend limit or the tactile manifold reports a bending indicator, abort before exceeding it.
- `closing_velocity ≤ embodiment.limits.v_grasp`; `min_clearance(controlled_frame, static_model \ target) ≥ clearance` during the final approach.
- On any breach: release to a safe state within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `target_underdetermined` | `target` unresolved / over uncertainty bound | no closing |
| `capability_absent` | `lateral_grasp` not declared | reject; no attempt |
| `object_too_thick` **†** | thickness > `lateral_grasp_max_thickness` | reject; no attempt |
| `unreachable` | IK on `grasp_pose` / no feasible clamp | no closing |
| `no_contact_confirmation` | closed but clamp contact unconfirmed | open to safe state; result ≠ `success` |
| `force_exceeded` | force saturates without closure | stop at budget; result ≠ `success` |
| `slip` | tactile slip (in-plane shear) fires post-closure | apply `slip_response`; else report `slip` |
| `bend_abort` **†** | bending indicator / declared bend limit exceeded | abort + release |
| `timeout` | wall clock vs `timeout` | open to safe state |

**Conformance test sketch.**
- **C1 — nominal lateral + pull hold.** Present a bench-fixtured thin object (e.g. a card analog); command `grasp.lateral(target, force_budget = 5 N)`. PASS iff `result == success` ∧ a hold test pulling along the **clamp-normal** (pull-out) direction retains the object ∧ contact confirmed across the thin dimension ∧ peak force within budget ∧ the result declares friction-limited in-plane retention.
- **C2 — too-thick rejection.** Present an object thicker than `lateral_grasp_max_thickness`. PASS iff `result == object_too_thick` (not falsely attempted) ∧ safe state.

#### 2.6 `grasp.platform`

**Intent.** Support a target object by placing a support surface beneath it and taking its weight, so the object rests in stable balance with its center of mass over the support contact polygon — without enclosing or squeezing it. Neither force nor form closure: stability is gravity-and-friction balance over a support polygon.

**Parameters.** (Support-mode deltas marked **†**.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target` | `ObjectTarget` | — (required) | — | must carry a center-of-mass estimate; via `LetBind` |
| `load_budget` **†** | `Force` | — (required) | N | `> 0`; max supported weight; clamped to `embodiment.limits.payload_support` |
| `support_pose` | `Pose6D \| auto` | `auto` | — | pose of the support surface under the object; `auto` = planner-derived to center CoM |
| `support_normal` **†** | `Direction \| auto` | `auto` | — | direction the support resists (usually anti-gravity); `auto` = opposing the load |
| `controlled_frame` | `FrameRef` | `embodiment.default_support_frame` | — | support-surface frame **†** |
| `tactile_target` | `TactileTarget \| auto` | `auto` | — | `auto` = distributed load borne over the support area with CoM inside the polygon |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- *Capability:* `embodiment` declares `platform_support`; distributed tactile or force-torque sensing **preferred** for load / CoM confirmation (else proxy via measured load only).
- `target.estimated_mass ≤ embodiment.limits.payload_support`, and `target` carries a resolvable center-of-mass estimate **†**.
- The support surface can be placed so the object's CoM projection along `support_normal` lies **strictly inside** the support contact polygon (static stability) **†**.
- `controlled_frame` (support surface) is free; the object is accessible for support transfer.

**Postconditions (on `success`).**
- `object_supported(target, mode = platform, closure = support, stable_directions = {support_normal})`: the object rests on the support surface, weight borne, CoM inside the support polygon.
- **Balance-only stability (declared):** stable against `support_normal` load but **not retained against tilt or lateral acceleration beyond friction** — recorded so `transport` keeps the support level and accelerates within the tip-over margin.
- Borne load confirmed (force-torque / distributed tactile, or proxy); `controlled_frame` bears `target`; `end_effector_free = false` (object supported, not enclosed).

**Safety envelope (holds throughout execution).**
- Support-placement motion respects `min_clearance(controlled_frame, static_model \ target) ≥ clearance` and `embodiment.limits.v_cartesian_max`.
- Borne load `≤ load_budget`.
- **Static-stability margin:** the CoM projection stays inside the support polygon by a margin; approaching the polygon edge (tip-over risk) triggers a controlled response.
- **Support-specific safe state:** a balanced object cannot be released by opening; on any breach the safe response is to lower the support to the nearest surface, minimizing fall height — not to drop.
- On breach: execute the support-specific safe state within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `capability_absent` | `platform_support` not declared | reject; no attempt |
| `com_unknown` **†** | `target` has no resolvable CoM estimate | reject; no attempt (stability undecidable) |
| `unstable_placement` **†** | CoM projection cannot be brought strictly inside the polygon | do not take weight; result ≠ `success` |
| `overload` | borne load > `load_budget` | do not complete; lower to safe state |
| `load_not_confirmed` | positioned but weight not borne (object not on support) | result ≠ `success`; safe state |
| `timeout` | wall clock vs `timeout` | safe state |

**Conformance test sketch.**
- **C1 — nominal support + stability.** Place a bench object onto the support; command `grasp.platform(target, load_budget = 10 N)`. PASS iff `result == success` ∧ a **level, gentle** static-stability hold test retains the object ∧ borne weight detected ∧ measured CoM projection inside the support polygon ∧ the result declares balance-only stability.
- **C2 — unstable-placement rejection.** Present an object whose CoM cannot be centered over the support polygon (e.g. too large / off-balance). PASS iff `result == unstable_placement` ∧ weight not taken ∧ safe state.

#### 2.7 `grasp.pin`

**Intent.** Pin a target object against an external supporting surface with a single effector contact, using the environment surface as the opposing element (extrinsic force closure). Stabilizes an object too large or awkward for the embodiment's own opposition, or holds it for an in-surface regrasp. The pinned object is surface-bound: it cannot be freely transported.

**Parameters.** (Extrinsic deltas marked **†**.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target` | `ObjectTarget` | — (required) | — | object to pin; via `LetBind` |
| `against_surface` **†** | `SurfaceTarget` | — (required) | — | external surface to pin against (point + outward normal); reuses the `reach` `SurfaceTarget` type |
| `force_budget` | `Force` | — (required) | N | `> 0`; normal force pressing object onto the surface; clamped to `grip_force_max` and `target.max_contact_force` |
| `pin_pose` | `Pose6D \| auto` | `auto` | — | effector contact pose on the object's exposed face; `auto` = opposite `against_surface` |
| `controlled_frame` | `FrameRef` | `embodiment.default_grasp_frame` | — | declared contact frame |
| `tactile_target` | `TactileTarget \| auto` | `auto` | — | `auto` = effector contact **and** confirmed reaction force from the surface |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- *Capability:* `embodiment` declares `pin_grasp` (broadly supported — a single force-controlled contact suffices); `tactile_sensing` / force sensing preferred (else proxy).
- `target` and `against_surface` both resolvable in a `calibration_valid` frame; the object lies between the effector approach and the surface.
- The derived `pin_pose` admits ≥ 1 IK solution on the object face opposite the surface.
- `controlled_frame` free.

**Postconditions (on `success`).**
- `object_held(target, mode = pin, closure = force, extrinsic = true, surface_bound = true, stable_directions = {against_surface.normal})`: object pinned against the surface by normal force.
- **Surface-bound (declared):** the grasp is valid only while `against_surface` is present and the effector maintains force; the object **cannot be freely transported** (doing so loses the surface). In-surface `slide` / `regrasp` are the permitted successors. In-plane retention is friction-limited.
- Effector contact and surface reaction confirmed (or proxy); `controlled_frame` is committed; `end_effector_free = false`.

**Safety envelope (holds throughout execution).**
- `normal_force ≤ force_budget · (1 + transient_margin)`; `≤ target.max_contact_force` if declared (do not crush the object against the surface).
- Surface-reaction monitoring: a force drop or unexpected object motion indicates the surface yielded / was not rigid — treat as loss of pin.
- `min_clearance(controlled_frame, static_model \ {target, against_surface}) ≥ clearance` during approach.
- On breach: the object is held only by the pin; the safe response is to maintain the pin if feasible, else a controlled lowering — not an abrupt release that drops the object.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `capability_absent` | `pin_grasp` not declared | reject; no attempt |
| `target_underdetermined` / `surface_underdetermined` **†** | `target` or `against_surface` unresolved | reject; no attempt |
| `unreachable` | IK on `pin_pose` | no contact |
| `no_contact_confirmation` | effector contacted but no surface reaction (object not between effector and surface) | withdraw to safe state; result ≠ `success` |
| `surface_yielded` **†** | reaction force drops / surface moves under load | release pin to safe state; result ≠ `success` |
| `force_exceeded` / `crush_abort` | force over budget / deformation | abort; controlled release |
| `timeout` | wall clock vs `timeout` | safe state |

**Conformance test sketch.**
- **C1 — nominal pin + press hold.** Pin a bench object against a rigid fixtured surface; command `grasp.pin(target, against_surface, force_budget = 8 N)`. PASS iff `result == success` ∧ a hold test pressing **toward the surface** retains the pin ∧ surface reaction force confirmed ∧ peak normal force within budget ∧ the result declares `surface_bound = true`.
- **C2 — surface-yield detection.** Use a deliberately compliant / movable `against_surface`. PASS iff `result == surface_yielded` (not falsely reported `success`) ∧ controlled release to a safe state.

#### 2.8 `grasp.envelope`

**Intent.** Enclose a target with a compliant or caging grasp — a soft-body effector conforming to the object's shape, or a loose cage that traps the object without rigid fixation — prioritizing gentleness and robustness to geometric uncertainty over rigid stability. For fragile, irregular, or imprecisely localized objects on pneumatic / underactuated embodiments.

**Parameters.** (Envelope deltas marked **†**.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target` | `ObjectTarget` | — (required) | — | tolerates higher pose / geometry uncertainty; via `LetBind` |
| `force_budget` | `Force` | — (required) | N | `> 0`; gentle distributed force (typically below power); clamped to `grip_force_max`, `target.max_contact_force` |
| `mode` **†** | `{conform, cage}` | `conform` | — | `conform` = compliant contact following the object shape; `cage` = geometric trap with clearance |
| `cage_clearance` **†** | `Length \| auto` | `auto` | mm | (`cage` mode) allowed residual object mobility inside the enclosure |
| `grasp_pose` | `Pose6D \| auto` | `auto` | — | enclosure pose; the grasp tolerates pose uncertainty |
| `controlled_frame` | `FrameRef` | `embodiment.default_grasp_frame` | — | declared grasp frame |
| `tactile_target` | `TactileTarget \| auto` | `auto` | — | `conform`: distributed gentle contact; `cage`: enclosure closed around object |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- *Capability:* `embodiment` declares `envelope_grasp` (compliant / underactuated / caging effector); `tactile_sensing` preferred (else proxy).
- `target.estimated_mass ≤ embodiment.limits.payload_grasp_envelope`; size within the enclosure range.
- For `cage` mode: the enclosure geometry can trap the object (the object's escape dimensions exceed the cage openings).
- **Relaxed uncertainty precondition †:** the `grasp_pose` uncertainty bound is looser than for `grasp.power` — tolerating geometric uncertainty is this primitive's purpose.
- `controlled_frame` free.

**Postconditions (on `success`).**
- `conform`: `object_held(target, mode = envelope_conform, closure = form, compliant = true, stable_directions = omnidirectional)`: conformed enclosure, gentle distributed contact.
- `cage`: `object_held(target, mode = envelope_cage, closure = form, residual_mobility = cage_clearance)`: trapped, cannot escape, but may move within `cage_clearance` **†**.
- **Uncertainty-robust (declared):** the grasp succeeded under the relaxed geometric-uncertainty bound — recorded for capability negotiation (uncertain-geometry tasks prefer envelope).
- Gentle contact suited to fragile objects; `controlled_frame` owns `target`; `end_effector_free = false`.

**Safety envelope (holds throughout execution).**
- `contact_force ≤ force_budget · (1 + transient_margin)`, distributed; **crush protection is primary** (envelope targets fragile objects) — abort on `target` fragility / deformation-rate breach.
- `cage` mode: the object retains `residual_mobility`; this is recorded so `transport` accounts for in-cage shifting.
- `closing_velocity ≤ embodiment.limits.v_grasp`; `min_clearance(controlled_frame, static_model \ target) ≥ clearance` during approach.
- On any breach: open the enclosure / release gently to a safe state within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `capability_absent` | `envelope_grasp` not declared | reject; no attempt |
| `object_escapes_cage` **†** | (`cage`) escape dimension < cage opening | result ≠ `success`; open to safe state |
| `incomplete_conform` **†** | (`conform`) contact not established over sufficient area | open to safe state; result ≠ `success` |
| `object_too_large` | size exceeds enclosure range | reject; no attempt |
| `crush_abort` | fragility / deformation-rate exceeded | abort + gentle release |
| `timeout` | wall clock vs `timeout` | open to safe state |

**Conformance test sketch.**
- **C1 — nominal cage + escape resistance.** Cage a bench-fixtured irregular object; command `grasp.envelope(target, mode = cage, force_budget = 3 N)`. PASS iff `result == success` ∧ a perturbation (shake) test confirms the object **cannot escape** the enclosure ∧ measured residual mobility `≤ cage_clearance` ∧ contact force gentle (within budget).
- **C2 — uncertainty robustness.** Command `grasp.envelope` with a deliberately perturbed (uncertain) `target` pose within the relaxed bound. PASS iff `result == success` despite the pose error (the robustness property is the verifiable claim), OR a clean failure if the error exceeds the relaxed bound — never a false `success` with no actual enclosure.

#### 2.9 `grasp.adjust`

**Intent.** Modify an already-established grasp in place — change grip force, re-center contacts, or recover from incipient slip — without releasing the object and without changing the grasp's identity (mode, closure, contact topology). A held → held operation that preserves grasp continuity throughout.

**Parameters.** (Held → held deltas marked **†**.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` **†** | `GraspRef \| active` | `active` | — | the established grasp to modify; `active` = the current grasp on `controlled_frame`; explicit handle via `LetBind` from the originating `grasp.*` result |
| `new_force_budget` | `Force \| auto` | `auto` | N | adjusted grip force; `auto` = re-derive from feedback; clamped to `grip_force_max`, `target.max_contact_force` |
| `reason` **†** | `{slip_recovery, force_adapt, recenter, manual}` | `manual` | — | adjustment intent; informs strategy |
| `recenter` | `bool` | `false` | — | re-center contacts on the object |
| `preserve_pose` | `bool` | `true` | — | keep object pose fixed during the adjustment |
| `position_tolerance` | `Length` | `2` | mm | allowed object pose drift during adjust |
| `tactile_target` | `TactileTarget \| auto` | `auto` | — | target contact state after adjustment |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- **†** An object is currently held on `controlled_frame` (`object_held = true`, `end_effector_free = false`) — `grasp.adjust` requires an established grasp; there is nothing to adjust otherwise.
- The requested adjustment is within the grasp's capability (`new_force_budget` within limits; `recenter` feasible for the grasp mode).
- If `preserve_pose`: the adjustment does not require moving the object.

**Postconditions (on `success`).**
- The grasp is modified (new force and/or re-centered contacts); **grasp identity is preserved** (same `mode`, `closure`, contact topology).
- `object_held` remains `true` with updated parameters; the originating grasp's stability metadata (`closure`, `stable_directions`, …) is unchanged.
- If `preserve_pose`: object pose unchanged within `position_tolerance`.
- Updated tactile / force state confirmed.

**Safety envelope (holds throughout execution).**
- **Grasp continuity †:** the holding force never drops below the minimum required to retain the object (`min_holding_force`, derived from `target` mass, grasp mode, friction) — the grasp is never momentarily released. This is the defining invariant of a held → held primitive.
- `grip_force ≤ new_force_budget · (1 + transient_margin)`; `≤ target.max_contact_force`; crush protection per fragility.
- If `preserve_pose`: object pose drift `≤ position_tolerance` throughout.
- On any breach: revert to the last known stable grasp state (adjust is reversible) within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` **†** | no object held on `controlled_frame` | reject; no attempt |
| `adjustment_infeasible` | requested force / recenter exceeds capability | reject; prior grasp preserved unchanged |
| `grasp_lost` **†** | object slips / drops during adjustment | safe state; report (this is the failure adjust exists to prevent) |
| `pose_drift` | `preserve_pose` and drift > `position_tolerance` | revert to prior stable grasp |
| `crush_abort` | new force / deformation exceeds limit | abort; revert to prior force |
| `timeout` | wall clock vs `timeout` | revert to prior stable grasp |

**Conformance test sketch.**
- **C1 — force adjust + continuity.** Establish a grasp, then command `grasp.adjust(new_force_budget = higher)`. PASS iff `result == success` ∧ measured grip force changed to the new budget ∧ the force trace shows holding force **never dropped below `min_holding_force`** (continuity — object never released) ∧ object pose preserved within tolerance.
- **C2 — slip-recovery.** Induce a calibrated incipient slip, then command `grasp.adjust(reason = slip_recovery)`. PASS iff `result == success` ∧ slip arrested ∧ object retained ∧ grasp identity (mode/closure) unchanged.

#### 2.10 `grasp.release`

**Intent.** Release a held object by opening the contact and withdrawing the effector clear of the object envelope, returning the controlled frame to a free state — provided the object is in a stable resting state (or release is explicitly permitted to drop it). Completes the grasp lifecycle (held → free).

**Parameters.** (Held → free deltas marked **†**.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp to release; `active` = current grasp on `controlled_frame` |
| `controlled_frame` | `FrameRef` | `embodiment.default_grasp_frame` | — | declared grasp frame |
| `withdraw_axis` | `SignedAxis` | `−embodiment.default_tool_axis` | — | direction to withdraw after opening (reuses `reach.retract` convention) |
| `clearance_distance` | `Length \| auto` | `auto` | mm | distance to withdraw clear of the object envelope; `auto` from object geometry |
| `allow_drop` **†** | `bool` | `false` | — | permit release when the object is not stably supported |
| `confirm_release` | `bool` | `true` | — | verify no residual contact after opening |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- An object is currently held on `controlled_frame` (held → free; nothing to release otherwise).
- **†** Unless `allow_drop`: the object is in a stable resting state (supported such that it will not fall when released). If unsupported and `allow_drop = false`, the grasp is **not** opened.
- The withdraw corridor along `withdraw_axis`, inflated by clearance, is collision-free against the static model and the now-free object.

**Postconditions (on `success`).**
- The grasp is opened (grip force → 0) and the effector has withdrawn clear of the object envelope (`clearance_distance` achieved, no residual contact).
- `object_held = false`; the active grasp on `controlled_frame` is cleared; `end_effector_free = true`.
- The object's pose is unchanged from before release (release does not move the object — that is `transport` / `place`), modulo settling if it was just placed.
- If `confirm_release`: no-contact confirmed (object not adhered to the effector).

**Safety envelope (holds throughout execution).**
- Opening rate bounded so the object is not flung by rapid release; `‖cartesian_velocity(controlled_frame)‖ ≤ embodiment.limits.v_cartesian_max` during withdraw.
- Withdraw respects `min_clearance(controlled_frame, static_model ∪ {released object}) ≥ clearance` — do not knock the freed object.
- Directional force monotonicity during withdraw (per `reach.retract`): a force increase signals a withdraw collision → halt.
- On any breach (e.g. blocked withdraw): **maintain the grasp** rather than releasing into an unsafe state; return to the last stable held state.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | no object held on `controlled_frame` | reject; no attempt |
| `unsupported_object` **†** | object not stably supported and `allow_drop = false` | grasp maintained; object **not** dropped; result ≠ `success` |
| `withdraw_blocked` | withdraw corridor obstructed | maintain grasp; report |
| `release_not_confirmed` | opened but residual contact (adhesion / stuck) | report; object did not cleanly release |
| `timeout` | wall clock vs `timeout` | maintain grasp; safe state |

**Conformance test sketch.**
- **C1 — nominal release + clear.** Hold a bench object resting on a surface; command `grasp.release`. PASS iff `result == success` ∧ grip force → 0 ∧ the effector withdrew clear (no residual contact) ∧ object pose unchanged (not dragged / knocked) ∧ `end_effector_free = true`.
- **C2 — unsupported rejection.** Hold an object in mid-air (unsupported); command `grasp.release(allow_drop = false)`. PASS iff `result == unsupported_object` ∧ the grasp is maintained (object **not** dropped) ∧ safe state.

## Open issues for v0.1 freeze

- [ ] Final primitive list per category — pending external red-team review (see `CONTRIBUTING.md` spec-change discipline) across structurally distinct partners (tendon-driven / direct-drive / pneumatic at minimum)
- [ ] Predicate language scope: closed enumeration vs. extensible vocabulary — current draft permits both via `user_defined`, but tradeoff needs explicit decision before v0.1 freeze
- [ ] Type system formality: gradually typed JSON schema vs. fully typed algebraic specification — currently gradually typed, with type assertions in the JSON schema
- [ ] Reactive operator's `until` semantics under concurrent envelope violation (subsection forthcoming)
- [ ] Determinism boundary: which Skill ISA constructs are guaranteed deterministic at the algebra level vs. which inherit non-determinism from the Translation Layer's planner

## Cross-reference

- Translation Layer responsibility for resolving primitives into canonical actions: `02-translation-layer.md`
- Driver-side compliance test for primitives: `05-conformance.md` § Class 4 (end-to-end execution)
- Extension primitive registration rules: `06-extension-registry.md`
