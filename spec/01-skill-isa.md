# Skill ISA — Specification (v0.1)

> **Status**: per-primitive semantic design complete for all 50 primitives across all seven categories (2026-05-30). Each primitive carries the full seven-field rubric (intent, parameters, preconditions, postconditions, safety envelope, failure modes, conformance-test sketch); the Skill ISA type system and grasp state model are specified. Remaining before freeze: the cross-cutting items in § Open issues (owned by `02`–`05`), the full JSON Schema, and the normative algebra/predicate formalization. Quantitative bounds are throughout expressed relative to embodiment-declared descriptor fields (Principle 1).

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
                                  "else" "(" Composition ")"
                                  [ "," "unknown" "(" Composition ")" ] ")"

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

1. **`Reactive` until-predicate semantics** are specified in § Predicates, verdicts, and three-valued control flow: the `until` predicate terminates the body only on a confident `true`, and a concurrent envelope violation takes precedence over a clean `until` completion (deterministically).
2. **`LetBind`** is included to allow a planner-derived pose (e.g., the result of `sense.locate`) to flow into subsequent primitives without re-derivation. This is what enables a single Skill ISA file to express "find the cable end, then grasp it, then insert" as a single composition. It is also the canonical way a perception-derived `SurfaceTarget` (point + outward normal) reaches `reach.approach`: `let (t := sense.locate(...)) in (reach.approach(target: &t, standoff: 50mm))`. RFL does not define how `t` is perceived — only how it flows once resolved.
3. **`Predicate` extensibility via `user_defined`** is the escape valve: domain-specific predicates can ship as extensions without modifying the core grammar.
4. **`Auto` value** lets the spec author defer choice to the Translation Layer's planner (e.g., a `grasp_pose: auto` parameter delegates pose selection).

### Predicates, verdicts, and three-valued control flow

The `Predicate` of the grammar and the `StatePredicate` of `sense.verify` are **one type**. A `Predicate` evaluates to a `Verdict`, and the same `Verdict` drives `branch`, `reactive`, and a `force.scrub` `state_change` completion.

**`Verdict`.** Evaluating a `Predicate` yields:

```
Verdict := {
  value:      {true, false, indeterminate},
  confidence: Ratio,
  evidence:   the observations that supported the verdict,
}
```

`value` is `true` / `false` only when `confidence ≥ confidence_threshold`; otherwise it is `indeterminate` — RFL does not assert a predicate it cannot support (honesty over a forced boolean). `Measurement.predicate_result` is a `Verdict`.

**Two evaluation paths.** A predicate over the embodiment's own state — `pose_reached`, `force_exceeds`, `elapsed` — evaluates directly from internal state (confidence ≈ 1). A predicate over external state — `object_present`, `tactile_contact`, `user_defined` — is evaluated by `sense.verify`, the general perception-backed evaluator; the grammar's specific predicates are its specializations and `user_defined` is the extension hook.

**Three-valued control flow.** `indeterminate` is never silently coerced to `true` or `false`:

