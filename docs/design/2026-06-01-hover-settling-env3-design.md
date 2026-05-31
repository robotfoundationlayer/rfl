# Design: reach.hover settling ENV3 — the disturbance-rejection twin for the no-object interval primitive

Status: approved design, pre-implementation (2026-06-01; reconciled to the existing
`spec/01` § 1.5 contract before planning — see § 0).

This is the fifteenth reference-implementation increment. It closes the deferred twin that
the fourteenth (`2026-06-01-env3-disturbance-injection-design.md`, lines 46-49 / 151)
explicitly parked: ENV3 for `reach.hover`. ENV3 (`spec/05` § Conformance obligations,
§ Disturbance injection) requires a disturbance-rejecting interval-invariant primitive be
*perturbed* — the bench injects a calibrated disturbance, verifies the invariant holds under
perturbation, and verifies graceful degradation above the envelope. `transport.carry` got
this in increment 14; `reach.hover` is the **other** interval-invariant primitive (ENV1,
`spec/05`:31,49) and currently has **no perturbation test at all** — its `check_envelope` arm
is structural only (every sample carries a `realized_pose`), never checking the pose is
*correct*. So "conformance complete" silently covers 1 of 2 interval-invariant primitives.
This increment makes hover's station-keeping falsifiable. **The contract is already fully
written in `spec/01` § 1.5; this increment implements it** — the spec is authoritative.

## 0. Reconciliation to the existing spec (the stop-gate that reshaped this design)

`spec/01` § 1.5 (lines 630-685) and `schemas/skill-isa.schema.json` `ReachHoverParams`
(lines 781-836) **already specify hover's settling contract in full** — the implementation
just hasn't realized it (rfl-core's `ReachHover` parses only the v0 subset
target/standoff/duration). The pre-reconciliation draft of this doc invented names that are
wrong; the real spec is:
- **`station_tolerance`** (`Length`, default `2 mm`) — "allowed positional excursion from the
  hover setpoint" (§ 1.5 table line 642). *Not* a new `settling_tolerance`.
- **`settling_time`** (`Duration | auto`, default `auto`) — "max time to return within
  `station_tolerance` after a disturbance" (line 646). Already the right name.
- **No `disturbance_budget` on hover.** The § 1.5 table is "the twelve parameters … no more"
  (schema `additionalProperties: false`) and does *not* include one. Carry declares
  `disturbance_budget` because it is a *control* input (it feeds `carry_a_max`); hover has no
  such reservation — the bench applies "a calibrated lateral impulse" (C2, line 685) and the
  envelope it may exceed is the hover's own control authority, not a declared budget.
- The recovery rule is **verbatim** the leg this increment adds (line 668): "after a bounded
  disturbance, station error returns ≤ `station_tolerance` within `settling_time`; failure to
  recover is an envelope violation."
