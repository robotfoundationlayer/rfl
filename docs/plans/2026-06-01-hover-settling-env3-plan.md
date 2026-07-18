# reach.hover settling ENV3 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement
> this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> **LOCAL-ONLY** — never `git add` this file.

**Goal:** Make `reach.hover`'s station-keeping falsifiable by implementing its `spec/01` § 1.5
C2 disturbance-rejection contract (recover within `settling_time` to `station_tolerance`, or
abort to a safe state) in the reference implementation and conformance bench.

**Architecture:** Lower the *existing* `reach.hover` settling params (`station_tolerance`,
`settling_time`) into a new `Envelope.station_keeping` object; introduce a `station_error`
telemetry scalar (the one additive schema change); strengthen the `IntervalInvariant`
`check_envelope` arm with a vacuous-when-absent settled-tail leg; add `check_settling` +
`HoverSettlingDriver`. No `spec/01` / skill-isa schema change (the contract is already specced).

**Tech Stack:** Rust (rfl-core / rfl-conformance), insta golden snapshots, boon (JSON-schema),
`schemas/validate.py` (C1–C7). Design: `docs/design/2026-06-01-hover-settling-env3-design.md`.

**Discipline (every task):** TDD red→green. After green, run `cargo test` (workspace, NOT `-p`)
+ `uv run --with jsonschema --with pyyaml python schemas/validate.py` in a batch SEPARATE from
the commit. Before each push: `git fetch -q && git merge-base --is-ancestor origin/main HEAD`,
`git add <explicit paths>` (never `-A`/`.`), commit, `git push origin main`, then verify
`git rev-list --left-right --count HEAD...origin/main` is `0 0` and `git show --stat HEAD` lists
only the intended files. Stop-gate: if a golden differs from the prediction below, reconcile and
explain every changed line before accepting.

---

### Task 1: `station_error` telemetry scalar (schema + struct)

**Files:**
- Modify: `schemas/driver-interface.schema.json` (TelemetryFeedback.properties, ~line 107)
- Modify: `crates/rfl-core/src/driver.rs` (Telemetry struct ~line 97; test literal ~line 158)
- Test: `crates/rfl-conformance/tests/driver_protocol.rs` (new test)

- [ ] **Step 1: Write the failing test** — append to `driver_protocol.rs`:

```rust
#[test]
fn telemetry_with_station_error_is_schema_valid() {
    use rfl_core::driver::{RealizedPose, Telemetry};
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/driver-interface.schema.json");
    let schema: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&schema_path).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource("driver-interface.schema.json", schema).unwrap();
    let idx = compiler.compile("driver-interface.schema.json", &mut schemas).unwrap();

    let t = Telemetry {
        message: "telemetry",
        action_id: "s/e/0001-hover".to_string(),
        t: 1.0,
        realized_pose: Some(RealizedPose::placeholder()),
        wrench: None,
        securing_force: None,
        station_error: Some(rfl_core::quantity::Quantity("1 mm".to_string())),
        tactile: vec![],
        events: vec![],
        fidelity_tier: None,
    };
    let v = serde_json::to_value(&t).unwrap();
    schemas.validate(&v, idx).expect("telemetry with station_error must be schema-valid");
}
```

- [ ] **Step 2: Run to verify it fails (compile error — no field yet)**

Run: `cargo test -p rfl-conformance --test driver_protocol telemetry_with_station_error 2>&1 | tail -5`
Expected: compile error `no field 'station_error' on type Telemetry`.

- [ ] **Step 3: Add the struct field** — in `crates/rfl-core/src/driver.rs`, after the
`securing_force` field (the `pub securing_force: Option<Quantity>,` block ~line 97), insert:

```rust
    /// The positional station error vs. the `reach.hover` setpoint (`spec/01` § 1.5
    /// "externally measured station error"; the interval-invariant settling leg / ENV3
    /// samples it). Present only for a hover carrying a `station_keeping` contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub station_error: Option<Quantity>,
```

- [ ] **Step 4: Fix the in-crate Telemetry literal** — in the same file's test (~line 158,
`fn telemetry_serializes_required_fields_and_skips_empty`), add `station_error: None,` after
the `securing_force: None,` line.

- [ ] **Step 5: Run — compiles, but the schema test now fails at runtime**

