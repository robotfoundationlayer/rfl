# Design: ENV3 disturbance injection (transport.carry C2) — the final closing increment

Status: proposed design, pre-implementation (2026-06-01)

This is the fourteenth reference-implementation increment and the second of the two that
close the conformance story. It adds ENV3 (`spec/05` § Conformance obligations): a
disturbance-rejecting interval test injects calibrated disturbances up to the primitive's
`disturbance_budget`, verifies the invariant holds *under perturbation*, and verifies
*graceful degradation* (object secured) above budget. It targets `transport.carry`'s C2
sketch (`spec/01` § 4.4). With it the conformance story is complete: all four envelope
classes, interval coverage (ENV2), disturbance rejection (ENV3), held-transport
grasp-continuity, force/torque trajectory (ENV4), each verified against adversarial
drivers. The specification under `spec/` is authoritative; this document describes how the
reference implementation realizes it. **After this increment, in-process incrementing ends.**

## 1. Why this increment, and what ENV3 adds

ENV2 (increment 13) verifies the held interval invariant *nominally and against a
mid-interval drop*. ENV3 adds the adversarial **perturbation** dimension that the
disturbance-rejecting primitive exists for. `spec/05` ENV3: "injects calibrated
disturbances up to the primitive's `disturbance_budget`, verifies the invariant under
perturbation, and verifies graceful degradation (object secured) above budget." `spec/01`
§ 4.4 C2: "Apply a perturbation exceeding `disturbance_budget`. PASS iff `result ==
disturbance_exceeded` ∧ the embodiment halted to a stable configuration with the object
**still secured** (not dropped) — a graceful degradation, not a loss."

ENV3 therefore splits one over-budget report into **two opposite verdicts**:
- `check_envelope(IntervalInvariant)` → **Fail** — the carry did not reach its goal under
  the excess disturbance (it correctly did not claim success).
- a new `check_graceful_degradation` → **Pass** — it halted with the object still secured.

The non-circular proof is that an adversarial driver which *drops* the object, or one which
*pretends success*, fails `check_graceful_degradation`.

`spec/05` § Disturbance injection: "Disturbance injection is part of the bench, not the
primitive." So the bench injects; a conformant driver responds. In the v0 no-physics
reference implementation, a `DisturbanceDriver` parameterized by the injected magnitude
**models** that response (read `disturbance_budget` from the goal, maintain the invariant
under budget, halt-with-object-secured over budget) — the same symbolic posture as v0's
placeholder poses and schematic forces. The injection magnitude is the bench's input; the
response is what the checks judge.

## 2. Scope

`transport.carry` only — it is the primitive that *declares* `disturbance_budget`, which
increment 13 emits into the execute message's `force_profile`. `reach.hover`'s ENV3 is a
*different* contract (recover within `settling_time`, no held object to secure) and v0
`reach.hover` emits no budget, so it is deferred (it would need rfl-core lowering to emit a
settling/disturbance budget plus a recover-or-abort response model).

**No rfl-core / spec / schema change.** Everything lives in `rfl-conformance`, exactly like
the C2 envelope checkers (increment 5): the carry already emits `disturbance_budget`, the
graceful-halt report uses the existing `Outcome::Failed` + the open `failure_class` /
`failure_detail` fields + the open telemetry floors, and boon auto-revalidates. The schema
`status` requires only `[message, action_id, outcome]`; `failure_class` is an optional enum
that includes `"blocked"`; `failure_detail` is an optional lowercase token — so
`disturbance_exceeded` is a valid detail and the graceful report is schema-valid.

## 3. Component 1 — the disturbance bench (`rfl-conformance/src/lib.rs`)

A driver that models a conformant (or adversarial) response to an injected disturbance:

```rust
/// How a driver responds to an over-budget injected disturbance (the under-budget case is
/// always the nominal maintained invariant). The conformant response is `Graceful`; the two
/// adversarial responses prove `check_graceful_degradation` bites.
pub enum DisturbanceResponse {
    /// Conformant (§ 4.4 C2): halt with the object still secured.
    Graceful,
    /// Adversarial: the object is dropped (securing_force below floor) — a loss, not a halt.
    Drops,
    /// Adversarial: claim success despite the over-budget disturbance.
    ClaimsSuccess,
}

