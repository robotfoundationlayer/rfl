# Design: transport.carry — held interval-invariant (the held leg of the interval class)

Status: proposed design, pre-implementation (2026-05-31)

This is the thirteenth reference-implementation increment and the first of the two that
close the conformance story. It adds the `transport.carry` primitive — transport a held
object while actively maintaining grasp stability under perturbation (spec/01 § 4.4) —
the disturbance-robust counterpart of plain transport and the held-object analogue of
`reach.hover`'s station-keeping. It is the first primitive whose interval invariant is a
*held* invariant: it exercises interval sampling (like hover) **and** the held-secured
floor (like the grasp-continuity base mode) at the same time, under one envelope class.
The specification under `spec/` is authoritative; this document describes how the
reference implementation realizes it.

## 1. Why this increment, and the ENV1 correction

The four-class envelope taxonomy is complete (increment 12). `transport.carry` is the
remaining sustained primitive: it is the only interval-invariant primitive that carries a
*held object*, so it is what makes the interval class exercise the held-secured floor
that increment 10 began propagating onto plain transport.

**ENV1 fixes the class assignment, and it is single-valued.** spec/05 § Envelope-class
obligations, ENV1: *"Every primitive is verified against **exactly one** envelope class,
fixed by category: `reach` terminal (except `hover`); `reach.hover` / `transport.carry`
interval-invariant; `grasp` / `in_hand` / `transport` / `place` grasp-continuity; `force`
force/torque-trajectory."* So `transport.carry` is pulled out of the
transport→grasp-continuity bucket and assigned to **interval-invariant only**. It is not
in two classes; the held-secured aspect is *part of* its interval invariant, not a second
class membership. spec/01 § 4.4 postconditions confirm this reading: the interval
invariant is "at every instant the grasp held the object within tolerance despite
perturbations up to `disturbance_budget`; no in-grasp slip beyond tolerance occurred."

Consequence: **no signature change** to `envelope_class_for`. One new arm
(`"carry" → IntervalInvariant`), and the multi-sample driver inherits N=3 automatically —
it was gated on `envelope_class_for(suffix) == IntervalInvariant` in increment 12 for
exactly this forward case. The held-secured floor is realized by *enriching the existing
interval-invariant check* (§ 5), not by adding machinery.

## 2. Spec basis for the carry acceleration clamp

spec/02 § Grasp-force and stability derivations, Dynamic stability (line 137): *"A
`transport.carry`'s acceleration is clamped **below** this [the dynamic-stability
`a_max`] to reserve margin for its `disturbance_budget` (`05` ENV3)."* The spec states the
*relation* (carry `a_max` < plain transport `a_max`) but pins no closed form — so the
exact reduction is a v0 reference-implementation choice, documented and golden-pinned,
the same posture as `k_holding = 2.0` and `R_GRIP = 0.02`.

The reduction is the existing GF2c inertial model with §4.4's own precondition inequality
made concrete. spec/01 § 4.4 precondition: *"The grasp's holding capacity exceeds the
inertial load **plus** `disturbance_budget` by at least `stability_margin`."* With the
GF2c worst-case-collinear inertial model `inertial_load = weight·(1 + a/g₀)`, holding
capacity = the mode's rated payload (a weight, N), `D = disturbance_budget` (N), and
`m = stability_margin` (dimensionless):

```
payload ≥ (1 + m) · ( weight·(1 + a/g₀) + D )
⟹ a_max_carry = g₀ · ( ( payload/(1+m) − D ) / weight − 1 ),  clamped ≥ 0
```

