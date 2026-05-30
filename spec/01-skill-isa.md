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
| 1.1 | `reach.to_pose` | Move end-effector to an absolute task-frame pose |
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
2. **`LetBind`** is included to allow a planner-derived pose (e.g., the result of `sense.locate`) to flow into subsequent primitives without re-derivation. This is what enables a single Skill ISA file to express "find the cable end, then grasp it, then insert" as a single composition.
3. **`Predicate` extensibility via `user_defined`** is the escape valve: domain-specific predicates can ship as extensions without modifying the core grammar.
4. **`Auto` value** lets the spec author defer choice to the Translation Layer's planner (e.g., a `grasp_pose: auto` parameter delegates pose selection).

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