- `branch(pred, then, else [, unknown])` takes an optional `unknown` arm taken on `indeterminate`. Without it, `indeterminate` **escalates** (raises to the caller) rather than guessing `then` or `else`.
- `reactive(body, until(pred))` terminates the body only on a confident `true`; `false` and `indeterminate` continue it (the body's own envelope and timeout are the backstop). The reactive never completes cleanly on an unsupported predicate.

**Envelope precedence.** When, in the same evaluation step, the body's safety envelope is violated and the `until` predicate reads `true`, the **safety-abort takes precedence**: the `reactive` terminates by abort, not by clean completion. This is deterministic — an identical trace yields an identical termination kind — and resolves the BNF note on concurrent envelope violation.

**Verdict honesty and evidence audit.** A `Verdict` always carries its `evidence`. Safety-critical verifications (e.g. "is the fastener torqued?") persist that evidence so the L4 Certification and L8 Insurance loops can audit *what* an assertion rested on — the same traceability requirement as `in_hand.flip`'s `momentary_release`. RFL owns the production of the evidence-bearing verdict; the audit loops that consume the persisted evidence are owned by `05-conformance.md` / the certification-insurance loops.

## Skill ISA type system

**Scope.** This section defines the value types referenced across all primitive specifications. Types whose values originate from perception (`SurfaceTarget`, `ObjectTarget`, `FeatureRef`, `ScanRegion`) are **consumed** by RFL but **produced** outside it: RFL fixes the type (fields + uncertainty bound), not the estimation method (layer discipline, Principle 4). Such values reach a primitive via `LetBind` (typically a `sense.*` result).

### Dimensioned scalars

| Type | Canonical unit (wire) | Notes |
|---|---|---|
| `Length` | m | parameter tables may author in mm; the value is SI |
| `Angle` | rad | geodesic where applied to SO(3) |
| `Force` | N | |
| `Torque` | N·m | force × length (rotational) |
| `Velocity` | m/s | |
| `Acceleration` | m/s² | |
| `AngularVelocity` | rad/s | |
| `Frequency` | Hz | 1 / s (oscillation rate) |
| `Duration` | s | |
| `Ratio` | dimensionless | `[0,1]` unless stated |

> **Canonical-unit rule.** All wire / interchange values are SI (m, rad, N, s). The "Units" column in primitive parameter tables is an authoring convenience; conformance and `retarget` operate on SI values. This is binding for `retarget` byte-for-byte determinism (`02-translation-layer.md`).

### Geometric types

| Type | Definition | Constraint |
|---|---|---|
| `FrameRef` | reference to a calibrated frame on the kinematic tree or in the world | must be `calibration_valid` when used |
| `Axis` | one of `{x, y, z}` of a named frame | — |
| `SignedAxis` | `±` an `Axis` | direction-bearing |
| `Direction` | unit vector in a named frame | `‖·‖ = 1` |
| `Pose6D` | rigid pose (position + orientation) in a `FrameRef` | representation (SE(3) / quat+t / axis-angle) deferred to `02`; MUST admit a single-scalar geodesic orientation error |

> **Frame-field resolution.** The `embodiment.default_*_frame` defaults and `embodiment.default_tool_axis` that primitive parameter tables reference are defined in `03-driver-interface.md` § Embodiment frame model. In particular `embodiment.default_tool_axis` is per control frame — it resolves to the `tool_axis` of the currently resolved `controlled_frame`, not an embodiment-global constant.

> **`EffectorRef` resolution.** `EffectorRef` (the type of `transport.handoff`'s `receiver`) addresses a control frame on the same embodiment (a bare frame name, the `FrameRef` case) or another (`embodiment_id:frame_name`); its resolution semantics and the coordination-channel requirement for the remote case are defined in `03-driver-interface.md` § Multi-embodiment addressing.

### Target types (perception-derived; carry uncertainty)

| Type | Fields | Used by |
|---|---|---|
| `SurfaceTarget` | `point` (a `Pose6D` position), `normal: Direction` (outward), `frame: FrameRef`, `uncertainty: UncertaintyBound` | `reach.approach`, `reach.hover`, `grasp.pin.against_surface` |
| `ObjectTarget` | `pose: Pose6D`, `geometry: GeometryRef`, `estimated_mass: Force`, `center_of_mass` (a `Pose6D` position), `max_contact_force: Force?` (fragility), `features: set<FeatureRef>`, `uncertainty: UncertaintyBound` | all `grasp.*`, `in_hand.*`, `transport.*`, `place.*` |
| `FeatureRef` | `parent: ObjectTarget`, `kind: {handle, loop, bar, edge, face, …}`; resolves to a `SurfaceTarget` or sub-geometry | `grasp.hook.hook_feature`, `grasp.lateral.grasp_edge` |
| `ScanRegion` | a `Region` (below) specialized with `kind: {volume, surface, path}` for sweep coverage | `reach.scan` |
| `UncertaintyBound` | scalar or covariance bounding the pose / geometry estimate error | every target type; gates `*_underdetermined` |

> `estimated_mass` is typed `Force` (weight under standard gravity) for unit consistency with force budgets; a future extension may separate mass (kg) from weight if needed.

### Tactile + grasp reference types

| Type | Definition | Used by |
|---|---|---|
| `TactileTarget` | contact criterion expressed in TactileManifold terms (sites, feature thresholds); see `04-tactile-manifold.md` | all `grasp.*` |
| `GraspRef` | handle to an established grasp on a `controlled_frame`; `active` resolves to the current grasp | `grasp.adjust`, `grasp.release`, `in_hand.*`, `transport.*`, `place.*` |

### Spatial, motion, and structural types

| Type | Definition | Used by |
|---|---|---|
| `Region` | a spatial extent — `geometry` in a `frame: FrameRef`. The base type; `ScanRegion` is a `Region` plus a sweep `kind` | `in_hand.flip` (`catch_envelope`, `safe_drop_zone`), `sense.locate` (`search_region`) |
| `Trajectory` | an ordered path of a `Pose6D` — waypoints or a spline, plus timing, in a `frame`. Shares the ordered-pose-sequence backbone with `reach.scan`'s generated sweep set `Σ`, but a `Trajectory` is caller-specified whereas `Σ` is generated by `02` | `transport.follow_trajectory`, `force.wipe` (`wipe_path`), `force.cut` (`cut_path`) |
| `MoveSpec` | a sum type `to_pose(Pose6D) \| trajectory(Trajectory)` — lets a primitive wrap either relocation form | `transport.carry` (`motion`) |
| `OrientationSpec` | a set of `(object_axis → Direction)` constraints (e.g. `label_normal → up`); functional labels are converted to geometry by the caller. Related to `reach.align`'s axis-by-axis constraint | `place.orient` (`required_orientation`) |
| `ContactConfig` | a contact configuration — which effector regions touch which object surfaces; `auto` = planner-derived | `in_hand.regrasp` (`target_contacts`), `grasp.*` |

### Object reference, geometry, and measurement

| Type | Definition | Used by |
|---|---|---|
| `ObjectRef` | an object *identity* whose pose is not yet known — the precursor to a pose-resolved `ObjectTarget`. The resolution flow is `ObjectRef → sense.locate → Measurement(pose) → ObjectTarget` | `sense.locate` (`target_ref`) |
| `GeometryRef` | a reference to an object's geometric model (perception-derived; RFL fixes the queryable aspects, not the estimation). Exposes a **rolling surface** aspect (a rollable surface — cylinder / sphere / cone section — and its rolling axis) for `in_hand.roll`, and a **container** aspect (opening + interior envelope) for `place.insert_loose` (see § World-state model) | `ObjectTarget.geometry`; `in_hand.roll`; `place.insert_loose` |
| `Measurement` | the perception-derived **output** type of the `sense` category — the dual of the input target types. Optional fields `presence: bool`, `location` (a `Pose6D` position), `normal: Direction`, `stiffness`, `mass: Force`, `center_of_mass` (a `Pose6D` position), `pose: Pose6D`, `predicate_result: Verdict` (the three-valued `sense.verify` outcome, defined with the algebra), plus an `UncertaintyBound`. Flows to later primitives via `LetBind`; `mass` / `center_of_mass` / `pose` populate the matching `ObjectTarget` fields (the observe→target→manipulate loop) | all `sense.*` (output) |

### Stop / completion conditions

Seven primitives across `force` and `in_hand` take a stop / completion condition, and each had its own sum type (`SlideStop`, `SeatingSpec`, `PullStop`, `ScrewStop`, `ActuationSpec`, `CutStop`, `ScrubStop`). These share structure: a completion is reached when the operation hits a progress value, an effort level, an effort discontinuity, a reference, a count, a time, or a sensed predicate. They are unified into one `StopCondition` family; each named type is a restriction of it.

```
StopCondition :=
  | reached(Progress)            # progress hit a value: distance / depth / turns / advance
  | effort_rise(Effort)          # resisting effort reached a level: force / tension / torque
  | effort_drop                  # a sudden resistance drop event: breakaway / separation / disengagement / torque_drop
  | detent                       # a rise-then-drop effort signature (a click)
  | count(n)                     # a repetition count: turns / passes
  | elapsed(Duration)            # elapsed time
  | landmark(Ref)                # a declared reference reached: tactile_landmark / external_reference / path_complete
  | predicate(StatePredicate)    # a sense.verify predicate became true (the force↔sense coupling point)
  | all_of(set<StopCondition>)   # conjunction: force_and_depth, torque_and_advance
  | any_of(set<StopCondition>)   # disjunction
```

- `Progress` is the operation's progress quantity — `Length` (linear), `Angle` (rotational), or a depth — and `Effort` its resisting quantity — `Force`, `Torque`, or tension (a `Force`). The family abstracts over which quantity an operation uses; a given primitive's `StopCondition` is typed to its own progress and effort axes.
- `effort_drop` and `detent` denote *kinds* of completion event; their physical detection (a force-derivative drop for breakaway / separation, a rise-then-drop signature for a detent) is owned by `04-tactile-manifold.md`. The type fixes the event class; the manifold fixes how it is sensed.
- `predicate(StatePredicate)` ends the operation when a `sense.verify` predicate holds — the point where the `force` and `sense` categories couple. `StatePredicate` itself is unified with the algebra `Predicate` in a separate `01` item; here `StopCondition` only references it.

The seven named types are restrictions of the family (the per-primitive parameter spellings are unchanged; aligning the prose to the family is a pre-freeze formatting pass):

| Named type | Restriction |
|---|---|
| `SlideStop` (`in_hand.slide`) | `reached(distance) \| landmark(tactile) \| landmark(external)` |
| `force.insert_fit` `stop_condition` (was `SeatingSpec`) | `effort_rise(force) \| reached(depth) \| all_of{effort_rise(force), reached(depth)}` |
| `PullStop` (`force.pull`) | `reached(distance) \| effort_drop \| effort_rise(tension)` |
| `ScrewStop` (`force.screw` / `unscrew`) | `effort_rise(torque) \| count(turns) \| all_of{effort_rise(torque), reached(advance)}`; `unscrew` adds `effort_drop` (disengagement / torque_drop) |
| `ActuationSpec` (`force.press_button` / `snap_engage`) | `detent \| effort_rise(force_threshold)` |
| `CutStop` (`force.cut`) | `landmark(path_complete) \| effort_drop \| reached(depth)` |
| `ScrubStop` (`force.scrub`) | `elapsed(duration) \| count(passes) \| predicate(state_change)` |

**Force-at-state outcome rule.** The `all_of` conjunction is what distinguishes a genuine completion from a fault: `all_of{effort_rise(F), reached(depth d)}` means the effort rose *at* the expected depth — **seated**. The same effort rise with `reached(depth)` *unmet* is a **jam**, not a success. A bare effort threshold is never sufficient (`force.insert_fit` seating/jam; `force.screw` torque-at-advance). The conjunction makes this discrimination part of the type, not per-primitive prose.

`StopCondition` evaluation is deterministic: an identical trace yields an identical completion verdict (binding for `retarget` determinism and `05` fixture reproducibility); event variants (`effort_drop`, `detent`) evaluate from declared thresholds / signatures per the `04` feature definitions. How `retarget` encodes the monitored condition into the canonical action is owned by `02-translation-layer.md`.

### The `auto` value

Any parameter typed `T | auto` may take the literal `auto`, deferring the value to the Translation Layer's planner (per the BNF `Auto` production). `auto` resolution MUST be deterministic given identical inputs.

## World-state model

Where the grasp state model (§ Grasp state model and stability metadata) tracks what an effector holds, the **world-state model** tracks how objects rest on and contain one another. `place.*` writes these relations; the stability predicates and later operations (removing an object without toppling the stack it supports) read them. It is the structural basis that makes "is this safe to release?" mechanically decidable.

**Scope.** RFL owns the *relations* and the *predicates* over them. The geometry they range over — a center of mass, a contact polygon, a container envelope — is perception-derived (`ObjectTarget.center_of_mass`, `GeometryRef`), produced outside RFL (layer discipline, Principle 4). The world-state model records relations and decides predicates; it does not estimate geometry.

### Support and containment relations

The model tracks two relations between objects (and surfaces):

- **support** — "object A rests on object B (or surface S)". Written by `place.put_down` (rests-on-surface) and `place.stack` (rests-on-object); the basis for recursive stack stability.
- **containment** — "object A is contained in container C". Written by `place.insert_loose`.

A relation persists in the world-state until a later operation changes it (a subsequent grasp lifts A, clearing its support relation).

### Supported-state predicate

`supported(object)` holds when the object's center of mass projects strictly inside its **contact polygon** — the polygon of contact with whatever it rests on — by a stability margin. This is the `grasp.platform` CoM-over-polygon test (§ Grasp-mode capabilities, `03`) applied to a *resting* object rather than a platform grasp. It gates `grasp.release` and `place.*`: before opening the grasp, the primitive confirms `supported(object)`; if it fails and `require_stable` is set, the grasp is not released (the object is never abandoned in a pose from which it will fall).

### Recursive stack-stability predicate

`stack_stable(object placed on a stack)` holds when, using the support relations above to enumerate the levels:

1. the object's CoM projects inside the supporting level's top-face polygon;
2. at every level down, that level's CoM projects inside the level-below polygon; and
3. the augmented stack's combined CoM projects inside the base support polygon.

`place.stack` confirms `stack_stable` before release — the supported-state predicate applied recursively over the tracked support chain, not just the top object's local rest. The support-relation tracking is what makes the recursion decidable, and what lets a later removal reason about which levels it disturbs.

### Containment predicate and container geometry

`GeometryRef` exposes a **container aspect** — an opening and an interior envelope (the addition deferred from the type system). `contained(object, container)` holds when the object lies fully within the container's interior envelope — not protruding through the opening, not jammed. `place.insert_loose` confirms `contained` before release.

The container aspect also fixes the **`insert_loose` vs. `insert_fit` selection criterion**: when the opening clearance exceeds the object's insertion cross-section, the insertion is a geometric drop-in (`place.insert_loose`, clearance, world-state); otherwise it is a tolerance fit (`force.insert_fit`, force-controlled). The planner selects the primitive by this criterion.

### Shared break-contact clause

Three primitives leave a contact rather than form one — `reach.retract`, `grasp.release`, `place.put_down`. They share one reusable safety-envelope clause, **`break_contact`**, defined once here rather than per primitive:

- **Postcondition** — no task contact remains; `external_force ≈ 0`.
- **Directional force monotonicity** — external force along the retreat / withdraw direction is non-increasing beyond a noise margin; an *increase* signals an obstacle behind the moving frame and triggers `contact_response` (a force *opposing* the retreat — the surface being left — is expected to decay and is not a violation).

## Composition validity

A composition is more than a sequence of valid primitives: a primitive's preconditions must hold *given the state the preceding primitives left*. The checks that enforce this are stated per primitive (each `*_inadmissible`, `no_active_grasp`, `orientation_unreachable` failure mode); this section collects them as one mechanically-checkable layer the algebra and planner apply at composition time, before execution. The rules live here; the full conformance tables (the lifecycle transition table, the stability-class → permitted-successor table) are owned by `05-conformance.md`.

### DOF-admissibility

An `in_hand` operation that moves a degree of freedom is admissible only if that DOF is `friction_held` (movable) in the grasp's `secured_dof` (§ Grasp state model) — never `form_held` or `rotation_constrained`. This single rule has one instance per primitive: `rotation_inadmissible`, `translation_inadmissible`, `roll_inadmissible`, `pivot_inadmissible`, `slide_inadmissible`. The same stability metadata gates transport: a `surface_bound` (`pin`) grasp is not freely transportable (`transport_inadmissible`).

### Lifecycle transitions

A primitive requiring a `held` input rejects a `free` frame (`no_active_grasp`); a `surface_bound` grasp rejects a free-transport successor (`transport_inadmissible`). These are the mechanically-checkable transition guards; the full transition table and the stability-class → permitted-successor table are `05`'s.

### `GraspRef` supersession and dangling-reference prevention

`in_hand.regrasp` and `transport.handoff` produce a **new** `GraspState` and supersede the originating `GraspRef` (the new grasp's `mode` / `closure` / topology differ, or ownership moves to another effector). A `GraspRef` bound by a `LetBind` is **invalidated** the moment it is superseded; using the stale handle in a later primitive is a composition error caught at validation, not a runtime surprise. This is the dangling-reference guard for grasp handles, the grasp-state counterpart of the `LetBind` flow checks below.

### Last-resort admissibility

`in_hand.flip` (the only continuity-suspending primitive) is admissible only when no continuity-preserving primitive — `in_hand.rotate`, `roll`, or `regrasp` — achieves the same reorientation (`continuity_alternative_exists` rejection). The "does a continuity-preserving alternative exist?" check is part of composition validity: the planner must establish that `flip` is genuinely the last resort before composing it.

### Reverse-dependency composition

When a primitive's precondition depends on an *earlier* reorientation, composition validity recognizes the "compose a prerequisite primitive first" pattern. `place.orient` rejects (`orientation_unreachable`) when the current grasp cannot present the required orientation and recommends a reorient-first; the planner composes an `in_hand.rotate` / `regrasp` *before* the placement so the placement's precondition holds. Each primitive keeps a single responsibility — `place.orient` never silently reorients — and the cross-category coupling is resolved by composition rather than by overloading a primitive.

### `LetBind` flow checks

A `LetBind` that carries a `sense` result into a downstream primitive is checked at composition time:

- **Uncertainty matching (precision honesty).** When a `sense.locate` pose flows into a primitive, the algebra checks `measured uncertainty ≤ that primitive's target-resolution bound`. A pose may never be stamped with an uncertainty better than was achieved; conformance forbids overstating precision.
- **Measured-value supply (the observe–act loop).** A `Measurement`'s `mass` / `center_of_mass` / `pose` fields populate the matching `ObjectTarget` fields via the binding, after which the downstream primitive's preconditions (e.g. `estimated_mass ≤ payload`) become checkable. This closes the observe → target-field → manipulate loop inside a composition — the primitive-level form of the white paper's L1 Data loop.

## Per-primitive semantic specification

> **Status**: complete — all 50 primitives across all seven categories carry v0.1 freeze-ready text (2026-05-30). The enumeration table is the index; this section is the normative semantics.
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
| `distance` | `Length` | — (required) | mm | `> 0`; retreat travel along `direction` |
| `controlled_frame` | `FrameRef` | `embodiment.default_control_frame` | — | declared control frame |
| `direction` | `Direction` | `−tool_axis` | — | retreat direction: a signed axis (e.g. `−tool_axis`, `−z`) or a unit vector; default the reverse of the controlled frame's tool axis |
| `frame` | `FrameRef` | `controlled_frame` | — | frame the axis is expressed in (default: move along own tool axis) |
| `break_contact` | `bool` | `true` | — | require any starting contact to be cleared |
| `position_tolerance` | `Length` | `2` | mm | `> 0` |
| `max_velocity` | `Velocity \| auto` | `auto` | m/s | clamped to `embodiment.limits.v_cartesian_max` |
| `clearance` | `Length` | `0` | mm | `≥ 0`; margin to the full static model (no target exclusion) |
| `contact_response` | `{abort, stop, comply}` | `abort` | — | reaction to a force *increase* during retreat |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `controlled_frame` is resolvable and `calibration_valid`.
- The retreat target pose (`start + distance · direction`) admits ≥ 1 IK solution.
- The retreat corridor, inflated by `clearance`, is collision-free against the static model in the retreat direction.
- Starting contact is permitted (no precondition forbidding it), unlike `reach.to_pose`/`reach.approach`.

**Postconditions (on `success`).**
- `controlled_frame` displaced by `distance` along `direction` from the start pose, within `position_tolerance`.
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

> **Capability-key normalization.** The capability a `grasp` primitive requires is keyed by its primitive identifier — `grasp.pinch`, `grasp.power`, etc. — under the capability manifest defined in `03-driver-interface.md` § Capability manifest. The top-level spellings used in these preconditions (`pinch_grasp`, `power_grasp`, …) denote those same dotted keys; aligning the prose to the dotted form is a pre-freeze formatting pass, not a semantic change.

All ten `grasp` primitives share a common **grasp core** (`target`, `force_budget`, `grasp_pose`, tactile confirmation, capability requirement, hold-test verification) and differ only in a **contact-pattern abstraction** (antipodal pair / whole-volume enclosure / hook / tripod / lateral clamp / support / extrinsic pin / compliant-or-caged enclosure). The two held-state operations (`grasp.adjust`, `grasp.release`) act on the active-grasp state defined next rather than forming a new contact.

##### Grasp state model and stability metadata

**Active-grasp state.** The Skill ISA maintains, per `controlled_frame`, a first-class **active-grasp state** that `grasp.*`, `in_hand.*`, `transport.*`, and `place.*` read and write. Earlier prose references to `object_held` / `end_effector_free` are shorthand for this state.

```
GraspState := {
  status:       {free, held, manipulated, placed},
  held_object:  ObjectTarget | None,
  mode:         GraspMode | None,    # pinch | power | hook | tripod | lateral
                                     # | platform | pin | envelope_conform | envelope_cage
  stability:    StabilityMetadata | None,
  force_budget: Force | None,
}
```

A `GraspRef` is a handle to a `GraspState`; the literal `active` resolves to the current `GraspState` of the addressed `controlled_frame`. An operation that produces a new `GraspState` — `in_hand.regrasp`, `transport.handoff` — **supersedes** the originating `GraspRef`: the old handle is invalidated and a stale use is a composition error (§ Composition validity, `GraspRef` supersession).

**Grasp lifecycle.**

```
   free ──grasp.*──▶ held ──in_hand.*──▶ manipulated ──┐
    ▲                 │  ▲                    │         │
    │                 │  └────────────────────┘         │
    │                 │     (returns to held)           │
    │                 ▼                                  │
    └──grasp.release──┴──place.*──▶ placed ──release──▶ free
```

| Primitive class | Transition |
|---|---|
| `grasp.{pinch, power, hook, …}` | `free → held` (sets `held_object`, `mode`, `stability`) |
| `grasp.adjust` | `held → held` (modifies parameters; preserves identity) |
| `in_hand.*` | `held → manipulated → held` (preserves grasp identity) |
| `transport.*` | `held → held` (object pose changes; grasp preserved) |
| `place.*` | `held → placed` |
| `grasp.release` | `held \| placed → free` (clears state) |

**Composition validity.** The algebra checks transitions: a primitive requiring `held` input rejects a `free` frame (`no_active_grasp`); a `surface_bound` grasp rejects a free-transport successor (see flags below). This is the mechanically checkable basis for composition conformance (`05-conformance.md`).

**Stability metadata.** Decomposed into orthogonal axes so grasp variety does not explode the type:

```
StabilityMetadata := {
  closure:           {force, form, support},
  secured_dof:       Map<DOF, {form_held, friction_held, balance_held}>,
  stable_directions: set<Direction> | omnidirectional,
  flags:             subset of {
                       extrinsic,            # opposed by an environment surface (pin)
                       surface_bound,        # invalid if surface lost; no free transport (pin)
                       compliant,            # soft / conforming contact (envelope_conform)
                       rotation_constrained, # resists torque about the grasp axis (tripod)
                     },
  residual_mobility: Length | None,          # caged object's in-enclosure freedom (envelope_cage)
  min_holding_force: Force,                  # below which the object drops; from mass / mode / friction
}
```

| Grasp mode | closure | securing summary | flags |
|---|---|---|---|
| pinch / power | force | friction_held (all DOF) | — |
| precision_tripod | force | friction_held + rotation about axis form-constrained | `rotation_constrained` |
| lateral | force | clamp-normal friction_held; in-plane friction_held (weaker) | — |
| hook | form | `stable_directions` form_held; reverse / lateral free | — |
| platform | support | balance_held over the CoM polygon | — |
| pin | force | clamp-normal friction_held | `extrinsic`, `surface_bound` |
| envelope_conform | form | distributed friction_held, gentle | `compliant` |
| envelope_cage | form | trapped, not fixed | `residual_mobility` set |

Downstream primitives consume this: `transport` reads `secured_dof` / `flags` to choose conservative acceleration; `in_hand.rotate` reads `rotation_constrained`; `grasp.release` reads `min_holding_force` and the supported-state predicate.

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
- `object_held(target, mode = pinch, closure = force, stable_directions = omnidirectional)`: the object is held in a stable two-opposing-point pinch (force closure), grip force `≤ force_budget`.
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
- `object_held(target, mode = power, closure = force, stable_directions = omnidirectional)`: enclosed grasp with distributed contact (force closure), grip force `≤ force_budget`.
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

### Category 3 — `in_hand`

`in_hand` primitives manipulate a held object **without releasing it** — the mirror image of `reach` (which moves an effector with no object). They drive the `held → manipulated → held` segment of the grasp lifecycle, preserving grasp identity (`mode`, `closure`, contact topology) across the operation. Two foundations from Category 2 are consumed throughout: the **active-grasp state** (every `in_hand` primitive reads a `GraspRef` and asserts `status = held`), and the **stability metadata** — an operation is admissible only if the DOF it moves is `friction_held` (movable), not `form_held` (geometrically fixed). The defining safety invariant is a strengthened **grasp continuity**: the object is never in an unsecured state, including across intermediate regrips (finger gaiting).

#### 3.1 `in_hand.rotate`

**Intent.** Reorient a held object about an axis, relative to the grasp frame, without releasing it — repositioning the object within the grasp while preserving grasp identity and stability class throughout.

**Parameters.** (Held → manipulated → held; in_hand core.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the established grasp to manipulate within |
| `axis` | `Direction` | — (required) | — | rotation axis, in the grasp frame |
| `angle` | `Angle` | — (required) | rad | signed rotation magnitude |
| `controlled_frame` | `FrameRef` | (from `grasp_handle`) | — | grasp frame |
| `orientation_tolerance` | `Angle` | `2` | deg | `> 0`; geodesic on the achieved reorientation |
| `keep_position` | `bool` | `true` | — | hold the object's in-grasp position fixed (rotate, not translate) |
| `position_tolerance` | `Length` | `2` | mm | drift bound while rotating (when `keep_position`) |
| `max_angular_velocity` | `AngularVelocity \| auto` | `auto` | rad/s | clamped to `embodiment.limits.w_inhand_max` |
| `regrip_policy` | `{gaiting, continuous, auto}` | `auto` | — | how the embodiment effects the reorientation (finger gaiting vs continuous) — a hint; resolved by the Translation Layer |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState` (`status = held`) — there is an object to manipulate.
- **The requested rotation is admissible under the grasp's stability metadata:** the rotation `axis` lies within the grasp's manipulable DOF — i.e. it is not a `form_held` DOF. A `rotation_constrained` grasp (e.g. tripod) **rejects** rotation about its constrained axis.
- The reorientation is within the embodiment's in-hand workspace (`embodiment.limits.inhand_rotation_range` for the grasp mode), possibly via intermediate regrips.
- `embodiment` declares `in_hand_manipulation` capability with rotation support.

**Postconditions (on `success`).**
- The held object's orientation, relative to the grasp frame, has changed by `angle` about `axis` within `orientation_tolerance`.
- **Grasp identity preserved:** `mode`, `closure`, and contact topology are unchanged from before; `GraspState` returns to `status = held`.
- If `keep_position`: the object's in-grasp position is unchanged within `position_tolerance`.
- The object remains held throughout (continuity); world-frame object pose reflects the in-grasp reorientation composed with the (stationary) grasp-frame pose.

**Safety envelope (holds throughout execution).**
- **Continuity (strengthened):** holding force never drops below `min_holding_force` AND the grasp's stability class (`closure`, `secured_dof`) is maintained throughout — including across intermediate regrips (gaiting), where each transient sub-grasp must itself satisfy `min_holding_force`. The object is never in an unsecured state.
- `‖object_angular_velocity (grasp frame)‖ ≤ min(max_angular_velocity, embodiment.limits.w_inhand_max)`.
- `grip_force ≤ force_budget · (1 + transient_margin)`; crush protection per `target` fragility.
- If `keep_position`: in-grasp position drift `≤ position_tolerance`.
- On any breach: arrest motion and revert to the last stable held configuration within `embodiment.limits.stop_time` (object retained — never dropped to "recover").

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt |
| `capability_absent` | no `in_hand_manipulation` rotation support | reject; no attempt |
| `rotation_inadmissible` | `axis` is a `form_held` / `rotation_constrained` DOF | reject; grasp preserved unchanged |
| `out_of_range` | `angle` exceeds in-hand rotation range even with regrips | partial rotation to limit, or reject; report achieved angle |
| `grasp_lost` | continuity breach (force or stability-class drop) mid-rotation | arrest; revert to last stable held; report |
| `orientation_not_reached` | post-motion angle vs tolerance | MUST NOT report `success` |
| `timeout` | wall clock vs `timeout` | revert to last stable held |

**Conformance test sketch.**
- **C1 — nominal rotate + continuity.** Establish a force-closure grasp on a bench object, then command `in_hand.rotate(axis, angle = 90°)`. PASS iff `result == success` ∧ externally measured object reorientation within `orientation_tolerance` of 90° about `axis` ∧ the force trace shows holding force **never dropped below `min_holding_force`** throughout (continuity, incl. any gaiting) ∧ grasp mode/closure unchanged ∧ (with `keep_position`) in-grasp position drift within tolerance.
- **C2 — inadmissible-axis rejection.** Establish a `rotation_constrained` (tripod) grasp; command `in_hand.rotate` about the constrained axis. PASS iff `result == rotation_inadmissible` (not falsely attempted) ∧ grasp preserved unchanged.

#### 3.2 `in_hand.translate`

**Intent.** Shift a held object's position within the grasp envelope, relative to the grasp frame, without releasing it and without reorienting it — preserving grasp identity and stability class throughout.

**Parameters.** (Held → manipulated → held; in_hand core.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the established grasp to manipulate within |
| `direction` | `Direction` | — (required) | — | translation direction, in the grasp frame |
| `distance` | `Length` | — (required) | mm | `> 0`; translation magnitude |
| `controlled_frame` | `FrameRef` | (from `grasp_handle`) | — | grasp frame |
| `position_tolerance` | `Length` | `2` | mm | `> 0`; on the achieved displacement |
| `keep_orientation` | `bool` | `true` | — | hold the object's in-grasp orientation fixed (translate, not rotate) |
| `orientation_tolerance` | `Angle` | `2` | deg | drift bound while translating (when `keep_orientation`) |
| `max_velocity` | `Velocity \| auto` | `auto` | m/s | clamped to `embodiment.limits.v_inhand_max` |
| `regrip_policy` | `{gaiting, continuous, auto}` | `auto` | — | gaiting vs continuous — a hint; resolved by the Translation Layer |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`.
- **Admissible under stability metadata:** the translation `direction` lies within the grasp's manipulable (`friction_held`) DOF — not a `form_held` DOF. (A `pin` grasp permits translation only within the support surface plane; a `hook` only along its compatible directions.)
- The displacement is within the grasp's in-hand translation range (`embodiment.limits.inhand_translation_range` for the grasp mode), possibly via intermediate regrips.
- `embodiment` declares `in_hand_manipulation` with translation support.

**Postconditions (on `success`).**
- The held object's position, relative to the grasp frame, has shifted by `distance` along `direction` within `position_tolerance`.
- **Grasp identity preserved:** `mode`, `closure`, contact topology unchanged; `GraspState` returns to `held`.
- If `keep_orientation`: in-grasp orientation unchanged within `orientation_tolerance`.
- Object held throughout (continuity); world-frame object pose reflects the in-grasp shift.

**Safety envelope (holds throughout execution).**
- **Continuity (strengthened):** holding force never below `min_holding_force` AND stability class maintained throughout, including across gaiting regrips.
- `‖object_velocity (grasp frame)‖ ≤ min(max_velocity, embodiment.limits.v_inhand_max)`.
- `grip_force ≤ force_budget · (1 + transient_margin)`; crush protection per fragility.
- If `keep_orientation`: in-grasp orientation drift `≤ orientation_tolerance`.
- On any breach: arrest and revert to the last stable held configuration within `embodiment.limits.stop_time` (object retained).

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt |
| `capability_absent` | no `in_hand_manipulation` translation support | reject; no attempt |
| `translation_inadmissible` | `direction` is a `form_held` DOF | reject; grasp preserved unchanged |
| `workspace_limit` | `distance` exceeds in-hand translation range even with regrips | partial translate to limit, or reject; report achieved distance |
| `grasp_lost` | continuity breach mid-translation | arrest; revert to last stable held; report |
| `position_not_reached` | post-motion displacement vs tolerance | MUST NOT report `success` |
| `timeout` | wall clock vs `timeout` | revert to last stable held |

**Conformance test sketch.**
- **C1 — nominal translate + continuity.** Establish a force-closure grasp, then command `in_hand.translate(direction, distance = 20 mm)`. PASS iff `result == success` ∧ externally measured in-grasp displacement within `position_tolerance` of 20 mm along `direction` ∧ continuity (force never below `min_holding_force`) ∧ grasp mode/closure unchanged ∧ (with `keep_orientation`) orientation drift within tolerance.
- **C2 — inadmissible-direction rejection.** Establish a `pin` grasp; command `in_hand.translate` along the surface-normal (`form_held`, out-of-plane) direction. PASS iff `result == translation_inadmissible` ∧ grasp preserved unchanged.

#### 3.3 `in_hand.regrasp`

**Intent.** Transition a held object from its current grasp to a different stable grasp — changing contact topology and possibly grasp mode — without releasing the object, using make-before-break so the object is continuously secured throughout the handover.

**Parameters.** (Held → manipulated → held, with a **changed** grasp identity.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the current grasp to transition from |
| `target_mode` | `GraspMode` | — (required) | — | the grasp mode to transition to (may equal current mode with different contacts) |
| `target_contacts` | `ContactConfig \| auto` | `auto` | — | desired new contact configuration; `auto` = planner-derived for `target_mode` |
| `target_force_budget` | `Force \| auto` | `auto` | N | force budget of the new grasp; `auto` = re-derive; clamped per `target_mode` |
| `controlled_frame` | `FrameRef` | (from `grasp_handle`) | — | grasp frame |
| `preserve_pose` | `bool` | `true` | — | keep the object's world pose fixed during the handover |
| `position_tolerance` | `Length` | `2` | mm | object pose drift bound during handover |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`.
- **A make-before-break path exists:** there is a transition in which the new contacts can be established while the old contacts still secure the object — i.e. an overlap window where old ∪ new jointly satisfy `min_holding_force` exists for the embodiment and `target_mode`.
- The `target_mode` is supported (`embodiment` declares the corresponding grasp capability) and feasible on `target.geometry`.
- `embodiment` declares `in_hand_manipulation` with `regrasp` support.

**Postconditions (on `success`).**
- The object is now held in a **new** `GraspState`: `mode = target_mode`, with new contact topology and `stability` recomputed for the new grasp; `status = held`.
- **Grasp identity changed (by design):** unlike `rotate` / `translate`, `mode` / `closure` / topology differ from the input grasp. The originating `GraspRef` is superseded; the result returns a new `GraspRef`.
- If `preserve_pose`: the object's world pose is unchanged within `position_tolerance` (the handover repositioned contacts, not the object).
- Object held throughout (continuity via make-before-break).

**Safety envelope (holds throughout execution).**
- **Make-before-break continuity:** at every instant of the handover, the union of currently-engaged contacts (old, new, or both) secures the object at `≥ min_holding_force`. The old grasp is released **only after** the new grasp is confirmed to secure the object. There is no instant of unsecured state.
- `grip_force ≤ max(force_budget, target_force_budget) · (1 + transient_margin)` across the transition; crush protection per fragility throughout.
- If `preserve_pose`: object world-pose drift `≤ position_tolerance` during handover.
- On any breach (new grasp fails to establish): **abort to the original grasp** — never release the old grasp until the new one is confirmed. The object falls back to the known-good prior state.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt |
| `capability_absent` | no `regrasp` support, or `target_mode` unsupported | reject; original grasp preserved |
| `no_makebeforebreak_path` | no overlap window where old ∪ new secures the object | reject; original grasp preserved (do not attempt an unsafe break-before-make) |
| `target_grasp_failed` | new contacts fail to confirm securing | **retain original grasp**; result ≠ `success` |
| `pose_drift` | `preserve_pose` and drift > `position_tolerance` | abort to original grasp |
| `crush_abort` | force / deformation exceeded during handover | abort to original grasp |
| `timeout` | wall clock vs `timeout` | abort to original grasp |

**Conformance test sketch.**
- **C1 — nominal regrasp + make-before-break.** Establish a `pinch` grasp on a bench object, then command `in_hand.regrasp(target_mode = power)`. PASS iff `result == success` ∧ the final grasp is confirmed `power` (new stability metadata) ∧ the force trace shows **at no instant did total securing force drop below `min_holding_force`** (make-before-break — there is always a securing contact set) ∧ (with `preserve_pose`) object world pose unchanged within tolerance.
- **C2 — failed-target fallback.** Force the target grasp to fail to establish (e.g. `target_mode` infeasible on the presented geometry). PASS iff `result == target_grasp_failed` ∧ the **original grasp is retained** (object not dropped, not in an unsecured state) ∧ the original `GraspRef` remains valid.

#### 3.4 `in_hand.roll`

**Intent.** Continuously roll a held object about a rolling axis through rolling contact — the contact point migrating over both the object surface and the effector surface — typical for cylindrical or spherical objects, enabling reorientation beyond the fixed-contact range of `in_hand.rotate`.

**Parameters.** (Held → manipulated → held; rolling contact.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the established grasp to roll within |
| `roll_axis` | `Direction` | — (required) | — | rolling axis, in the grasp frame (the object's rolling axis) |
| `angle` | `Angle` | — (required) | rad | signed roll magnitude (may exceed 2π for continuous rolling) |
| `controlled_frame` | `FrameRef` | (from `grasp_handle`) | — | grasp frame |
| `orientation_tolerance` | `Angle` | `2` | deg | `> 0` |
| `keep_contact_line` | `bool` | `true` | — | keep the object's rolling axis stationary in the grasp frame |
| `max_angular_velocity` | `AngularVelocity \| auto` | `auto` | rad/s | clamped to `embodiment.limits.w_inhand_max` |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`.
- **Rollable geometry:** `target.geometry` exposes a rolling surface (cylinder / sphere / cone section) whose rolling axis is compatible with `roll_axis` — a non-rollable (e.g. flat-faced box) target is rejected.
- **Admissible under stability metadata:** rolling is compatible with the grasp mode (force-closure modes that permit controlled slip; a `rotation_constrained` tripod or a `form_held` lock about `roll_axis` is rejected).
- `embodiment` declares `in_hand_manipulation` with `roll` support.

**Postconditions (on `success`).**
- The held object has rolled by `angle` about `roll_axis` (reorientation via rolling contact), within `orientation_tolerance`.
- **Grasp identity preserved:** `mode`, `closure`, contact topology unchanged (the *contact point migrates*, but the grasp's identity does not); `GraspState` returns to `held`.
- If `keep_contact_line`: the object's rolling axis stayed stationary in the grasp frame (rolled in place).
- Object held throughout (continuity under rolling contact).

**Safety envelope (holds throughout execution).**
- **Continuity under rolling:** holding force never below `min_holding_force` AND `rolling_stability` maintained — rolling contact tends to degenerate to line / point contact, so the envelope monitors that the migrating contact remains a securing contact throughout.
- `‖object_angular_velocity (grasp frame)‖ ≤ min(max_angular_velocity, embodiment.limits.w_inhand_max)`.
- `grip_force ≤ force_budget · (1 + transient_margin)`; crush protection per fragility.
- Slip discrimination: rolling is *intended* contact-point migration; the envelope must distinguish intended rolling from unintended gross slip (loss of control) and abort only on the latter.
- On any breach: arrest the roll and revert to a stable held configuration within `embodiment.limits.stop_time` (object retained).

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt |
| `capability_absent` | no `in_hand_manipulation` roll support | reject; no attempt |
| `non_rollable_geometry` | `target.geometry` has no rolling surface for `roll_axis` | reject; grasp preserved unchanged |
| `roll_inadmissible` | grasp mode forbids rolling about `roll_axis` (`form_held` / `rotation_constrained`) | reject; grasp preserved |
| `gross_slip` | contact migration exceeds the rolling model (loss of control) | arrest; revert to last stable held; report |
| `orientation_not_reached` | post-motion roll angle vs tolerance | MUST NOT report `success` |
| `timeout` | wall clock vs `timeout` | revert to last stable held |

**Conformance test sketch.**
- **C1 — nominal roll + continuity.** Establish a grasp on a bench cylinder, then command `in_hand.roll(roll_axis, angle = 180°)`. PASS iff `result == success` ∧ externally measured object roll within `orientation_tolerance` of 180° about `roll_axis` ∧ continuity (force never below `min_holding_force`) throughout the rolling ∧ grasp mode/closure unchanged ∧ (with `keep_contact_line`) the rolling axis stayed stationary in the grasp frame.
- **C2 — non-rollable rejection.** Present a flat-faced (non-rollable) object; command `in_hand.roll`. PASS iff `result == non_rollable_geometry` ∧ grasp preserved unchanged.

#### 3.5 `in_hand.pivot`

**Intent.** Pivot a held object about a single contact point, swinging the object's body around that fixed pivot — optionally exploiting gravity or an external wrench to drive the rotation — while keeping the pivot contact secured. Used to reorient elongated objects through a larger angle than in-grasp rotation allows.

**Parameters.** (Held → manipulated → held; controlled under-actuation about one DOF.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the established grasp providing the pivot contact |
| `pivot_point` | `FeatureRef \| auto` | `auto` | — | the contact point to pivot about; `auto` = current securing contact |
| `pivot_axis` | `Direction` | — (required) | — | axis of the pivot rotation, in the grasp frame |
| `angle` | `Angle` | — (required) | rad | target swing angle about `pivot_axis` |
| `drive` | `{actuated, gravity, external}` | `actuated` | — | what drives the swing: active control, gravity, or a declared external wrench |
| `controlled_frame` | `FrameRef` | (from `grasp_handle`) | — | grasp frame |
| `orientation_tolerance` | `Angle` | `5` | deg | `> 0`; looser default for passively-driven pivots |
| `max_angular_velocity` | `AngularVelocity \| auto` | `auto` | rad/s | bounds the swing rate (esp. for gravity drive) |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState` providing a pivot contact that can secure the object while one rotational DOF is released.
- **Pivot DOF admissible:** the `pivot_axis` rotation is a DOF the grasp can release while the remaining DOF still secure the object at `≥ min_holding_force` (controlled under-actuation, not a full release).
- For `drive = gravity`: the gravity-induced swing about `pivot_axis` is in the intended direction (the geometry permits gravity to drive, not fight, the pivot).
- `embodiment` declares `in_hand_manipulation` with `pivot` support.

**Postconditions (on `success`).**
- The held object has pivoted by `angle` about `pivot_axis` around `pivot_point`, within `orientation_tolerance` (looser than active in-hand rotation, reflecting partially-passive drive).
- **Grasp identity preserved:** the pivot contact and grasp `mode` are unchanged; the released DOF is re-secured at completion; `GraspState` returns to `held` (fully secured again).
- The pivot point stayed fixed in the grasp frame; the object body swung around it.
- Object retained throughout (the pivot contact never dropped below `min_holding_force`).

**Safety envelope (holds throughout execution).**
- **Controlled under-actuation:** exactly the `pivot_axis` DOF is released; all other DOF keep the object secured at `≥ min_holding_force` throughout. The released DOF is the *only* under-constrained freedom — the object is never fully unsecured.
- Swing-rate bound: `‖object_angular_velocity‖ ≤ min(max_angular_velocity, embodiment.limits.w_inhand_max)` — critical for `gravity` / `external` drive, where an unchecked swing could exceed safe velocity or overshoot.
- Overshoot guard: the object must arrest at `angle` and not continue swinging past it (gravity / inertia overshoot is an envelope violation).
- `grip_force ≤ force_budget · (1 + transient_margin)`; crush protection per fragility.
- On any breach: re-secure the released DOF (arrest the swing) and revert to a fully-held state within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt |
| `capability_absent` | no `in_hand_manipulation` pivot support | reject; no attempt |
| `pivot_inadmissible` | releasing `pivot_axis` would drop the object (remaining DOF cannot secure) | reject; grasp preserved fully |
| `wrong_drive_direction` | `drive = gravity` but gravity opposes the intended pivot | reject; no attempt |
| `overshoot` | swing passed `angle` beyond tolerance (passive overshoot) | arrest; re-secure; result ≠ `success` |
| `pivot_lost` | pivot contact dropped below securing force mid-swing | arrest; re-secure if possible; report |
| `orientation_not_reached` | post-pivot angle vs tolerance | MUST NOT report `success` |
| `timeout` | wall clock vs `timeout` | re-secure to fully-held state |

**Conformance test sketch.**
- **C1 — actuated pivot + continuity.** Establish a grasp on a bench elongated object, command `in_hand.pivot(pivot_axis, angle = 90°, drive = actuated)`. PASS iff `result == success` ∧ externally measured pivot within `orientation_tolerance` of 90° about `pivot_axis` ∧ the pivot contact force **never dropped below `min_holding_force`** (controlled under-actuation, never fully released) ∧ grasp mode unchanged ∧ object re-secured (fully held) at completion.
- **C2 — gravity-drive overshoot guard.** Command `in_hand.pivot(drive = gravity, angle = 45°)` on a pendulum-like object. PASS iff the swing arrests at 45° within tolerance **without overshoot**, OR `result == overshoot` with the object re-secured — never an uncontrolled continued swing.

#### 3.6 `in_hand.slide`

**Intent.** Slide a held object along one effector contact surface through controlled sliding contact — intentionally permitting relative slip along one translational DOF, then re-securing at a stop condition — to reposition or feed the object without a full regrasp. The translational counterpart of `in_hand.roll`.

**Parameters.** (Held → manipulated → held; controlled slip along one translational DOF.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the established grasp to slide within |
| `slide_direction` | `Direction` | — (required) | — | direction of slip along the contact surface, in the grasp frame |
| `stop_condition` | `SlideStop` | — (required) | — | what ends the slide: `distance(d)`, `tactile_landmark(ref)`, or `external_reference(ref)` |
| `drive` | `{actuated, gravity}` | `actuated` | — | active feed vs gravity-fed slip |
| `controlled_frame` | `FrameRef` | (from `grasp_handle`) | — | grasp frame |
| `position_tolerance` | `Length` | `3` | mm | `> 0`; looser default (slip is friction-dependent) |
| `max_velocity` | `Velocity \| auto` | `auto` | m/s | bounds slip rate (esp. for gravity feed) |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`.
- **Slide DOF admissible:** `slide_direction` is a `friction_held` translational DOF that can be partially released (reduce normal force to permit slip) while the remaining DOF keep the object from dropping (`≥ min_holding_force` orthogonal to the slide).
- The `stop_condition` is resolvable (a finite distance, a detectable tactile landmark, or a valid external reference).
- `embodiment` declares `in_hand_manipulation` with `slide` support and closed-loop slip sensing (or a force / position proxy).

**Postconditions (on `success`).**
- The held object has slid along `slide_direction` until `stop_condition` was met; final in-grasp position recorded.
- **Grasp identity preserved:** `mode`, `closure`, contact topology unchanged (the contact *slid*, identity did not change); `GraspState` returns to `held`, fully re-secured (normal force restored).
- For `stop_condition = distance(d)`: displacement within `position_tolerance` of `d`.
- Object retained throughout (never dropped; orthogonal DOF kept securing).

**Safety envelope (holds throughout execution).**
- **Controlled slip:** only the `slide_direction` DOF is allowed to slip; orthogonal DOF maintain `≥ min_holding_force` so the object cannot fall. Normal force is reduced to permit slip, never to zero on the securing DOF.
- Slip-rate bound: `‖slip_velocity‖ ≤ min(max_velocity, embodiment.limits.v_inhand_max)` — bounds gravity-fed runaway.
- Overshoot guard: closed-loop re-securing must arrest the slide at `stop_condition`; sliding past it beyond `position_tolerance` is an envelope violation.
- Distinguish intended slip (along `slide_direction`) from unintended drop-slip (object escaping the grasp) — abort only on the latter.
- On any breach: re-secure (restore normal force, arrest slip) to a fully-held state within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt |
| `capability_absent` | no `slide` support / no slip sensing | reject; no attempt |
| `slide_inadmissible` | `slide_direction` is `form_held`, or releasing it drops the object | reject; grasp preserved |
| `stop_unreachable` | `stop_condition` cannot be met within range | slide to limit; re-secure; result ≠ `success` |
| `overshoot` | slid past `stop_condition` beyond tolerance | re-secure; result ≠ `success` |
| `drop_slip` | unintended slip on a securing DOF (object escaping) | abort; re-secure if possible; report |
| `position_not_reached` | (distance stop) displacement vs tolerance | MUST NOT report `success` |
| `timeout` | wall clock vs `timeout` | re-secure to fully-held state |

**Conformance test sketch.**
- **C1 — nominal slide-to-distance + continuity.** Establish a grasp on a bench rod, command `in_hand.slide(slide_direction, stop_condition = distance(30 mm))`. PASS iff `result == success` ∧ externally measured slip within `position_tolerance` of 30 mm along `slide_direction` ∧ orthogonal securing force **never dropped below `min_holding_force`** (object never escaped) ∧ grasp identity unchanged ∧ object re-secured (full normal force restored) at the stop.
- **C2 — landmark stop + overshoot guard.** Use `stop_condition = tactile_landmark(ref)` (slide until a feature is detected). PASS iff the slide arrests at the landmark within tolerance, OR `result == overshoot` / `stop_unreachable` with the object re-secured — never a continued uncontrolled slide or a drop.

#### 3.7 `in_hand.flip`

**Intent.** Reorient a held object through a large angle (typically ~180°) that requires a **momentary release** — tossing or releasing-and-recatching the object so the effector can re-engage on a previously inaccessible face. This is the single `in_hand` primitive that suspends grasp continuity, and it does so under a strictly bounded, recoverable unsecured window.

**Parameters.** (Held → [unsecured window] → held; the continuity exception.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the current grasp to flip from |
| `flip_axis` | `Direction` | — (required) | — | reorientation axis, in the grasp frame |
| `angle` | `Angle` | — (required) | rad | reorientation magnitude (the angle unreachable without release) |
| `target_mode` | `GraspMode \| same` | `same` | — | grasp mode to re-establish after the flip |
| `catch_envelope` | `Region \| auto` | `auto` | — | spatial region within which the re-catch must occur; `auto` = planner-derived |
| `max_release_time` | `Duration` | — (required) | s | `> 0`; hard upper bound on the unsecured window |
| `safe_drop_zone` | `Region` | — (required) | — | region below the operation where an uncaught object lands without harm |
| `controlled_frame` | `FrameRef` | (from `grasp_handle`) | — | grasp frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`.
- **The reorientation genuinely requires release:** the `angle` about `flip_axis` is unreachable by `in_hand.rotate` / `roll` / `regrasp` (which preserve continuity). `flip` is rejected if a continuity-preserving alternative exists (`flip` is the last resort).
- **Safe-drop precondition (mandatory):** a `safe_drop_zone` is established below the operation such that, if the re-catch fails, the object falls into it without harm. `flip` MUST NOT be attempted without a safe drop zone.
- The predicted ballistic / release trajectory keeps the object within reach for a re-catch inside `catch_envelope` within `max_release_time`.
- `embodiment` declares `in_hand_manipulation` with `flip` support.

**Postconditions (on `success`).**
- The object is re-secured in a new `GraspState` (`mode = target_mode` or the prior mode if `same`), reoriented by `angle` about `flip_axis`; `status = held`.
- **`momentary_release = true` (declared):** the result explicitly records that grasp continuity was suspended — visible to downstream primitives and audit.
- The unsecured window did not exceed `max_release_time`; the re-catch occurred within `catch_envelope`.
- A new `GraspRef` is returned (the prior grasp was fully released and re-established).

**Safety envelope (the continuity exception, bounded).**
- **Bounded unsecured window:** the object is unsecured for at most `max_release_time`; exceeding it is an envelope violation that triggers the safe-drop contingency.
- **Re-catch-or-safe-drop:** if the re-catch is not confirmed within `catch_envelope` and `max_release_time`, the object is allowed to fall into `safe_drop_zone` (a controlled failure, not a hazard) — the envelope guarantees no uncaught object lands outside the safe zone.
- Release velocity / toss energy bounded so the object stays within `catch_envelope` and does not become a projectile beyond the safe zone.
- This is the *only* primitive whose envelope permits an unsecured object state; the permission is explicit, time-bounded, and paired with a mandatory safe-drop guarantee.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt |
| `capability_absent` | no `flip` support | reject; no attempt |
| `continuity_alternative_exists` | a `rotate` / `roll` / `regrasp` achieves `angle` without release | reject; recommend the continuity-preserving primitive (flip is last resort) |
| `no_safe_drop_zone` | `safe_drop_zone` absent or invalid | reject; **flip is never attempted without a safe drop zone** |
| `recatch_failed` | object not re-secured within `catch_envelope` / `max_release_time` | object falls into `safe_drop_zone`; report (controlled failure) |
| `orientation_not_reached` | re-caught but angle out of tolerance | result ≠ `success` (object held but mis-oriented) |
| `timeout` | wall clock vs `timeout` | safe-drop contingency |

**Conformance test sketch.**
- **C1 — nominal flip + bounded window.** Establish a grasp on a bench object requiring a 180° flip; command `in_hand.flip(flip_axis, angle = 180°, max_release_time, safe_drop_zone)`. PASS iff `result == success` ∧ re-caught and re-secured with reorientation within tolerance ∧ the measured unsecured window `≤ max_release_time` ∧ re-catch occurred within `catch_envelope` ∧ the result declares `momentary_release = true`.
- **C2 — recatch failure → safe drop.** Force a re-catch failure (perturb the toss). PASS iff `result == recatch_failed` ∧ the object landed **within `safe_drop_zone`** (controlled failure, no hazard outside the zone) ∧ no attempt was made without a safe drop zone in the first place.

### Category 4 — `transport`

`transport` primitives relocate a **held** object through space, preserving the grasp throughout (`held → held`). They are the held-object counterparts of `reach`: where `reach` moves an effector through free space, `transport` moves an effector *plus its grasped load*. The defining new safety axis is **dynamic grasp stability** — under acceleration the object's inertial load must not exceed the grasp's holding capacity, or the object slips in (or escapes) the grasp. Every `transport` primitive therefore reads the grasp's stability metadata (`secured_dof`, `closure`, `flags`) to clamp acceleration: `power` grasps tolerate aggressive motion, `platform`/`support` grasps require level-keeping and a tip-over margin, `hook`/directional grasps bound acceleration off the stable directions, and `surface_bound` (`pin`) grasps are not freely transportable at all. The static hold test that validated a grasp at rest is necessary but not sufficient here; transport adds the dynamic condition.

#### 4.1 `transport.move_to_pose`

**Intent.** Move a held object to an absolute target pose in a reference frame, terminating at rest, keeping the grasp secured against inertial loads throughout — the held-object counterpart of `reach.to_pose`.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp whose held object is transported |
| `target_pose` | `Pose6D` | — (required) | m / rad | pose of the **held object** (not the effector) in `frame` |
| `frame` | `FrameRef` | `task` | — | calibrated reference frame |
| `position_tolerance` | `Length` | `2` | mm | `> 0` |
| `orientation_tolerance` | `Angle` | `2` | deg | `> 0`; geodesic |
| `max_velocity` | `Velocity \| auto` | `auto` | m/s | clamped to `embodiment.limits.v_cartesian_max` |
| `max_acceleration` | `Acceleration \| auto` | `auto` | m/s² | **clamped to the grasp's dynamic-stability limit** (derived from stability metadata) |
| `clearance` | `Length` | `0` | mm | `≥ 0`; margin for the **swept volume of the object + effector** |
| `contact_response` | `{abort, stop, comply}` | `abort` | — | reaction to unplanned contact of object or effector |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`.
- **Transport admissible under stability flags:** the grasp is freely transportable — `surface_bound` grasps (`pin`) are **rejected** (`transport_inadmissible`); `support` grasps (`platform`) require the level-keeping / tip-over constraints below.
- `target_pose` (of the object) admits ≥ 1 IK solution for the held object + grasp configuration.
- The swept volume of object + effector to `target_pose`, inflated by `clearance`, is collision-free against the static model.
- `embodiment` declares `transport` capability.

**Postconditions (on `success`).**
- The held object's pose in `frame` is within `(position_tolerance, orientation_tolerance)` of `target_pose`; embodiment + object at rest.
- **Grasp preserved (held → held):** `mode`, `closure`, stability metadata unchanged; the object did not slip in the grasp beyond `position_tolerance` relative to the grasp frame.
- No unplanned contact was formed.

**Safety envelope (holds throughout execution).**
- **Dynamic grasp stability (the new axis):** the inertial load on the grasp (`mass · acceleration` + gravity, resolved against `secured_dof`) never exceeds the grasp's holding capacity — i.e. `inertial_load ≤ grasp_holding_capacity` with margin. `max_acceleration` is clamped so this holds. Exceeding it risks in-grasp slip and is an envelope violation.
- For `support` / `platform` grasps: the support stays level within tolerance and the object's CoM stays inside the support polygon under acceleration (tip-over guard); acceleration is further clamped to the tip-over margin.
- For `hook` / directional grasps: acceleration along non-`stable_directions` is bounded so the load does not disengage the grasp.
- `‖velocity‖ ≤ min(max_velocity, embodiment.limits.v_cartesian_max)`; `min_clearance(object ∪ effector, static_model) ≥ clearance`.
- `external_force` (object or effector) `≤ contact_abort_threshold`; a breach triggers `contact_response`.
- On any breach: decelerate to rest within `embodiment.limits.stop_time`, keeping the object secured (never drop to "recover").

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt |
| `transport_inadmissible` | `surface_bound` (pin) grasp | reject; no attempt (use regrasp first) |
| `unreachable` | IK on object `target_pose` | no motion |
| `no_collision_free_path` | swept-volume (object + effector) blocked | no motion past the last safe config |
| `in_grasp_slip` | object slipped in grasp beyond tolerance (dynamic-stability breach) | decelerate; re-secure; result ≠ `success` |
| `tip_over_risk` | (support) CoM approached the polygon edge under acceleration | reduce acceleration / abort to safe state |
| `unexpected_contact` | object / effector external force > threshold | react per `contact_response` |
| `pose_not_reached` | object pose vs tolerance | MUST NOT report `success` |
| `timeout` | wall clock vs `timeout` | decelerate to rest, object secured |

**Conformance test sketch.**
- **C1 — nominal transport + grasp retention.** Establish a force-closure grasp on a bench object, command `transport.move_to_pose(target_pose = P)`. PASS iff `result == success` ∧ externally measured **object** pose within tolerance of `P` ∧ at rest ∧ the object did not slip in the grasp beyond `position_tolerance` (measured grasp-frame-relative pose unchanged) ∧ no telemetry sample exceeded the dynamic-stability acceleration clamp.
- **C2 — surface-bound rejection.** Establish a `pin` grasp; command `transport.move_to_pose`. PASS iff `result == transport_inadmissible` (the surface-bound grasp is not freely transportable) ∧ no motion attempted ∧ grasp preserved.

#### 4.2 `transport.follow_trajectory`

**Intent.** Transport a held object so that it tracks a caller-specified parameterized trajectory through the workspace, within a tracking tolerance, keeping the grasp secured against inertial loads throughout. Where `transport.move_to_pose` delegates the path to the planner, `follow_trajectory` lets the caller own the path (replaying a learned policy, a taught motion, or a required shape).

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp whose held object is transported |
| `trajectory` | `Trajectory` | — (required) | — | parameterized path of the **held object** in `frame` (waypoints / spline + timing) |
| `frame` | `FrameRef` | `task` | — | calibrated reference frame |
| `tracking_tolerance` | `Length` | `3` | mm | `> 0`; max deviation of the object from the path |
| `orientation_tolerance` | `Angle` | `2` | deg | `> 0`; along the path |
| `timing_mode` | `{strict, time_scalable}` | `time_scalable` | — | whether the trajectory's timing may be slowed to respect dynamic stability |
| `max_velocity` | `Velocity \| auto` | `auto` | m/s | clamped to `embodiment.limits.v_cartesian_max` |
| `max_acceleration` | `Acceleration \| auto` | `auto` | m/s² | clamped to the grasp's dynamic-stability limit |
| `clearance` | `Length` | `0` | mm | `≥ 0`; for the swept volume of object + effector along the path |
| `contact_response` | `{abort, stop, comply}` | `abort` | — | reaction to unplanned contact |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`; transport admissible (not `surface_bound`).
- Every point of `trajectory` admits ≥ 1 IK solution for the object + grasp; the whole path's swept volume is collision-free at `clearance`.
- **Dynamic feasibility:** the trajectory's curvature-and-speed profile is within the grasp's dynamic-stability limit — OR `timing_mode = time_scalable`, permitting the timing to be slowed (path shape preserved) until it is. A `strict` trajectory exceeding the limit is rejected.
- `embodiment` declares `transport` with `follow_trajectory` support.

**Postconditions (on `success`).**
- The held object tracked `trajectory` within `(tracking_tolerance, orientation_tolerance)` at every point; ends at rest at the trajectory's final pose.
- **Grasp preserved (held → held):** identity and stability metadata unchanged; no in-grasp slip beyond tolerance.
- If timing was scaled (`time_scalable`), the achieved timing is reported; path shape was preserved.

**Safety envelope (holds throughout execution).**
- **Dynamic grasp stability along the path:** at every point the inertial load (centripetal from curvature × speed, plus tangential acceleration, plus gravity, resolved against `secured_dof`) `≤ grasp_holding_capacity`. `time_scalable` slows timing to maintain this; `strict` would have been rejected at precondition.
- **Tracking bound:** object deviation from the path `≤ tracking_tolerance` throughout; exceeding it is an envelope violation (the object is not where the caller required).
- Support / directional grasp constraints as in `move_to_pose` (tip-over margin, off-stable-direction bound), evaluated continuously along the path.
- `‖velocity‖ ≤ caps`; `min_clearance(object ∪ effector, static_model) ≥ clearance`; `external_force ≤ threshold → contact_response`.
- On any breach: decelerate to rest on or near the path within `embodiment.limits.stop_time`, object secured.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` / `transport_inadmissible` | not `held` / `surface_bound` | reject; no attempt |
| `unreachable` | some trajectory point has no IK | no motion (reject whole path) |
| `no_collision_free_path` | swept volume blocked along the path | no motion past the last safe config |
| `dynamically_infeasible` | `strict` timing exceeds the dynamic-stability limit | reject; recommend `time_scalable` |
| `tracking_exceeded` | object deviation > `tracking_tolerance` mid-path | decelerate; result ≠ `success` |
| `in_grasp_slip` | dynamic-stability breach (slip) | decelerate; re-secure; result ≠ `success` |
| `tip_over_risk` | (support) CoM near polygon edge along the path | reduce speed / abort |
| `unexpected_contact` | object / effector force > threshold | react per `contact_response` |
| `timeout` | wall clock vs `timeout` | decelerate to rest, object secured |

**Conformance test sketch.**
- **C1 — nominal tracking + dynamic stability.** Establish a force-closure grasp; command `transport.follow_trajectory` along a bench-defined curved path. PASS iff `result == success` ∧ the externally measured object trajectory stayed within `tracking_tolerance` of the path at every sampled point (interval sampling) ∧ no in-grasp slip beyond tolerance ∧ no sample exceeded the dynamic-stability acceleration clamp ∧ ends at rest at the final pose.
- **C2 — time-scaling vs strict.** Submit a trajectory whose timing exceeds the dynamic-stability limit. PASS iff (`time_scalable`) the path shape is tracked within tolerance at a reported slower timing, OR (`strict`) `result == dynamically_infeasible` with no motion — never a tracked path with an in-grasp slip.

#### 4.3 `transport.handoff`

**Intent.** Transfer a held object from the current grasp to a partner effector's grasp — bimanual (two effectors of one embodiment) or inter-robot (two embodiments) — using make-before-break so the object is continuously secured by at least one party throughout. The two-party counterpart of `in_hand.regrasp`.

**Parameters.** (Held → manipulated → held, with ownership transferred to a partner.)

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the giver's current grasp |
| `receiver` | `EffectorRef` | — (required) | — | the partner effector (a control frame on the same or another embodiment) |
| `receiver_mode` | `GraspMode \| auto` | `auto` | — | the grasp the receiver should form; `auto` = planner-derived |
| `handoff_pose` | `Pose6D \| auto` | `auto` | — | object pose at which the transfer occurs; `auto` = mutually reachable pose |
| `cograsp_force_budget` | `Force \| auto` | `auto` | N | max **combined** force during the dual-grasp window; `auto` = min of the two parties' budgets, clamped to `target.max_contact_force` |
| `frame` | `FrameRef` | `task` | — | shared reference frame |
| `position_tolerance` | `Length` | `2` | mm | object pose drift during transfer |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState` (the giver holds the object).
- The `receiver` is available (its control frame is `free`) and declares a grasp capability compatible with `receiver_mode` on `target.geometry`.
- **Mutual reachability:** a `handoff_pose` exists that is reachable by both the giver and the receiver, with a make-before-break window (both can secure the object simultaneously without exceeding `cograsp_force_budget`).
- Both parties declare `handoff` capability (and, for inter-robot, a coordination channel exists — see Open issue).

**Postconditions (on `success`).**
- The object is held by the **receiver** in a new `GraspState` (`mode = receiver_mode`); the giver's `controlled_frame` is `free` (`status = free`, grasp cleared).
- **Ownership transferred:** the result returns the receiver's new `GraspRef`; the giver's `GraspRef` is superseded / invalidated.
- The object remained secured by at least one party throughout (continuity via make-before-break); its world pose stayed within `position_tolerance` of `handoff_pose` during the transfer.

**Safety envelope (holds throughout execution).**
- **Two-party make-before-break:** the receiver's grasp is confirmed securing (`≥ min_holding_force`) **before** the giver releases; at no instant is the object unsecured by both.
- **Co-grasp force bound:** during the dual-grasp window, the **combined** force from both parties `≤ cograsp_force_budget · (1 + transient_margin)` and `≤ target.max_contact_force` — the two effectors must not crush the object or fight each other (no opposing tug-of-war beyond budget).
- The object's pose stays within `position_tolerance` during the transfer (neither party yanks it).
- On any breach (receiver fails to secure): **the giver retains the object** — never release until the receiver is confirmed. The object falls back to the giver's known-good grasp.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | giver not `held` | reject; no attempt |
| `receiver_unavailable` | `receiver` not `free` / lacks capability | reject; giver retains object |
| `no_mutual_reach` | no `handoff_pose` reachable by both with a make-before-break window | reject; giver retains object |
| `coordination_unavailable` | (inter-robot) no coordination channel | reject; giver retains object |
| `receiver_grasp_failed` | receiver fails to confirm securing | **giver retains object**; result ≠ `success` |
| `cograsp_overforce` | combined force exceeds budget during dual grasp | abort; giver retains; result ≠ `success` |
| `pose_drift` | object pose drifted > `position_tolerance` during transfer | abort to giver's grasp |
| `timeout` | wall clock vs `timeout` | giver retains object |

**Conformance test sketch.**
- **C1 — bimanual handoff + make-before-break.** Establish a grasp on a bench object with effector A; command `transport.handoff(receiver = effector B)`. PASS iff `result == success` ∧ the object is finally held by B (new stability metadata) ∧ A is `free` ∧ the force trace shows **at least one party secured the object at all instants** (make-before-break) ∧ combined co-grasp force never exceeded `cograsp_force_budget` ∧ object pose within tolerance throughout.
- **C2 — receiver-failure fallback.** Force the receiver's grasp to fail. PASS iff `result == receiver_grasp_failed` ∧ the **giver retains the object** (not dropped, not unsecured) ∧ the giver's `GraspRef` remains valid.

#### 4.4 `transport.carry`

**Intent.** Transport a held object while actively maintaining grasp stability under perturbation — rejecting external disturbances (a moving base, environmental contact, an unstable path) so the object stays secured throughout. The disturbance-robust counterpart of plain transport, and the held-object analogue of `reach.hover`'s station-keeping.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp whose held object is carried |
| `motion` | `MoveSpec` | — (required) | — | the underlying relocation: a `to_pose(P)` or a `trajectory(T)` — `carry` wraps it with disturbance rejection |
| `disturbance_budget` | `Force \| auto` | `auto` | N | the magnitude of external perturbation the carry must reject without losing the grasp; `auto` = derived from `min_holding_force` margin |
| `stability_margin` | `Ratio` | `auto` | — | required headroom of holding capacity over inertial + disturbance load; `auto` = embodiment default |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `position_tolerance` | `Length` | `3` | mm | `> 0`; on the object relative to `motion` (looser — disturbances perturb the path) |
| `max_velocity` | `Velocity \| auto` | `auto` | m/s | clamped; conservative under disturbance |
| `max_acceleration` | `Acceleration \| auto` | `auto` | m/s² | clamped to leave `stability_margin` for disturbance rejection |
| `contact_response` | `{abort, stop, comply}` | `comply` | — | default `comply` (carry expects environmental contact) |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`; transport admissible (not `surface_bound`).
- The grasp's holding capacity exceeds the inertial load **plus** `disturbance_budget` by at least `stability_margin` (the grasp can absorb the expected perturbation without slip).
- The underlying `motion` is itself feasible (its own preconditions hold).
- `embodiment` declares `transport` with `carry` (disturbance-rejection) support.

**Postconditions (on `success`).**
- The underlying `motion` completed (object reached `to_pose` target or tracked `trajectory`), within `position_tolerance`.
- **Stability maintained throughout (the interval invariant):** at every instant the grasp held the object within tolerance despite perturbations up to `disturbance_budget`; no in-grasp slip beyond tolerance occurred.
- Grasp preserved (held → held), identity and stability metadata unchanged.

**Safety envelope (holds throughout execution).**
- **Disturbance-robust dynamic stability (interval invariant):** at every instant, `inertial_load + disturbance_load ≤ grasp_holding_capacity` with `stability_margin` headroom. Acceleration is clamped below the plain-transport limit to reserve capacity for disturbance rejection.
- Active rejection: a detected disturbance is countered (grip adjust / motion adaptation) to keep the object secured; failure to reject within the margin is an envelope violation.
- `comply` contact response by default: environmental contact during carry is expected and is accommodated, not treated as an abort (unlike free transport) — within force limits.
- Support / directional grasp constraints continuously evaluated under disturbance (tip-over margin is tighter under perturbation).
- On any breach (disturbance exceeds budget, slip imminent): decelerate / halt to the most stable reachable configuration within `embodiment.limits.stop_time`, object secured.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` / `transport_inadmissible` | not `held` / `surface_bound` | reject; no attempt |
| `insufficient_stability_margin` | holding capacity < inertial + `disturbance_budget` + margin | reject; no attempt (grasp too weak for this carry) |
| `motion_infeasible` | underlying `motion` preconditions fail | reject; propagate the motion's failure |
| `disturbance_exceeded` | actual perturbation > `disturbance_budget` | halt to most stable config; result ≠ `success` |
| `in_grasp_slip` | object slipped despite rejection | halt; re-secure; result ≠ `success` |
| `timeout` | wall clock vs `timeout` | halt to stable config, object secured |

**Conformance test sketch.**
- **C1 — carry under perturbation + stability.** Establish a grasp; command `transport.carry(motion = to_pose(P), disturbance_budget = D)` while applying calibrated perturbations `≤ D` during the motion. PASS iff `result == success` ∧ object reached `P` within tolerance ∧ at **every** sampled instant (interval sampling) the object stayed secured with no in-grasp slip beyond tolerance despite the applied disturbances ∧ holding-capacity headroom maintained `stability_margin` throughout.
- **C2 — over-budget disturbance.** Apply a perturbation exceeding `disturbance_budget`. PASS iff `result == disturbance_exceeded` ∧ the embodiment halted to a stable configuration with the object **still secured** (not dropped) — a graceful degradation, not a loss.

#### 4.5 `transport.lift`

**Intent.** Raise a held object vertically from a resting / supported state, managing the load-transfer transition — the moment the object's full weight shifts from its prior support onto the grasp — with anti-slip force monitoring so the grasp does not lose the object as the load engages.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp holding the object |
| `height` | `Length` | — (required) | mm | `> 0`; vertical lift distance |
| `up_direction` | `Direction` | `−gravity` | — | the "up" direction; defaults to anti-gravity (declared, not assumed) |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `position_tolerance` | `Length` | `2` | mm | `> 0` |
| `max_velocity` | `Velocity \| auto` | `auto` | m/s | clamped; conservative through load transfer |
| `max_acceleration` | `Acceleration \| auto` | `auto` | m/s² | clamped to dynamic-stability limit (full weight engaged) |
| `clearance` | `Length` | `0` | mm | `≥ 0`; for object + effector swept volume |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`; transport admissible (not `surface_bound`).
- **Load-transfer capacity:** the grasp's holding capacity exceeds the object's **full weight** (`estimated_mass`) by margin — the lift will transfer the entire weight onto the grasp, so a grasp validated at rest under partial load must still hold the full load.
- The lift path (height along `up_direction`), inflated by `clearance`, is collision-free for object + effector.
- `embodiment` declares `transport` with `lift` support.

**Postconditions (on `success`).**
- The held object has risen by `height` along `up_direction`; the object's full weight is borne by the grasp; embodiment + object at rest at the raised pose.
- **No load-transfer slip:** the object did not slip in the grasp as the weight engaged (anti-slip monitoring held).
- Grasp preserved (held → held); the object is now free of its prior support.

**Safety envelope (holds throughout execution).**
- **Anti-slip load-transfer monitoring:** through the lift-off transition, the holding force is maintained `≥ min_holding_force(full weight)`; the moment of weight engagement (when support reaction drops to zero) is the highest-risk instant and the envelope guards holding force there. In-grasp slip beyond tolerance is an envelope violation.
- Dynamic grasp stability with full weight engaged; `max_acceleration` clamped accordingly.
- `‖velocity‖ ≤ caps`; `min_clearance(object ∪ effector, static_model) ≥ clearance`.
- On any breach (slip during lift-off): **lower back to the supported state** — do not continue lifting with a slipping grasp; return the object to its support.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` / `transport_inadmissible` | not `held` / `surface_bound` | reject; no attempt |
| `insufficient_lift_capacity` | holding capacity < full weight + margin | reject; no attempt (do not lift what the grasp cannot hold) |
| `unreachable` | IK along the lift path | no motion |
| `no_collision_free_path` | lift swept volume blocked | no motion past the last safe config |
| `load_transfer_slip` | object slipped as weight engaged | lower back to support; result ≠ `success` |
| `pose_not_reached` | height vs tolerance | MUST NOT report `success` |
| `timeout` | wall clock vs `timeout` | lower to a supported / stable state, object secured |

**Conformance test sketch.**
- **C1 — nominal lift + anti-slip.** Place a bench object on a support, grasp it, command `transport.lift(height = 100 mm)`. PASS iff `result == success` ∧ externally measured object rose 100 mm within tolerance ∧ the force trace shows holding force stayed `≥ min_holding_force(full weight)` **through the lift-off instant** (anti-slip) ∧ no in-grasp slip beyond tolerance ∧ at rest at the raised pose.
- **C2 — over-weight rejection.** Present an object whose full weight exceeds the grasp's holding capacity; command `transport.lift`. PASS iff `result == insufficient_lift_capacity` ∧ no lift attempted (the object is not partially lifted then dropped) ∧ grasp / object state preserved.

#### 4.6 `transport.lower`

**Intent.** Lower a held object vertically onto a target surface with controlled deceleration and touchdown detection — softening the set-down contact and shedding the object's weight onto the surface — while keeping the grasp (release is a separate step).

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp holding the object |
| `stop_mode` | `{height, touchdown}` | `touchdown` | — | stop after a fixed descent, or when surface contact is detected |
| `height` | `Length \| auto` | `auto` | mm | (`height` mode) descent distance; `auto` = until touchdown |
| `down_direction` | `Direction` | `gravity` | — | descent direction; defaults to gravity (declared, not assumed) |
| `touchdown_force` | `Force \| auto` | `auto` | N | contact force threshold marking touchdown; `auto` = small fraction of weight |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `position_tolerance` | `Length` | `2` | mm | `> 0` |
| `approach_velocity` | `Velocity \| auto` | `auto` | m/s | slow contact-approach speed (soft landing) |
| `clearance` | `Length` | `0` | mm | `≥ 0`; for object + effector swept volume (excluding the set-down surface) |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`; transport admissible (not `surface_bound`).
- A set-down surface exists along `down_direction` within reach; the descent path (excluding that surface) is collision-free at `clearance`.
- `embodiment` declares `transport` with `lower` support (and contact / force sensing, or proxy, for touchdown).

**Postconditions (on `success`).**
- The held object has descended (by `height`, or until touchdown) and rests on the set-down surface; the object's weight is (partially or fully) shed onto the surface.
- **Soft set-down:** the contact force at touchdown did not exceed a safe set-down threshold (no slam); the object / surface were not damaged by impact.
- **Still grasped:** the grasp is preserved (held → held) — `transport.lower` sets the object down but does **not** release it; release is `grasp.release` / `place.*`.
- The object is now in a supported state (recorded, enabling a subsequent safe release).

**Safety envelope (holds throughout execution).**
- **Controlled deceleration:** the descent slows to `approach_velocity` before contact so touchdown is soft; impact force at touchdown `≤ safe_setdown_force` (no slam).
- **Touchdown detection:** contact is detected at `touchdown_force` and the descent stops; the object is not pushed into the surface beyond the set-down threshold (over-press protection).
- Grasp continuity maintained throughout (the object stays secured during descent and set-down).
- `min_clearance(object ∪ effector, static_model \ setdown_surface) ≥ clearance`; `‖velocity‖ ≤ caps`.
- On any breach: arrest the descent, keep the object grasped (do not drop), settle to a safe state within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` / `transport_inadmissible` | not `held` / `surface_bound` | reject; no attempt |
| `no_surface` | (`touchdown`) no surface reached within range | descend to limit; result ≠ `success`; object still grasped |
| `unreachable` | IK along the descent path | no motion |
| `no_collision_free_path` | descent swept volume blocked (excluding set-down surface) | no motion past the last safe config |
| `hard_contact` | touchdown force exceeded the safe set-down threshold (slam) | arrest; result ≠ `success` (set-down was not soft) |
| `pose_not_reached` | (`height` mode) descent vs tolerance | MUST NOT report `success` |
| `timeout` | wall clock vs `timeout` | arrest, object kept grasped |

**Conformance test sketch.**
- **C1 — touchdown set-down + soft landing.** Grasp a bench object; command `transport.lower(stop_mode = touchdown)` toward a surface. PASS iff `result == success` ∧ the object rests on the surface (weight shed, detected) ∧ the touchdown force `≤ safe_setdown_force` (soft, no slam) ∧ the object is **still grasped** (not released) ∧ supported state recorded.
- **C2 — soft-landing under perturbed surface height.** Place the surface 10 mm higher than expected (early contact). PASS iff touchdown is detected at the true surface (descent stops on contact, `stop_mode = touchdown`) with force `≤ safe_setdown_force` — never a hard slam from descending to a pre-computed height past the real surface.

### Category 5 — `place`

`place` primitives set a held object down and (usually) release it, completing the `held → placed → free` segment of the grasp lifecycle. They are largely the canonical *composition* of `transport.lower` (soft set-down with touchdown detection) and `grasp.release` (bounded open + withdraw), and add one guarantee neither sub-step provides alone: **post-placement stability** — before releasing, the object is confirmed to be in a stably-supported state (its centre of mass projects inside its surface-contact polygon, via the supported-state predicate introduced in `grasp.release`), so the object is never abandoned in a pose from which it will tip, roll, or fall. `place` introduces no new force-dynamics axis; it reuses `transport`/`grasp` capabilities and the supported-state predicate.

#### 5.1 `place.put_down`

**Intent.** Place a held object onto a target surface and release it — lowering with touchdown detection, confirming the object is stably supported, then releasing the grasp. The composition of set-down and release, with a post-placement stability guarantee that neither sub-step provides alone.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp holding the object |
| `target_surface` | `SurfaceTarget \| auto` | `auto` | — | where to place; `auto` = the surface directly below along gravity |
| `place_pose` | `Pose6D \| auto` | `auto` | — | object pose at placement; `auto` = current orientation, lowered onto the surface |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `position_tolerance` | `Length` | `2` | mm | `> 0` |
| `touchdown_force` | `Force \| auto` | `auto` | N | set-down contact threshold (per `transport.lower`) |
| `require_stable` | `bool` | `true` | — | confirm a stably-supported state before releasing |
| `withdraw_axis` | `SignedAxis` | `−embodiment.default_tool_axis` | — | effector withdraw direction after release (per `grasp.release`) |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`; transport admissible (not `surface_bound`).
- A `target_surface` exists and is reachable; the lowering path is collision-free (excluding the surface).
- The object can rest stably on `target_surface` at `place_pose` — its CoM will project inside its surface-contact polygon (the supported-state predicate is satisfiable). If not satisfiable and `require_stable`, the placement is rejected.
- `embodiment` declares `transport` (lower) and `grasp` (release) capability.

**Postconditions (on `success`).**
- The object rests on `target_surface` at `place_pose` within `position_tolerance`, in a **stably supported state** (CoM inside its contact polygon); the grasp is released and the effector withdrawn clear.
- `GraspState` transitions `held → placed → free`; `end_effector_free = true`.
- The object did not tip, roll, or shift beyond `position_tolerance` after release (post-placement stability held).

**Safety envelope (holds throughout execution).**
- **Set-down phase** (per `transport.lower`): controlled deceleration, soft touchdown `≤ safe_setdown_force`, grasp continuity maintained until release.
- **Stability gate (the new guarantee):** before opening the grasp, the supported-state predicate is confirmed — the object is resting such that it will not fall when released. If `require_stable` and the state is not stable, the grasp is **not** released (the object is not abandoned in an unstable pose).
- **Release phase** (per `grasp.release`): bounded opening (object not flung), withdraw clear without knocking the placed object (directional force monotonicity), no-contact confirmation.
- On any breach: if before release, keep the object grasped and settle to a safe state; never release into an unstable or unconfirmed placement.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` / `transport_inadmissible` | not `held` / `surface_bound` | reject; no attempt |
| `no_surface` | `target_surface` absent / unreachable | reject; object kept grasped |
| `unstable_placement` | (`require_stable`) supported-state predicate not satisfied at `place_pose` | do **not** release; keep grasped; result ≠ `success` |
| `hard_contact` | touchdown exceeded safe set-down force | arrest; object kept grasped; result ≠ `success` |
| `withdraw_blocked` | effector withdraw corridor obstructed | release done but report; object placed |
| `object_shifted` | object tipped / moved > `position_tolerance` after release | report (post-placement instability) |
| `timeout` | wall clock vs `timeout` | keep grasped if before release; safe state |

**Conformance test sketch.**
- **C1 — nominal put-down + stability.** Grasp a bench object; command `place.put_down(target_surface)` onto a level surface. PASS iff `result == success` ∧ object rests at `place_pose` within tolerance ∧ soft touchdown (`≤ safe_setdown_force`) ∧ the supported-state predicate was confirmed **before** release ∧ `end_effector_free` ∧ the object did not shift beyond tolerance after release.
- **C2 — unstable-placement refusal.** Command `place.put_down` onto a steeply tilted surface where the object's CoM would fall outside its contact polygon. PASS iff `result == unstable_placement` ∧ the grasp was **not** released (object kept secured, not dropped onto an unstable pose) ∧ object still held.

#### 5.2 `place.stack`

**Intent.** Place a held object on top of an existing object or stack, aligned over the supporting object's top face, releasing only after confirming the **whole stack** remains stable. The stacking specialization of `place.put_down`, where the support surface is another object and stability is recursive.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp holding the object to stack |
| `support_object` | `ObjectTarget` | — (required) | — | the object / stack to place on top of |
| `alignment` | `{centered, edge_aligned, pose}` | `centered` | — | how to align over `support_object`'s top face |
| `place_pose` | `Pose6D \| auto` | `auto` | — | (for `alignment = pose`) explicit placement pose |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `position_tolerance` | `Length` | `2` | mm | `> 0`; alignment tolerance over the support |
| `touchdown_force` | `Force \| auto` | `auto` | N | set-down threshold (gentle — not to disturb the stack) |
| `require_stack_stable` | `bool` | `true` | — | confirm whole-stack stability before releasing |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`; transport admissible.
- `support_object` is resolvable with a usable top face; the stacking pose is reachable; the descent path (excluding `support_object`) is collision-free.
- **Recursive stability satisfiable:** placing the object at the aligned pose keeps (i) the object's CoM inside `support_object`'s top-face polygon AND (ii) the combined CoM of the augmented stack inside the base's support polygon. If not satisfiable and `require_stack_stable`, rejected.
- `embodiment` declares `place` with `stack` support.

**Postconditions (on `success`).**
- The object rests on `support_object`, aligned per `alignment` within `position_tolerance`; the grasp is released and effector withdrawn.
- **Whole-stack stability confirmed:** every level's CoM is within the level-below's polygon, and the stack's combined CoM is within the base polygon (no impending topple).
- `GraspState`: `held → placed → free`; the new object is recorded as the stack's new top.

**Safety envelope (holds throughout execution).**
- **Gentle set-down on a stack:** touchdown force kept low enough not to disturb / topple the existing stack; the descent does not laterally load the stack.
- **Recursive stability gate:** before release, confirm the augmented stack's stability at **every** level (not just the top object's local rest) — the supported-state predicate applied recursively. If unstable and `require_stack_stable`, do not release.
- **Alignment verification:** the achieved alignment over `support_object` is within `position_tolerance` before release; a misaligned object (overhang risking topple) is not released.
- Release per `grasp.release`, withdrawing without nudging the stack.
- On any breach: keep the object grasped; never release onto a misaligned or unstable stack; never topple the existing stack.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` / `transport_inadmissible` | not `held` / `surface_bound` | reject; no attempt |
| `no_support_object` | `support_object` unresolved / no usable top face | reject; object kept grasped |
| `misaligned` | alignment over support outside `position_tolerance` | do not release; keep grasped; result ≠ `success` |
| `stack_unstable` | (`require_stack_stable`) recursive stability fails at some level | do not release; keep grasped; result ≠ `success` |
| `stack_disturbed` | existing stack shifted during set-down | arrest; keep grasped; report |
| `hard_contact` | touchdown exceeded gentle set-down force | arrest; keep grasped; result ≠ `success` |
| `timeout` | wall clock vs `timeout` | keep grasped if before release |

**Conformance test sketch.**
- **C1 — nominal stack + recursive stability.** Stack a bench object onto a fixtured base object; command `place.stack(support_object, alignment = centered)`. PASS iff `result == success` ∧ object centered over the support within tolerance ∧ recursive stability confirmed (every level's CoM inside the level-below polygon, combined CoM inside base) **before** release ∧ the existing stack was not disturbed ∧ `end_effector_free`.
- **C2 — unstable-stack refusal.** Command `place.stack` with an offset that would put the combined CoM outside the base polygon (topple). PASS iff `result == stack_unstable` (or `misaligned`) ∧ the object was **not** released ∧ the existing stack was not toppled.

#### 5.3 `place.insert_loose`

**Intent.** Insert a held object into a container with clearance — the object is smaller than the opening, so insertion is geometric (drop-in), not force-fitted — confirming the object is contained (inside, not jammed or protruding) before releasing. The clearance counterpart of `force.insert_fit`, which handles tolerance fits.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp holding the object |
| `container` | `ObjectTarget` | — (required) | — | the container / receptacle to insert into (opening + interior) |
| `insertion_pose` | `Pose6D \| auto` | `auto` | — | object pose for insertion; `auto` = aligned with the container opening |
| `insertion_depth` | `Length \| auto` | `auto` | mm | how far to insert; `auto` = until contained / resting |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `position_tolerance` | `Length` | `3` | mm | `> 0`; looser (clearance fit) |
| `jam_force` | `Force \| auto` | `auto` | N | contact force indicating a jam (wall collision), not a fit |
| `require_contained` | `bool` | `true` | — | confirm the object is contained before releasing |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`; transport admissible.
- `container` is resolvable with an opening whose clearance exceeds the object's insertion cross-section (it is a loose fit, not a tolerance fit — else use `force.insert_fit`).
- The insertion approach is reachable and collision-free up to the opening.
- `embodiment` declares `place` with `insert_loose` support.

**Postconditions (on `success`).**
- The object is inside `container` to `insertion_depth` (or resting), **contained** — fully within the container envelope, not protruding or jammed.
- If released: the grasp is released and withdrawn; the container supports the object. If the contained pose is not stable and `require_contained` / `require_stable`, the object is kept grasped.
- `GraspState`: `held → placed → free` (if released).

**Safety envelope (holds throughout execution).**
- **Low-force insertion:** because the fit is loose, insertion should encounter no significant resistance; a contact force exceeding `jam_force` indicates a wall collision (misalignment), **not** a seating force — the envelope treats it as a jam and halts (does not force the object in).
- **Containment gate:** before release, confirm the object is contained (inside the opening envelope, not protruding). If not contained and `require_contained`, do not release.
- Release per `grasp.release`, withdrawing clear of the container rim.
- On any breach (jam): retract slightly and re-align, or report; never force a jammed object deeper (that is `force.insert_fit` territory, with its own force budget).

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` / `transport_inadmissible` | not `held` / `surface_bound` | reject; no attempt |
| `no_container` | `container` unresolved / opening too small (tolerance fit) | reject (recommend `force.insert_fit`); object kept grasped |
| `jammed` | contact force > `jam_force` during insertion (wall collision) | halt; do not force; retract / report; result ≠ `success` |
| `not_contained` | (`require_contained`) object protrudes / not inside before release | do not release; keep grasped; result ≠ `success` |
| `unreachable` | IK on insertion approach | no insertion |
| `timeout` | wall clock vs `timeout` | keep grasped if before release |

**Conformance test sketch.**
- **C1 — nominal loose insertion + containment.** Insert a bench object into a clearance container; command `place.insert_loose(container)`. PASS iff `result == success` ∧ object contained within the container envelope (not protruding) ∧ insertion encountered no force exceeding `jam_force` (loose, low-force) ∧ containment confirmed before release ∧ `end_effector_free`.
- **C2 — jam detection.** Mis-position the container so the object contacts the rim / wall. PASS iff `result == jammed` ∧ the object was **not** forced in (contact force bounded at `jam_force`) ∧ object kept grasped (retracted / reported, not jammed deeper).

#### 5.4 `place.orient`

**Intent.** Place a held object onto a surface in a required orientation (label-up, port-out, terminal-up, etc.), verifying the placed orientation, then releasing. The orientation-constrained specialization of `place.put_down`.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp holding the object |
| `target_surface` | `SurfaceTarget \| auto` | `auto` | — | where to place |
| `required_orientation` | `OrientationSpec` | — (required) | — | the orientation the placed object must have (an object axis → world direction mapping, e.g. `label_normal → up`) |
| `orientation_tolerance` | `Angle` | `3` | deg | `> 0`; on the placed orientation |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `position_tolerance` | `Length` | `2` | mm | `> 0` |
| `require_stable` | `bool` | `true` | — | confirm stable support before release (per `put_down`) |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`; transport admissible.
- **The required orientation is achievable from the current grasp:** the held object can be brought to `required_orientation` at the placement pose by the embodiment's reachable wrist / arm range. If it cannot (the grasp presents the object such that the required orientation is unreachable), `place.orient` is **rejected** with a recommendation to reorient first (`in_hand.rotate` / `regrasp`) — `place.orient` does not silently reorient; reorientation is a separate, explicit composition step (single-responsibility).
- `target_surface` reachable; the object rests stably at the required orientation (supported-state predicate).
- `embodiment` declares `place` with `orient` support.

**Postconditions (on `success`).**
- The object rests on `target_surface` at `required_orientation` within `orientation_tolerance` and `position_tolerance`, in a stably supported state; grasp released, effector withdrawn.
- `GraspState`: `held → placed → free`.
- The placed orientation was **verified** (not assumed) before release.

**Safety envelope (holds throughout execution).**
- Set-down + stability + release per `place.put_down` (soft touchdown, supported-state gate, bounded release).
- **Orientation verification gate:** before release, the achieved object orientation is confirmed within `orientation_tolerance` of `required_orientation`. If the orientation is wrong, **do not release** (releasing a mis-oriented object defeats the primitive's purpose).
- The required orientation must also be a stable resting orientation (an object cannot be released label-up if it would immediately topple from that orientation) — both the orientation gate and the stability gate must pass.
- On any breach: keep the object grasped; never release mis-oriented or unstable.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` / `transport_inadmissible` | not `held` / `surface_bound` | reject; no attempt |
| `orientation_unreachable` | required orientation not achievable from the current grasp | reject; recommend reorient-first (`in_hand` / `regrasp`); object kept grasped |
| `orientation_unstable` | required orientation is not a stable resting pose | reject; object kept grasped |
| `orientation_not_verified` | achieved orientation outside tolerance before release | do not release; keep grasped; result ≠ `success` |
| `unstable_placement` | supported-state predicate fails | do not release; keep grasped |
| `timeout` | wall clock vs `timeout` | keep grasped if before release |

**Conformance test sketch.**
- **C1 — nominal oriented placement.** Grasp a bench object with a marked face; command `place.orient(target_surface, required_orientation = marked_face_up)`. PASS iff `result == success` ∧ the placed object's marked face is up within `orientation_tolerance` (externally measured) ∧ stably supported ∧ orientation verified **before** release ∧ `end_effector_free`.
- **C2 — unreachable-orientation rejection.** Grasp the object such that the required orientation cannot be reached from the current grasp; command `place.orient`. PASS iff `result == orientation_unreachable` ∧ the object was **not** released in the wrong orientation ∧ a reorient-first recommendation is reported.

#### 5.5 `place.hand_to`

**Intent.** Hand a held object to a human, releasing it only upon detecting that the human has taken its weight — a controlled, human-safe release governed by weight-transfer detection rather than make-before-break (the human's grasp cannot be robot-confirmed in advance). The human-recipient counterpart of `transport.handoff`.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp holding the object |
| `handover_pose` | `Pose6D \| auto` | `auto` | — | pose to present the object to the human; `auto` = a reachable, ergonomic offer pose |
| `weight_transfer_threshold` | `Ratio` | `auto` | — | fraction of the object's weight the human must take before release; `auto` = embodiment default (e.g. ≥ 0.5) |
| `max_interaction_force` | `Force \| auto` | `auto` | N | hard cap on force exchanged with the human (human-safety limit, per ISO 10218 / 13482 context) |
| `present_timeout` | `Duration` | — (required) | s | how long to hold the offer before giving up |
| `on_no_take` | `{retain, retract}` | `retain` | — | if the human never takes it: keep holding, or withdraw to a safe pose |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`; transport admissible.
- A human recipient is present and a `handover_pose` is reachable that presents the object ergonomically (the human side is perception-derived and uncertain — approach is gentle, contact-based).
- `embodiment` declares `place` with `hand_to` support **and** human-collaboration safety capability (force-limited interaction, per the deployment's safety standard).

**Postconditions (on `success`).**
- The human has taken the object's weight (≥ `weight_transfer_threshold`); the robot has released and withdrawn clear; `GraspState`: `held → free`.
- No force exceeding `max_interaction_force` was exchanged with the human at any point.
- The object was not dropped (release happened only after weight transfer was confirmed).

**Safety envelope (holds throughout execution — human-safety critical).**
- **Weight-transfer-gated release:** the grasp opens **only after** the human is detected to support ≥ `weight_transfer_threshold` of the weight (the robot's borne load drops correspondingly). Until then, the object stays grasped — the robot never releases into the air hoping the human catches it.
- **Human-force cap:** force exchanged with the human never exceeds `max_interaction_force` — the robot does not pull, push, or resist the human's hand beyond this limit; if the human tugs, the robot yields (compliant), it does not fight.
- **No-take safety:** if `present_timeout` elapses with no weight transfer, execute `on_no_take` (retain the object held, or retract to a safe pose) — never drop the object, never leave it half-released.
- Gentle, contact-based approach to the (uncertain) human hand; abort the approach on unexpected contact above the human-force cap.
- On any breach: yield to the human, keep the object secured, settle to a safe state.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` / `transport_inadmissible` | not `held` / `surface_bound` | reject; no attempt |
| `capability_absent` | no human-collaboration safety capability | reject (must not hand to a human without force-limited safety) |
| `no_recipient` | no reachable human / ergonomic offer pose | reject; object kept grasped |
| `not_taken` | `present_timeout` elapsed, weight not transferred | execute `on_no_take`; object never dropped |
| `overforce_abort` | interaction force exceeded `max_interaction_force` | yield; keep secured; report (human-safety event) |
| `premature_release_blocked` | weight not yet transferred when release was due | release **withheld**; object kept grasped (this guard is the point) |
| `timeout` | wall clock vs `timeout` | `on_no_take`; object secured |

**Conformance test sketch.**
- **C1 — weight-transfer release (instrumented dummy hand).** Present a bench object to an instrumented recipient (a force / weight-sensing dummy hand); command `place.hand_to`. PASS iff `result == success` ∧ the grasp opened **only after** the dummy took ≥ `weight_transfer_threshold` of the weight (force trace shows robot load dropping before release) ∧ no exchanged force exceeded `max_interaction_force` ∧ object not dropped.
- **C2 — no-take safety.** Present the object but never take it (no weight transfer) until `present_timeout`. PASS iff `result == not_taken` ∧ the object was **never released** (executed `on_no_take = retain` or `retract`) ∧ no object on the floor.

#### 5.6 `place.discard`

**Intent.** Release a held object into a target region without a precise final pose — dropping it into a bin, hopper, or coarse area — guaranteeing the object lands within a designated discard zone and that the drop is safe, while deliberately relaxing the precise-placement and final-pose guarantees of the other `place` primitives.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp holding the object |
| `discard_zone` | `Region` | — (required) | — | the region the object must land within (bin interior, hopper, coarse area) |
| `release_pose` | `Pose6D \| auto` | `auto` | — | pose from which to release; `auto` = a pose over `discard_zone` minimizing drop height |
| `max_drop_height` | `Length \| auto` | `auto` | mm | cap on release height above the landing surface (limit impact) |
| `safe_impact` | `bool` | `true` | — | require the predicted impact to be non-damaging (drop height + object / zone tolerate the impact) |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`; transport admissible.
- A `release_pose` exists over `discard_zone` from which the released object will land **within** the zone (ballistic / drop prediction), reachable and collision-free.
- If `safe_impact`: the predicted drop (from `release_pose`, ≤ `max_drop_height`) is non-damaging to the object and the zone (e.g. not a fragile object dropped from height onto a hard floor).
- `embodiment` declares `place` with `discard` support.

**Postconditions (on `success`).**
- The object was released and landed **within `discard_zone`**; the grasp is released; `GraspState`: `held → free`.
- The final pose of the object is **not** guaranteed (it may roll / settle arbitrarily within the zone) — this is the intended relaxation.
- If `safe_impact`: the impact was within safe bounds (no damage).

**Safety envelope (holds throughout execution).**
- **Zone containment (the retained guarantee):** the release is performed only from a pose whose drop prediction lands the object inside `discard_zone`. Precise final pose is relaxed; **landing within the zone is not** — the object does not become a projectile outside the zone (same philosophy as `in_hand.flip`'s `safe_drop_zone`).
- **Safe impact:** drop height `≤ max_drop_height`; if `safe_impact`, the predicted impact is non-damaging — a fragile object is not discarded from a height that would shatter it.
- Release per `grasp.release` opening dynamics (object not flung beyond the zone by the release itself).
- On any breach (cannot guarantee zone containment or safe impact): do **not** release; keep the object grasped and report (a discard that cannot be contained is refused).

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` / `transport_inadmissible` | not `held` / `surface_bound` | reject; no attempt |
| `no_discard_zone` | `discard_zone` absent / unreachable | reject; object kept grasped |
| `containment_unassured` | no `release_pose` whose drop lands within the zone | do not release; keep grasped; result ≠ `success` |
| `unsafe_impact` | (`safe_impact`) predicted drop would damage object / zone | do not release; keep grasped; result ≠ `success` |
| `landed_outside_zone` | object landed outside `discard_zone` | report (containment failure) |
| `timeout` | wall clock vs `timeout` | keep grasped if not released |

**Conformance test sketch.**
- **C1 — nominal discard + containment.** Grasp a bench object; command `place.discard(discard_zone = bin)`. PASS iff `result == success` ∧ the object landed **within** `discard_zone` (externally measured) ∧ drop height `≤ max_drop_height` ∧ `end_effector_free` ∧ (final object pose is **not** checked — relaxation is intended).
- **C2 — fragile / unsafe-impact refusal.** Command `place.discard(safe_impact = true)` for a fragile object with only a high release pose available (hard floor, damaging impact). PASS iff `result == unsafe_impact` ∧ the object was **not** released (kept grasped) — discard does not become a way to smash fragile objects.

### Category 6 — `force`

`force` primitives make contact force the *objective*, not merely a constraint. Where `reach` forms no contact, and `grasp`/`transport` *maintain* a holding force, `force` primitives *actively apply and control* contact force — the third force-dynamics axis. This is where embodiment heterogeneity bites hardest (the white paper's reason for making `force` the largest category): the same "insert this connector" intent resolves to bounded joint torques on a tendon hand, a pneumatic pressure profile on a bellows hand, and a jaw-force/impedance schedule on a parallel-jaw gripper. Four conventions hold across the category:

- **Force-trajectory bound.** The safety envelope bounds the *force profile over the whole motion* (interval-sampled), not a single endpoint — a mid-motion force spike (e.g. a jam) is an envelope violation, not a success signal. This is the force-axis analogue of `reach`'s velocity envelope and `transport`'s inertial envelope.
- **Compliance is first-class.** Most `force` primitives require a declared compliance capability (passive / active / virtual force control); a rigid-only embodiment cannot run them. Compliance is requested in the parameters and asserted as a precondition.
- **Outcome discrimination by force-at-state.** Success vs failure is read from force *in context* — "force rise at the expected depth" is seating; "force rise without depth" is a jam. A bare force threshold is never sufficient.
- **Held throughout.** `force` primitives operate on a held part (or a tool) and do not release it; releasing/regrasping is a separate step. They also inherit a **grasp-under-reaction-load** condition: the contact reaction must not exceed the grasp's holding capacity, or the part slips before the task completes.

The determinism boundary from `reach.scan` / `in_hand.pivot` applies throughout: `retarget` deterministically *generates* the canonical action, but compliant search executes against contact dynamics, so the realized trajectory is not byte-for-byte reproducible.

#### 6.1 `force.insert_fit`

**Intent.** Insert a held object into a tolerance fit (peg-in-hole, connector mating) using compliant force-controlled search and seating — feeling for alignment and pushing to a confirmed seated state within a force budget — where clearance is too small for geometric drop-in. The force-controlled counterpart of `place.insert_loose`.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp holding the part being inserted |
| `target_fit` | `FeatureRef` | — (required) | — | the hole / socket / receptacle (tolerance fit) |
| `insertion_axis` | `Direction \| auto` | `auto` | — | nominal insertion direction, in `frame`; `auto` = planner-derived from `target_fit` |
| `force_budget` | `Force` | — (required) | N | max force along `insertion_axis` (over-insertion / pin damage limit) |
| `lateral_force_budget` | `Force \| auto` | `auto` | N | max lateral force during search (cocking / side-load limit) |
| `stop_condition` | `StopCondition` | — (required) | — | what defines "seated", a `StopCondition` (§ Stop / completion conditions): `effort_rise(F)`, `reached(depth d)`, or `all_of{effort_rise(F), reached(depth d)}` — the conjunction is the seating/jam discriminator |
| `search_strategy` | `{spiral, tilt, hop, auto}` | `auto` | — | compliant-search pattern to find alignment; resolved by the Translation Layer |
| `compliance` | `{passive, active, auto}` | `auto` | — | compliance mode required for the fit |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState`; the held part's grasp can withstand the insertion reaction force (`force_budget` `≤` grasp holding capacity along the insertion axis — else the part slips in the grasp before seating).
- `target_fit` is resolvable; the part and fit are a tolerance fit (clearance below the loose-fit threshold — else use `place.insert_loose`).
- `embodiment` declares `force` with `insert_fit` support **and** the required `compliance` capability (passive / active / VFC).
- The pre-insertion pose aligns the part with `target_fit` within the search-capturable range.

**Postconditions (on `success`).**
- The part is seated in `target_fit` per `stop_condition` (force rise and / or depth reached); the mate is complete.
- Throughout, axial force stayed `≤ force_budget` and lateral force `≤ lateral_force_budget` (no over-insertion, no damaging side-load).
- The held part did not slip in the grasp beyond tolerance; `GraspState` remains `held` (insertion does not release — release / regrasp is a separate step).

**Safety envelope (holds throughout execution — force trajectory bound).**
- **Force-trajectory bound (the new axis):** at every instant, axial force `≤ force_budget` and lateral force `≤ lateral_force_budget`. These are *trajectory* bounds (held through the search-and-push profile), not a single endpoint check — a force spike mid-insertion (jam) is an envelope violation, not a seating signal.
- **Compliant search:** misalignment is accommodated by compliance (the part gives laterally rather than cocking / jamming); a rising lateral force beyond budget indicates a cocked / jammed insertion and aborts (do not force a jammed fit).
- **Seating discrimination:** the `stop_condition` (force rise at depth) distinguishes true seating from a jam — a force rise *without* the expected depth is a jam, not a seat.
- Grasp continuity under reaction load: the insertion reaction must not exceed the grasp's holding capacity (else the part slips); monitored throughout.
- On any breach (jam, over-force, grasp slip): retract along `−insertion_axis` to a safe, unloaded pose; do not leave the part jammed under load.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt |
| `capability_absent` | no `insert_fit` / required compliance | reject; no attempt |
| `wrong_primitive` | clearance is a loose fit (not tolerance) | reject (recommend `place.insert_loose`) |
| `axial_overforce` | axial force > `force_budget` without seating | retract; result ≠ `success` (do not force in) |
| `jammed` | lateral force > budget / force rise without depth (cocked) | retract; re-search or report; result ≠ `success` |
| `grasp_slip_under_load` | part slipped in grasp under reaction force | retract; re-secure; result ≠ `success` |
| `not_seated` | `stop_condition` not met within range | retract; result ≠ `success` |
| `timeout` | wall clock vs `timeout` | retract to safe unloaded pose |

**Conformance test sketch.**
- **C1 — nominal fit + seating.** Present a bench peg-in-hole (tolerance fit); command `force.insert_fit(target_fit, insertion_axis, force_budget = 10 N, stop_condition = force_and_depth(...))`. PASS iff `result == success` ∧ the part is seated (force rise at the expected depth) ∧ the **force trajectory** stayed within `force_budget` and `lateral_force_budget` at **every** sampled instant (interval sampling) ∧ no in-grasp slip ∧ retractable to an unloaded state.
- **C2 — jam detection (no over-force).** Mis-align so the part cocks in the hole. PASS iff `result == jammed` ∧ the force trajectory never exceeded the budgets (the part was **not** forced in past the jam) ∧ the part retracted to a safe unloaded pose — never a forced-through or stuck-under-load outcome.

#### 6.2 `force.push`

**Intent.** Apply a controlled directional force against a target — to hold, brace, press, or stabilize it — driving the contact force to a target without displacing the target beyond a threshold. The static-force counterpart of motion-producing force primitives.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `controlled_frame` | `FrameRef` | `embodiment.default_tool_axis` frame | — | the frame / tool applying the force; may be a held part or the effector itself |
| `target` | `SurfaceTarget` | — (required) | — | surface to push against (point + inward direction) |
| `push_direction` | `Direction \| auto` | `auto` | — | direction to apply force; `auto` = `−target.normal` (into the surface) |
| `target_force` | `Force` | — (required) | N | the contact force to establish and hold |
| `max_displacement` | `Length \| auto` | `auto` | mm | max allowed target displacement; `auto` = small (push, do not move) |
| `hold_duration` | `Duration \| until` | `until` | s | how long to hold the force; `until` defers to an enclosing `reactive` |
| `compliance` | `{passive, active, auto}` | `auto` | — | required compliance mode |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- If pushing with a held part: `grasp_handle` is `held` and the grasp withstands the reaction (grasp-under-reaction-load).
- `target` is resolvable; `push_direction` is into the contact (not tangential — that is `force.wipe`).
- `embodiment` declares `force` with `push` support and the required `compliance` capability.

**Postconditions (on `success`).**
- The contact force along `push_direction` reached and held `target_force` (within tolerance) for `hold_duration` (or until the enclosing predicate fired).
- The target displaced by `≤ max_displacement` (the push applied force without moving the target beyond threshold).
- Held part (if any) retained throughout; `GraspState` unchanged.

**Safety envelope (holds throughout execution — force trajectory bound).**
- **Force-trajectory bound:** the applied force rises toward `target_force` monotonically (no overshoot beyond `target_force · (1 + transient_margin)`) and holds within tolerance; a force excursion above budget is an envelope violation.
- **Displacement bound:** the target's displacement stays `≤ max_displacement`; exceeding it means the target is moving (not just being pushed) — distinguish "pushing a fixed target" from "shoving a movable one." If the target yields beyond threshold, halt (this is not the intended static push).
- Reaction load on the grasp (if held part) `≤` holding capacity.
- For a sustained push (`hold_duration` / `reactive`): the force is held as an **interval invariant** (per `reach.hover`'s maintained-invariant class) — interval-sampled.
- On any breach: reduce force to zero along `push_direction` and retract to an unloaded pose within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `capability_absent` | no `push` / required compliance | reject; no attempt |
| `no_contact` | no surface reached to push against | result ≠ `success` (nothing to push) |
| `force_overshoot` | applied force > `target_force · (1 + margin)` | reduce force; result ≠ `success` |
| `target_yielded` | displacement > `max_displacement` (target moved) | halt; report (target not fixed) |
| `grasp_slip_under_load` | held part slipped under reaction | reduce force; re-secure; result ≠ `success` |
| `force_not_reached` | could not establish `target_force` (e.g. lost contact) | result ≠ `success` |
| `timeout` | wall clock vs `timeout` | reduce force to zero, retract |

**Conformance test sketch.**
- **C1 — nominal push + force hold.** Push against a bench-fixtured rigid surface; command `force.push(target, target_force = 15 N, hold_duration = 3 s)`. PASS iff `result == success` ∧ the measured contact force reached `15 N` and held within tolerance over the 3 s (interval sampling) ∧ never overshot beyond `target_force · (1 + transient_margin)` ∧ surface displacement `≤ max_displacement`.
- **C2 — yielding-target detection.** Push against a target free to move (e.g. a lightly-held object). PASS iff `result == target_yielded` (the displacement exceeded `max_displacement`, detected) ∧ the force was not driven past budget chasing a receding target.

#### 6.3 `force.pull`

**Intent.** Apply tensile force to draw a target toward the effector — extracting a cable, opening a drawer, tensioning a line — within a force budget, detecting and safely handling the breakaway moment when resistance suddenly drops (the target releases or reaches its limit).

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp on the target being pulled (pulling requires a grip / hook) |
| `pull_direction` | `Direction` | — (required) | — | direction of tensile force, in `frame` |
| `force_budget` | `Force` | — (required) | N | max tensile force to apply |
| `stop_condition` | `PullStop` | — (required) | — | `distance(d)`, `breakaway` (resistance drops), or `tension(F)` |
| `breakaway_response` | `{arrest, continue}` | `arrest` | — | on a sudden resistance drop: stop immediately (default), or keep moving |
| `compliance` | `{passive, active, auto}` | `auto` | — | required compliance mode |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState` on the target — pulling requires a secured grip / hook (you cannot pull what you do not hold).
- **The grasp withstands tensile reaction:** `force_budget ≤` the grasp's holding capacity along `pull_direction` — pulling loads the grasp in its most slip-prone direction; if the budget exceeds capacity, the grasp slips before the target moves.
- `embodiment` declares `force` with `pull` support and the required `compliance`.

**Postconditions (on `success`).**
- Tensile force was applied along `pull_direction` until `stop_condition` was met (distance reached, breakaway detected, or tension reached), with force `≤ force_budget` throughout.
- On `breakaway`: the resistance drop was detected and handled per `breakaway_response` (default: arrested promptly, no follow-through lurch).
- The target remained gripped throughout (no grasp slip); `GraspState` unchanged.

**Safety envelope (holds throughout execution — force trajectory bound).**
- **Force-trajectory bound:** tensile force `≤ force_budget` at every instant; exceeding it (e.g. pulling against a stuck target) is an envelope violation — do not yank.
- **Breakaway handling:** a sudden resistance drop (target released / limit reached) is detected; per `breakaway_response = arrest`, motion stops promptly so the effector does not lurch forward when resistance vanishes (and does not overshoot into the workspace).
- **Grasp continuity under tension:** holding force is maintained against the tensile load; `grasp_slip_under_load` (the dominant pull failure) aborts the pull.
- On any breach: reduce tensile force to zero and arrest within `embodiment.limits.stop_time`, target still gripped.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt (cannot pull without a grip) |
| `capability_absent` | no `pull` / required compliance | reject; no attempt |
| `force_budget_exceeded` | required force > `force_budget` (stuck target) | arrest; result ≠ `success` (do not yank) |
| `grasp_slip_under_load` | target slipped from grip under tension | arrest; re-secure if possible; result ≠ `success` |
| `unexpected_breakaway` | resistance dropped before `stop_condition` (target broke / released early) | arrest per `breakaway_response`; report |
| `stop_unreachable` | `stop_condition` not met within range / budget | arrest; result ≠ `success` |
| `timeout` | wall clock vs `timeout` | reduce tension to zero, target gripped |

**Conformance test sketch.**
- **C1 — nominal pull-to-distance + force bound.** Grip a bench drawer handle; command `force.pull(pull_direction, force_budget = 20 N, stop_condition = distance(100 mm))`. PASS iff `result == success` ∧ the target moved 100 mm along `pull_direction` ∧ tensile force stayed `≤ force_budget` at every sampled instant (interval sampling) ∧ no grasp slip under load.
- **C2 — breakaway arrest.** Grip a connector with a known extraction force; command `force.pull(stop_condition = breakaway, breakaway_response = arrest)`. PASS iff `result == success` ∧ the breakaway (resistance drop at extraction) was detected ∧ the effector **arrested promptly without lurching forward** past a small post-breakaway tolerance ∧ force never exceeded `force_budget`.

#### 6.4 `force.screw`

**Intent.** Drive a threaded fastener (or threaded part) into a mating thread by coupled rotation and axial advance — turning within a torque budget while feeding axially at the thread pitch — to a confirmed tight / seated state, detecting cross-threading.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp on the fastener, or on the tool driving it (tool-mediated) |
| `thread_axis` | `Direction` | — (required) | — | the screw / thread axis, in `frame` |
| `torque_budget` | `Torque` | — (required) | N·m | max torque about `thread_axis` (thread-strip / fastener-break limit) |
| `force_budget` | `Force \| auto` | `auto` | N | max axial seating force |
| `thread_pitch` | `Length \| auto` | `auto` | mm/rev | couples rotation to advance; `auto` = from `target_fit` thread spec |
| `completion` | `ScrewStop` | — (required) | — | `torque_rise(T)` (tight), `turns(n)`, or `torque_and_advance(T,d)` |
| `tool_mediated` | `bool` | `auto` | — | whether a held tool (driver) transmits the torque; `auto` = inferred from grasp |
| `compliance` | `{passive, active, auto}` | `auto` | — | required compliance mode |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState` on the fastener or driving tool; the grasp transmits the required torque without slip (torque reaction `≤` grasp's rotational holding capacity).
- `thread_axis` is aligned with the mating thread within the start-capturable range (the first thread can engage).
- `embodiment` declares `force` with `screw` support and the required `compliance`; if `tool_mediated`, a compatible tool is grasped.

**Postconditions (on `success`).**
- The fastener advanced along `thread_axis` coupled to rotation at `thread_pitch`, reaching `completion` (tight torque rise, turn count, or torque-at-advance).
- Torque stayed `≤ torque_budget` and axial force `≤ force_budget` throughout (no thread strip, no fastener break).
- No cross-threading occurred; the grasp / tool transmitted torque without slip; `GraspState` unchanged.

**Safety envelope (holds throughout execution — torque + force trajectory bound).**
- **Torque-trajectory bound (the new axis):** torque about `thread_axis` `≤ torque_budget` at every instant; axial force `≤ force_budget`. Both are trajectory bounds — a torque spike without the expected advance is cross-threading, not seating.
- **Cross-threading discrimination:** torque rising **without** axial advance (per pitch) indicates cross-threading or a jam — abort and back off; torque rising **at** the seated advance is correct tightening. (The screw analogue of `force.insert_fit`'s seating / jam rule.)
- **Coupled-motion constraint:** rotation and advance stay coupled at `thread_pitch`; a decoupling (advancing without turning, or turning without advancing) signals stripped threads or disengagement.
- Grasp / tool continuity under torque reaction; tool not dropped or slipped.
- On any breach (cross-thread, over-torque, strip): stop turning, back off slightly to relieve load (do not continue driving a cross-threaded fastener), report.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt |
| `capability_absent` | no `screw` / required compliance / tool absent | reject; no attempt |
| `cross_threaded` | torque rise without advance (early, off-pitch) | stop; back off; result ≠ `success` (do not drive through) |
| `over_torque` | torque > `torque_budget` before completion | stop; back off; result ≠ `success` |
| `stripped` | advance without torque / decoupled motion | stop; report (threads stripped) |
| `tool_slip` | grasp / tool slipped under torque reaction | stop; re-secure; result ≠ `success` |
| `not_seated` | `completion` not reached within range | back off; result ≠ `success` |
| `timeout` | wall clock vs `timeout` | stop turning, relieve load |

**Conformance test sketch.**
- **C1 — nominal drive + tight seating.** Present a bench threaded fastener + mating thread; command `force.screw(thread_axis, torque_budget = 2 N·m, completion = torque_rise(...))`. PASS iff `result == success` ∧ the fastener seated tight (torque rose at the seated advance) ∧ torque and axial force stayed within budgets at every sampled instant (interval sampling) ∧ rotation-advance stayed coupled at `thread_pitch` ∧ no tool slip.
- **C2 — cross-thread detection.** Start the fastener mis-aligned so it cross-threads. PASS iff `result == cross_threaded` (torque rise without proper advance detected) ∧ the fastener was **not** driven through (torque never exceeded budget chasing a cross-thread) ∧ backed off to relieve load.

#### 6.5 `force.unscrew`

**Intent.** Extract a threaded fastener by reverse coupled rotation and axial retreat — overcoming the initial breakaway torque, turning out at the thread pitch within a torque budget, and detecting full disengagement — then handling the now-freed fastener.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp on the fastener, or on the tool driving it |
| `thread_axis` | `Direction` | — (required) | — | the thread axis, in `frame` |
| `torque_budget` | `Torque` | — (required) | N·m | max loosening torque (tool-strip / fastener-break / break-loose limit) |
| `thread_pitch` | `Length \| auto` | `auto` | mm/rev | couples reverse rotation to retreat |
| `completion` | `ScrewStop` | `disengagement` | — | `disengagement` (fully out), `turns(n)`, or `torque_drop` |
| `on_disengagement` | `{retain, drop_safe}` | `retain` | — | when the fastener frees: keep it gripped, or release into a safe zone |
| `tool_mediated` | `bool` | `auto` | — | whether a held tool transmits the torque |
| `compliance` | `{passive, active, auto}` | `auto` | — | required compliance mode |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState` on the fastener or driving tool; the grasp transmits the loosening torque without slip.
- `thread_axis` is engaged with the fastener (the tool / grip is seated on it).
- `embodiment` declares `force` with `unscrew` support and the required `compliance`.

**Postconditions (on `success`).**
- The fastener turned out (reverse rotation coupled to retreat at `thread_pitch`), reaching `completion` (full disengagement, turn count, or torque drop).
- Torque stayed `≤ torque_budget` throughout, including the initial breakaway peak.
- On disengagement: the freed fastener was handled per `on_disengagement` (retained in grip, or released into a safe zone) — **not dropped uncontrolled**.
- The grasp / tool transmitted torque without slip.

**Safety envelope (holds throughout execution — torque trajectory bound).**
- **Torque-trajectory bound:** loosening torque `≤ torque_budget` at every instant. The **initial breakaway peak** (highest torque, to overcome static friction + preload) is the riskiest instant — if it exceeds budget, the fastener is seized (do not over-torque past the tool / fastener limit chasing it).
- **Coupled reverse motion:** reverse rotation couples to axial retreat at `thread_pitch`; decoupling signals stripping.
- **Disengagement handling (the asymmetry vs screw):** at full disengagement the fastener becomes free and could fall; the envelope guarantees the freed fastener is retained or released into a safe zone per `on_disengagement` — never an uncontrolled drop. (Reuses `force.pull`'s breakaway detection for the final-thread release.)
- Grasp / tool continuity under torque reaction.
- On any breach (seized, over-torque, strip): stop turning, relieve load, report; do not exceed the torque budget on a seized fastener.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt |
| `capability_absent` | no `unscrew` / required compliance / tool absent | reject; no attempt |
| `seized` | breakaway torque > `torque_budget` (fastener will not loosen) | stop; relieve load; result ≠ `success` (do not over-torque) |
| `over_torque` | torque > `torque_budget` mid-extraction | stop; report |
| `stripped` | reverse rotation without retreat (decoupled) | stop; report (threads / head stripped) |
| `tool_slip` | grasp / tool slipped under torque | stop; re-secure; result ≠ `success` |
| `disengagement_uncontrolled` | fastener freed but not retained / safe-released | guarded against by design; if detected, report (the guard exists to prevent this) |
| `timeout` | wall clock vs `timeout` | stop turning, relieve load |

**Conformance test sketch.**
- **C1 — nominal extraction + controlled disengagement.** Present a bench fastener torqued to a known value; command `force.unscrew(thread_axis, torque_budget = 3 N·m, completion = disengagement, on_disengagement = retain)`. PASS iff `result == success` ∧ the fastener fully disengaged (reverse rotation coupled to retreat) ∧ torque stayed `≤ torque_budget` including the breakaway peak (interval sampling) ∧ at disengagement the fastener was **retained in grip** (not dropped) ∧ no tool slip.
- **C2 — seized-fastener refusal.** Present a fastener torqued beyond `torque_budget` to break loose. PASS iff `result == seized` ∧ the torque **never exceeded `torque_budget`** (the tool / fastener was not over-stressed chasing a seized fastener) ∧ load relieved.

#### 6.6 `force.press_button`

**Intent.** Press a button, switch, or key — producing a sub-millimeter displacement until the actuation event (force detent / click, or force threshold) is detected — then releasing the press without over-travel that would damage the mechanism.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `controlled_frame` | `FrameRef` | effector / held tool | — | the frame pressing (fingertip, held stylus, etc.) |
| `target` | `SurfaceTarget` | — (required) | — | the button surface (point + press direction) |
| `press_direction` | `Direction \| auto` | `auto` | — | `auto` = `−target.normal` (into the button) |
| `actuation` | `ActuationSpec` | — (required) | — | what marks actuation: `detent` (force rise-then-drop / click), or `force_threshold(F)` |
| `force_budget` | `Force` | — (required) | N | max press force (over-travel / mechanism-damage limit) |
| `max_travel` | `Length \| auto` | `auto` | mm | max displacement (over-travel guard); `auto` = small |
| `release_after` | `bool` | `true` | — | release the press after actuation (momentary), or hold |
| `compliance` | `{passive, active, auto}` | `auto` | — | required compliance mode |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- If pressing with a held tool: `grasp_handle` is `held` and withstands the reaction.
- `target` (button) is resolvable; `press_direction` is into the button.
- `embodiment` declares `force` with `press_button` support and fine force resolution (or proxy) sufficient to detect the actuation.

**Postconditions (on `success`).**
- The actuation event was detected (detent click or force threshold reached); the button was actuated.
- Press force stayed `≤ force_budget` and travel `≤ max_travel` (no over-travel / mechanism damage).
- If `release_after`: the press was released (button allowed to return); else the press is held.

**Safety envelope (holds throughout execution — force trajectory bound).**
- **Force-trajectory bound:** press force `≤ force_budget` at every instant; travel `≤ max_travel`.
- **Actuation-event detection + over-travel guard:** on detecting actuation (detent: force rise-then-drop; or threshold crossed), stop advancing immediately — do **not** continue pressing past the actuation point (over-travel damages the mechanism). The actuation event, not a fixed depth, ends the press.
- Distinguish a real actuation (detent signature / threshold at expected travel) from bottoming-out (force rise without a detent, hitting the travel limit) — the latter is a missed / stuck button, not an actuation.
- On any breach: withdraw along `−press_direction` to an unloaded pose; do not crush the button.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `capability_absent` | no `press_button` / insufficient force resolution | reject; no attempt |
| `no_contact` | button surface not reached | result ≠ `success` |
| `no_actuation` | `max_travel` / `force_budget` reached without an actuation event | withdraw; result ≠ `success` (button not actuated) |
| `over_travel` | travel exceeded `max_travel` (no detent, bottoming out) | withdraw; result ≠ `success` |
| `force_exceeded` | press force > `force_budget` | withdraw; result ≠ `success` |
| `grasp_slip_under_load` | held tool slipped | withdraw; re-secure; result ≠ `success` |
| `timeout` | wall clock vs `timeout` | withdraw to unloaded pose |

**Conformance test sketch.**
- **C1 — nominal press + actuation detection.** Press a bench button with a known detent; command `force.press_button(target, actuation = detent, force_budget = 5 N)`. PASS iff `result == success` ∧ the actuation (detent click) was detected ∧ press force `≤ force_budget` and travel `≤ max_travel` throughout (interval sampling) ∧ on `release_after`, the press was released ∧ no over-travel past the detent.
- **C2 — over-travel / no-detent guard.** Press a surface with no actuating button (a solid spot). PASS iff `result == no_actuation` (or `over_travel`) ∧ the force never exceeded `force_budget` and travel never exceeded `max_travel` (the mechanism / surface was not crushed seeking a non-existent detent).

#### 6.7 `force.cut`

**Intent.** Separate material along a cut path using a held cutting tool, applying continuous shear within a force budget until the cut is complete (path traversed or separation detected) — an **irreversible** operation demanding strict path bounding and post-separation handling.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp on the cutting tool (cut is tool-mediated) |
| `cut_path` | `Trajectory` | — (required) | — | the path along which to cut, in `frame` (bounds *exactly* where the cut goes) |
| `shear_force_budget` | `Force` | — (required) | N | max shear force (tool-damage / over-cut / kickback limit) |
| `completion` | `CutStop` | — (required) | — | `path_complete`, `separation` (resistance drop), or `depth(d)` |
| `feed_rate` | `Velocity \| auto` | `auto` | m/s | tool advance speed along the path |
| `on_separation` | `{retain, drop_safe}` | `retain` | — | handling of the freed (cut-off) part |
| `tool_mediated` | `bool` | `true` | — | always true for cut (a cutting tool is used) |
| `compliance` | `{passive, active, auto}` | `auto` | — | required compliance mode |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState` on a cutting tool; the grasp transmits shear without slip and withstands kickback.
- `cut_path` is fully resolvable and bounded; the path and the material beyond it are confirmed (the cut goes exactly where intended — **irreversibility demands this**).
- `embodiment` declares `force` with `cut` support, the required `compliance`, and the cutting tool's safety capability (per the deployment's safety standard — cutting tools are hazardous).

**Postconditions (on `success`).**
- The material was separated along `cut_path` per `completion` (path traversed, separation detected, or depth reached).
- Shear force stayed `≤ shear_force_budget` throughout; the cut did **not** extend beyond `cut_path` (bounded separation).
- The freed part was handled per `on_separation` (retained / safe-released) — not dropped or flung.
- The tool is withdrawn to a safe, non-hazardous pose.

**Safety envelope (holds throughout execution — force trajectory bound + irreversibility).**
- **Force-trajectory bound:** shear force `≤ shear_force_budget` at every instant; a spike (hitting a hard inclusion, kickback) is an envelope violation — stop, do not force the cut.
- **Path bounding (irreversibility guard):** the cut follows `cut_path` exactly; the tool does **not** stray beyond the path (an over-cut is irreversible). Lateral deviation from the path beyond tolerance aborts.
- **Separation handling:** at separation (resistance drop, per `force.pull` breakaway), the freed part is retained / safe-released; the tool does not lurch through into the workspace.
- **Tool hazard management:** the exposed cutting edge is moved at bounded speed and withdrawn to a safe pose; no uncontrolled cutting motion outside the operation.
- On any breach: stop the feed, hold the tool stationary (do not retract *through* uncut material), relieve shear, report. Irreversible state is reported precisely (how far the cut progressed).

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` (no tool) | reject; no attempt |
| `capability_absent` | no `cut` / compliance / tool-safety capability | reject; no attempt |
| `path_unbounded` | `cut_path` not fully resolvable / confirmed | reject (irreversibility demands a bounded path) |
| `shear_exceeded` | shear force > `shear_force_budget` (hard inclusion / kickback) | stop feed; relieve; result ≠ `success` |
| `path_deviation` | tool strayed beyond `cut_path` tolerance | abort; report (potential over-cut) |
| `tool_slip` | tool slipped in grasp under shear | stop; re-secure; result ≠ `success` |
| `incomplete_cut` | `completion` not reached within range | stop; report (partial, irreversible — state reported) |
| `timeout` | wall clock vs `timeout` | stop feed, hold tool, relieve shear |

**Conformance test sketch.**
- **C1 — nominal cut + bounded separation.** With a bench cutting tool and instrumented test material, command `force.cut(cut_path, shear_force_budget = 30 N, completion = separation, on_separation = retain)`. PASS iff `result == success` ∧ the material separated along `cut_path` ∧ shear force `≤ shear_force_budget` at every sampled instant (interval sampling) ∧ the cut did **not** extend beyond `cut_path` ∧ the freed part was retained ∧ tool withdrawn to a safe pose.
- **C2 — hard-inclusion / over-force halt.** Embed a hard inclusion in the test material. PASS iff `result == shear_exceeded` ∧ the shear force never exceeded `shear_force_budget` (the tool did not force through the inclusion, avoiding kickback / tool damage) ∧ the feed stopped with the tool held (not retracted through uncut material).

#### 6.8 `force.wipe`

**Intent.** Maintain a controlled normal contact force against a surface while moving tangentially along a path — wiping, applying, dragging, or smoothing over the surface — using hybrid force/position control (force-controlled normal, position-controlled tangential) that follows the surface contour.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `controlled_frame` | `FrameRef` | effector / held tool | — | the frame in contact (fingertip, held cloth / tool) |
| `surface` | `SurfaceTarget` | — (required) | — | the surface to wipe over |
| `wipe_path` | `Trajectory` | — (required) | — | the tangential path along the surface, in `frame` |
| `normal_force` | `Force` | — (required) | N | the contact force to maintain normal to the surface |
| `normal_force_tolerance` | `Force \| auto` | `auto` | N | allowed deviation of the maintained normal force |
| `feed_rate` | `Velocity \| auto` | `auto` | m/s | tangential speed along the path |
| `contour_following` | `bool` | `true` | — | accommodate surface height variation to hold normal force |
| `compliance` | `{passive, active, auto}` | `auto` | — | required compliance mode (normal-direction compliance is essential) |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- If wiping with a held tool / cloth: `grasp_handle` is `held` and withstands the contact reaction.
- `surface` and `wipe_path` are resolvable; the path lies on the surface within the contour-following range.
- `embodiment` declares `force` with `wipe` support and **normal-direction compliance** (hybrid force/position control) capability.

**Postconditions (on `success`).**
- The `controlled_frame` traversed `wipe_path` tangentially while the normal contact force was maintained at `normal_force` (within `normal_force_tolerance`) throughout.
- Contact was held over the whole path (no loss of contact, no excessive force); the surface contour was followed.
- Held tool (if any) retained; `GraspState` unchanged.

**Safety envelope (holds throughout execution — force trajectory bound, maintained).**
- **Hybrid force/position invariant (interval-sampled):** at every instant along the path, the normal force is within `normal_force ± normal_force_tolerance` (force-controlled) AND the tangential position tracks `wipe_path` within tolerance (position-controlled). Loss of contact (normal force → 0) or excessive normal force (> budget) is an envelope violation. (The maintained-invariant class of `reach.hover`, applied to a contact force along a path.)
- **Contour following:** surface height variation is accommodated by normal compliance to hold the force; if the surface deviates beyond the compliance range (a step, a hole), contact is lost — detected and handled (do not gouge or lose the surface).
- Tangential (friction / drag) force bounded so the wipe does not damage the surface or stall.
- On any breach: lift off normal (reduce force to zero), arrest tangential motion, retract within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `capability_absent` | no `wipe` / no normal-direction compliance | reject; no attempt |
| `no_contact` | surface not reached at path start | result ≠ `success` |
| `contact_lost` | normal force → 0 mid-path (surface dropped away beyond compliance) | arrest; report; result ≠ `success` |
| `normal_force_exceeded` | normal force > budget (surface rose / hard spot) | lift off; result ≠ `success` |
| `path_deviation` | tangential tracking outside tolerance | arrest; result ≠ `success` |
| `grasp_slip_under_load` | held tool slipped under contact reaction | lift off; re-secure; result ≠ `success` |
| `timeout` | wall clock vs `timeout` | lift off, retract |

**Conformance test sketch.**
- **C1 — nominal wipe + maintained normal force.** Wipe a bench surface (with a known contour) along a path; command `force.wipe(surface, wipe_path, normal_force = 5 N)`. PASS iff `result == success` ∧ at **every** sampled instant the normal force was within `normal_force ± normal_force_tolerance` AND the tangential position tracked `wipe_path` within tolerance (hybrid-control interval invariant) ∧ contact held over the whole path.
- **C2 — contour following over a height step.** Introduce a calibrated height variation along the path. PASS iff the normal force stayed within tolerance across the variation (compliance accommodated it), OR — if the step exceeds the compliance range — `result == contact_lost` / `normal_force_exceeded` with a clean lift-off (no gouge, no surface damage).

#### 6.9 `force.scrub`

**Intent.** Apply oscillating tangential motion over a surface region while regulating normal contact force — scrubbing, polishing, sanding, or abrading — using hybrid force/position control with a periodic tangential trajectory, until a completion condition (duration, passes, or state change) is met.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `controlled_frame` | `FrameRef` | effector / held tool | — | the frame in contact (held pad / abrasive / fingertip) |
| `surface` | `SurfaceTarget` | — (required) | — | the surface to scrub |
| `region` | `ScanRegion \| auto` | `auto` | — | the area to cover; `auto` = the local contact region |
| `normal_force` | `Force` | — (required) | N | regulated contact force normal to the surface |
| `amplitude` | `Length` | — (required) | mm | tangential oscillation amplitude |
| `frequency` | `Frequency \| auto` | `auto` | Hz | oscillation frequency; clamped to `embodiment.limits.oscillation_max` |
| `completion` | `ScrubStop` | — (required) | — | `duration(t)`, `passes(n)`, or `state_change(pred)` (e.g. surface clean / smooth) |
| `normal_force_tolerance` | `Force \| auto` | `auto` | N | allowed normal-force deviation |
| `compliance` | `{passive, active, auto}` | `auto` | — | required compliance mode |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- If scrubbing with a held tool / pad: `grasp_handle` is `held` and withstands the oscillating contact reaction (the grasp must survive periodic load reversal).
- `surface` and `region` are resolvable; the scrub stays within `region`.
- `embodiment` declares `force` with `scrub` support and normal-direction compliance; `frequency` within `embodiment.limits.oscillation_max`.

**Postconditions (on `success`).**
- Oscillating tangential motion (`amplitude`, `frequency`) was applied over `region` while the normal force was regulated at `normal_force` (within tolerance), until `completion`.
- Contact and normal-force regulation held throughout; the scrub stayed within `region` (no straying onto adjacent areas).
- Held tool retained through the periodic load reversals; `GraspState` unchanged.

**Safety envelope (holds throughout execution — force trajectory bound, maintained + oscillatory).**
- **Hybrid invariant under oscillation (interval-sampled):** normal force within `normal_force ± normal_force_tolerance` at every instant despite the oscillation; tangential motion stays within `region` and within `amplitude`.
- **Oscillation stability:** at each tangential reversal the held tool / object is retained (periodic inertial reversal must not exceed grasp holding capacity — the oscillation analogue of dynamic grasp stability); `frequency` and `amplitude` are clamped so the reversal load stays within capacity.
- **Wear / heat bound:** sustained oscillation under normal force abrades and heats; if the deployment declares a wear / heat limit (or `state_change` is reached), stop — do not over-scrub past surface / tool damage.
- **Region containment:** the scrub does not stray beyond `region` (no abrading adjacent surfaces).
- On any breach: lift off, arrest oscillation, retract within `embodiment.limits.stop_time`.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `capability_absent` | no `scrub` / compliance, or `frequency` > `oscillation_max` | reject; no attempt |
| `no_contact` | surface not reached | result ≠ `success` |
| `contact_lost` | normal force → 0 during oscillation | arrest; report |
| `normal_force_exceeded` | normal force > budget | lift off; result ≠ `success` |
| `oscillation_unstable` | tool / object slips at a tangential reversal | arrest; re-secure; result ≠ `success` |
| `region_strayed` | motion left `region` | arrest; report |
| `completion_unreached` | `completion` not met within range | stop; report (partial) |
| `timeout` | wall clock vs `timeout` | lift off, arrest oscillation |

**Conformance test sketch.**
- **C1 — nominal scrub + regulated force.** Scrub a bench surface; command `force.scrub(surface, normal_force = 8 N, amplitude = 20 mm, completion = passes(10))`. PASS iff `result == success` ∧ at **every** sampled instant the normal force was within tolerance despite the oscillation (interval invariant) ∧ the motion stayed within `region` and `amplitude` ∧ the held tool was retained through all 10 passes (no reversal slip) ∧ completion (10 passes) met.
- **C2 — reversal-stability / over-frequency.** Command a `frequency` near or above `oscillation_max`. PASS iff (within limit) the tool is retained through reversals with normal force regulated, OR (above limit) `result == capability_absent` (rejected) — never an `oscillation_unstable` outcome with the tool flung or the object lost.

#### 6.10 `force.snap_engage`

**Intent.** Engage a bistable mechanism — a clip, latch, or snap-fit — by driving the parts together through the engagement force peak until the snap-in event (force rise-then-drop signature) is detected and the bistable connection is confirmed held, without over-forcing past engagement and breaking the mechanism.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp on the part being snapped into place |
| `mate_feature` | `FeatureRef` | — (required) | — | the bistable mechanism / receptacle to engage |
| `engage_direction` | `Direction` | — (required) | — | direction to drive engagement, in `frame` |
| `force_budget` | `Force` | — (required) | N | max engagement force (mechanism-break / over-force limit, above the expected snap peak) |
| `snap_signature` | `ActuationSpec` | `detent` | — | what marks snap-in: `detent` (rise-then-drop), or `force_threshold(F)` |
| `confirm_held` | `bool` | `true` | — | verify the bistable connection holds after engagement (release-test) |
| `compliance` | `{passive, active, auto}` | `auto` | — | required compliance mode |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState` on the part; the grasp withstands the engagement reaction.
- `mate_feature` is resolvable and aligned within the engagement-capturable range; the snap-in peak force is below `force_budget` (the mechanism can be engaged without exceeding the break limit).
- `embodiment` declares `force` with `snap_engage` support, the required `compliance`, and force resolution sufficient to detect the snap signature.

**Postconditions (on `success`).**
- The bistable mechanism engaged: the snap-in event (force rise-then-drop) was detected, and (if `confirm_held`) the connection was confirmed to hold a retaining load.
- Engagement force stayed `≤ force_budget` throughout; the mechanism was not over-forced past the engaged state.
- The connection is now in its **engaged bistable state** (a persistent state change; disengagement requires a reverse operation).
- The held part did not slip; `GraspState` unchanged.

**Safety envelope (holds throughout execution — force trajectory bound).**
- **Force-trajectory bound:** engagement force `≤ force_budget` at every instant. The expected profile rises to the snap peak then drops; a force rise that **continues past the budget without the drop** means the mechanism is not snapping (mis-aligned / wrong part) — abort, do not crush it.
- **Snap-in detection + over-force guard:** on detecting snap-in (rise-then-drop, per `force.press_button`'s detent), stop driving immediately — do not continue past engagement (over-force breaks the clip / snap-fit). The snap event, not a fixed depth, ends the engagement.
- **Engagement confirmation:** if `confirm_held`, a light retaining-load test confirms the bistable connection actually holds (distinguishes a true snap-in from a partial / false engagement) before reporting success.
- On any breach: back off along `−engage_direction` to an unloaded pose; do not leave the mechanism partially engaged under load or broken.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject; no attempt |
| `capability_absent` | no `snap_engage` / compliance / force resolution | reject; no attempt |
| `no_snap` | force budget / range reached without a snap signature | back off; result ≠ `success` (did not engage — do not over-force) |
| `over_force` | force > `force_budget` without snap-in | back off; result ≠ `success` (mis-aligned / wrong part) |
| `false_engagement` | (`confirm_held`) snap detected but connection does not hold | back off; report; result ≠ `success` |
| `grasp_slip_under_load` | part slipped under engagement reaction | back off; re-secure; result ≠ `success` |
| `timeout` | wall clock vs `timeout` | back off to unloaded pose |

**Conformance test sketch.**
- **C1 — nominal snap-in + held confirmation.** Present a bench snap-fit / clip; command `force.snap_engage(mate_feature, engage_direction, force_budget = 25 N)`. PASS iff `result == success` ∧ the snap-in event (force rise-then-drop) was detected ∧ engagement force `≤ force_budget` throughout (interval sampling) ∧ (with `confirm_held`) a retaining-load test confirmed the connection holds ∧ no over-force past the snap.
- **C2 — mis-aligned over-force guard.** Mis-align so no snap can occur. PASS iff `result == no_snap` (or `over_force`) ∧ the engagement force never exceeded `force_budget` (the mechanism was **not** crushed seeking a snap that cannot happen) ∧ backed off to an unloaded pose.

### Category 7 — `sense`

`sense` primitives are the only category that does **not** change object state — their output is a measurement or a state assertion, not a manipulation. They are the dual of `force`: where `force` drives contact force to a target, `sense` *suppresses* contact force to a non-disturbing minimum. The category's defining invariant is **measurement non-disturbance** — the act of sensing must not move, deform, or otherwise alter the target beyond measurement tolerance. Sensing outputs flow to later primitives through `LetBind` (a `sense.locate` pose into a `grasp`, a `sense.verify` predicate into a `reactive` body), so the output **`Measurement`** type is as load-bearing as the input target types; it is the perception-derived *output* counterpart to the perception-derived *input* targets defined in the type system. As with `reach.scan`, RFL defines what is sensed and how the act is bounded, not the perception that interprets it.

#### 7.1 `sense.probe`

**Intent.** Make a single light tactile contact at a target pose to measure local surface properties (presence, contact location, normal, stiffness) — gently touching to sense, without displacing or deforming the target. The output is a measurement; no object state changes.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `controlled_frame` | `FrameRef` | `embodiment.default_tactile_frame` | — | the probing frame (fingertip / probe tip) |
| `target_pose` | `Pose6D` | — (required) | m / rad | where to probe, in `frame` |
| `probe_direction` | `Direction \| auto` | `auto` | — | approach direction to contact; `auto` = expected surface normal |
| `contact_force` | `Force` | — (required) | N | the light force at which contact is registered (measurement, not actuation) |
| `max_probe_force` | `Force \| auto` | `auto` | N | hard cap to guarantee non-disturbance; `auto` = small fraction below the target's disturb threshold |
| `measure` | `set<{presence, location, normal, stiffness}>` | `{presence, location}` | — | what to measure on contact |
| `compliance` | `{passive, active, auto}` | `auto` | — | compliance for a gentle, non-disturbing touch |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `target_pose` is reachable; the approach to it (excluding the probed surface) is collision-free.
- `embodiment` declares `sense` with `probe` support and tactile / force sensing (or proxy) sufficient for the requested `measure` set.
- `max_probe_force` is below the target's disturbance threshold (probing must not move or deform the target — if no force low enough can register contact, the target cannot be non-disturbingly probed).

**Postconditions (on `success`).**
- The probe contacted at (or near) `target_pose` and produced a `Measurement` with the requested fields (presence true / false, contact location, surface normal, stiffness estimate).
- **Non-disturbance:** the target's pose and shape are unchanged within measurement tolerance (the probe sensed without disturbing).
- No grasp state changed; the embodiment retracted to a clear pose after probing.

**Safety envelope (holds throughout execution — measurement non-disturbance).**
- **Non-disturbance bound (the sense-category invariant):** the contact force never exceeds `max_probe_force`, which is held below the target's disturb threshold — the probe measures without moving or deforming the target. A force excursion that would disturb the target is an envelope violation (the opposite-direction constraint to `force`'s force budget: here force is *suppressed*, not driven).
- Approach per `reach.approach` / `reach.to_pose` (velocity, clearance) until contact; gentle contact onset (no impact spike that would disturb).
- On any breach (force exceeds the non-disturbance cap before a valid reading): retract; report a measurement failure rather than disturbing the target.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `capability_absent` | no `probe` / insufficient sensing for `measure` | reject; no attempt |
| `unreachable` | IK on `target_pose` | no motion |
| `no_contact` | no surface met at `target_pose` (within range) | report `presence = false` (a valid measurement, not a failure) |
| `disturb_risk` | required contact force to register exceeds `max_probe_force` | retract; result ≠ `success` (cannot probe without disturbing) |
| `unexpected_contact` | contact before `target_pose` (en-route obstacle) | retract; report |
| `timeout` | wall clock vs `timeout` | retract to clear pose |

**Conformance test sketch.**
- **C1 — nominal probe + non-disturbance.** Probe a bench surface at a known pose; command `sense.probe(target_pose, contact_force = 0.5 N, measure = {presence, location, normal})`. PASS iff `result == success` ∧ the returned `Measurement` reports presence true with contact location and normal within measurement tolerance of ground truth ∧ the externally measured target pose was **unchanged** (non-disturbance) ∧ contact force never exceeded `max_probe_force`.
- **C2 — presence-false is a valid result.** Probe at a pose with no surface (empty space). PASS iff `result == success` ∧ the `Measurement` reports `presence = false` (absence is a valid measurement, not an error) ∧ no force was applied into empty space chasing a non-existent surface.

#### 7.2 `sense.inspect`

**Intent.** Observe a target's state by non-contact sensing — placing a sensor frame at an observation pose where the target is within field of view and unoccluded, and capturing the observation. The contactless counterpart of `sense.probe`; the contract guarantees the observation was *capturable*, not its interpretation.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `sensor_frame` | `FrameRef` | `embodiment.default_sensor_frame` | — | the observing sensor frame |
| `target` | `ObjectTarget \| SurfaceTarget \| FrameRef` | — (required) | — | what to observe |
| `observe` | `set<{presence, pose, appearance, defect, custom}>` | `{presence, pose}` | — | what to capture (interpretation is out of RFL scope) |
| `observation_pose` | `Pose6D \| auto` | `auto` | — | sensor pose for the observation; `auto` = a pose with the target in FOV, unoccluded, at working distance |
| `standoff` | `Length \| auto` | `auto` | mm | sensor-to-target distance; `auto` = sensor working range |
| `dwell` | `Duration` | `0` | s | observation integration time |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `target` is resolvable enough to derive an `observation_pose`; `embodiment.sensors[sensor_frame]` covers the requested `observe` modalities.
- An `observation_pose` exists that is reachable, collision-free, and places the target within FOV and unoccluded at a working distance.
- `embodiment` declares `sense` with `inspect` support and the requisite sensor.

**Postconditions (on `success`).**
- The sensor frame reached `observation_pose` with the target in FOV and unoccluded; an observation was captured over `dwell`, producing a `Measurement` with the requested `observe` fields (and their uncertainty).
- **Non-disturbance (trivial for contactless):** no contact was made; the target is unchanged. (The contract is purely that the observation was *capturable* — FOV, unoccluded, in range — not that any interpretation succeeded; interpretation is perception / VLA, out of scope, per `reach.scan`.)
- No grasp / object state changed.

**Safety envelope (holds throughout execution).**
- Approach to `observation_pose` per `reach.to_pose` (velocity, clearance, no unplanned contact — inspect is contactless and must stay so).
- **Observability guard:** the contract requires the target in FOV and unoccluded at the observation; if occlusion or out-of-range is detected, the observation is reported unobtainable rather than returning a bad reading as if valid.
- On any breach (unexpected contact during a contactless operation, or loss of observability): retract; report.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `capability_absent` | no `inspect` / sensor lacks the `observe` modality | reject; no attempt |
| `no_observation_pose` | no reachable pose with target in FOV, unoccluded, in range | result ≠ `success` (cannot observe) |
| `occluded` | target occluded at the observation pose | report occlusion; result ≠ `success` (not a bad reading) |
| `out_of_range` | target outside sensor working range | report; result ≠ `success` |
| `unexpected_contact` | contact during the contactless approach | retract; report |
| `timeout` | wall clock vs `timeout` | retract to clear pose |

**Conformance test sketch.**
- **C1 — nominal inspect + observability.** Observe a bench target with a marked state; command `sense.inspect(target, observe = {presence, pose})`. PASS iff `result == success` ∧ the sensor reached an `observation_pose` with the target in FOV and unoccluded (externally verified geometry) ∧ a `Measurement` was produced with the requested fields and uncertainty ∧ no contact occurred. (The *interpretation* of the observation is not scored — only that it was capturable.)
- **C2 — occlusion reported, not faked.** Place an occluder between the sensor and the target. PASS iff `result == occluded` (or `no_observation_pose`) ∧ the primitive reported the target unobservable rather than returning a confident `Measurement` from an occluded view.

#### 7.3 `sense.weigh`

**Intent.** Estimate the mass (and optionally the center of mass) of a held object from the force/torque reaction the end-effector measures while supporting or moving it — a held-object measurement that produces the `estimated_mass` other categories consume. The only `sense` primitive that requires a held object.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `grasp_handle` | `GraspRef \| active` | `active` | — | the grasp holding the object to weigh |
| `method` | `{static, dynamic, auto}` | `auto` | — | `static` = support against gravity (`F = mg`); `dynamic` = known acceleration (`F = ma`) |
| `measure` | `set<{mass, center_of_mass}>` | `{mass}` | — | what to estimate (CoM needs multiple poses) |
| `up_direction` | `Direction` | `−gravity` | — | gravity reference for the static method (declared, not assumed) |
| `settle_time` | `Duration \| auto` | `auto` | s | time to let force readings settle before sampling |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `grasp_handle` resolves to a `held` `GraspState` (there is an object to weigh).
- `embodiment` declares `sense` with `weigh` support and force/torque sensing (or a proxy) sufficient to resolve the expected mass range.
- For `dynamic`: a small measurement motion within the grasp's dynamic-stability limit is feasible (the object can be accelerated without slipping).

**Postconditions (on `success`).**
- A `Measurement` reports the object's `mass` (and `center_of_mass` if requested), with uncertainty, from the measured force/torque reaction.
- **Non-disturbance:** the grasp and the object are unchanged — the measurement motion (if any) did not slip the grasp, deform the object, or change `GraspState`. The object's `ObjectTarget.estimated_mass` may now be populated from measurement (feeding `grasp` / `transport` / `place`).
- `GraspState` remains `held`.

**Safety envelope (holds throughout execution).**
- **Grasp continuity during measurement:** any measurement motion (`dynamic` acceleration, or support) stays within the grasp's dynamic-stability limit — the object never slips while being weighed (a slip both corrupts the reading and risks dropping the object).
- **Measurement-motion non-disturbance:** the motion used to weigh is bounded so it does not deform a fragile object or exceed the grasp's holding capacity; the object is held throughout.
- For `static`: the support force is held steady (settle) before sampling, so the reading reflects `mg`, not transient dynamics.
- On any breach (grasp slip, instability): arrest, keep the object held, report a measurement failure rather than dropping it.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `no_active_grasp` | `grasp_handle` not `held` | reject (nothing to weigh) |
| `capability_absent` | no `weigh` / insufficient force-torque sensing | reject; no attempt |
| `out_of_sensing_range` | mass below / above resolvable range | report; result ≠ `success` (cannot resolve) |
| `grasp_slip_during_measure` | object slipped while weighing | arrest; keep held; result ≠ `success` |
| `unsettled` | (`static`) reading did not settle within `settle_time` | extend / report; result ≠ `success` |
| `timeout` | wall clock vs `timeout` | arrest, object kept held |

**Conformance test sketch.**
- **C1 — nominal weigh + accuracy.** Grasp a bench object of known mass; command `sense.weigh(method = static, measure = {mass})`. PASS iff `result == success` ∧ the returned `Measurement.mass` is within stated uncertainty of the true mass ∧ the grasp did not slip (object held throughout) ∧ no object deformation ∧ `GraspState` unchanged.
- **C2 — slip-during-measure safety.** Force a marginal grasp so a `dynamic` measurement motion induces slip. PASS iff `result == grasp_slip_during_measure` ∧ the object was **kept held** (arrested, not dropped) ∧ no false mass reading reported as valid.

#### 7.4 `sense.locate`

**Intent.** Estimate the pose of a referenced object — by visual, tactile, or fused sensing — producing a pose with uncertainty that subsequent primitives consume to satisfy their target-resolution preconditions. The most common entry point of a manipulation (the canonical `LetBind` source).

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `target_ref` | `ObjectRef \| FeatureRef` | — (required) | — | what to locate (an object identity / feature to estimate the pose of) |
| `modality` | `{visual, tactile, fused, auto}` | `auto` | — | sensing modality; `auto` = best available for the required precision |
| `required_precision` | `Length \| auto` | `auto` | mm | the pose precision the result must achieve; `auto` = embodiment default |
| `search_region` | `Region \| auto` | `auto` | — | where to look; `auto` = the expected vicinity |
| `frame` | `FrameRef` | `task` | — | reference frame for the returned pose |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `target_ref` denotes a locatable object / feature; `embodiment` declares `sense` with `locate` support and a modality able to meet `required_precision`.
- For `tactile` / `fused`: contact used for localization stays non-disturbing (per `sense.probe`'s non-disturbance bound).

**Postconditions (on `success`).**
- A `Measurement` reports the target's `pose` in `frame` with an `uncertainty` `≤ required_precision`.
- **Non-disturbance:** the target was not moved by the localization (contactless modalities trivially; tactile localization stays within the probe non-disturbance bound).
- The returned pose + uncertainty is suitable for `LetBind` into a subsequent primitive, whose target-resolution precondition (`uncertainty ≤ its bound`) is checked against this result.

**Safety envelope (holds throughout execution).**
- Non-disturbance (per the `sense` invariant): any contact used for tactile localization stays below the target's disturb threshold; contactless localization makes no contact.
- Approach / observation motion per `reach` (velocity, clearance, no unplanned contact).
- **Precision honesty:** if the achieved precision cannot meet `required_precision`, the result reports the lower precision (or fails) rather than overstating confidence — a pose is never returned with an uncertainty better than measured.
- On any breach: retract; report.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `capability_absent` | no `locate` / no modality meeting `required_precision` | reject; no attempt |
| `not_found` | `target_ref` not located in `search_region` | result ≠ `success` (target not found — a definite negative, honestly reported) |
| `precision_unmet` | located but achievable uncertainty > `required_precision` | report the located pose with its true (larger) uncertainty; result flagged (do not overstate) |
| `occluded` / `out_of_range` | (visual) target not observable | report; result ≠ `success` |
| `disturb_risk` | (tactile) localization would disturb the target | retract; result ≠ `success` |
| `timeout` | wall clock vs `timeout` | retract; report |

**Conformance test sketch.**
- **C1 — nominal locate + precision.** Present a bench object at a known pose; command `sense.locate(target_ref, required_precision = 1 mm)`. PASS iff `result == success` ∧ the returned `Measurement.pose` is within `required_precision` of ground truth ∧ the reported `uncertainty ≤ required_precision` ∧ the target was not disturbed ∧ the result is `LetBind`-usable (its uncertainty satisfies a downstream grasp's bound).
- **C2 — precision honesty.** Present the object under degraded sensing (poor lighting / partial occlusion) so the true uncertainty exceeds `required_precision`. PASS iff the result either fails (`precision_unmet`) or returns the pose with its **true, larger uncertainty** — never a pose stamped with a confident `uncertainty` it did not achieve.

#### 7.5 `sense.verify`

**Intent.** Check whether a stated predicate about an external state holds — "is the connector seated?", "is the bin empty?", "is the part present and correctly oriented?" — returning a boolean with confidence and supporting evidence. The `sense` implementation of the algebra's `Predicate`, driving `reactive` / `branch` control flow.

**Parameters.**

| Name | Type | Default | Units | Constraint |
|---|---|---|---|---|
| `predicate` | `StatePredicate` | — (required) | — | the condition to check (a caller-defined criterion over observable state) |
| `evidence_modalities` | `set<{visual, tactile, force, proprioceptive}>` | `auto` | — | which observations to use; `auto` = those sufficient for the predicate |
| `confidence_threshold` | `Ratio` | `auto` | — | min confidence to assert true / false rather than `indeterminate` |
| `frame` | `FrameRef` | `task` | — | reference frame |
| `timeout` | `Duration \| auto` | `auto` | s | `> 0` |

**Preconditions.**
- `predicate` is a resolvable criterion over state RFL can observe (the *criterion* is caller-defined; RFL observes, the caller defines what counts as true — perception-scope boundary).
- `embodiment` declares `sense` with `verify` support and the sensing for `evidence_modalities`.
- Any contact-based evidence stays non-disturbing (per the `sense` invariant).

**Postconditions (on `success`).**
- A `Measurement` reports the predicate's truth value (`true` / `false` / `indeterminate`), a `confidence`, and the `evidence` (which observations supported the verdict).
- The verdict is `true` / `false` only when `confidence ≥ confidence_threshold`; otherwise `indeterminate` (RFL does not assert a predicate it cannot support — honesty over a forced answer).
- **Non-disturbance:** verification did not alter the state being checked.
- The verdict is usable as an algebra `Predicate` (in `reactive` / `branch`) and as a `force.scrub` `state_change` completion.

**Safety envelope (holds throughout execution).**
- Non-disturbance per the `sense` invariant (any probing evidence stays below the disturb threshold; the act of verifying does not change the verified state).
- Observation motion per `reach` (velocity, clearance).
- **Verdict honesty:** the predicate is asserted `true` / `false` only above `confidence_threshold`; below it, `indeterminate` is returned with the evidence — never a forced boolean. Safety-critical verifications (e.g. "is the fastener torqued?") carry their `evidence` for audit (the L4 Certification / L8 Insurance loops consume it).
- On any breach: report `indeterminate` with whatever evidence was gathered, rather than a confident wrong answer.

**Failure modes (detection → invariant).**

| Mode | Detection | Invariant |
|---|---|---|
| `capability_absent` | no `verify` / sensing for `evidence_modalities` | reject; no attempt |
| `predicate_unresolvable` | the predicate is not a criterion over observable state | reject (out of scope — not a sensing failure) |
| `indeterminate` | confidence < `confidence_threshold` | report `indeterminate` + evidence (a valid, honest outcome — not a crash) |
| `disturb_risk` | (contact evidence) verifying would disturb the state | use a contactless modality, or report `indeterminate` |
| `timeout` | wall clock vs `timeout` | report `indeterminate` + partial evidence |

**Conformance test sketch.**
- **C1 — nominal verify + evidence.** Set up a bench state with a known truth (e.g. a connector definitely seated); command `sense.verify(predicate = seated)`. PASS iff `result == success` ∧ the verdict matches ground truth (`true`) ∧ `confidence ≥ confidence_threshold` ∧ the `evidence` is present and consistent with the verdict ∧ the state was not disturbed.
- **C2 — indeterminate honesty.** Set up an ambiguous state (evidence insufficient to decide). PASS iff `result` is `indeterminate` (not a forced `true` / `false`) ∧ the evidence and (sub-threshold) confidence are reported — the primitive does not fabricate a confident verdict it cannot support.

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