- The failure token is **`station_exceeded`** (line 679: "error > `station_tolerance` beyond
  `settling_time` → abort + safe state"). *Not* `settling_exceeded`.
- The bench checks are already written: **C1 — sustained hold** (line 684, already covered by
  the existing ENV2 structural check) and **C2 — disturbance rejection** (line 685): "apply a
  calibrated lateral impulse. PASS iff station error returns ≤ `station_tolerance` within
  `settling_time` (recover-and-continue), OR — if the impulse exceeds the envelope — the
  primitive aborts within `stop_time` to a safe state."

**Consequences:** (i) **no skill-isa schema change** — the params already exist; (ii) **no
`spec/01` change** — the contract is already complete; (iii) the only schema change is the
telemetry `station_error` signal (§ 2); (iv) the only spec prose change is generalizing the
`spec/05` ENV3 clause, currently carry-only, to name hover's degradation shape.

## 1. Why hover's ENV3 is NOT carry's

Carry's danger under excess disturbance is dropping the payload → its contract is "halt with
the **object secured**." Hover holds **no object**; its danger is the controlled frame
drifting off-station → its contract is **recover-to-station or abort-to-safe-state**. So:
- **Recovery, not securing.** Under a sub-envelope impulse the hover must *return* to within
  `station_tolerance` within `settling_time` and hold — a settling property carry has no
  analogue of (carry's invariant is static "stay secured at every sample").
- **Honest abort, not object-secured.** Over the envelope, the conformant degradation is to
  abort to a safe state and report `station_exceeded` (no payload to secure).

## 2. The real gap: there is no station-keeping signal

Hover's interval check is vacuous because the protocol has **no scalar for how far the frame
drifted from its station**. `Telemetry` (`rfl-core/src/driver.rs`) carries `realized_pose`,
`wrench`, `securing_force`, an open `events` array (`ForceEventFloor`: breakaway / detent),
and tactile readings — none expresses positional station error. Carry's ENV3 was "no schema
change" only because `securing_force` already existed. Hover has no equivalent:

> Closing the gap honestly requires introducing the station-keeping signal — a `station_error`
> scalar (a **Length**, the "externally measured station error" of § 1.5 C1) — an additive,
> backward-compatible field on `TelemetryFeedback`. That schema change *is* the gap-closure.

It is geometry-free at the conformance layer: the driver self-reports the scalar exactly as
it self-reports `securing_force`, so no concrete-pose comparison is needed and the deferred
`spec/02` concrete-geometry work is not pulled in.

## 3. The contract (as realized)

Lowered from the **existing** `reach.hover` params when an explicit `settling_time` is given
(the ENV3 opt-in marker, mirroring carry's explicit `disturbance_budget`); a bare hover
(`settling_time` absent / `auto`) emits nothing and stays ENV2-only:
- `station_tolerance` — `Length`, default `2 mm` if omitted.
- `settling_time` — explicit `Duration` (an `auto`/absent value → no `station_keeping`, no
  ENV3, v0; `auto` derivation deferred).

**Under-envelope (the interval property, strengthened — § 1.5 line 668).** PASS iff the hover
Succeeded, every sample carries `realized_pose` (existing structural leg), **and the settled
tail holds**: every telemetry sample with `t ≥ telemetry[0].t + settling_time` has
`station_error` present and ≤ `station_tolerance` (samples inside the leading `settling_time`
grace window are exempt — recovery in progress), with ≥ 1 such tail sample (non-vacuous). A
driver that drifts and *claims success* fails here.

**Over-envelope (graceful degradation — § 1.5 C2, line 685 / failure mode line 679).** Splits
one report into two opposite verdicts, exactly as carry's does:
- `check_envelope(IntervalInvariant)` → **Fail** (outcome not Succeeded → the first arm
  check fails; the hover correctly did not claim success).
- a new `check_settling` → **Pass** iff `outcome != Succeeded` ∧ `failure_detail ==
  "station_exceeded"` (abort to a safe state with the right halt reason). v0 realizes "safe
  state within `stop_time`" as the honest non-claim; the timing/geometry of the safe state is
  deferred (§ 7).

## 4. Surface (files)

**Schema (the one additive change — breaks the prior "no schema change" streak, by design):**
- `schemas/driver-interface.schema.json` — add optional `station_error` (referencing the
  existing `Length` `$def`, pattern `m|mm|cm`) to `TelemetryFeedback.properties`.
  Backward-compatible (optional; the block is `additionalProperties: false`, so it MUST be
  declared). C1–C7-neutral (those check capability enum / tactile closed-core / extension
  patterns, not telemetry properties). The hover golden's execute lines and the existing
  example telemetry stay schema-valid (the field is absent there).

**rfl-core:**
- `src/driver.rs` — `Telemetry.station_error: Option<Quantity>` (`skip_serializing_if =
  Option::is_none`), `None` in the placeholder sample and the existing serialization test.
- `src/skill_isa.rs` — `ReachHover` gains `station_tolerance: Option<Quantity>` and
  `settling_time: Option<serde_yaml::Value>` (parse the `Duration | auto` form; treat a
  non-string/`auto` as "no ENV3"), both `#[serde(default)]`. No new schema (already declared).
- `src/canonical.rs` — `Envelope.station_keeping: Option<serde_json::Value>` (parallel to
  `force_profile`; `None` by default). A `Length` + a `Duration` do not fit `force_profile`
  (force/torque) semantically. Add `station_keeping: None` to the three `Envelope` literal
  sites (`canonical.rs` test `sample()`, `translation.rs:184`, `base_envelope`).
- `src/translation.rs` — `lower_reach_hover` emits `station_keeping{station_tolerance,
  settling_time}` when `settling_time` is an explicit `Duration` string; `station_tolerance`
  defaults to `"2 mm"`. Emits nothing otherwise (a bare hover is unchanged → the existing
  `hover_lowers_to_a_standoff_setpoint` unit test is untouched).

**rfl-conformance (`src/lib.rs`):**
- `ReferenceDriver::execute` — echo `station_error = Some("0 mm")` on every sample when the
  action carries `station_keeping` (the nominal hover holds station perfectly), `None`
  otherwise — exactly the pattern that echoes `securing_force` from `min_holding_force`.
- Add `station_keeping_violation(goal, report) -> Option<String>` (parallel to
  `securing_floor_violation`): reads `station_tolerance` + `settling_time` from
  `station_keeping`; returns a reason if any settled-tail sample is missing `station_error` or
  exceeds tolerance, else `None`. Returns `None` when `station_keeping` is absent → carry /
  bare hover unaffected (the vacuous-leg pattern).
- Strengthen the `IntervalInvariant` arm of `check_envelope` with a call to
  `station_keeping_violation` (after the existing `securing_floor_violation` call).
- Add `check_settling(goal, report) -> CheckOutcome` (over-envelope honest-abort, parallel to
  `check_graceful_degradation`).
- Add `HoverSettlingDriver { inner, response }` with `HoverResponse`: `Recovers` (sample[0]
  `station_error` above tolerance during the grace window, settled tail ≤ tolerance,
  Succeeded — recover-and-continue), `FailsToRecover` (all samples above tolerance yet claims
  Succeeded — adversarial), `Aborts` (over-envelope conformant: Failed + `blocked` +
  `station_exceeded`), `ClaimsSuccess` (over-envelope adversarial: Succeeded).

**spec (this repo's lane):**
- `spec/05-conformance.md` — generalize the § Disturbance-injection clause / ENV3 bullet
  (currently carry-only: "object secured") to name hover's degradation shape: recover within
  `settling_time` or abort to a safe state (`station_exceeded`), per `spec/01` § 1.5 C2. No
  `spec/01` change (already complete).

**examples + goldens:**
- `examples/02-surface-scan/skill-hover.yaml` — add `station_tolerance: 2 mm` and
  `settling_time: 1 s` to the hover action (the ENV3 opt-in).
- Regenerate the three `surface_scan_hover` goldens (`INSTA_UPDATE=always`): the hover execute
  line gains `"station_keeping":{"station_tolerance":"2 mm","settling_time":"1 s"}` between
  `motion_bounds` and `stop_time`. The `inspect` line is unchanged. Predicted, explainable.
- Tests in `tests/envelope_conformance.rs`: settled-tail PASS (`Recovers`) + `FailsToRecover`
  FAIL (interval) + `Aborts` PASS (`check_settling`) + opposite-verdict interval FAIL +
  `ClaimsSuccess` FAIL (`check_settling`), plus a lib unit test for `check_settling` on a
  hand-built report (covering the check independently of the driver).

## 5. Non-vacuity (the anti-vacuity guard)

Two adversarial drivers prove the new teeth bite non-circularly, each targeting a different
new mechanism:
- **`FailsToRecover`** (settled-tail `station_error` > `station_tolerance`, claims Succeeded)
  → the strengthened `IntervalInvariant` settled-tail leg **Fails**. Proves `station_error`
  earns its keep: without it a non-recovering hover claiming success is indistinguishable from
  a real recovery.
- **`ClaimsSuccess`** (over-envelope, Succeeded) → `check_settling` **Fails** (false success).

The conformant `Recovers` / `Aborts` drivers produce the opposite-verdicts property
(under-envelope: interval Pass; over-envelope: interval Fail + `check_settling` Pass),
mirroring carry's ENV3. The timing is consistent with the v0 driver: with hover as action 1,
samples land at `t = 1, 2, 3`; `settling_time = 1 s` puts the deadline at `t = 2`, so
sample 0 (`t = 1`) is the grace window and samples 1–2 (`t = 2, 3`) are the settled tail.

## 6. TDD / commit shape (executing-plans, inline)

Four commits, each red→green; `cargo test` (workspace) + `validate.py` read in a batch
*separate* from the commit; each ff-pushed with a `git show --stat` self-check:
1. **schema + telemetry field** — `station_error` in `driver-interface.schema.json` +
   `Telemetry.station_error`; driver-protocol round-trip + `validate.py` stay green.
2. **lowering** — `ReachHover` args + `Envelope.station_keeping` (+ 3 literal sites) +
   `lower_reach_hover` emit + `examples/02` skill-hover args + `surface_scan_hover` golden
   regen. No new `Primitive`/`EnvelopeClass` variant → no exhaustive-match cross-arm churn.
3. **conformance** — `ReferenceDriver` echo + `station_keeping_violation` + strengthen the
   `IntervalInvariant` arm + `check_settling` + `HoverSettlingDriver`/`HoverResponse` + the
   five tests + the lib unit test.
4. **spec + README** — `spec/05` ENV3 clause generalization + README status line.

## 7. Faithfulness, determinism, and what stays deferred

- **Faithful:** the abort report reuses the increment-4 ownership split (`failure_class =
  "blocked"` protocol-level + `failure_detail = "station_exceeded"` the `01` primitive
  reason). `station_error` is the protocol form of § 1.5's "externally measured station
  error," self-reported exactly as `securing_force` is — the same symbolic v0 posture as
  placeholder poses and schematic forces.
- **Deterministic:** the bench mutates a deterministic nominal report by fixed rules; the only
  new float reformat is `station_error`'s `Quantity` string via the existing path. One golden
  changes (`surface_scan_hover`, gaining `station_keeping`); pass/fail asserts elsewhere.
- **Deferred behind named prerequisites:** the **safe-state geometry / "within `stop_time`"
  timing** of the abort (v0 = honest non-claim); **`settling_time: auto` derivation** (an
  embodiment-derived recovery budget); **`track_target: true`** tracking-bandwidth checks;
  **physics-based injection** (v0 models the response, not the physics). The rest of the
  standing deferred list (force.cut/wipe/scrub/press_button, Σ arc/path/volume, full
  held-interval GC1, GC2–6, per-skill ε-table, ROS 2 binding, Class 4, `{trajectory}`
  MoveSpec, `disturbance_budget: auto`) stays parked.
