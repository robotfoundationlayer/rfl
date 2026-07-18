# force.press_button (detent) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement
> this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> **LOCAL-ONLY** — never `git add` this file.

**Goal:** Implement `force.press_button` (detent mode) to activate the conformance-inert
`events` ForceEvent channel: event-gated success (Succeeded requires a detent) plus a
force-trajectory budget leg.

**Architecture:** Fix the latent `Telemetry.events` drift (`Vec<String>` → `Vec<Value>`), add
the `ForcePressButton` primitive (struct + variant + lower + capability gate), reuse
`ForceTrajectory` for the force-budget leg, and add a new `check_actuation` reading `events`.
No skill-isa / capability JSON-schema change (all pre-specced). Design:
`docs/design/2026-06-01-force-press-button-design.md`.

**Tech Stack:** Rust (rfl-core / rfl-conformance), insta goldens, boon (JSON-schema),
`schemas/validate.py`.

**Discipline (every task):** TDD red→green. After green, run `cargo test` (workspace, NOT `-p`)
+ `uv run --with jsonschema --with pyyaml python schemas/validate.py` in a batch SEPARATE from
the commit. Before each push: `git fetch -q && git merge-base --is-ancestor origin/main HEAD`,
`git add <explicit paths>`, commit, `git push origin main`, then verify
`git rev-list --left-right --count HEAD...origin/main` == `0 0` and `git show --stat HEAD` lists
only the intended files. Stop-gate: reconcile and explain any golden diff vs the prediction.

---

### Task 1: fix the `Telemetry.events` drift (`Vec<String>` → `Vec<serde_json::Value>`)

**Files:**
- Modify: `crates/rfl-core/src/driver.rs` (Telemetry.events field ~line 103)
- Test: `crates/rfl-conformance/tests/driver_protocol.rs` (new test)

- [ ] **Step 1: Write the failing test** — append to `driver_protocol.rs`:

```rust
#[test]
fn telemetry_with_detent_event_is_schema_valid() {
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
        action_id: "s/e/0001-press_button".to_string(),
        t: 1.0,
        realized_pose: Some(RealizedPose::placeholder()),
        wrench: None,
        securing_force: None,
        station_error: None,
        tactile: vec![],
        events: vec![serde_json::json!({ "kind": "detent" })],
        fidelity_tier: None,
    };
    let v = serde_json::to_value(&t).unwrap();
    schemas.validate(&v, idx).expect("a detent ForceEvent object must be schema-valid");
}
```

- [ ] **Step 2: Run to verify it fails (compile error — events is Vec<String>)**

Run: `cargo test -p rfl-conformance --test driver_protocol telemetry_with_detent_event 2>&1 | grep -E "error\[|mismatched|test result" | head`
Expected: compile error — `serde_json::json!{...}` (a `Value`) does not match `String` in `events`.

- [ ] **Step 3: Change the field type** — in `crates/rfl-core/src/driver.rs`, replace the
`events` field:

```rust
    /// ForceEvents fired this sample (`04` § Force events; `ForceEventFloor` objects:
    /// `kind` / `at` / `magnitude`, e.g. a detent click). Read by `check_actuation`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<serde_json::Value>,
```

(The four `events: vec![]` literals — driver.rs test, lib.rs ReferenceDriver + two lib tests —
are unaffected: an empty `vec![]` infers `Vec<serde_json::Value>`.)

- [ ] **Step 4: Run to verify it passes (the object validates against `ForceEventFloor`)**

Run: `cargo test -p rfl-conformance --test driver_protocol telemetry_with_detent_event 2>&1 | grep -E "test result"`
Expected: PASS (1 passed).

- [ ] **Step 5: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `driver_protocol` 9 (was 8, +1); everything else unchanged (the cable example
emits no events, so all existing goldens are byte-identical).

