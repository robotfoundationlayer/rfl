# Design: mass-dependent grasp-force numerics (GF1c / GF2c / GF3c)

Status: approved design, pre-implementation (2026-05-31)

This is the third reference-implementation increment. It retires the symbolic
placeholders the v0 lowering left behind (the `lower_transport_move_to_pose` and
`lower_force_insert_fit` doc comments both say "a later increment, design § 3").
It derives, numerically and deterministically, the force and acceleration bounds
of `spec/02` § Grasp-force and stability derivations, and emits them in the
`execute` message. It is the first increment in which the Translation Layer does
**embodiment-specific physical computation** — not just the structural mapping of
v0 or the generative path of the surface-scan increment. The specification under
`spec/` is authoritative; this document describes how the reference implementation
realizes it.

## 1. Why this increment

v0 clamped every motion bound to the embodiment's **kinematic** ceiling
(`a_cartesian_max`) and carried every force budget through unchanged except for the
`grip_force_max` clamp. The grasp's *physics* — how heavy is the held object, how
hard must the grip squeeze to keep it, how fast can it be accelerated before it
slips, how much insertion reaction the grasp can bear — was left symbolic.

`spec/02` § Grasp-force defines four derivations sharing one underlying quantity,
the grasp's **holding capacity**. This increment implements three of them (GF4c is
deferred — no cable-insertion primitive transmits force through a held tool):

- **GF1c — `min_holding_force`**: the static grip floor below which the object
  falls under its own weight.
- **GF2c — dynamic-stability clamp**: the largest acceleration at which inertial +
  gravity load stays within holding capacity, clamping every `transport`
  primitive's `max_acceleration`.
- **GF3c — reaction-load limit**: a `force` primitive's reaction may not exceed the
  grasp's capacity along the reaction axis, bounding the force budget so the held
  part does not slip before the task completes.

Demonstrating these proves the engine computes per-embodiment bounds from the same
embodiment-agnostic skill, the Principle-1 point: one task, three hands, three
different derived envelopes.

## 2. Scope

In scope:

- The three derivations GF1c / GF2c / GF3c as **pure functions** of declared inputs
  (object weight, grasp mode, descriptor limits) and documented schematic
  constants — never runtime-measured, so `retarget` generation stays
  byte-deterministic (RD1c, `spec/02`:119).
- A small **grasp-context accumulator** threaded through the sequence walk so a
  `transport` / `force` primitive knows what grasp currently holds what object
  (the v0 `retarget` walk is stateless across primitives).
- An `estimated_mass` input on the skill's task objects — a minimal optional field
  added to the (currently closed) `ObjectDecl` in `schemas/skill-isa.schema.json`,
  reusing the existing `$defs/Force` and mirroring `ObjectTarget.estimated_mass`
  (`schema`:452) — resolved to the held object through the existing let-binding
  chain.
- The three example-01 insta goldens regenerated to carry the derived values;
  conformance test class 2 unchanged in shape.

Deferred (consistent with prior deferrals and YAGNI):

- **GF4c** (tool-mediated transmission, `force.screw` rotation↔advance coupling):
  no cable-insertion primitive exercises it; it needs a tool-bearing example.
- **Payload-precondition violation** (`estimated_mass > payload_grasp_<mode>`): the
  fixture weight is kept below every hand's pinch payload so the object is always
  holdable. The "object too heavy for the grasp" signal is a precondition-check
  concern (closer to the capability gate) and gets its own increment.
- The retighten head-room model (a slipping grasp tightening up to `grip_force_max`
  is approximated by the v0 reaction model below, not dynamically simulated).
- Everything still deferred from prior increments (routing RD4c, time-scaling TG2c,
  multi-embodiment handoff RD3c, conformance class 3/4, the spiral/arc Σ patterns).

## 3. The capacity model (schematic per-mode factor)

