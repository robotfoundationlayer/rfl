# Design: envelope-class checkers + adversarial drivers (Test Class 3, part 2 — C2)

Status: approved design, pre-implementation (2026-05-31)

This is the fifth reference-implementation increment, and the second half of
conformance **Test Class 3**. C1 closed the protocol round-trip (a driver consumes
`execute` and reports `telemetry` + `status`, schema-valid and correlated). C2 makes
the verification **non-circular**: it implements the envelope-class *checkers* that
judge a report against each primitive's envelope (`spec/05` § The envelope-class
taxonomy, ENV1–ENV4), and proves with adversarial drivers that each checker
**rejects** a violation rather than merely passing the nominal driver. The
specification under `spec/` is authoritative; this document describes how the
reference implementation realizes it.

## 1. Why this increment

C1's nominal `ReferenceDriver` always succeeds, so C1 verifies only that the
round-trip is *well-formed* — not that a misbehaving driver would be *caught*. A
conformance suite that only passes the nominal case is circular. C2 adds the
judgement: the four envelope classes (`spec/05` ENV1) are the reusable verification
shapes, and a primitive's conformance test is its envelope class instantiated with
that primitive's bounds. Three of the four are exercised by the cable example;
crucially, increment 3's emitted bounds (`min_holding_force`, the axial budget) are
exactly what the grasp-continuity and force-trajectory checkers sample, and C1
confirmed they flow through the round-trip — so C2 can read them straight from the
`execute` message.

## 2. Scope

In scope (C2):

- An `EnvelopeClass` enum + the ENV1 primitive→class mapping.
- Three checkers (`check_envelope`): terminal-postcondition (structural),
  grasp-continuity (numeric, GC1), force/torque-trajectory (numeric, ENV4).
- A fault-injecting adversarial driver (`FaultyDriver`) and a conformance test where
  the nominal driver passes every checker and each fault is rejected by the
  matching checker.

Deferred (documented, consistent with YAGNI — no cable primitive exercises them):

- **Interval-invariant** (ENV2/ENV3): needs `reach.hover` / `transport.carry`; the
  cable skill has neither. (Disturbance injection, ENV3, comes with it.)
- **Held-transport GC1 propagation:** `transport.move_to_pose` is grasp-continuity
  by ENV1, but its `execute` message carries no `min_holding_force` (increment 3 put
  the floor only on `grasp.pinch`). Propagating the floor from the grasp through the
  held transport is a refinement (a stateful checker + a driver that emits
  `securing_force` on the held transport).
- **GC3 / GC4 / GC6** continuity sub-modes (gaiting, regrasp, pivot, two-party
  handoff): no cable primitive; only **GC1 base continuity** is exercised.
- The per-skill **ε-tolerance table** (`spec/05` § Open issues, data-dependent): the
  v0 checks use exact/within-budget comparison on the emitted bounds; the tolerance
  refinement awaits reference-implementation data.

## 3. The envelope-class mapping (ENV1)

`spec/05` ENV1 assigns every primitive exactly one class by category: `reach`
terminal-postcondition (except `hover`); `reach.hover` / `transport.carry`
interval-invariant; `grasp` / `in_hand` / `transport` / `place` grasp-continuity;
`force` force/torque-trajectory. `sense.*` has no motion envelope (perception).

The mapping is keyed by the action-id suffix the retarget engine emits:

| suffix | EnvelopeClass | checked in v0? |
|---|---|---|
| `align`, `retract`, `scan` | TerminalPostcondition | yes (align, retract) |
| `pinch`, `release`, `transport` | GraspContinuity | `pinch` numeric; `transport`/`release` vacuous (no floor on their execute message) |
| `insert_fit` | ForceTrajectory | yes |
| `locate`, `inspect` | None (perception) | no |

```
enum EnvelopeClass { TerminalPostcondition, GraspContinuity, ForceTrajectory }
fn envelope_class_for(suffix: &str) -> Option<EnvelopeClass>
```

Interval-invariant is intentionally not a variant in v0 (no exercising primitive); it
is added when a hover/carry example exists.

## 4. The three checkers

`check_envelope(class, goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome`,
where `CheckOutcome` is a pass / fail with a human-readable reason. Each is a pure
function of the commanded `execute` message and the driver's report.

