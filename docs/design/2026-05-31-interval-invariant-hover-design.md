# Design: interval-invariant envelope class (reach.hover, ENV2)

Status: approved design, pre-implementation (2026-05-31)

This is the twelfth reference-implementation increment and the session's largest. It
introduces the fourth and final normative envelope class — **interval-invariant** — by
adding the `reach.hover` primitive (sustained station-keeping) and verifying it with
interval sampling: the maintained invariant is checked at *every* sample, so a
mid-interval violation fails even when the endpoint conforms (spec/05 ENV2). With it
the four-class envelope taxonomy (terminal-postcondition / grasp-continuity /
force-trajectory / interval-invariant) is complete in the reference implementation. The
specification under `spec/` is authoritative; this document describes how the reference
implementation realizes it.

## 1. Why this increment

spec/05 § The envelope-class taxonomy names four classes; three are implemented
(terminal, grasp-continuity, force/torque-trajectory). The interval-invariant class —
"a maintained invariant (station-keeping, dynamic stability), sampled at every step over
the interval" — has had no exercising primitive in the worked examples, so it was
omitted in v0 (`EnvelopeClass` carries a comment to that effect). ENV2: "an interval
class samples the invariant at every step over the whole interval; a mid-interval
violation fails the test even when the endpoint conforms." This is the property that
distinguishes the three interval classes from the endpoint (terminal) class.

`reach.hover` (spec/01 § 1.5) is the archetype: "maintain the controlled frame at a
standoff pose … over a bounded interval, without forming contact." Its conformance C1
PASSes iff `result == success` ∧ *at every sampled instant* the station error ≤
`station_tolerance` (interval sampling, not endpoint). Adding it lets the reference
implementation realize the interval-invariant class and demonstrate ENV2 directly.

## 2. v0 realization (structural interval check)