This **reduces to `dynamic_a_max`** when `m = 0, D = 0` (= `g₀·(payload/weight − 1)`), and
is provably ≤ `dynamic_a_max` for `m, D ≥ 0` (so it is strictly "below the plain-transport
limit", as spec/02:137 requires, whenever `m > 0` or `D > 0`). The pre-clamp going
negative is the §4.4 `insufficient_stability_margin` case (the grasp is too weak for this
carry under this disturbance) — surfaced in v0 as `a_max_carry = 0` ("cannot accelerate"),
exactly as `dynamic_a_max` already does for an over-payload weight. The
precondition-reject failure mode is deferred (v0 has no precondition checker; it emits the
clamped bound, parallel to GF2c).

## 3. The two `auto`-valued parameters

- **`stability_margin: auto`** = the embodiment default. spec/03 § Transport capabilities
  makes `stability_margin` a descriptor limit (a dimensionless ratio), and the descriptor
  schema's M2 rule **requires** `limits.stability_margin` whenever `transport.carry` is an
  asserted capability. So `auto` resolves to `descriptor.limits.stability_margin` — the
  M2-required limit is genuinely consumed. An explicit ratio in the skill overrides it.
- **`disturbance_budget`** is **explicit** in the worked example (a `Force`). §4.4 types it
  `Force | auto` with `auto = "derived from min_holding_force margin"`, but neither spec/02
  nor spec/05 pins a derivation formula — so `auto` is **deferred and documented**, the same
  decision as the BLOCKED `coverage_overlap: auto` (do not invent a normative formula). The
  explicit form exercises the whole clamp and gives increment-2's ENV3 bench a concrete
  budget to inject against.

## 4. Component 1 — rfl-core (`transport.carry`)

One commit (the `lower` / `check_capability` matches have no catch-all; the new
`Primitive` variant must land with both arms or rfl-core will not compile).

- **`TransportCarry` struct** (v0 subset of `$defs/TransportCarryParams`, §4.4 table):
  `motion: serde_yaml::Value` (required; `MoveSpec` carried opaquely — the `{to_pose: P}`
  form is lowered in v0, `{trajectory: T}` deferred), `disturbance_budget:
  Option<DisturbanceArg>` (`Force | auto`; v0 lowers the explicit `Force`, `auto` deferred),
  `stability_margin: Option<StabilityMarginArg>` (`Ratio | auto`; `auto` → descriptor
  default), `contact_response: Option<serde_yaml::Value>` (carried). The remaining table
  params (`grasp_handle` / `frame` / `position_tolerance` / `max_velocity` /
  `max_acceleration` / `timeout`) carry their spec defaults and are not emitted in v0 —
  mirrors how `TransportMoveToPose` models only `target_pose`.
- **`Primitive::TransportCarry`** + `#[serde(rename = "transport.carry")]`.
- **`grasp_force::carry_a_max(weight_n, payload_n, disturbance_n, stability_margin) -> f64`**
  — the § 2 formula. A pure fn beside `dynamic_a_max`, documented as a v0 reference choice;
  unit-tested for the reduction (`m=0,D=0 ⟹ dynamic_a_max`), the strict-below property, and
  the clamp-at-zero case.
- **`lower_transport_carry`**: resolves `motion.to_pose` to a `FrameRelative` pose (same path
  as `lower_transport_move_to_pose`); when `ctx.held` is set, emits
  `force_profile.min_holding_force` (the held-secured floor — identical to increment 10) and
  clamps `motion_bounds.a_max` via `carry_a_max(weight, payload, D, m)` then `min` with the
  kinematic ceiling (the authored ceiling string is kept verbatim when the clamp is not
  strictly tighter — the established no-reformat guard). Emits `disturbance_budget` and the
  resolved `stability_margin` into `force_profile` (the foothold increment-2 reads).
  `timing_mode: TimeScalable`.

**No schema change.** `disturbance_budget` / `stability_margin` ride the open
`force_profile` object and `a_max` the open `motion_bounds` — both pass the open `Envelope`
floor of the driver-interface schema (the boon round-trip test auto-revalidates), exactly
as `min_holding_force` and the GF2c `a_max` did.

## 5. Component 2 — rfl-conformance (the enriched interval check)

- **`envelope_class_for("carry") → Some(EnvelopeClass::IntervalInvariant)`** — one arm, no
  signature change. The N=3 multi-sample driver is inherited (it gates on this exact value).
- **Enrich the `IntervalInvariant` arm** of `check_envelope`. After the existing structural
  checks (`Succeeded` ∧ every sample carries a `realized_pose`), add: *if the action carries
  `force_profile.min_holding_force`, then `securing_force ≥ floor` at **every** interval
  sample.* This is the held-secured leg of §4.4's interval invariant, sampled over the whole
  interval. Factor the floor comparison into a shared helper used by both the
  `GraspContinuity` arm and this one (no logic duplication).
  - **Vacuous for `hover`** (it emits no `force_profile`) → hover goldens and the existing
    interval tests are unchanged. The enrichment only bites a *held* interval action.
- **Adversarial coverage, reusing existing drivers:** nominal carry passes (3 samples, each
  carrying `realized_pose` + the `securing_force` echoed from the floor);
  `FaultyDriver(UnderSecure)` fails carry (floor breached across the interval — the held leg);
  `FaultyDriver(MidIntervalDrop)` fails carry (mid-interval pose gap — the station leg). Both
  legs of the held interval invariant are adversarially verified with drivers that already
  exist. No rfl-core / spec / schema change in this component.

## 6. Component 3 — descriptors + worked example (skill variant in 01)

The example is a **skill variant in `examples/01-cable-insertion/`**, reusing the connector
mass fixture and the three structurally-distinct descriptors.

- **`skill-carry.yaml`**: `let connector_t = sense.locate(connector)` → `grasp.pinch(connector_t,
  …)` → `transport.carry(motion: {to_pose: …}, disturbance_budget: <explicit Force>,
  stability_margin: auto)`. Three statements, suffixes `[locate, pinch, carry]`. The pinch
  establishes `ctx.held` from the connector's declared `estimated_mass: 1.45 N` (the same
  fixture that gives `min_holding_force = 2.9 N`), so the carry has a held object to keep
  secured — the discipline requirement that a held-grasp increment declare `estimated_mass`.
