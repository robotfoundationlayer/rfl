# Design: driver-protocol round-trip (Test Class 3, part 1 — C1)

Status: approved design, pre-implementation (2026-05-31)

This is the fourth reference-implementation increment, and the first half of
conformance **Test Class 3** (`spec/05` § Four test classes: "a driver
implementation accepts the canonical actions, executes them within stated
tolerances, and reports back via the protocol"). C1 closes the **protocol
round-trip**: it makes the retarget output *consumable* by defining the driver-side
report types and an in-process reference driver, and proves `execute → telemetry →
status` is well-formed, schema-valid, correlated, and deterministic. The
verification *content* — the four envelope-class checkers and the adversarial
drivers that prove they reject violations — is the second half (C2). The
specification under `spec/` is authoritative; this document describes how the
reference implementation realizes it.

## 1. Why this increment

Increments 1–3 produced the `execute` stream (structural retarget, generative Σ,
mass-dependent grasp-force bounds) but nothing has ever *consumed* it. `spec/05`
Class 3 is the consumer side: a driver accepts canonical actions and reports back
via `telemetry` (Feedback) and `status` (Result). The conformance crate already
anticipates an **in-process** path: its doc names "the local in-process trait, for
unit testing", so C1 needs no ROS 2 binary.

Closing the round-trip is the milestone: it demonstrates the Driver Interface
contract is constructible *from both sides* (the retarget output is consumable, and
a driver can report in-protocol), and it lays the `Driver` trait + report types that
C2's envelope checkers verify against. It is also where increment 3 pays off later —
the emitted `min_holding_force` / `a_max` / axial budget are exactly what C2's
grasp-continuity / interval / force-trajectory checkers will sample — but C1 only
carries them through, it does not yet check them.

## 2. Scope

In scope (C1):

- The driver-side protocol report types in `rfl-core::driver` (`driver.rs` is
  currently a TODO stub), typed from `driver-interface.schema.json`'s
  `TelemetryFeedback` / `StatusResult` variants and the reference instances.
- An in-process `Driver` trait and a **nominal-echo** `ReferenceDriver` that, given
  an `execute` message, emits one `telemetry` + one terminal `status`.
- A Class 3 **round-trip** conformance test: schema validation (boon),
  `action_id` correlation, terminal-status outcome, fidelity-tier echo, generate-
  twice determinism, and a per-embodiment golden of the report stream.

Deferred to C2 (consistent with YAGNI — C1 is the protocol, C2 is the verification):

- The four **envelope-class checkers** (terminal-postcondition / interval-invariant
  / grasp-continuity / force-torque-trajectory, `spec/05` § The envelope-class
  taxonomy) that judge a report against the primitive's envelope.
- The **adversarial misbehaving drivers** (securing_force below `min_holding_force`,
  wrench over budget, accel over `a_max`) that prove each checker *rejects*
  violations rather than self-passing. This is what makes Class 3 non-circular.
- The per-skill ε-tolerance table (`spec/05` § Open issues, data-dependent) the
  interval checkers will need; C1 needs none of it.
- ROS 2 transport bindings, Class 4 (end-to-end on hardware / conformant simulator).

## 3. The protocol report types (`rfl-core::driver`)

Typed from the schema variants (required fields minimal; the rich fields are
optional and validate against open *floors* — `Pose6DFloor` / `WrenchFloor` /
`VerdictFloor` — the same representation-deferral as the `execute` side, so concrete
content passes without the spec committing to a pose representation):

```
RealizedPose { position: [f64;3], orientation: [f64;4] }   // round6 on build (RD1c)
Wrench       { force: [f64;3], torque: [f64;3] }
TactileReading { feature: String, reading: String }
Verdict      { value: bool, confidence: f64, evidence: Vec<String> }   // v0 two-valued
Outcome      enum { Succeeded, Failed, Indeterminate }   // serde rename_all snake_case

Telemetry {                                  // schema TelemetryFeedback; required: message, action_id, t
  message: &'static "telemetry", action_id: String, t: f64,
  realized_pose: Option<RealizedPose>, wrench: Option<Wrench>,
  securing_force: Option<Quantity>, tactile: Vec<TactileReading>,
  events: Vec<String>, fidelity_tier: Option<String>,
}
Status {                                     // schema StatusResult; required: message, action_id, outcome
  message: &'static "status", action_id: String, outcome: Outcome,
  verdict: Option<Verdict>, fidelity_tier: Option<String>, final_pose: Option<RealizedPose>,
  failure_class: Option<String>, failure_detail: Option<String>,
}
DriverReport { telemetry: Vec<Telemetry>, status: Status }
```

Field order is fixed and optionals use `skip_serializing_if` so JSON is
byte-deterministic (RD1c). `serde(rename_all = "snake_case")` on `Outcome`.

## 4. The `Driver` trait and the nominal reference driver

```
pub trait Driver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport;
}
```

`ExecuteGoal` (`rfl-core::canonical`) is the message a driver receives. The trait
lives in `rfl-core::driver` (the protocol contract); the **`ReferenceDriver`** that
implements it lives in `rfl-conformance` (a test/reference artifact, the "in-process
trait for unit testing").

The reference driver is **nominal-echo** — it does not simulate physics; it reports
the in-protocol result a conformant driver would on a nominal execution,
deterministically, from the `execute` message:

- `t` = a deterministic per-action value (the action's 1-based index as `f64`);
- `fidelity_tier` echoed from the `execute` message's `tactile_target`
  (`"auto" → "manifold"`, a `{proxy:…}` object → `"proxy"`, else absent) — so
  allegro / leap report `manifold` and pneumatic reports `proxy`;
- `realized_pose` / `final_pose` = a deterministic **placeholder** concrete pose
  (`{position:[0,0,0], orientation:[0,0,0,1]}`). v0 retarget poses are symbolic (the
  concrete `Pose6D` representation is spec/02's), so the reference driver cannot
  echo a real target; the placeholder is a documented v0 reference choice, pinned by
  golden, and concrete tracking arrives with the pose representation;
- where the `execute` message carries a `force_budget`, echo it as plausible
  in-budget content (`securing_force` for the grip, a small axial `wrench`) — this
  carries increment 3's bounds through so C2's checkers have something to sample,
  but C1 does **not** check them;
- `outcome = Succeeded`; `verdict = { value: true, confidence: 1.0, evidence:
  ["nominal reference-driver execution"] }`.

One `telemetry` sample + one `status` per action (the reference instance shows a
single mid-action sample; interval multi-sampling is a C2 envelope concern).

## 5. The Class 3 round-trip test (`rfl-conformance/tests/driver_protocol.rs`)

For each embodiment (allegro / leap / pneumatic-6f): retarget the cable skill, then
drive every `ExecuteGoal` through the `ReferenceDriver`, collecting the reports.
Assertions:

- **(a) schema validity (boon):** every `telemetry` and `status` JSON validates
  against `driver-interface.schema.json` (the `TelemetryFeedback` / `StatusResult`
  variants), reusing the boon compiler setup from `retarget_determinism.rs`.
- **(b) action_id correlation:** for each action, `telemetry.action_id ==
  status.action_id ==` the originating `execute` goal's `action_id`.
- **(c) terminal status:** every action's `status.outcome == Succeeded` (the nominal
  driver never fails).
- **(d) fidelity-tier echo:** allegro / leap report `manifold`; pneumatic reports
  `proxy` — confirming the driver reads the `execute` message's degraded
  `tactile_target`.
- **(e) determinism:** the report stream is byte-identical across two runs.
- **golden:** an insta snapshot of the serialized report stream per embodiment
  (three snapshots), pinning the reference driver — matching the Class 2 golden
  discipline.

## 6. Determinism, module placement, and what does not change

- **Determinism (RD1c):** `t` is a fixed function of action index; poses are
  `round6`'d placeholders; verdict/confidence are fixed; field order is fixed and
  optionals skip when empty. No wall-clock, no RNG.
- **Placement:** `rfl-core::driver` = protocol types + `Driver` trait + `DriverReport`
  (reusable contract); `rfl-conformance` = `ReferenceDriver` + the round-trip test
  (the harness). This keeps `rfl-core` the contract and `rfl-conformance` the
  reference/test artifacts.
- **No `spec/` or `schemas/` edits** — the types are transcribed from the existing
  `driver-interface.schema.json`. `schemas/validate.py` (C1–C7) and the existing
  46 rfl-core + 5+5 conformance tests stay green.

## 7. Sections to transcribe during implementation

- `schemas/driver-interface.schema.json` `$defs/TelemetryFeedback`,
  `$defs/StatusResult`, `Pose6DFloor` / `WrenchFloor` / `VerdictFloor`, `FidelityTier`,
  `Identifier`, `Force` (the exact required / optional field sets and enums; already
  read for this design — telemetry required `[message, action_id, t]`, status
  required `[message, action_id, outcome]`, outcome enum `{succeeded, failed,
  indeterminate}`).
- `examples/01-cable-insertion/driver-messages/telemetry.yaml` and `status.yaml`
  (the reference instances pinning the concrete content shape).
- `spec/03` § Canonical driver messages (the telemetry / status semantics) and
  `spec/05` § Four test classes (the Class 3 definition).