Run: `cargo test -p rfl-conformance --test driver_protocol telemetry_with_station_error 2>&1 | tail -8`
Expected: FAIL — boon rejects `station_error` (TelemetryFeedback `additionalProperties: false`).

- [ ] **Step 6: Add the schema property** — in `schemas/driver-interface.schema.json`,
TelemetryFeedback.properties, replace:

```json
        "securing_force": { "$ref": "#/$defs/Force" },
```
with:
```json
        "securing_force": { "$ref": "#/$defs/Force" },
        "station_error": { "$ref": "#/$defs/Length", "description": "Distance from the reach.hover station setpoint (01 § 1.5 'externally measured station error'); sampled by the interval-invariant settling check (05 ENV3)." },
```

- [ ] **Step 7: Run the new test — green**

Run: `cargo test -p rfl-conformance --test driver_protocol telemetry_with_station_error 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 8: Full suite (separate batch — do NOT fold into the commit)**

Run: `cargo test 2>&1 | grep -E "test result|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -3`
Expected: all green; `rfl-core` 71, conformance unchanged + driver_protocol now 8; `PASS` C1–C7.
(Existing goldens unchanged: the cable-insertion driver emits no `station_keeping`, so every
`station_error` is `None` → skipped → byte-identical serialization.)

- [ ] **Step 9: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add schemas/driver-interface.schema.json crates/rfl-core/src/driver.rs crates/rfl-conformance/tests/driver_protocol.rs && \
git commit -m "feat(driver): add station_error telemetry scalar for hover settling

reach.hover has no station-keeping signal, so its interval check can only
assert realized_pose is present, never correct. Add an optional station_error
Length to TelemetryFeedback (additive, backward-compatible) — the protocol
form of spec/01 § 1.5's 'externally measured station error'. C1-C7-neutral.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only the 3 files listed.

---

### Task 2: lower the hover settling contract into `station_keeping`

**Files:**
- Modify: `crates/rfl-core/src/canonical.rs` (Envelope struct ~line 168; test literal ~line 322)
- Modify: `crates/rfl-core/src/skill_isa.rs` (ReachHover struct ~line 426)
- Modify: `crates/rfl-core/src/translation.rs` (lower_reach_hover ~line 434; literals 184, 211)
- Modify: `examples/02-surface-scan/skill-hover.yaml`
- Regenerate: `crates/rfl-conformance/tests/snapshots/surface_scan_hover__*.snap` (3)
- Test: `crates/rfl-core/src/translation.rs` (new unit test)

- [ ] **Step 1: Write the failing test** — in `translation.rs` tests module, add:

```rust
    #[test]
    fn hover_with_settling_time_emits_station_keeping() {
        let yaml = "skill: t\nbody:\n  sequence:\n    - reach.hover: { target: panel, standoff: 50 mm, station_tolerance: 2 mm, settling_time: 1 s }\n";
        let skill = Skill::parse_yaml(yaml).unwrap();
        let emb = test_allegro();
        let out = retarget(&skill, &emb).unwrap();
        let env = &out.actions[0].safety_envelope;
        let sk = env.station_keeping.as_ref().expect("station_keeping emitted");
        assert_eq!(sk.get("station_tolerance").and_then(|v| v.as_str()), Some("2 mm"));
        assert_eq!(sk.get("settling_time").and_then(|v| v.as_str()), Some("1 s"));
    }

    #[test]
    fn bare_hover_emits_no_station_keeping() {
        let yaml = "skill: t\nbody:\n  sequence:\n    - reach.hover: { target: panel, standoff: 50 mm }\n";
        let skill = Skill::parse_yaml(yaml).unwrap();
        let out = retarget(&skill, &test_allegro()).unwrap();
        assert!(out.actions[0].safety_envelope.station_keeping.is_none());
    }
```

Note: reuse the existing test embodiment constructor. Check the existing hover test
(`hover_lowers_to_a_standoff_setpoint`, ~line 1121) for the exact helper name it uses to build
the allegro embodiment, and substitute it for `test_allegro()` above if different.

- [ ] **Step 2: Run to verify it fails (no field / no parse)**

Run: `cargo test -p rfl-core hover_with_settling_time 2>&1 | tail -6`
Expected: compile error `no field 'station_keeping' on type Envelope`.

