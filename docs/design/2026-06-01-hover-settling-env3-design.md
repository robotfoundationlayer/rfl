# Design: reach.hover settling ENV3 — the disturbance-rejection twin for the no-object interval primitive

Status: approved design, pre-implementation (2026-06-01)

This is the fifteenth reference-implementation increment. It closes the deferred twin that
the fourteenth (`2026-06-01-env3-disturbance-injection-design.md`, lines 46-49 / 151)
explicitly parked: ENV3 for `reach.hover`. ENV3 (`spec/05` § Conformance obligations,
§ Disturbance injection) requires that a disturbance-rejecting interval-invariant primitive
be *perturbed* — the bench injects calibrated disturbances up to the primitive's budget,
verifies the invariant holds under perturbation, and verifies graceful degradation above
budget. `transport.carry` got this in increment 14; `reach.hover` is the **other**
interval-invariant primitive (ENV1, `spec/05`:31,49) and currently has **no perturbation
test at all** — its `check_envelope` arm is structural only (every sample carries a
`realized_pose`), never checking the pose is *correct*. So "conformance complete" silently
covers 1 of 2 interval-invariant primitives. This increment makes hover's station-keeping
falsifiable. The specification under `spec/` is authoritative; this document describes how
the reference implementation realizes it (and the small spec additions it requires).

## 1. Why this increment, and what hover's ENV3 adds (and why it is NOT carry's)

Carry's ENV3 contract is "halt with the **object secured**, do not drop it" — its danger
under excess disturbance is losing the payload. Hover holds **no object**; its danger is the
controlled frame drifting off its station uncontrolled. So hover's ENV3 is a genuinely
*different* contract, not a copy:

- **Recovery, not securing.** Under a sub-budget perturbation the hover must *return* to its
  station within a bounded window and hold it — a settling property the carry has no analogue
  of (carry's invariant is "stay secured at every sample," with no recover-from-excursion
  notion).
- **Honest non-claim, not object-secured.** Over budget, the hover's conformant degradation
  is to fail honestly (not pretend it held station). There is no payload to secure, so the
  "object secured" leg of carry's `check_graceful_degradation` is replaced by *failure-shape
  correctness* (the right halt reason, no false success).

## 2. The real gap: there is no station-keeping signal

The reason hover's interval check is vacuous is concrete: the protocol has **no scalar for
how far the controlled frame drifted from its station**. `Telemetry` (`03`,
`rfl-core/src/driver.rs`) carries `realized_pose`, `wrench`, `securing_force`, an open
`events` array (`ForceEventFloor`: breakaway / detent), and tactile readings — none of which
expresses positional station error. Carry's ENV3 was "no schema change" only because
`securing_force` already existed (from the held-transport GC1 increment) and carried its
invariant. Hover has no equivalent, so:

> Closing the gap honestly requires introducing the station-keeping signal — a `station_error`
> scalar — which is an additive, backward-compatible schema change. That schema change *is*
> the gap-closure, not incidental breadth.

`station_error` is a **Length** (distance from the standoff setpoint), not a Force — the
hover excursion is positional. It is geometry-free at the conformance layer: the bench/driver
reports the scalar (exactly as it self-reports `securing_force`), so no concrete pose
comparison is needed and the deferred concrete-geometry work (`spec/02`) is not pulled in.

## 3. The contract

Authored on `reach.hover` (three new optional args; all symbolic-free quantities):
- `disturbance_budget: Force | auto` — the perturbation the hover must reject. Reuses carry's
  `DisturbanceArg` enum; `auto` resolves to `0` in v0 (deferred derivation), the same
  convention as carry, so an un-perturbed hover is unaffected.
- `settling_time: Duration` — the recovery window: after a perturbation, the station must be
  re-established within this long.
- `settling_tolerance: Length` — the station-error threshold defining "recovered."

`settling_tolerance` is **authored**, not `auto`-derived: embodiment descriptors carry
`stability_margin` but **no** position/settling tolerance limit, so an `auto` form would
require an embodiment-descriptor schema change — deferred (§ 7).