- [ ] **Step 6: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-core/src/driver.rs crates/rfl-conformance/tests/driver_protocol.rs && \
git commit -m "fix(driver): Telemetry.events carries ForceEventFloor objects

events was Vec<String> but the schema's ForceEventFloor is an object
(kind/at/magnitude). They never disagreed only because events was always
empty (skipped). Make the struct match the schema so a detent ForceEvent
can be emitted and conformance-checked (force.press_button, next).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only the 2 files.

---

### Task 2: the `ForcePressButton` primitive (struct + variant + lower + gate) + example + golden

**Files:**
- Modify: `crates/rfl-core/src/skill_isa.rs` (Primitive enum ~line 145; new struct near ForceScrew ~line 478)
- Modify: `crates/rfl-core/src/translation.rs` (use import ~line 20; check_capability ~line 133; lower dispatch ~line 160; new lower fn; new tests)
- Create: `examples/03-screw-fasten/skill-press.yaml`
- Modify: `examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml` (skills list)
- Create: `crates/rfl-conformance/tests/press_button.rs`

- [ ] **Step 1: Write the failing capability-gate test** — in `translation.rs` tests module,
add (place near `screw_capability_absent_when_not_declared`):

```rust
    const PRESS_SKILL: &str = "skill: t\nbody:\n  sequence:\n    - force.press_button: { target: button, actuation: detent, force_budget: 5 N }\n";

    #[test]
    fn press_button_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(PRESS_SKILL).unwrap();
        let emb = load("allegro").1; // cable allegro lacks force.press_button
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: force.press_button"), "got {err}");
    }
```

- [ ] **Step 2: Run to verify it fails (no ForcePressButton variant)**

Run: `cargo test -p rfl-core press_button_capability_absent 2>&1 | grep -E "error\[|no variant|test result" | head`
Expected: compile error — `force.press_button` deserializes to no known `Primitive` variant
(the enum has no `ForcePressButton`), so `Skill::parse_yaml` fails / the variant is unknown.

- [ ] **Step 3: Add the `ForcePressButton` struct** — in `skill_isa.rs`, after the `ForceScrew`
struct (before `ForceUnscrew`, ~line 478):

```rust
/// `force.press_button` parameters (v0 subset of `$defs/ForcePressButtonParams`, § 6.6).
/// `target` + `actuation` + `force_budget` are required. v0 lowers the force_budget (the
/// force-trajectory leg) + the detent actuation marker; `press_direction` / `max_travel` /
/// `release_after` are schema-carried but symbolic in v0 (the over-travel guard is deferred).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForcePressButton {
    /// The button surface / frame to press (v0: a frame ref).
    pub target: FrameRef,
    /// `ActuationSpec` — what marks actuation: `detent` (v0) | `effort_rise(force_threshold)`.
    pub actuation: serde_yaml::Value,
    /// Max press force (over-travel / mechanism-damage limit) — the force-trajectory bound.
    pub force_budget: Quantity,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
}
```

- [ ] **Step 4: Add the Primitive variant** — in `skill_isa.rs` `enum Primitive`, after the
`TransportCarry` variant (~line 145):

```rust
    /// `force.press_button`.
    #[serde(rename = "force.press_button")]
    ForcePressButton(ForcePressButton),
```

- [ ] **Step 5: Wire the import + capability gate + lower dispatch** — in `translation.rs`:

In the `use crate::skill_isa::{...}` block (~line 20), add `ForcePressButton` to the import list.

In `check_capability` (~line 133), add after the `ForceUnscrew` arm:

```rust
        Primitive::ForcePressButton(_) => "force.press_button",
```

In the `lower` dispatch `match prim` (~line 160), add after the `ForceUnscrew` arm:

```rust
        Primitive::ForcePressButton(p) => (lower_force_press_button(p, e), "press_button"),
```

- [ ] **Step 6: Add the lowering fn** — in `translation.rs`, after `lower_force_unscrew`:

```rust
/// Lower `force.press_button` (`spec/01` § 6.6): an effector press bounded by a force
/// trajectory (press force ≤ `force_budget`) and gated by an actuation event. v0 emits the
/// `force_budget` (the ForceTrajectory leg), the detent actuation marker into `force_profile`
/// (the bench echoes the detent; `check_actuation` verifies it), and the actuation as a
/// `Monitor` stop condition. Detent only; `effort_rise(force_threshold)` is carried but
/// unmarked (deferred). The over-travel / `max_travel` guard needs a displacement signal
/// (deferred).
fn lower_force_press_button(p: &ForcePressButton, e: &Embodiment) -> CanonicalAction {
    let monitors = vec![Monitor { stop_condition: yaml_to_json(&p.actuation) }];
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    if actuation_is_detent(&p.actuation) {
        env.force_profile = Some(serde_json::json!({ "actuation": "detent" }));
    }
    CanonicalAction {
        target_frame: e.control_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target.clone() },
        force_budget: Some(p.force_budget.clone()),
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::TimeScalable,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors,
        safety_envelope: env,
    }
}

/// True if an `ActuationSpec` is the detent mode (bare `detent` or `{detent: ...}`).
fn actuation_is_detent(v: &serde_yaml::Value) -> bool {
    v.as_str() == Some("detent")
        || v.as_mapping()
            .is_some_and(|m| m.contains_key(serde_yaml::Value::String("detent".to_string())))
}
```

- [ ] **Step 7: Run the capability test (green) — variant now exists**

Run: `cargo test -p rfl-core press_button_capability_absent 2>&1 | grep -E "test result"`
Expected: PASS (cable allegro lacks `force.press_button` → `capability_absent: force.press_button`).

- [ ] **Step 8: Create the example skill** — `examples/03-screw-fasten/skill-press.yaml`:

```yaml
# Example 03 — Press button (force.press_button variant)
# A momentary button press: advance into the button until its actuation detent (click)
# fires, within a press-force budget, without over-travel past the click. Exercises the
# event-gated force class (spec/05 ENV4 + the ForceEvent channel): a detent marks
# actuation; a force rise with no detent is no_actuation (a stuck / absent button).
skill: press-button
description: >
  Press a panel button until its actuation detent fires, within a press-force budget.

body:
  sequence:

    - force.press_button:
        target: button
        actuation: detent
        force_budget: 5 N
        compliance: active
```

- [ ] **Step 9: Declare the capability on the 3 descriptors** — in each of
`examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml`, add `force.press_button`
to the `skills:` array. For allegro the line becomes:

```yaml
    skills: [grasp.pinch, grasp.lateral, grasp.envelope, transport, force.insert_fit, force.screw, force.unscrew, force.press_button, sense.locate]
```

For `leap` and `pneumatic-6f`, add the `force.press_button` token to their existing `skills:`
array (read each line first; insert before `sense.locate`).

- [ ] **Step 10: Add the positive lowering test** — in `translation.rs` tests module:

```rust
    #[test]
    fn press_button_lowers_force_budget_and_detent_actuation() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill-press.yaml")).unwrap()).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .unwrap();
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["press_button"]);
        let a = &out.actions[0];
        assert_eq!(a.force_budget.as_ref().map(|q| q.0.as_str()), Some("5 N"));
        let fp = serde_json::to_string(&a.safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"actuation\":\"detent\""), "got {fp}");
        let mon = serde_json::to_string(&a.monitors).unwrap();
        assert!(mon.contains("detent"), "got {mon}");
    }
```

- [ ] **Step 11: Run both translation tests (green)**

Run: `cargo test -p rfl-core press_button 2>&1 | grep -E "test result|press_button"`
Expected: both PASS.