- **Terminal-postcondition** (`reach.*`) — structural, since v0 retarget targets are
  symbolic (no concrete target pose to measure against): passes iff
  `status.outcome == Succeeded` and `status.final_pose` is present (the action
  reached a reported rest state). `spec/05`: "the end state, pose at rest".
- **Grasp-continuity / GC1** (`grasp.*` etc.) — numeric: read
  `min_holding_force` from the goal's `safety_envelope.force_profile` (the
  `"<x> N"` string GF1c emits); if present **and** the telemetry reports a
  `securing_force`, every sample must satisfy `securing_force ≥ min_holding_force`.
  When the floor is absent (transport / release carry none in v0) the check passes
  vacuously — there is no claimed floor to violate. `spec/05` GC1: "a securing
  contact set maintains the object at ≥ `min_holding_force` at every sampled
  instant".
- **Force/torque-trajectory / ENV4** (`force.*`) — numeric: read the force budget
  from the goal (`canonical_action.force_budget`); every telemetry sample must
  satisfy `|wrench.force| ≤ budget` (the magnitude along the loaded axis; v0 emits
  and checks the axial component). `spec/05` ENV4: "bounds the force profile
  interval-sampled against per-axis budgets; a mid-motion spike is a violation".

Quantities are parsed via `Quantity::parse` (`"8 N" → 8.0`); the `force_profile`
value is a `serde_json::Value` object whose `min_holding_force` string is read and
parsed.

## 5. The adversarial driver

```
enum Fault { UnderSecure, OverForce, NeverSettle }
struct FaultyDriver { inner: ReferenceDriver, fault: Fault }
impl Driver for FaultyDriver { /* nominal report, then inject one fault */ }
```

`FaultyDriver` composes the nominal `ReferenceDriver` (DRY), then mutates one field
to a **schema-valid but envelope-violating** value (so the report still validates
against the driver-interface schema — the violation is semantic, caught by the
checker, not the schema):

- `UnderSecure` — set any telemetry `securing_force` to `"0.1 N"` (below every
  derived floor); violates GC1 on the grasp action.
- `OverForce` — set any telemetry `wrench.force` to `[0, 0, 999]` (above every
  budget); violates the force-trajectory bound on the force action.
- `NeverSettle` — set `status.outcome = Indeterminate` and `status.final_pose =
  None`; violates the terminal-postcondition on the reach actions.

Injecting uniformly (on every action that carries the relevant field) keeps the
driver simple; the test selects the action where the matching checker must fail.

## 6. Tests (`crates/rfl-conformance/tests/envelope_conformance.rs`)

- **Nominal conformance:** over allegro / leap / pneumatic, for every action with an
  `EnvelopeClass`, `check_envelope` passes. (The nominal `ReferenceDriver` conforms.)
- **Adversarial bite (the non-circular proof):**
  - `FaultyDriver(UnderSecure)` → the `grasp.pinch` action's GraspContinuity check
    **fails** (securing 0.1 N < floor 2.9 N).
  - `FaultyDriver(OverForce)` → the `force.insert_fit` action's ForceTrajectory
    check **fails** (999 N > budget).
  - `FaultyDriver(NeverSettle)` → a `reach` action's TerminalPostcondition check
    **fails** (outcome indeterminate / no final pose).
- A unit test of `envelope_class_for` (the ENV1 mapping) for each suffix.

No new goldens — the checks are pass/fail assertions. `validate.py` (C1–C7) and the
existing rfl-core + conformance tests (incl. C1's `driver_protocol`) stay green; no
`rfl-core`, `spec/`, or `schemas/` file changes.

## 7. Sections to transcribe during implementation

- `spec/05` § The envelope-class taxonomy (ENV1 class assignment, the
  terminal vs interval distinction, ENV4 trajectory bounding) and § Grasp-continuity
  modes (GC1 base continuity — already read for this design).
- The C1 types (`rfl-core::driver` `Telemetry`/`Status`/`Wrench`/`DriverReport`) and
  `rfl-core::canonical::ExecuteGoal` / `CanonicalAction` (the `force_budget` +
  `safety_envelope.force_profile` fields the checkers read) — already in place from
  C1 and increment 3.