**Under-budget (the interval property, strengthened).** Bench injects ≤ `disturbance_budget`.
PASS iff the hover Succeeded, every sample carries `realized_pose` (existing structural leg),
**and the settled tail holds**: every telemetry sample with `t ≥ first_t + settling_time` has
`station_error ≤ settling_tolerance` (samples inside the leading `settling_time` grace window
are exempt — recovery in progress), with at least one such tail sample present (non-vacuous).
A driver that drifts and *claims success* fails here.

**Over-budget (graceful degradation, hover flavor).** Bench injects > `disturbance_budget`.
This splits one report into two opposite verdicts, exactly as carry's does:
- `check_envelope(IntervalInvariant)` → **Fail** (the hover did not hold its station — it
  correctly did not claim success).
- a new `check_settling` → **Pass** iff `outcome != Succeeded` ∧ `failure_detail ==
  "settling_exceeded"` (honest non-claim with the right halt reason).

The bounded-excursion / "did-not-run-away" leg (a `max_excursion` cap, the positional
analogue of carry's "object secured") is **deferred** to avoid a fourth arg this increment
(§ 7); the over-budget contract here is honest-non-claim only.

## 4. Surface

**Schema (the one additive change — breaks the prior "no schema change" streak, by design):**
- `schemas/driver-interface.schema.json` — add optional `station_error` to
  `TelemetryFeedback.properties`, referencing a Length floor (mirror the existing `Force`
  `$def`; add a `Length`/`Distance` floor `$def` if none exists). Backward-compatible
  (optional; `TelemetryFeedback` is `additionalProperties: false`, so it MUST be declared).
  C1–C7-neutral (those check capability enum / tactile closed-core / extension patterns, not
  telemetry properties). `validate.py` and the driver-protocol round-trip re-validate; the
  existing examples omit the field and stay valid.

**rfl-core:**
- `src/driver.rs` — `Telemetry.station_error: Option<Quantity>` (`skip_serializing_if =
  Option::is_none`), `None` in the placeholder sample.
- `src/skill_isa.rs` — `ReachHover` gains the three `#[serde(default)]` args.
- `src/canonical.rs` — `Envelope.station_keeping: Option<serde_json::Value>` (parallel to
  `force_profile`; `None` by default). A Duration + a Length do not fit `force_profile`
  (force/torque) semantically, so a parallel field is clean. Verify the driver-interface
  CanonicalAction floor permits the extra envelope key (it is `02`-owned / floored); add to
  the schema only if the execute-goal validation rejects it.
- `src/translation.rs` — `lower_reach_hover` emits `station_keeping{disturbance_budget,
  settling_time, settling_tolerance}` when the args are present (parse `auto → 0`, same as
  `lower_transport_carry`); emits nothing when absent (a bare hover is unchanged → no golden
  churn for skills that omit the args).

**rfl-conformance (`src/lib.rs`):**
- Strengthen the `IntervalInvariant` arm with a **settled-tail leg**, *vacuous when
  `station_keeping` is absent* — the same pattern the held-secured floor uses
  (`securing_floor_violation` returns `None` with no floor). Carry and bare hover are
  unaffected because they emit no `station_keeping`.
- Add `check_settling(goal, report) -> CheckOutcome`, the over-budget honest-halt check,
  parallel to `check_graceful_degradation`.
- Add `HoverSettlingDriver { inner, injected_n, response }` reading
  `station_keeping.disturbance_budget` + `settling_tolerance`, with a `HoverResponse` enum:
  `Recovers` (sub-budget conformant: settled tail ≤ tolerance, Succeeded), `FailsToRecover`
  (sub-budget adversarial: tail `station_error` > tolerance yet claims Succeeded),
  `GracefulHalt` (over-budget conformant: Failed + `settling_exceeded`), `ClaimsSuccess`
  (over-budget adversarial: Succeeded).

**spec (this repo's lane):**
- `spec/01-skill-isa.md` § 1.5 — document the three `reach.hover` args + the settling
  contract.
- `spec/05-conformance.md` — extend the ENV3 / Disturbance-injection clause to spell out
  hover's recovery/degradation contract (the spec already lists hover as interval-invariant
  and frames ENV3 generically; this fills in hover's previously-unstated degradation shape).

**examples + goldens:**
- `examples/02-surface-scan/skill-hover.yaml` — add the three args to the hover action.
- Regenerate the `surface_scan_hover` golden (hover now emits `station_keeping`).
- Tests: settled-tail pass + `FailsToRecover` fail + over-budget `check_settling` pass +
  `ClaimsSuccess` fail, plus a lib unit test for `check_settling` on a hand-built report
  (covering the check independently of the driver). Co-located in
  `tests/envelope_conformance.rs` (the carry ENV3 tests live there too).

## 5. Non-vacuity (the anti-vacuity guard)

Two adversarial drivers prove the new teeth bite non-circularly, and each targets a different
new mechanism:
- **`FailsToRecover`** (sub-budget, `station_error` tail > `settling_tolerance`, claims
  Succeeded) → the strengthened `IntervalInvariant` settled-tail leg **Fails**. Proves
  `station_error` earns its keep: without it, a non-recovering hover that claims success could
  not be distinguished from a real recovery.
- **`ClaimsSuccess`** (over-budget, Succeeded) → `check_settling` **Fails** (false success).

The conformant `Recovers` / `GracefulHalt` drivers produce the opposite-verdicts property
(under-budget: interval Pass; over-budget: interval Fail + `check_settling` Pass), exactly
mirroring carry's ENV3.

## 6. TDD / commit shape (executing-plans, inline)

Four commits, each red→green; `cargo test` (workspace) + `validate.py` read in a batch
*separate* from the commit; each ff-pushed with a `git show --stat` self-check that only the
intended files moved:
1. **schema + telemetry field** — `station_error` in `driver-interface.schema.json` +
   `Telemetry.station_error`; driver-protocol round-trip + `validate.py` stay green.
2. **lowering** — `ReachHover` args + `Envelope.station_keeping` + `lower_reach_hover` emit +
   `examples/02` skill-hover args + `surface_scan_hover` golden regen. (Exhaustive-match: no
   new `Primitive`/`EnvelopeClass` variant, so no cross-arm churn — this is a field addition,
   not a variant addition.)
3. **conformance** — strengthen the `IntervalInvariant` arm + `check_settling` +
   `HoverSettlingDriver` + the four tests + the lib unit test.
4. **spec + README** — `spec/01` § 1.5 + `spec/05` ENV3 clause + README status line.

## 7. Faithfulness, determinism, and what stays deferred

- **Faithful:** the graceful report reuses the increment-4 ownership split
  (`failure_class = "blocked"` protocol-level + `failure_detail = "settling_exceeded"` the
  `01` primitive-specific reason). `station_error` is reported by the driver exactly as
  `securing_force` is — the same symbolic posture as v0's placeholder poses and schematic
  forces; the injection magnitude is the bench's input, the response is what the checks judge.
- **Deterministic:** the bench mutates a deterministic nominal report by fixed rules; the only
  new float reformat is `station_error`'s `Quantity` string via the existing path. One golden
  changes (`surface_scan_hover`, gaining `station_keeping`); pass/fail asserts elsewhere.
- **Deferred behind named prerequisites:** the over-budget **bounded-excursion `max_excursion`
  leg** (a fourth hover arg + a positional "did-not-run-away" check); **`settling_tolerance:
  auto`** (needs an embodiment-descriptor position-tolerance limit → descriptor-schema
  change); **physics-based injection** (v0 models the response, not the physics);
  **`disturbance_budget: auto` derivation** (the standing carry/hover-shared deferred). The
  rest of the standing deferred list (force.cut/wipe/scrub/press_button, Σ arc/path/volume,
  full held-interval GC1, GC2–6, per-skill ε-table, ROS 2 binding, Class 4, `{trajectory}`
  MoveSpec) stays parked.