- [ ] **Step 12: Create the golden test** — `crates/rfl-conformance/tests/press_button.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for force.press_button: the retarget output is byte-deterministic,
//! matches a committed golden, and every line is a valid driver-interface execute message.
//! (The event-gated actuation verification is in envelope_conformance.rs.)

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-press.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_press_allegro() {
    insta::assert_snapshot!("press_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_press_leap() {
    insta::assert_snapshot!("press_leap", jsonl_for("leap"));
}

#[test]
fn golden_press_pneumatic() {
    insta::assert_snapshot!("press_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn press_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn press_every_line_is_a_valid_execute_message() {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/driver-interface.schema.json");
    let schema: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&schema_path).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource("driver-interface.schema.json", schema).unwrap();
    let idx = compiler.compile("driver-interface.schema.json", &mut schemas).unwrap();
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        for line in jsonl_for(stem).lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            schemas.validate(&v, idx).unwrap_or_else(|e| panic!("{stem}: {e}"));
        }
    }
}
```

- [ ] **Step 13: Generate the goldens + stop-gate inspect**

Run: `INSTA_UPDATE=always cargo test -p rfl-conformance --test press_button 2>&1 | grep -E "test result|error\["`
Then: `cat crates/rfl-conformance/tests/snapshots/press_button__press_allegro.snap`
Expected (STOP-GATE): one execute line, `action_id` `press-button/wonik-allegro-v4/0001-press_button`,
`force_budget":"5 N"`, `force_profile":{"actuation":"detent"}`, `target_pose":{"ref":"button"}`,
`monitors":[{"stop_condition":"detent"}]`, `compliance":"active"`. Three stems differ only by
embodiment id / frame / motion_bounds. Reconcile if anything else appears.

- [ ] **Step 14: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `rfl-core` 75 (was 73, +2); new `press_button` golden binary 5 tests; existing
goldens byte-identical. `validate.py` PASS (the 3 descriptors gained a valid capability token; C1
checks the capability enum ⊇ usage, unaffected).

- [ ] **Step 15: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-core/src/skill_isa.rs crates/rfl-core/src/translation.rs examples/03-screw-fasten/skill-press.yaml examples/03-screw-fasten/embodiments/allegro.yaml examples/03-screw-fasten/embodiments/leap.yaml examples/03-screw-fasten/embodiments/pneumatic-6f.yaml crates/rfl-conformance/tests/press_button.rs crates/rfl-conformance/tests/snapshots && \
git commit -m "feat(core): add force.press_button primitive (detent, force-trajectory)

ForcePressButton struct + Primitive variant + capability gate + lowering:
emit the press force_budget (the ForceTrajectory leg) + a detent actuation
marker in force_profile + the actuation as a monitor. detent only;
effort_rise / max_travel deferred. examples/03 skill-press.yaml + the
capability on 3 descriptors + a byte-deterministic golden.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head -15
```
Verify: sync `0 0`; only the listed files (skill_isa/translation/skill-press/3 descriptors/press_button.rs/3 snaps).

---

### Task 3: conformance — detent echo, `check_actuation`, `PressButtonDriver`

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (ReferenceDriver echo; envelope_class_for; check_actuation; PressButtonDriver; lib unit test)
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (imports + 4 tests)

- [ ] **Step 1: Write the failing integration tests** — append to `envelope_conformance.rs`
(reuse the existing `screw_dir()` + `suffix_of()`):

```rust
// --- force.press_button event-gated actuation (spec/01 § 6.6) --------------------------------

#[test]
fn nominal_press_button_passes_actuation_and_force_trajectory() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-press.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "press_button");
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
    assert_eq!(check_actuation(goal, report), CheckOutcome::Pass);
}