`spec/02`:129 fixes the capacity model as **schematic and provider-neutral**
(Principle 4): "RFL fixes the *dependency* — capacity is a function of closure,
secured DOF, grip / normal force, geometry, and friction — not a single vendor's
friction law." This increment honors that by collapsing the `μ · geometry` term
into **documented dimensionless reference factors per closure mode**, rather than
inventing a typed friction field the spec deliberately left as prose. The factors
are an explicit v0 reference-implementation choice, pinned by golden, non-normative
— the same posture as v0's symbolic poses and the surface-scan raster transcription.

Because `target.estimated_mass` is typed `Force` ("weight under standard gravity",
`spec/01`:262), the object weight `W = m·g` is already a single force quantity, so
the static floor needs no gravity constant at all; only the dynamic clamp uses
`g₀ = 9.80665 m/s²` as a scale.

The three derivations, with `W = target.estimated_mass`:

| Obligation | Quantity | Reference relation (v0) |
|---|---|---|
| GF1c | `min_holding_force` (grip floor) | `W · k_pinch`, `k_pinch = 2.0` (≈ 1/(2μ), μ=0.25 conservative; force-closure) |
| GF2c | `a_max` (transport accel) | `min( a_cartesian_max, g₀·(payload_grasp_pinch / W − 1) )` |
| GF3c | insert reaction limit | `min( force_budget, grip_force_max / k_reaction )`, `k_reaction = 2.0` |

GF2c uses the descriptor's `payload_grasp_pinch` as the mode's rated capacity
ceiling (the max holdable weight); the worst-case inertial load is taken collinear
with gravity, so the effective load is `W·(1 + a/g₀)` and the clamp solves
`W·(1 + a_max/g₀) ≤ payload`. GF3c bases the reaction capacity on `grip_force_max`
(the grip the `slip_response: retighten` pinch can muster against axial pull-out),
divided by the same schematic factor.

These are intentionally distinct capacity notions: GF1c is a *grip command* the
mode requires (weight → grip), GF2c is bounded by the mode's *rated payload*, GF3c
by the hand's *maximum grip*. All three are deterministic functions of declared
inputs.

## 4. Architecture

Extended units in `rfl-core` (existing module layout extended, not restructured;
**no `Primitive` enum variant is added**, so the catch-all-less `lower` /
`check_capability` match arms are untouched):

```
rfl-core
  grasp_force.rs  [new]   the three pure derivations + the per-mode factor table:
                          min_holding_force(weight, mode) -> f64 (N)
                          dynamic_a_max(weight, payload, kinematic_ceiling) -> f64 (m/s^2)
                          reaction_limit(force_budget, grip_force_max, mode) -> f64 (N)
  quantity.rs     [extend] Quantity::from_si(value: f64, unit) — deterministic
                          unit-suffixed string from a round6'd f64 (shortest round-trip)
  skill_isa.rs    [extend] ObjectDecl gains estimated_mass: Option<Quantity>
                          (mirrors spec/01 ObjectTarget.estimated_mass: Force)
  translation.rs  [extend] GraspContext { held: Option<HeldObject> } threaded through
                          the sequence walk; weights map (let-var -> weight); the three
                          lowerings consume it (pinch sets, release clears, transport +
                          insert_fit read)
  canonical.rs    [reuse]  round6 (existing); grasp.pinch force_profile now carries
                          min_holding_force; motion_bounds.a_max / insert force_budget
                          carry the clamped strings
```

One change outside `rfl-core`, to `schemas/skill-isa.schema.json`: the closed
`ObjectDecl` (`required: [ref]`, `additionalProperties: false`) gains an **optional**
`estimated_mass` property `$ref`-ing the existing `$defs/Force`. This is the only
edit to the machine-readable layer; it keeps `ObjectDecl` closed and `ref` required,
so it is non-breaking, and it lets the extended `skill.yaml` validate (otherwise the
new field would be rejected and `validate.py` would fail). It mirrors
`ObjectTarget.estimated_mass`, which the schema already defines (`schema`:452), so it
introduces no new type. No `spec/` prose and no `02`/`03`/`04`/`05` schema is touched.

