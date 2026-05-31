# Design: force.press_button (detent) — activating the inert ForceEvent conformance channel

Status: approved design, pre-implementation (2026-06-01)

This is the sixteenth reference-implementation increment. It implements `force.press_button`
(`spec/01` § 6.6) in the detent actuation mode. Its leverage is not category-6 breadth: it
turns on a conformance dimension that exists structurally but has never been exercised — the
`events` ForceEvent telemetry channel. `force.screw` / `force.unscrew` use detents for
*completion*, the schema defines `ForceEventFloor`, and `Telemetry` carries an `events` field,
yet **no `check_envelope` arm reads `t.events` and `ReferenceDriver` always emits
`events: vec![]`** — so event detection is unfalsifiable. `press_button`'s defining semantic is
event-gated success (a detent marks actuation; a force rise without one is bottoming-out, not
success), which makes it the natural primitive to activate the channel. The spec is
authoritative; this realizes it.

## 1. The gap (and a latent drift it surfaces)

`press_button` succeeds only when an actuation event is detected (§ 6.6 postcondition: "the
actuation event was detected"); a force rise that reaches the budget *without* a detent is
`no_actuation` (a stuck / absent button), and continuing to press would be over-travel that
damages the mechanism. Verifying this requires reading the ForceEvent channel — which today is:

- **conformance-inert:** no checker reads `t.events`; the nominal driver never emits one;
- **latently wrong:** `rfl-core`'s `Telemetry.events` is `Vec<String>`, but the schema's
  `ForceEventFloor` (`driver-interface.schema.json`) is an **object** (`kind / at / magnitude`,
  "owned by `04`, floored"). A string event would fail schema validation. They have never
  disagreed only because `events` is always empty (skipped by `skip_serializing_if`). The
  anti-drift `validate.py` checks cross-*schema* consistency, not Rust-struct-vs-schema, so it
  never caught this.

This increment turns the channel on (event-gated `check_actuation`) and, to emit a schema-valid
detent, fixes the drift (`events: Vec<String>` → `Vec<serde_json::Value>`).

## 2. No schema fight (the contract is already specced)

Like `reach.hover`: `force.press_button` is in the skill-isa PrimitiveId enum,
`ForcePressButtonParams` is fully typed (`required: target, actuation, force_budget`),
`ActuationSpec` = `detent | effort_rise(force_threshold)` is defined, and `force.press_button`
is in the embodiment-descriptor capability enum. So **no skill-isa / capability JSON-schema
change**. The `events` type fix is a `rfl-core` struct change that makes the implementation
*match* the existing `ForceEventFloor` object floor — also not a JSON-schema change.

## 3. The contract (v0, detent only)

`effort_rise(force_threshold)` actuation is deferred (§ 7); v0 implements the **detent** mode,
the event-gated one that activates the channel.

- **Force-trajectory leg (reuse).** Press force ≤ `force_budget` at every sample. The lowering
  emits `force_budget` on the `CanonicalAction`; the existing `check_envelope(ForceTrajectory)`
  bounds `wrench.force` against it, and the existing `Fault::OverForce` already bites a crush.
- **Actuation leg (new).** The lowering emits `force_profile.actuation = "detent"`. A new
  `check_actuation(goal, report)` is **vacuous unless the action carries an `actuation`
  contract**; when present, `outcome == Succeeded ⟹ at least one telemetry ForceEvent with
  `kind == "detent"`. A press that *claims success without a detent* fails — the bite that
  makes `events` falsifiable.
- **Bottoming-out (the § 6.6 C2 conformant case).** `outcome == Failed` +
  `failure_detail == "no_actuation"` + no detent + force ≤ budget (it did not crush seeking a
  non-existent detent) → `check_actuation` passes vacuously (not Succeeded) and
  `check_envelope(ForceTrajectory)` passes. The pair {bottoming-out-honest vs ClaimsActuation},
  on identical telemetry, is the executable definition of event-gating.

## 4. Surface

**rfl-core — commit A (the drift fix, lands first):**
- `Telemetry.events: Vec<String>` → `Vec<serde_json::Value>` (`src/driver.rs`). Carries
  schema-valid `{"kind":"detent"}`. The four `events: vec![]` literals are unaffected (an empty
  vec infers either element type). Not a JSON-schema change.

**rfl-core — commit B (the primitive; exhaustive-match → one commit):**
- `struct ForcePressButton` (`src/skill_isa.rs`) mirroring `ForceScrew`: `target` (FrameRef),
  `force_budget` (Quantity), `actuation` (`serde_yaml::Value`, like `screw.completion`), and
  carried `press_direction` / `max_travel` / `release_after` / `compliance` / `grasp_handle`
  (`#[serde(default)]`, symbolic in v0).
- `Primitive::ForcePressButton(ForcePressButton)` + the `check_capability` arm
  `Primitive::ForcePressButton(_) => "force.press_button"` + the lower dispatch arm
  `Primitive::ForcePressButton(p) => (lower_force_press_button(p, e, ctx), "press_button")` —
  all in this commit (the compiler forces the match arms together).
- `lower_force_press_button` (`src/translation.rs`): `force_budget: Some(p.force_budget)`;
  `env.force_profile = {"actuation":"detent"}` **only** when the actuation is a detent
  (effort_rise carried but unmarked → deferred); the actuation lowered into a `Monitor`
  (faithful, like `screw.completion`); `target_pose: PoseExpr::Ref { ref: target }`
  (press_direction symbolic, geometry deferred); `compliance` mapped like screw/insert_fit.

**rfl-conformance — commit C:**
- `envelope_class_for("press_button") => ForceTrajectory`.
- `ReferenceDriver::execute`: when `ca.safety_envelope.force_profile.actuation` is present,
  emit `events: vec![json!({"kind":"detent"})]` on the sample and keep `Succeeded` (the nominal
  actuation — the detent echo, mirroring the `securing_force` / `station_error` echoes).
  `press_button` is `ForceTrajectory` → `n_samples == 1`, so the detent lands on the one sample.
- `pub fn check_actuation(goal, report) -> CheckOutcome` — vacuous without an `actuation`
  contract; else `Succeeded ⟹ a kind=="detent" ForceEvent present`.
- `PressButtonDriver { inner: ReferenceDriver, response: PressButtonResponse }` +
  `PressButtonResponse { Actuates, Bottoms, ClaimsActuation }`: `Actuates` = passthrough
  (ReferenceDriver detent + Succeeded); `Bottoms` = clear events + `Failed` + `failure_class
  "blocked"` + `failure_detail "no_actuation"` (force kept ≤ budget); `ClaimsActuation` = clear
  events, keep `Succeeded` (adversarial).

**examples + goldens:**
- `examples/03-screw-fasten/skill-press.yaml` (a `force.press_button` with `target`,
  `actuation: detent`, `force_budget`).
- `force.press_button` added to the `force` capability list of the three 03 embodiment
  descriptors (allegro / leap / pneumatic-6f).
- New `crates/rfl-conformance/tests/press_button.rs` golden (3 stems, like
  `surface_scan_hover.rs`) + its snapshots.

**tests:**
- translation unit: `press_button_lowers_force_budget_and_actuation`,
  `press_button_capability_absent_when_not_declared` (mirrors `screw_capability_absent`).
- golden: 3 stems + byte-identical + every-line-schema-valid (the detent is NOT in the execute
  golden — it is driver telemetry; the golden is the retarget output, so the actuation marker
  appears in `force_profile`).
- envelope_conformance: `nominal_press_button_passes` (ForceTrajectory Pass + check_actuation
  Pass), `press_button_bottoming_out_is_honest` (Bottoms → check_actuation Pass + ForceTrajectory
  Pass + not Succeeded), `press_button_false_actuation_fails` (ClaimsActuation → check_actuation
  Fail), `press_button_over_force_fails` (`Fault::OverForce` → ForceTrajectory Fail).
- lib unit: `check_actuation` on hand-built reports (Succeeded+detent Pass; Succeeded-no-detent
  Fail; Failed Pass vacuous).

## 5. Non-vacuity

`ClaimsActuation` (Succeeded, no detent) → `check_actuation` FAIL — proves `events` is now
load-bearing (previously a Succeeded-no-event report passed everything). `Fault::OverForce` →
`ForceTrajectory` FAIL — reuses the existing fault. The conformant `Actuates` / `Bottoms`
responses, on telemetry that differs only by outcome claim, give the event-gating contrast.

## 6. Commit shape (executing-plans, inline)

Four commits, each red→green; full `cargo test` + `validate.py` read in a batch *separate* from
the commit; each ff-pushed with a `git show --stat` self-check:
1. **events drift fix** — `Telemetry.events` → `Vec<Value>` + a `driver_protocol` test that a
   detent ForceEvent object validates (red: `Vec<String>` can't hold the object → compile error).
2. **rfl-core primitive** — struct + variant + lower + gate + `skill-press.yaml` + 3 descriptors
   + `press_button.rs` golden + translation tests.
3. **conformance** — `envelope_class_for` + ReferenceDriver detent echo + `check_actuation` +
   `PressButtonDriver`/`PressButtonResponse` + the four integration tests + the lib unit test.
4. **README** status line (spec/05 already states force-event detections are evaluated on the
   sampled trajectory, so no spec change).

## 7. Faithfulness, determinism, and what stays deferred

- **Faithful:** the abort report reuses the increment-4 ownership split (`failure_class
  "blocked"` + `failure_detail "no_actuation"`, the § 6.6 token). The detent is a `ForceEventFloor`
  object, the schema's own representation. `force_budget` on the action drives the existing
  ForceTrajectory linear-force leg exactly as `force.insert_fit` does.
- **Deterministic:** the bench mutates a deterministic nominal report by fixed rules; the detent
  object is a constant. New `press_button` goldens are added (not mutations of existing ones);
  existing goldens are byte-identical (no action emits an actuation marker but press_button).
- **Deferred behind named prerequisites:** `effort_rise(force_threshold)` actuation (a force
  crossing, no event); the `max_travel` / `over_travel` guard (needs a displacement telemetry
  signal, the press analogue of `station_error`); `release_after` hold-vs-release semantics;
  inline `SurfaceTarget` geometry (v0 lowers a symbolic target ref). The standing deferred list
  (force.cut/wipe/scrub/snap_engage, hover over-envelope bounded-excursion, Σ arc/path/volume,
  full held-interval GC1, GC2-6, per-skill ε-table, ROS 2, Class 4, {trajectory} MoveSpec,
  disturbance_budget:auto / settling_time:auto, physics injection) stays parked.