#[test]
fn press_button_bottoming_out_is_honest() {
    let dir = screw_dir();
    let pairs = drive(
        PressButtonDriver::new(PressButtonResponse::Bottoms),
        &dir.join("skill-press.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // no detent fired, force held: not a success, and check_actuation does not falsely fail it.
    assert_eq!(check_actuation(goal, report), CheckOutcome::Pass);
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
    assert!(!matches!(report.status.outcome, rfl_core::driver::Outcome::Succeeded));
}

#[test]
fn press_button_false_actuation_fails() {
    let dir = screw_dir();
    let pairs = drive(
        PressButtonDriver::new(PressButtonResponse::ClaimsActuation),
        &dir.join("skill-press.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // claims success but emitted no detent -> the events channel bites.
    assert!(matches!(check_actuation(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn press_button_over_force_fails_trajectory() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverForce),
        &dir.join("skill-press.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}
```

Update the `use rfl_conformance::{...}` block to add `check_actuation, PressButtonDriver,
PressButtonResponse`.

- [ ] **Step 2: Run to verify it fails (unresolved imports)**

Run: `cargo test -p rfl-conformance --test envelope_conformance press_button 2>&1 | grep -E "error\[|unresolved|test result" | head`
Expected: compile error — `check_actuation` / `PressButtonDriver` / `PressButtonResponse` not found.

- [ ] **Step 3: Add `press_button` to `envelope_class_for`** — in `lib.rs`, change the force arm:

```rust
        "insert_fit" | "screw" | "unscrew" | "press_button" => Some(EnvelopeClass::ForceTrajectory),
```

- [ ] **Step 4: Emit the detent echo in `ReferenceDriver`** — in `lib.rs`
`ReferenceDriver::execute`, after the `station_error` binding, add:

```rust
        // Echo the detent ForceEvent for an action carrying a detent actuation contract
        // (force.press_button, spec/01 § 6.6): the nominal press detects the actuation click.
        // Absent for every other action (events stays empty).
        let events: Vec<serde_json::Value> = match ca
            .safety_envelope
            .force_profile
            .as_ref()
            .and_then(|fp| fp.get("actuation"))
            .and_then(serde_json::Value::as_str)
        {
            Some("detent") => vec![serde_json::json!({ "kind": "detent" })],
            _ => vec![],
        };
```

and in the `Telemetry { ... }` literal in the sample map closure, replace `events: vec![],`
with `events: events.clone(),`.

- [ ] **Step 5: Add `check_actuation`** — in `lib.rs`, after `check_settling`:

```rust
/// Verify the § 6.6 actuation postcondition for `force.press_button`: an actuated (Succeeded)
/// press MUST show the detent ForceEvent that marks actuation. Vacuous unless the action
/// carries an `actuation` contract (every non-press_button action passes). Makes the `events`
/// ForceEvent channel falsifiable — a success claimed without a detent fails.
#[must_use]
pub fn check_actuation(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    if goal
        .canonical_action
        .safety_envelope
        .force_profile
        .as_ref()
        .and_then(|fp| fp.get("actuation"))
        .is_none()
    {
        return CheckOutcome::Pass; // not an actuated press -> vacuous
    }
    if matches!(report.status.outcome, Outcome::Succeeded) {
        let has_detent = report.telemetry.iter().any(|t| {
            t.events
                .iter()
                .any(|e| e.get("kind").and_then(serde_json::Value::as_str) == Some("detent"))
        });
        if !has_detent {
            return CheckOutcome::Fail(
                "press_button claimed success without a detent actuation event".to_string(),
            );
        }
    }
    CheckOutcome::Pass
}
```

- [ ] **Step 6: Add `PressButtonDriver` + `PressButtonResponse`** — in `lib.rs`, after the
`HoverSettlingDriver` impl:

```rust
/// How a driver reports a `force.press_button` press (`spec/01` § 6.6). `Actuates` is the
/// nominal detent + success; `Bottoms` is the conformant `no_actuation` (a force rise with no
/// detent — a stuck / absent button); `ClaimsActuation` is adversarial (success, no detent).
#[derive(Debug, Clone, Copy)]
pub enum PressButtonResponse {
    /// Conformant: the actuation detent fired and the press succeeded (ReferenceDriver nominal).
    Actuates,
    /// Conformant: no detent fired -> no_actuation reported honestly (force kept within budget).
    Bottoms,
    /// Adversarial: claim success though no detent fired.
    ClaimsActuation,
}

/// The `force.press_button` bench: models a driver's actuation outcome. Reuses the nominal
/// `ReferenceDriver` (which echoes the detent for an actuation contract) and mutates it per
/// `response`. Non-press actions pass through unchanged.
#[derive(Debug)]
pub struct PressButtonDriver {
    inner: ReferenceDriver,
    response: PressButtonResponse,
}

impl PressButtonDriver {
    /// A press-button driver with the given actuation outcome.
    #[must_use]
    pub fn new(response: PressButtonResponse) -> Self {
        PressButtonDriver { inner: ReferenceDriver::default(), response }
    }
}

impl Driver for PressButtonDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        match self.response {
            PressButtonResponse::Actuates => {} // nominal: detent echoed + Succeeded
            PressButtonResponse::Bottoms => {
                for t in &mut report.telemetry {
                    t.events.clear(); // no detent fired
                }
                report.status.outcome = Outcome::Failed;
                report.status.failure_class = Some("blocked".to_string());
                report.status.failure_detail = Some("no_actuation".to_string());
                if let Some(v) = report.status.verdict.as_mut() {
                    v.value = false; // honest: the button was not actuated
                }
            }
            PressButtonResponse::ClaimsActuation => {
                for t in &mut report.telemetry {
                    t.events.clear(); // no detent, yet claims success
                }
            }
        }
        report
    }
}
```

- [ ] **Step 7: Add the lib unit test for `check_actuation`** — in `lib.rs` tests module, after
`check_settling_accepts_station_exceeded_abort_rejects_pretended_success`:

```rust
    #[test]
    fn check_actuation_requires_a_detent_on_success() {
        use rfl_core::canonical::{CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode};
        use rfl_core::driver::{Outcome, RealizedPose, Status, Telemetry, Verdict};
        let action = CanonicalAction {
            target_frame: "control".into(),
            target_pose: PoseExpr::Ref { r#ref: "button".into() },
            force_budget: Some(rfl_core::quantity::Quantity("5 N".into())),
            timing: TimingHints { nominal_duration: None, timing_mode: TimingMode::TimeScalable, stop_at_goal: true },
            tactile_target: None,
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: Some(serde_json::json!({ "actuation": "detent" })),
                station_keeping: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
        };
        let goal = ExecuteGoal::wrap("s/e/0001-press_button".to_string(), action);
        let sample = |events: Vec<serde_json::Value>| Telemetry {
            message: "telemetry",
            action_id: "s/e/0001-press_button".to_string(),
            t: 1.0,
            realized_pose: Some(RealizedPose::placeholder()),
            wrench: None,
            securing_force: None,
            station_error: None,
            tactile: vec![],
            events,
            fidelity_tier: None,
        };
        let status = |outcome: Outcome| Status {
            message: "status",
            action_id: "s/e/0001-press_button".to_string(),
            outcome,
            verdict: Some(Verdict { value: true, confidence: 1.0, evidence: vec![] }),
            fidelity_tier: None,
            final_pose: Some(RealizedPose::placeholder()),
            failure_class: None,
            failure_detail: None,
        };
        // Succeeded + detent -> Pass.
        let ok = DriverReport {
            telemetry: vec![sample(vec![serde_json::json!({ "kind": "detent" })])],
            status: status(Outcome::Succeeded),
        };
        assert_eq!(check_actuation(&goal, &ok), CheckOutcome::Pass);
        // Succeeded + no detent -> Fail (the events bite).
        let claims = DriverReport { telemetry: vec![sample(vec![])], status: status(Outcome::Succeeded) };
        assert!(matches!(check_actuation(&goal, &claims), CheckOutcome::Fail(_)));
        // Not succeeded -> vacuously Pass.
        let bottoms = DriverReport { telemetry: vec![sample(vec![])], status: status(Outcome::Failed) };
        assert_eq!(check_actuation(&goal, &bottoms), CheckOutcome::Pass);
    }
```

- [ ] **Step 8: Run the new tests (green)**

Run: `cargo test -p rfl-conformance press_button check_actuation 2>&1 | grep -E "test result|press_button|check_actuation"` (run the two filters separately if the runner rejects two args: `cargo test -p rfl-conformance press_button` then `cargo test -p rfl-conformance check_actuation`).
Expected: the 4 integration tests + the lib unit test PASS.

- [ ] **Step 9: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `rfl-conformance` lib 11 (was 10, +1), `envelope_conformance` 27 (was 23, +4).

- [ ] **Step 10: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs && \
git commit -m "feat(conformance): verify force.press_button event-gated actuation

envelope_class_for(press_button)=>ForceTrajectory + ReferenceDriver echoes a
detent ForceEvent for an actuation contract + check_actuation (Succeeded =>
a detent event present, vacuous otherwise). PressButtonDriver{Actuates,
Bottoms,ClaimsActuation}: ClaimsActuation (success without a detent) fails
check_actuation, OverForce fails ForceTrajectory. Turns the events channel
from inert to falsifiable.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `lib.rs` + `envelope_conformance.rs`.

---

### Task 4: README status line

**Files:**
- Modify: `README.md` (the reference-implementation status line ~line 84)

- [ ] **Step 1: Extend the README status line** — find the sentence listing the worked
examples / force primitives (it mentions `force.screw` / `force.unscrew`) and the envelope
classes, and add `force.press_button` as a category-6 primitive whose **event-gated actuation**
(a detent marks success; a force rise with no detent is `no_actuation`) is conformance-checked
— the first primitive to exercise the `events` ForceEvent channel. Match the surrounding
phrasing; keep it to the existing sentence structure. (spec/05 already states force-event
detections are evaluated on the sampled trajectory, so no spec change.)

- [ ] **Step 2: Verify nothing regressed**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: unchanged green (README prose has no test).

- [ ] **Step 3: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add README.md && \
git commit -m "docs(readme): note force.press_button event-gated actuation

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `README.md`.

---

## Post-increment

- Final full-suite read: expected totals `rfl-core` 75, `rfl-conformance` lib 11,
  `driver_protocol` 9, `envelope_conformance` 27, new `press_button` 5, others unchanged.
- Update `project_rfl.md` "Implementation track" (16th increment, real hashes + test deltas)
  and the `MEMORY.md` RFL line. Move deferred items (effort_rise actuation, max_travel /
  over_travel guard, release_after, inline SurfaceTarget) into the parked list.

## Self-review (spec coverage)

- events drift fix (design § 1, § 4 commit A) → Task 1. ✓
- ForcePressButton primitive + lowering + gate (design § 3, § 4 commit B) → Task 2. ✓
- envelope_class_for + detent echo + check_actuation + PressButtonDriver + non-vacuity
  (design § 3-5, § 4 commit C) → Task 3. ✓
- README (design § 6 commit 4) → Task 4. ✓
- No skill-isa / capability schema change (design § 2) → confirmed: no task touches those
  schemas; the descriptors gain a capability *token* (data), not a schema change. ✓
- Type consistency: `ForcePressButton{target: FrameRef, actuation: Value, force_budget:
  Quantity, compliance}`; `Telemetry.events: Vec<serde_json::Value>`; `check_actuation(goal,
  report)`; `PressButtonResponse{Actuates,Bottoms,ClaimsActuation}` used identically across
  lib.rs and envelope_conformance.rs; `force_profile.actuation == "detent"` written by the
  lowering and read by both the ReferenceDriver echo and check_actuation. ✓
```