### Decision A: grasp-context accumulator over the sequence walk

`retarget` currently calls `lower(prim, embodiment)` per statement with no state
carried across the sequence. GF2c and GF3c need to know, at a `transport` /
`force.insert_fit` step, **what grasp holds what object** — established earlier by
`grasp.pinch`. A small `GraspContext { held: Option<HeldObject{ weight, mode }> }` is threaded
through the walk: set when `grasp.pinch` lowers, cleared when `grasp.release`
lowers, read by `transport.move_to_pose` (GF2c) and `force.insert_fit` (GF3c).
`lower` gains a `&mut GraspContext` parameter (plus a `&weights` reference). The
context carries only what the derivations consume: the held object's `weight`
(GF2c) and `mode` (which selects the `payload_grasp_<mode>` key for GF2c and the
`k_reaction` factor for GF3c); the grip ceiling GF3c bounds against comes straight
from the descriptor's `grip_force_max`, so no commanded-grip is stored. This
mirrors the grasp lifecycle (`spec/01` § Grasp state model) and is the minimal
state needed; modelling the full GraspState FSM is out of scope.

In the cable-insertion sequence the held grasp is live across exactly the right
span: `pinch`(1) sets it, `transport`(2) and `insert_fit`(5) read it, `release`(6)
clears it, `retract`(7) holds nothing.

### Decision B: object weight resolved through the let-binding chain

The held object's weight is a **declared prior** on the skill's task object
(`objects.connector.estimated_mass`, the optional `ObjectDecl` field added in § 4),
not a runtime measurement — required for AOT determinism (`spec/02`:119). At the start of `retarget`, a `weights` map is built by
joining each `LetBind{ let, from: sense.locate{ target_ref } }` with
`objects[target_ref].estimated_mass`, giving `let-var -> weight`
(`connector_t -> objects[connector].estimated_mass`). `grasp.pinch.target` (the
let-ref `connector_t`) resolves through it. An object without a declared mass simply
produces no derivation (the bound stays at the kinematic ceiling), so the change is
backward-compatible with the surface-scan example (no grasped object).

### Decision C: derived numerics emit as unit-suffixed strings

Like the surface-scan poses, the derived forces and accelerations are computed
`f64`s, so they cross the string-carry determinism lever. Each is `round6`'d (the
existing 6-decimal round) and formatted as a unit-suffixed string
(`Quantity::from_si`) so the `execute` message's force / motion fields stay
uniformly string-typed (consistent with `force_budget` and `motion_bounds`, and
with the open `Envelope` floor in `driver-interface.schema.json`). The shortest
round-trip `f64` Display is deterministic across platforms, so `format!("{} {}",
round6(v), unit)` is byte-stable: `10.0 -> "10 N"`, `7.5 -> "7.5 N"`,
`2.9 -> "2.9 N"`, `0.3381606… -> "0.338161 m/s^2"`.

When the dynamic clamp does not bite (the kinematic ceiling is tighter), the
descriptor's original `a_cartesian_max` string is emitted verbatim (no reformat),
preserving the v0 behavior for that case.

## 5. Output placement in the `execute` message

- `grasp.pinch`: `force_profile: { min_holding_force: "<W·k_pinch> N" }` (the lower
  bound the grasp-continuity invariant `05` GC1 samples). The top-level
  `force_budget` becomes `clamp(task_budget, floor = min_holding_force, ceiling =
  grip_force_max)` — the floor never raises it above the ceiling.
- `transport.move_to_pose`: `safety_envelope.motion_bounds.a_max` becomes the GF2c
  clamp (computed string when the dynamic limit bites, descriptor string otherwise).
- `force.insert_fit`: the top-level `force_budget` becomes the GF3c reaction limit;
  the axial `force_profile` mirrors it.