- **Descriptor edits (additive)** to allegro / leap / pneumatic-6f: add `transport.carry` to
  `capabilities.skills` and `stability_margin: <ratio>` to `limits` (M2-required). Per-hand
  `stability_margin` (and the per-hand `payload_grasp_pinch`) make the carry `a_max` clamp
  **distinct per hand** — Principle-1 for the carry bound. Values are chosen so the clamp
  visibly bites (golden-pinned).
- **Byte-identical guard:** the additions do not touch any non-carry lowering, so the
  existing cable goldens (retarget_determinism, driver_protocol, envelope_conformance) and
  the screw goldens must stay byte-identical — re-run and `git diff`-verify before staging,
  the established guard.

## 7. Tests and validation

- **rfl-core unit tests**: `carry_a_max` (reduction / strict-below / clamp-at-zero); the
  lowering (held carry emits `min_holding_force` + `disturbance_budget` + `stability_margin`,
  `a_max` clamped per hand, `stability_margin: auto` reads the descriptor); `transport.carry`
  parse.
- **New conformance test** `tests/carry.rs` (or `transport_carry.rs`): per-hand retarget
  goldens (distinct carry `a_max`) + generate-twice determinism + boon schema validity +
  nominal interval pass + `UnderSecure` fails (held leg) + `MidIntervalDrop` fails (station
  leg).
- **Class-1** of `skill-carry.yaml` via a one-off `uv … jsonschema` script (validate.py is
  ex01-`skill.yaml`-only).
- **validate.py C1–C7** stays green: the descriptors still validate (M2 satisfied by the
  added `stability_margin`; `transport.carry` is in the C1 capability enum), the cable
  `skill.yaml` is untouched.

## 8. Deferred (documented)

- `disturbance_budget: auto` derivation (no normative formula — same posture as
  `coverage_overlap: auto`).
- The `{trajectory: T}` MoveSpec form (v0 lowers `{to_pose: P}` only).
- **ENV3 over-budget disturbance injection** (= increment 2 / §4.4 C2: a calibrated impulse
  > `disturbance_budget` must degrade gracefully — halt with the object still secured). This
  increment emits `disturbance_budget` precisely so that bench has a budget to inject against.
- The concrete geometric interval invariant (`‖pose − S(t)‖ ≤ tol` ∀ t) — needs spec/02
  concrete poses, deferred with the rest of the v0 symbolic-pose posture.
- The §4.4 precondition-reject failure modes (`insufficient_stability_margin` etc.) — v0 has
  no precondition checker (parallel to GF2c).