- [ ] **Step 3: Add the Envelope field** — in `canonical.rs`, in `struct Envelope`, after the
`force_profile` field block (~line 168) insert:

```rust
    /// The station-keeping contract for an interval-invariant station hold
    /// (`reach.hover` settling, `spec/01` § 1.5): `{station_tolerance, settling_time}`.
    /// The disturbance-recovery leg of the interval-invariant check (`05` ENV3) samples it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub station_keeping: Option<serde_json::Value>,
```

- [ ] **Step 4: Fix all four Envelope literals** — add `station_keeping: None,` (after the
`force_profile: ...` line) in each:
  - `crates/rfl-core/src/canonical.rs` ~line 319 (test `sample()`).
  - `crates/rfl-core/src/translation.rs` ~line 185 (the `lower_sense_inspect` literal).
  - `crates/rfl-core/src/translation.rs` ~line 213 (`base_envelope`).
  - `crates/rfl-conformance/src/lib.rs` ~line 678 (test `sample_action()`).

- [ ] **Step 5: Add the ReachHover params** — in `skill_isa.rs`, in `struct ReachHover`
(~line 426), after the `duration` field, add:

```rust
    /// Allowed positional excursion from the hover setpoint (`spec/01` § 1.5, default 2 mm).
    #[serde(default)]
    pub station_tolerance: Option<Quantity>,
    /// Max time to return within `station_tolerance` after a disturbance (`Duration | auto`;
    /// an explicit Duration opts the hover into the ENV3 settling contract).
    #[serde(default)]
    pub settling_time: Option<serde_yaml::Value>,
```

- [ ] **Step 6: Emit `station_keeping` in the lowering** — in `translation.rs`, replace the
body of `fn lower_reach_hover` so it builds a mutable base envelope and conditionally emits the
contract (keep the existing `target_pose` / timing exactly):

```rust
fn lower_reach_hover(p: &ReachHover, e: &Embodiment) -> CanonicalAction {
    let mut env = base_envelope(e);
    // spec/01 § 1.5 C2: an explicit settling_time opts the hover into the ENV3 settling
    // contract; the disturbance-recovery check samples station_keeping. A bare hover
    // (settling_time absent / auto) emits nothing and stays ENV2-only.
    if let Some(settling) = p
        .settling_time
        .as_ref()
        .and_then(serde_yaml::Value::as_str)
        .filter(|s| *s != "auto")
    {
        let tol = p
            .station_tolerance
            .clone()
            .unwrap_or_else(|| crate::quantity::Quantity("2 mm".to_string()));
        env.station_keeping = Some(serde_json::json!({
            "station_tolerance": tol.0,
            "settling_time": settling,
        }));
    }
    CanonicalAction {
        target_frame: e.control_frame().to_string(),
        target_pose: PoseExpr::FrameRelative {
            frame: p.target.clone(),
            offset: serde_json::json!({ "along": "outward_normal", "distance": p.standoff.0.clone() }),
        },
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: env,
    }
}
```

- [ ] **Step 7: Run the new tests — green**

Run: `cargo test -p rfl-core hover_with_settling_time bare_hover_emits 2>&1 | tail -6`
Expected: both PASS. (`hover_lowers_to_a_standoff_setpoint` also still PASS — unchanged.)

- [ ] **Step 8: Add the args to the example skill** — in `examples/02-surface-scan/skill-hover.yaml`,
the `reach.hover` block becomes:

```yaml
    - reach.hover:
        target: panel
        standoff: 50 mm
        station_tolerance: 2 mm
        settling_time: 1 s
        duration: 5 s
```

- [ ] **Step 9: Regenerate the hover goldens**

Run: `INSTA_UPDATE=always cargo test -p rfl-conformance --test surface_scan_hover 2>&1 | tail -5`
Then: `git diff --stat crates/rfl-conformance/tests/snapshots/`
Expected change (STOP-GATE — verify exactly this, all 3 stems): the `0001-hover` execute line
gains `"station_keeping":{"station_tolerance":"2 mm","settling_time":"1 s"}` between
`"motion_bounds":{...}` and `"stop_time":...`. The `0002-inspect` line is UNCHANGED. No other
golden changes. If anything else moved, reconcile before continuing.