pub struct DisturbanceDriver {
    inner: ReferenceDriver,
    injected_n: f64,
    response: DisturbanceResponse,
}
```

`Driver::execute` reads `goal.canonical_action.safety_envelope.force_profile`
`["disturbance_budget"]`, parses its magnitude (N), and:
- `injected_n <= budget` → returns `inner.execute(goal)` **unchanged** (the invariant holds
  under perturbation — the ENV3 first clause).
- `injected_n > budget` → per `response`:
  - `Graceful`: `status.outcome = Failed`, `status.failure_class = Some("blocked")`,
    `status.failure_detail = Some("disturbance_exceeded")`, `final_pose` kept (halted to a
    stable config), every telemetry `securing_force` **kept ≥ floor** (object held). The
    `verdict.value` becomes `false` (honest: the postcondition did not hold).
  - `Drops`: as `Graceful` but every telemetry `securing_force` set below the floor
    (`"0.1 N"`, mirroring `Fault::UnderSecure`) — the object was lost.
  - `ClaimsSuccess`: returns `inner.execute(goal)` unchanged (a `Succeeded` report that
    ignores the disturbance).

If the goal carries no `disturbance_budget` (a non-carry action), the driver passes through
the nominal report (ENV3 does not apply).

## 4. Component 2 — the graceful-degradation check (`rfl-conformance/src/lib.rs`)

```rust
/// Verify the § 4.4 C2 graceful-degradation contract on an OVER-budget report: the carry
/// must NOT claim success, must report the `disturbance_exceeded` halt reason, and must keep
/// the object secured (securing_force >= floor at every sample) — a controlled halt, not a
/// drop. Distinct from `check_envelope`: this judges the *failure shape*, not correct execution.
pub fn check_graceful_degradation(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome
```

Pass iff all three hold; the failure reason names the first that does not:
1. `report.status.outcome != Outcome::Succeeded` (did not pretend success).
2. `report.status.failure_detail.as_deref() == Some("disturbance_exceeded")` (the right halt
   reason — § 4.4's `result == disturbance_exceeded`).
3. `securing_floor_violation(goal, report) == None` (object secured — reuses the increment-13
   helper; this is the "not dropped" clause).

`securing_floor_violation` already reads `min_holding_force` from the goal and checks every
telemetry `securing_force` against it, so a `Drops` report (securing below floor) fails
clause 3, and a `ClaimsSuccess` report fails clause 1.

## 5. Component 3 — tests (`rfl-conformance/tests/envelope_conformance.rs`)

All drive `examples/01-cable-insertion/skill-carry.yaml` (carry = action index 2) on allegro,
whose emitted `disturbance_budget` is `0.4 N`. Injected magnitudes straddle it.

- **`carry_invariant_holds_under_sub_budget_disturbance`** — `DisturbanceDriver { 0.3,
  Graceful }` → `check_envelope(IntervalInvariant, carry) == Pass` (the invariant holds under
  perturbation up to budget; non-vacuous: 3 interval samples).
- **`over_budget_disturbance_degrades_gracefully`** — `DisturbanceDriver { 0.6, Graceful }`
  → `check_graceful_degradation(carry) == Pass` **and**
  `check_envelope(IntervalInvariant, carry) == Fail` (the opposite-verdicts property: it did
  not succeed, but it halted with the object secured).
- **`over_budget_drop_fails_graceful_degradation`** — `DisturbanceDriver { 0.6, Drops }` →
  `check_graceful_degradation(carry) == Fail` (object lost — the non-circular bite).
- **`over_budget_false_success_fails_graceful_degradation`** — `DisturbanceDriver { 0.6,
  ClaimsSuccess }` → `check_graceful_degradation(carry) == Fail` (pretended success).

Plus one lib unit test for `check_graceful_degradation` on a hand-built report (Pass on a
secured-halt, Fail on a non-`disturbance_exceeded` outcome) so the check is covered
independently of the driver.

## 6. Faithfulness, determinism, and what stays deferred

- **Faithful:** the graceful report's `failure_class = "blocked"` (the protocol-level mode:
  the carry could not proceed) + `failure_detail = "disturbance_exceeded"` (the 01
  primitive-specific reason) follows the increment-4 ownership split. "Object secured" is the
  same `min_holding_force` floor the held interval invariant already samples.
- **Deterministic:** the bench mutates a deterministic nominal report by fixed rules; no
  float reformat beyond the existing securing-force string. No goldens (pass/fail asserts),
  like the C2 envelope checkers.
- **Deferred:** `reach.hover` settling-time ENV3 (different contract; needs hover to emit a
  budget); physics-based injection (v0 models the response, not the physics); the § 4.4
  precondition-reject failure modes (`insufficient_stability_margin` etc., no v0 precondition
  checker). After this increment, in-process incrementing ends; the standing deferred list
  (force.cut/wipe/scrub/press_button, Σ arc/path/volume, full held-interval GC1, GC2–6,
  per-skill ε-table, ROS 2 binding, Class 4, `{trajectory}` MoveSpec, `disturbance_budget:
  auto`) stays parked behind its named prerequisites.
