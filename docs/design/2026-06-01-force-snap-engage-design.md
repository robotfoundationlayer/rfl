# Design: force.snap_engage (semi-reversible REV2) — engagement-confirmation

Status: approved design, pre-implementation (2026-06-01)

This is the eighteenth reference-implementation increment. It implements `force.snap_engage`
(`spec/01` § 6.10), opening the **reversibility classification** (`spec/05` REV1-REV3), which
nothing exercises yet, at the **semi-reversible (REV2)** rung. Most of the increment is reuse:
`snap_engage`'s snap-in detection is *explicitly* "per `force.press_button`'s detent" (its
`snap_signature` is an `ActuationSpec`), so it consumes the existing `check_actuation` + detent
echo + `ForceTrajectory` machinery. The genuinely-new code is the `confirm_held`
engagement-confirmation leg. The spec is authoritative; this realizes it.

## 1. The gap, and the reuse as gap-closure

`spec/05` REV2: a semi-reversible-persistent change (`force.snap_engage`) "is confirmed via
engagement-confirmation (`confirm_held`) and carries a documented reverse-operation path
(`snap_disengage`); the persistence is intended, distinct from a continuity break." Nothing
exercises the reversibility spectrum today. `snap_engage` is the contained entry (the
irreversible REV3 `force.cut` is the heavy one).

Two reuses are themselves a kind of gap-closure — they test whether machinery built for one
primitive *generalizes*:
- **`check_actuation` (built for press_button)** verifies `snap_engage`'s snap-in (a detent).
  `snap_engage` is its first *independent* consumer — the test that the detent/event-gating
  abstraction was general, not press_button-shaped.
- **`verdict.evidence` (the AUD1 audit channel)** carries the held-confirmation. It is
  declared-but-unread today (like `events` was before press_button); `check_engagement` is its
  first reader.

## 2. No schema change

`force.snap_engage` is in the skill-isa PrimitiveId enum and the embodiment capability enum;
`ForceSnapEngageParams` is fully typed (`mate_feature` + `engage_direction` + `force_budget`
required; `snap_signature` `ActuationSpec` default `detent`; `confirm_held` bool default
`true`). The `confirm_held` marker rides in `force_profile` (an open object); the
held-confirmation rides in `verdict.evidence` (`Vec<String>`). Both reused → **pure rfl-core +
rfl-conformance + an example**.

## 3. The contract (v0)

- **`envelope_class_for("snap_engage") => ForceTrajectory`.** Engagement force ≤ `force_budget`
  (the existing arm, reused; `Fault::OverForce` already bites — the § 6.10 `over_force` mode).
- **Snap-in (reuse `check_actuation`).** The lowering emits `force_profile.actuation = "detent"`
  when `snap_signature` is detent (the default). `check_actuation`: `Succeeded ⟹ a detent
  ForceEvent present` (the § 6.10 `no_snap` distinction — a force rise without the drop is not
  a snap).
- **Engagement-confirmation (the new leg).** The lowering emits `force_profile.confirm_held =
  true` (the default). A new `check_engagement(goal, report)` is vacuous unless `confirm_held`
  is set; else `Succeeded ⟹ verdict.evidence contains "held_confirmed"`. A snap that *claims
  success without confirming the connection holds* (the § 6.10 `false_engagement` mode
  masquerading as success) fails.

## 4. Surface

**rfl-core (one commit — exhaustive-match):**
- `struct ForceSnapEngage` (`src/skill_isa.rs`): `mate_feature` (`serde_yaml::Value`, carried
  symbolic), `engage_direction` (`Direction`), `force_budget` (`Quantity`), `snap_signature`
  (`Option<serde_yaml::Value>`), `confirm_held` (`Option<bool>`), `compliance`
  (`Option<Compliance>`).
- `Primitive::ForceSnapEngage(ForceSnapEngage)` + `check_capability` arm `"force.snap_engage"`
  + the lower dispatch arm `(lower_force_snap_engage(p, e), "snap_engage")` — all this commit.