- [ ] **Step 10: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -3`
Expected: green; `rfl-core` 73 (was 71, +2 new); hover goldens validate as execute messages.

- [ ] **Step 11: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-core/src/canonical.rs crates/rfl-core/src/skill_isa.rs crates/rfl-core/src/translation.rs crates/rfl-conformance/src/lib.rs examples/02-surface-scan/skill-hover.yaml crates/rfl-conformance/tests/snapshots && \
git commit -m "feat(core): lower reach.hover settling contract into station_keeping

Parse the existing station_tolerance / settling_time params (spec/01 § 1.5)
and emit a station_keeping envelope object when an explicit settling_time
opts the hover into the ENV3 contract. A bare hover is unchanged.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only the listed files (canonical/skill_isa/translation/lib/skill-hover/3 snaps).

---

### Task 3: conformance — settled-tail leg, `check_settling`, `HoverSettlingDriver`

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (ReferenceDriver echo; new helper; IntervalInvariant
  arm; `check_settling`; `HoverSettlingDriver`/`HoverResponse`; one lib unit test)
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (imports + 4 tests)

- [ ] **Step 1: Write the failing integration tests** — append to `envelope_conformance.rs`
(reuse the existing `surface_dir()` and `suffix_of()` helpers in that file):

```rust
// --- ENV3 disturbance injection (reach.hover C2, spec/01 § 1.5) -----------------------------

#[test]
fn hover_recovers_within_settling_passes_interval() {
    let dir = surface_dir();
    let pairs = drive(
        HoverSettlingDriver::new(HoverResponse::Recovers),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "hover");
    assert_eq!(report.telemetry.len(), 3, "interval-sampled under perturbation");
    // sample 0 exceeds tolerance (grace window) but the settled tail recovered.
    assert_eq!(check_envelope(EnvelopeClass::IntervalInvariant, goal, report), CheckOutcome::Pass);
}

