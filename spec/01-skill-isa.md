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
| 1.4 | `reach.retract` | Move away from current pose along the negative approach direction |
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
