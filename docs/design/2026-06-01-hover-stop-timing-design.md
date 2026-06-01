# Design — hover over-envelope abort timing (`stop_time` conformance leg)

**Date:** 2026-06-01
**Increment:** 23
**Status:** approved
**Scope:** rfl-core (one additive `Status.stop_latency` field) + driver-interface schema (one
additive optional property) + rfl-conformance (`check_settling` strengthened to a timing leg +
one adversarial driver). No envelope class, no golden, no spec change.

## 0. Stop-gate reconciliation (why this is NOT "max_excursion")

The selected candidate was phrased "hover max_excursion + stop_time". Reading `spec/01` § 1.5
(`reach.hover`, lines 632-685) before drafting shows **`max_excursion` is not a spec concept** for
hover (zero matches across `spec/01`). What the spec actually obligates for the over-envelope case
is one thing:

> **C2 (line 685):** PASS iff station error returns ≤ `station_tolerance` within `settling_time`
> (recover-and-continue), **OR — if the impulse exceeds the envelope — the primitive aborts within
> `stop_time` to a safe state.** Either outcome must be the deterministic function of the impulse
> magnitude.

and the universal breach rule (line 669): "On any breach: decelerate to rest within
`embodiment.limits.stop_time`, ending in a safe state."

So this increment drops the invented `max_excursion` (a bounded-position-excursion guarantee the
hover spec does not state) and implements only the spec-grounded obligation: **the over-envelope
abort must reach a safe state within `stop_time`.** This is the same discipline that reshaped
increment 15 (which nearly invented `disturbance_budget`/`settling_tolerance` before the schema
read corrected it).

This is genuinely a new conformance *shape* — the **first latency / timing check**. Every prior
check is terminal-postcondition, interval-sample correctness, or a magnitude budget; none bounds a
*duration*.

## 1. The contract (v0)

For an over-envelope hover (the increment-15 `Aborts` path), `check_settling` today verifies only
the **outcome** of the abort: `outcome == Failed` ∧ `failure_detail == "station_exceeded"` (and it
rejects a pretended `Succeeded`). It does not verify the abort was *timely*.

Strengthen it: for a `station_exceeded` abort, additionally require that the externally measured
time from envelope breach to safe state (at rest) is **present** and **≤ `Envelope.stop_time`**.

The timing leg is **non-vacuous for the abort case** (an abort that omits its measured time is
malformed), but does not fire for a non-abort report (a pretended `Succeeded` is still caught by
the existing outcome leg, with no timing dependency). This keeps the failure semantics layered:
the outcome leg owns "did it abort honestly", the timing leg owns "did it abort in time".

## 2. Schema delta (additive; the project's 2nd schema change, mirroring increment 15)

`Envelope.stop_time` is **already emitted** on every canonical action (`canonical.rs:182`;
`translation.rs:242` copies it from `embodiment.limits.stop_time`, a mandatory reach-baseline
limit). So the *bound* needs no new plumbing.

The only new field is the *measurement*: add an optional `stop_latency` (Duration) to
`driver-interface.schema.json` `StatusResult`, and the matching `Status.stop_latency:
Option<Quantity>` in `driver.rs`. This mirrors exactly how increment 15 added the `station_error`
(Length) measurement to `TelemetryFeedback`. Additive + optional ⇒ existing reports stay
schema-valid; `skip_serializing_if = None` ⇒ existing driver outputs are byte-identical;
C1–C7 are skill-isa/embodiment-descriptor structure invariants and do not touch `StatusResult`
timing, so `validate.py` stays green. No retarget golden changes (a driver report is not golden
snapshotted; the canonical action — which is — is untouched).

Semantics: `stop_latency` is "the externally measured time from envelope breach to the at-rest
safe state", the time `stop_time` bounds. The analogue of "externally measured station error".

## 3. Implementation points

- **`crates/rfl-core/src/driver.rs`** — `Status.stop_latency: Option<Quantity>` (Duration,
  `skip_serializing_if = "Option::is_none"`), documented as the spec/01 § 1.5 abort timing
  measurement bounded by `embodiment.limits.stop_time`.
- **`schemas/driver-interface.schema.json`** — `StatusResult.properties.stop_latency` →
  `{ "$ref": "#/$defs/Duration", ... }` (add a `Duration` `$def` if absent, parallel to `Length`).
- **`crates/rfl-conformance/src/lib.rs`** — `check_settling(goal, report)` (signature gains `goal`
  to read `goal.canonical_action.safety_envelope.stop_time`). New leg: when
  `failure_detail == "station_exceeded"`, require `report.status.stop_latency` present and its
  magnitude ≤ the envelope `stop_time` magnitude; otherwise `Fail` with a precise reason. The
  existing outcome checks are unchanged.
- **`HoverSettlingDriver::Aborts`** (existing) — emit `stop_latency` within `stop_time` (e.g.
  `"0.05 s"` for a 0.1 s `stop_time`) so the strengthened contract stays green.

## 4. Adversarial drivers (non-circularity)

- **`AbortsTooSlow`** (new `HoverResponse` variant) — emits `station_exceeded` but
  `stop_latency = "0.5 s"` (> `stop_time`): the timing leg **Fails** while the outcome leg still
  **Passes**. This is the proof the `stop_latency` field is load-bearing — a check that ignored it
  could not distinguish this from the conformant abort.
- **`ClaimsSuccess`** (existing) — over-envelope yet `Succeeded`: caught by the existing outcome
  leg, no timing dependency.

(Optional, if cheap: `AbortsNoTiming` — `station_exceeded` with `stop_latency` omitted → malformed
abort `Fail`, hardening non-vacuity. Include only if it does not bloat the increment.)

## 5. Tests

- `check_settling` unit (lib.rs): abort-within-`stop_time` → Pass; abort-too-slow → Fail;
  pretended-`Succeeded` → Fail (outcome leg, unchanged).
- `envelope_conformance.rs` on `examples/02-surface-scan/skill-hover.yaml` hover: `Aborts`
  (within) → Pass; `AbortsTooSlow` → Fail.
- Follow the `check_settling` signature change at all call sites (3 in lib.rs tests, 2 in
  `envelope_conformance.rs`).

**Expected deltas:** rfl-core test count unchanged or +0 (the `stop_latency` field is exercised
through conformance, not a new rfl-core unit unless a driver-serialization unit is added);
conformance `lib` +1, `envelope_conformance` +1–2. All 12 golden binaries byte-identical.
`validate.py` C1–C7 PASS. A driver-protocol round-trip test may gain +1 if we assert
`stop_latency` serializes (parallel to `telemetry_with_station_error_is_schema_valid`).

## 6. Deferred (each behind a named prerequisite)

- **`max_excursion` / bounded-position-excursion during abort** — not a `spec/01` hover concept;
  revisit only if a future spec revision adds it.
- **Abort timing for non-hover primitives** — `stop_time` is a universal breach obligation
  (lines 449/559/613/727/856/909/…), but only the hover bench has a modeled disturbance→abort
  measurement path in v0. Generalizing needs each primitive's breach bench.
- `track_target:true` tracking-bandwidth checks; `settling_time:auto` derivation; physics-based
  disturbance injection (all pre-existing deferrals, unchanged).