- `lower_force_snap_engage` (`src/translation.rs`): `force_budget: Some(p.force_budget)`;
  `force_profile` = `{"actuation":"detent"}` when `snap_signature` is detent or absent (default)
  `+ "confirm_held": true` when `confirm_held != Some(false)` (default true); `snap_signature`
  (or default `detent`) lowered into a `Monitor`; `target_pose: PoseExpr::AxisRelative {
  direction: engage_direction, distance: "0 mm" }` (mirroring `lower_force_screw`'s thread_axis);
  compliance mapped.

**rfl-conformance (`src/lib.rs`):**
- `envelope_class_for("snap_engage") => ForceTrajectory`.
- Extend the `ReferenceDriver` verdict: push `"held_confirmed"` into `verdict.evidence` when
  `ca.safety_envelope.force_profile.confirm_held` is `true`. (The detent echo already fires from
  the `actuation` marker — reused unchanged.)
- `pub fn check_engagement(goal, report) -> CheckOutcome` — vacuous unless `force_profile.confirm_held`
  is true; else `Succeeded ⟹ verdict.evidence contains "held_confirmed"`.
- `SnapEngageDriver { inner: ReferenceDriver, response: SnapEngageResponse }` +
  `SnapEngageResponse { Engages, NoSnap, ClaimsHeld }`: `Engages` = passthrough (detent +
  held_confirmed + Succeeded); `NoSnap` = clear events + `Failed` + `failure_class "blocked"` +
  `failure_detail "no_snap"` (force kept ≤ budget); `ClaimsHeld` = keep Succeeded + detent but
  drop `"held_confirmed"` from `verdict.evidence` (adversarial).

**examples + goldens:** `examples/03-screw-fasten/skill-snap.yaml` (a `force.snap_engage` with
`mate_feature`, `engage_direction`, `force_budget`, `confirm_held: true`, `compliance`) +
`force.snap_engage` on the 3 descriptors; new `snap_engage.rs` golden (3 stems).

**tests:** translation (`snap_engage_lowers_actuation_confirm_held_and_force_budget`,
`snap_engage_capability_absent_when_not_declared`); golden (3 stems); envelope_conformance
(`nominal_snap_engage_passes_all_legs`: ForceTrajectory + check_actuation + check_engagement Pass;
`snap_engage_no_snap_is_honest`: NoSnap → not-Succeeded, ForceTrajectory Pass; `snap_engage_unconfirmed_hold_fails`:
ClaimsHeld → check_engagement Fail; `snap_engage_over_force_fails`: `Fault::OverForce` → ForceTrajectory Fail)
+ a lib unit for `check_engagement`.

## 5. Non-vacuity

`ClaimsHeld` (Succeeded + detent, no `"held_confirmed"` evidence) → `check_engagement` Fail —
proves the confirmation leg is load-bearing (previously a snap claiming success without the
hold-test passed everything). `Fault::OverForce` (reused) → `ForceTrajectory` Fail. The reused
`check_actuation` already bites a no-detent success. Conformant `Engages` / `NoSnap` give the
contrast on telemetry that differs only by the confirmation evidence + outcome.

## 6. Commit shape (executing-plans, inline)

Three commits, each red→green; full `cargo test` + `validate.py` read in a batch *separate* from
the commit; each ff-pushed with a `git show --stat` self-check:
1. **rfl-core primitive** — struct + variant + gate + `lower_force_snap_engage` + `skill-snap.yaml`
   + 3 descriptors + `snap_engage.rs` golden + translation tests.
2. **conformance** — `envelope_class_for` + verdict `held_confirmed` echo + `check_engagement` +
   `SnapEngageDriver` + the integration tests + the lib unit.
3. **README** status line (spec/05 REV1/REV2 already state the obligation; no spec change).

## 7. Faithfulness, determinism, and what stays deferred

- **Faithful:** the held-confirmation is § 6.10's `confirm_held` release-test, recorded as AUD1
  `verdict.evidence`. The snap-in detent is § 6.10's snap-in signature, the same `ForceEvent` as
  press_button. `force_budget` drives the existing ForceTrajectory leg.
- **Deterministic:** the bench mutates a deterministic nominal report by fixed rules; new
  `snap_engage` goldens are added (not mutations); existing goldens byte-identical (no other
  action emits a `confirm_held` marker).
- **Deferred behind named prerequisites:** `effort_rise(force_threshold)` snap_signature; the
  **documented reverse-operation path** (`snap_disengage`) — REV2's reverse-path declaration,
  deferred with the `snap_disengage` primitive; `grasp_handle` held-part reaction; `mate_feature`
  geometry; a conformant honest-`false_engagement` driver response (the `ClaimsHeld` adversary
  covers the bite). The standing deferred list (force.cut REV3, force.scrub, in_hand.flip AUD1,
  hover over-envelope max_excursion, Σ arc/path/volume, full held-interval GC1, GC2-6, per-skill
  ε-table, ROS 2, Class 4, {trajectory} MoveSpec, the various `auto` derivations, physics
  injection) stays parked.