## 6. The fixture (example 01 extension)

`examples/01-cable-insertion/skill.yaml` gains a declared connector mass:

```yaml
objects:
  connector:  { ref: connector, estimated_mass: 1.45 N }   # ≈148 g connector+cable stub
  receptacle: { ref: receptacle }                           # not grasped, no mass
```

`W = 1.45 N` is chosen deliberately so the derivations are **visible** in the
golden: it is below every hand's `payload_grasp_pinch` (so always holdable, no
payload violation), yet heavy enough that the dynamic-stability clamp drops below
the kinematic ceiling on the lowest-payload hand. The resulting per-hand deltas
(`k_pinch = k_reaction = 2.0`, `g₀ = 9.80665`):

| | min_holding_force (GF1c) | commanded grip | transport a_max (GF2c) | insert_fit budget (GF3c) |
|---|---|---|---|---|
| allegro (payload 3 N, grip_max 20, ceil 1.5) | 2.9 N | 8 N | 1.5 m/s² (kinematic; dyn ≈ 10.5) | 10 N |
| leap (payload 2 N, grip_max 15, ceil 2.0) | 2.9 N | 8 N | 2.0 m/s² (kinematic; dyn ≈ 3.72) | 7.5 N |
| pneumatic (payload 1.5 N, grip_max 12, ceil 0.8) | 2.9 N | 8 N | **0.338161 m/s²** (dynamic bites) | 6 N |

The story: the same skill and task give the weakest hand (pneumatic) the most
conservative bounds — limited on both acceleration *and* insertion force — while
the stronger hands keep their kinematic head-room. `min_holding_force` is a task
floor (constant across hands, 8 N command satisfies it); GF3c clamps the 15 N
insertion budget per hand by its maximum grip.

The connector weight is illustrative, in the same spirit as the descriptors'
illustrative limit values; the exact figure can be tuned without affecting the
derivation logic.

## 7. Determinism and testing

- `grasp_force` unit tests: each derivation hand-checked against the § 3 relations
  (the `W → floor` table; `W + payload + ceiling → a_max` including the
  `min`-with-ceiling boundary on each side; `budget + grip_max → reaction limit`).
- `quantity` from-SI test: `round6` + shortest-round-trip formatting yields the
  expected strings, twice-identically.
- `translation` tests: the held-context lifecycle (set on pinch, cleared on release)
  and the three lowerings producing the § 6 per-hand values.
- `rfl-conformance`: the three example-01 insta goldens regenerated to carry the
  derived values, the generate-twice byte-equality check, and per-line `execute`
  schema validation (the new `force_profile` / `motion_bounds` fields pass the open
  `canonical_action` floor — confirmed against `driver-interface.schema.json` during
  implementation before regenerating goldens).
- The surface-scan goldens stay unchanged and green; `validate.py` (C1 through C7)
  stays green after the one schema change (the extended `skill.yaml` validates
  against the now-`estimated_mass`-aware `ObjectDecl`, and the C1–C7 anti-drift
  invariants are unaffected). **No `spec/` prose is edited and the capacity model
  invents no new type** — the single schema edit reuses the existing `Force` `$def`.

## 8. Sections to transcribe during implementation

- `spec/02` § Grasp-force and stability derivations (the three relations and their
  inputs; already read for this design), and the GF1c–GF3c obligation statements.
- `spec/01` § type system `ObjectTarget` (the `estimated_mass: Force` field) and
  § Grasp state model `StabilityMetadata.min_holding_force` (the output slot and the
  grasp-mode → closure table).
- `schemas/skill-isa.schema.json` `$defs/ObjectDecl` (the closed shape the optional
  `estimated_mass` `$ref: #/$defs/Force` is added to) and `$defs/Force` (the unit
  pattern the new field reuses), so the extension stays consistent with the existing
  object-declaration shape.