Realized poses are symbolic placeholders in v0 (the concrete `Pose6D` representation is
spec/02's, deferred — `RealizedPose::placeholder`). So the interval-invariant check is
*structural*, exactly parallel to how the terminal-postcondition check is structural in
v0: **every interval telemetry sample must carry a `realized_pose`** (the station was
maintained at that sampled instant), and the action terminated cleanly (`Succeeded`).
The full geometric invariant (`‖pose(controlled_frame) − S(t)‖ ≤ station_tolerance`
∀ t) is deferred with concrete poses.

What the structural check *does* verify is the defining ENV2 property: it samples the
*whole interval*, not just the endpoint. The adversarial test (§ 5) proves this — a
report whose endpoint is pristine but whose middle sample is corrupt passes the terminal
check and fails the interval check. That contrast is the executable definition of the
class.

## 3. Component 1 — rfl-core (`reach.hover`)

One commit (the `lower` / `check_capability` matches have no catch-all; the new
`Primitive` variant lands with both arms or rfl-core will not compile).

- `ReachHover` struct (v0 subset of `$defs/ReachHoverParams`): `target` (a `Ref` /
  FrameRef — the surface or frame to hover over), `standoff: Quantity` (the maintained
  distance), `duration: Option<serde_yaml::Value>` (`Duration | until`, default `until`).
  The remaining § 1.5 parameters (`station_tolerance`, `approach_axis`, `track_target`,
  `settling_time`, `max_velocity`, `clearance`, `contact_response`) carry their spec
  defaults and are not emitted in v0.
- `Primitive::ReachHover` + `#[serde(rename = "reach.hover")]`.
- `lower_reach_hover`: `target_frame = e.control_frame()`; `target_pose =
  PoseExpr::FrameRelative { frame: <target>, offset: { along: "outward_normal", distance:
  <standoff> } }` (the symbolic standoff setpoint `S = p + standoff·n`, v0); `timing` =
  `nominal_duration` from the `duration` when it is a `Duration` quantity (`until` →
  `None`), `timing_mode: Strict`, `stop_at_goal: true` (hover reaches the setpoint and
  holds it). `safety_envelope = base_envelope(e)`. No monitors, no force budget.
- `check_capability`: the baseline `reach.*` arm (unkeyed — like `reach.align` /
  `reach.retract` / `reach.scan`; `reach.*` is excluded from the descriptor capability
  enum, validate.py C1).
- `lower` dispatch suffix: `"hover"`.

## 4. Component 2 — rfl-conformance (the interval-invariant class + multi-sample driver)

One commit.

- **`EnvelopeClass::IntervalInvariant`** (new variant) + `envelope_class_for("hover") =>
  Some(EnvelopeClass::IntervalInvariant)` + the `check_envelope` `IntervalInvariant` arm
  (exhaustive match — the variant and arm land together). The arm:
  `status.outcome == Succeeded`, and **every** `telemetry[i].realized_pose` is present;
  fail with the first missing sample index otherwise.
- **Multi-sample `ReferenceDriver`**: emit **N = 3** telemetry samples for an
  interval-invariant action, **1** otherwise — decided by
  `envelope_class_for(suffix_of(action_id)) == Some(IntervalInvariant)`. The `self.step`
  counter increments once per emitted sample (so each carries a distinct integer `t`).
  Single-sample actions stay byte-identical to today (one `step += 1`, one sample), so
  the existing `driver_protocol` goldens are unchanged — there is no hover in the cable /
  screw / unscrew examples (verified by re-running `driver_protocol`).
- **`Fault::MidIntervalDrop`**: set the *middle* telemetry sample's `realized_pose` to
  `None` (`telemetry[len/2]`), leaving `status.final_pose` intact — a schema-valid report
  whose interior violates the interval invariant while its endpoint conforms.

## 5. Component 3 — worked example + the ENV2 demonstration

One commit.

- `examples/02-surface-scan/skill-hover.yaml`: `reach.hover` over the `panel` frame at
  `standoff: 50 mm` for `duration: 5 s`, then `sense.inspect`. Reuses the three existing
  embodiment descriptors unchanged (`reach.*` is baseline; `sense.inspect` is already
  declared). `crates/rfl-conformance/tests/surface_scan_hover.rs`: three per-hand insta
  goldens (the Class-2 retarget output — the hover canonical action + inspect), plus
  generate-twice and boon execute-schema validation. Class-1 check of `skill-hover.yaml`
  via the one-off `uv run --with jsonschema` script.
- `envelope_conformance` tests driving the hover (action index 0):
  - *nominal*: the hover's `IntervalInvariant` check passes, and the report carries
    `telemetry.len() == 3` (the interval is actually sampled — non-vacuous).
  - *ENV2 demonstration*: `FaultyDriver(MidIntervalDrop)` on the hover **FAILS** the
    `IntervalInvariant` check **but PASSES** the `TerminalPostcondition` check on the
    same `(goal, report)` — the endpoint conforms; the mid-interval drop is caught only
    by interval sampling. This single test is the executable definition of ENV2.

## 6. Conformance and goldens

`validate.py` (C1–C7) stays green (no schema change; `reach.hover` is already in
`PrimitiveId`, and `reach.*` carries no descriptor capability). New
`surface_scan_hover` goldens; the `driver_protocol` goldens are unchanged (single-sample
emission is byte-identical; re-run to confirm); cable / screw / unscrew / surface_scan /
surface_scan_spiral are untouched.

## 7. Out of scope (deferred)

- **ENV3 disturbance injection** — `reach.hover`'s C2 (calibrated impulse → recover
  within `settling_time`, or abort within `stop_time`); needs a `disturbance_budget` and
  the injection bench.
- **`transport.carry`** — the held interval-invariant primitive (grasp-continuity *and*
  interval-invariant at once); a second interval-invariant primitive, deferred.
- **The concrete geometric station invariant** (`‖pose − S(t)‖ ≤ station_tolerance`) —
  needs spec/02 concrete poses; the v0 check is structural.
- `track_target` / `settling_time` / `station_tolerance` / `contact_response` richness in
  the lowering — carried as spec defaults, not emitted.

## 8. Sections to transcribe during implementation

- spec/01 § 1.5 `reach.hover` (the parameter table + the interval postcondition + the
  C1 interval-sampling sketch) — already read.
- spec/05 § The envelope-class taxonomy + ENV2 (interval coverage) — already read.
- `$defs/ReachHoverParams` (`schemas/skill-isa.schema.json`) — already read.
- `ReachAlign` / `lower_reach_align` (the baseline `reach.*` template), the `lower` /
  `check_capability` dispatch, `PoseExpr::FrameRelative`, `TimingHints`
  (`crates/rfl-core/src/{skill_isa,translation,canonical}.rs`).
- `EnvelopeClass` / `envelope_class_for` / `check_envelope` / the `ReferenceDriver`
  telemetry emit / the `Fault` enum + `FaultyDriver` / the `envelope_conformance.rs`
  test shape (`crates/rfl-conformance/src/lib.rs`, `tests/envelope_conformance.rs`), and
  `tests/surface_scan.rs` (the retarget-golden shape).