#[test]
fn hover_fails_to_recover_fails_interval() {
    let dir = surface_dir();
    let pairs = drive(
        HoverSettlingDriver::new(HoverResponse::FailsToRecover),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // drifts past station_tolerance throughout yet claims success -> the settled-tail leg bites.
    assert!(matches!(
        check_envelope(EnvelopeClass::IntervalInvariant, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn hover_over_envelope_aborts_gracefully() {
    let dir = surface_dir();
    let pairs = drive(
        HoverSettlingDriver::new(HoverResponse::Aborts),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // opposite-verdicts: aborts (Failed + station_exceeded) -> settling Pass, interval Fail.
    assert_eq!(check_settling(report), CheckOutcome::Pass);
    assert!(matches!(
        check_envelope(EnvelopeClass::IntervalInvariant, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn hover_over_envelope_false_success_fails_settling() {
    let dir = surface_dir();
    let pairs = drive(
        HoverSettlingDriver::new(HoverResponse::ClaimsSuccess),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (_, report) = &pairs[0];
    assert!(matches!(check_settling(report), CheckOutcome::Fail(_)));
}
```

Then update the `use rfl_conformance::{...}` import block at the top to add
`check_settling, HoverResponse, HoverSettlingDriver`.

- [ ] **Step 2: Run to verify it fails (unresolved imports)**

Run: `cargo test -p rfl-conformance --test envelope_conformance hover_ 2>&1 | tail -8`
Expected: compile error — `check_settling` / `HoverResponse` / `HoverSettlingDriver` not found.

- [ ] **Step 3: Echo `station_error` from the nominal driver** — in `lib.rs`
`ReferenceDriver::execute`, after the `securing_force` binding (~line 89) add:

```rust
        // Echo a zero station error for an action carrying a station_keeping contract
        // (reach.hover settling, spec/01 § 1.5): the nominal hover holds station perfectly,
        // so the settled-tail leg passes. Absent for every other action.
        let station_error = ca
            .safety_envelope
            .station_keeping
            .as_ref()
            .map(|_| rfl_core::quantity::Quantity("0 mm".to_string()));
```
and in the `Telemetry { ... }` literal in the `(0..n_samples).map(...)` closure, add after the
`securing_force: securing_force.clone(),` line: `station_error: station_error.clone(),`.

- [ ] **Step 4: Fix the two lib test Telemetry literals** — add `station_error: None,` (after
`securing_force: None,`) in both: the `sample` closure (~line 691) and the `report` builder's
`telemetry: vec![Telemetry { ... }]` (~line 745).

- [ ] **Step 5: Add the settled-tail helper** — in `lib.rs`, after `securing_floor_violation`
(~line 408) add:

```rust
/// If the action carries a `station_keeping` contract (`reach.hover` settling, `spec/01`
/// § 1.5) and any settled-tail sample — one stamped at `t >= first_t + settling_time`, after
/// the recovery grace window — is missing `station_error` or exceeds `station_tolerance`, the
/// failure reason; else `None`. Vacuous when `station_keeping` is absent (carry / bare hover
/// unaffected — the held-floor pattern). Requires >= 1 tail sample (non-vacuous).
fn station_keeping_violation(goal: &ExecuteGoal, report: &DriverReport) -> Option<String> {
    let sk = goal.canonical_action.safety_envelope.station_keeping.as_ref()?;
    let tol = sk
        .get("station_tolerance")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v))?;
    let settle = sk
        .get("settling_time")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v))?;
    let deadline = report.telemetry.first()?.t + settle;
    let mut tail_seen = false;
    for t in &report.telemetry {
        if t.t < deadline {
            continue; // recovery grace window — excursion permitted here
        }
        tail_seen = true;
        match t.station_error.as_ref().and_then(quantity_mag) {
            None => return Some(format!("settled-tail sample at t={} missing station_error", t.t)),
            Some(err) if err > tol => {
                return Some(format!("station_error {err} > station_tolerance {tol} at t={}", t.t));
            }
            _ => {}
        }
    }
    if !tail_seen {
        return Some(format!("no settled-tail sample at t >= {deadline}"));
    }
    None
}
```

- [ ] **Step 6: Strengthen the IntervalInvariant arm** — in `check_envelope`, in the
`EnvelopeClass::IntervalInvariant` arm, replace the trailing:

```rust
            if let Some(reason) = securing_floor_violation(goal, report) {
                return CheckOutcome::Fail(reason);
            }
            CheckOutcome::Pass
```
with:
```rust
            if let Some(reason) = securing_floor_violation(goal, report) {
                return CheckOutcome::Fail(reason);
            }
            // Station-keeping recovery (reach.hover settling, spec/01 § 1.5 line 668): the
            // settled tail must be within station_tolerance. Vacuous for transport.carry.
            if let Some(reason) = station_keeping_violation(goal, report) {
                return CheckOutcome::Fail(reason);
            }
            CheckOutcome::Pass
```

- [ ] **Step 7: Add `check_settling`** — in `lib.rs`, after `check_graceful_degradation`
(~line 519) add:

```rust
/// Verify the § 1.5 C2 over-envelope contract for `reach.hover`: the hover must NOT claim
/// success and must report the `station_exceeded` halt reason (abort to a safe state). There
/// is no held object to secure, so failure-shape correctness IS the contract. Distinct from
/// `check_envelope`: this judges the failure shape of an over-envelope impulse (`05` ENV3),
/// not correct recovery.
#[must_use]
pub fn check_settling(report: &DriverReport) -> CheckOutcome {
    if matches!(report.status.outcome, Outcome::Succeeded) {
        return CheckOutcome::Fail("claimed success under a station-exceeding disturbance".to_string());
    }
    if report.status.failure_detail.as_deref() != Some("station_exceeded") {
        return CheckOutcome::Fail(format!(
            "expected failure_detail station_exceeded, got {:?}",
            report.status.failure_detail
        ));
    }
    CheckOutcome::Pass
}
```

- [ ] **Step 8: Add `HoverResponse` + `HoverSettlingDriver`** — in `lib.rs`, after the
`DisturbanceDriver` impl (~line 281) add:

```rust
/// How a driver responds to a bench-injected lateral impulse on a `reach.hover` (`spec/01`
/// § 1.5 C2). `Recovers` / `Aborts` are the two conformant outcomes (recover-and-continue vs
/// abort-to-safe-state); `FailsToRecover` / `ClaimsSuccess` are adversarial.
#[derive(Debug, Clone, Copy)]
pub enum HoverResponse {
    /// Conformant: a transient excursion in the grace window, recovered in the settled tail.
    Recovers,
    /// Adversarial: drifts past station_tolerance throughout yet claims success.
    FailsToRecover,
    /// Conformant (over-envelope): abort to a safe state (Failed + station_exceeded).
    Aborts,
    /// Adversarial (over-envelope): claim success despite the station-exceeding impulse.
    ClaimsSuccess,
}

/// The reach.hover ENV3 bench (`spec/05` § Disturbance injection; `spec/01` § 1.5 C2): models
/// a driver's response to a calibrated lateral impulse. Reads `station_tolerance` from the
/// action's `station_keeping`; mutates the nominal report's `station_error` / outcome per
/// `response`. Non-hover actions (no `station_keeping`) pass through unchanged.
#[derive(Debug)]
pub struct HoverSettlingDriver {
    inner: ReferenceDriver,
    response: HoverResponse,
}

impl HoverSettlingDriver {
    /// A hover settling driver with the given response policy.
    #[must_use]
    pub fn new(response: HoverResponse) -> Self {
        HoverSettlingDriver { inner: ReferenceDriver::default(), response }
    }
}

impl Driver for HoverSettlingDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        let tol = goal
            .canonical_action
            .safety_envelope
            .station_keeping
            .as_ref()
            .and_then(|sk| sk.get("station_tolerance"))
            .and_then(serde_json::Value::as_str)
            .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v));
        let Some(tol) = tol else { return report }; // non-hover: passthrough
        let over = tol + 3.0; // above tolerance (the impulse / failed recovery)
        let under = tol / 2.0; // within tolerance (recovered)
        let q = |v: f64| Some(rfl_core::quantity::Quantity::from_si(v, "mm"));
        match self.response {
            HoverResponse::Recovers => {
                // sample 0 = transient excursion (grace window); tail = recovered.
                for (i, t) in report.telemetry.iter_mut().enumerate() {
                    t.station_error = q(if i == 0 { over } else { under });
                }
            }
            HoverResponse::FailsToRecover => {
                for t in &mut report.telemetry {
                    t.station_error = q(over);
                }
            }
            HoverResponse::Aborts => {
                report.status.outcome = Outcome::Failed;
                report.status.failure_class = Some("blocked".to_string());
                report.status.failure_detail = Some("station_exceeded".to_string());
                if let Some(v) = report.status.verdict.as_mut() {
                    v.value = false; // honest: the station was not held
                }
            }
            HoverResponse::ClaimsSuccess => {
                for t in &mut report.telemetry {
                    t.station_error = q(over); // ignored the impulse, still claims success
                }
            }
        }
        report
    }
}
```

- [ ] **Step 9: Add the lib unit test** — in `lib.rs` tests module, after
`graceful_degradation_accepts_secured_halt_rejects_pretended_success` add:

```rust
    #[test]
    fn check_settling_accepts_station_exceeded_abort_rejects_pretended_success() {
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        let status = |outcome: Outcome, detail: Option<&str>| Status {
            message: "status",
            action_id: "s/e/0001-hover".to_string(),
            outcome,
            verdict: Some(Verdict { value: false, confidence: 1.0, evidence: vec![] }),
            fidelity_tier: None,
            final_pose: Some(RealizedPose::placeholder()),
            failure_class: detail.map(|_| "blocked".to_string()),
            failure_detail: detail.map(str::to_string),
        };
        let report = |o, d| DriverReport { telemetry: vec![], status: status(o, d) };
        // abort to safe state: Failed + station_exceeded -> Pass.
        assert_eq!(check_settling(&report(Outcome::Failed, Some("station_exceeded"))), CheckOutcome::Pass);
        // pretended success -> Fail.
        assert!(matches!(check_settling(&report(Outcome::Succeeded, None)), CheckOutcome::Fail(_)));
        // wrong halt reason -> Fail.
        assert!(matches!(check_settling(&report(Outcome::Failed, Some("blocked"))), CheckOutcome::Fail(_)));
    }
```

- [ ] **Step 10: Run the new tests — green**

Run: `cargo test -p rfl-conformance hover_ check_settling 2>&1 | tail -10`
Expected: the 4 integration tests + the lib unit test PASS. The pre-existing
`nominal_hover_passes_interval_invariant` still PASSES (nominal `station_error = 0 mm ≤ 2 mm`).

- [ ] **Step 11: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -3`
Expected: green; `rfl-conformance` lib 10 (was 9, +1), `envelope_conformance` 23 (was 19, +4).

- [ ] **Step 12: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs && \
git commit -m "feat(conformance): verify reach.hover settling recovery (ENV3 C2)

Strengthen the interval-invariant arm with a settled-tail station-keeping leg
(vacuous without station_keeping, so carry is unaffected) + add check_settling
(over-envelope honest abort) + HoverSettlingDriver. Two adversarial responses
(FailsToRecover, ClaimsSuccess) prove the new station_error teeth bite.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `lib.rs` + `envelope_conformance.rs`.

---

### Task 4: spec/05 ENV3 generalization + README

**Files:**
- Modify: `spec/05-conformance.md` (Disturbance-injection clause + ENV3 bullet)
- Modify: `README.md` (status line)

- [ ] **Step 1: Generalize the spec/05 Disturbance-injection paragraph** — in
`spec/05-conformance.md`, the `### Disturbance injection for interval classes` paragraph
currently describes only `transport.carry`. Append a sentence naming hover's shape:

> `reach.hover` is perturbed the same way (a calibrated lateral impulse, `01` § 1.5 C2): a
> sub-envelope impulse must recover to within `station_tolerance` within `settling_time`
> (recover-and-continue), and an over-envelope impulse must abort to a safe state
> (`station_exceeded`) rather than claim it held station — the no-object analogue of carry's
> "object secured."

- [ ] **Step 2: Generalize the ENV3 obligation bullet** — replace the `**ENV3 — disturbance
injection.**` bullet with:

```markdown
- **ENV3 — disturbance injection.** A disturbance-rejecting interval test perturbs the
  primitive and verifies the invariant under perturbation plus graceful degradation above the
  envelope: `transport.carry` injects up to its `disturbance_budget` and degrades with the
  object secured; `reach.hover` injects a calibrated impulse (`01` § 1.5 C2) and either
  recovers within `settling_time` to `station_tolerance` or aborts to a safe state
  (`station_exceeded`).
```

- [ ] **Step 3: Verify spec/05 still validates** (no schema dependency, but confirm no broken
divider/markdown)

Run: `cargo test 2>&1 | grep -E "test result|error\[" | tail -5`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -3`
Expected: unchanged green (spec prose has no test; this confirms nothing regressed).

- [ ] **Step 4: Update the README status line** — find the line noting ENV3 disturbance
injection for `transport.carry` (added in commit `33f4f02`) and extend it to note
`reach.hover` settling ENV3 is now covered too (recover-or-abort). Match the surrounding
README phrasing; keep it one line.

- [ ] **Step 5: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add spec/05-conformance.md README.md && \
git commit -m "docs(spec): generalize ENV3 to reach.hover settling

ENV3 was written carry-only (object secured). Name hover's degradation shape:
recover within settling_time to station_tolerance, or abort to a safe state
(station_exceeded), per spec/01 § 1.5 C2. Both interval-invariant primitives
now have an ENV3 contract.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `spec/05-conformance.md` + `README.md`.

---

## Post-increment

- Final full-suite read: `cargo test` + `validate.py` green. Expected totals: `rfl-core` 73,
  `rfl-conformance` lib 10, `driver_protocol` 8, `envelope_conformance` 23, others unchanged.
- Update `project_rfl.md` "Implementation track" (15th increment, real hashes + test deltas)
  and the `MEMORY.md` RFL line. Move the deferred items (safe-state geometry / `stop_time`
  timing, `settling_time: auto`, `track_target` tracking-bandwidth, physics injection) into the
  parked list.

## Self-review (spec coverage)

- station_error signal (design § 2) → Task 1. ✓
- station_keeping lowering from existing params (design § 3-4) → Task 2. ✓
- settled-tail recovery leg + check_settling + driver + non-vacuity (design § 3-5) → Task 3. ✓
- spec/05 ENV3 generalization (design § 4) → Task 4. ✓
- No skill-isa / spec/01 change (design § 0) → confirmed: no task touches them. ✓
- Type consistency: `station_keeping` (Option<Value>), `station_error` (Option<Quantity>),
  `check_settling(report)` (no goal), `HoverResponse{Recovers,FailsToRecover,Aborts,ClaimsSuccess}`
  used identically in lib.rs and envelope_conformance.rs. ✓
