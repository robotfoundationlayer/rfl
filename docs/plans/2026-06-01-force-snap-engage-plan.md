# force.snap_engage (semi-reversible REV2) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement
> this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> **LOCAL-ONLY** — never `git add` this file.

**Goal:** Implement `force.snap_engage` (REV2 semi-reversible), reusing press_button's detent
(`check_actuation`) + `ForceTrajectory`, and adding the new `confirm_held` engagement-confirmation
leg (`check_engagement`, held-confirmation via `verdict.evidence`).

**Architecture:** Add the `ForceSnapEngage` primitive (struct + variant + lower + gate); lower
`force_profile = {actuation:detent, confirm_held:true}` + `force_budget`; `envelope_class_for =>
ForceTrajectory`; the ReferenceDriver pushes `"held_confirmed"` into `verdict.evidence` when
`confirm_held` is set; add `check_engagement` (Succeeded ⟹ held_confirmed) + a `SnapEngageDriver`.
No schema change. Design: `docs/design/2026-06-01-force-snap-engage-design.md`.

**Tech Stack:** Rust (rfl-core / rfl-conformance), insta goldens, boon, `schemas/validate.py`.

**Discipline (every task):** TDD red→green. After green, run `cargo test` (workspace, NOT `-p`)
+ `uv run --with jsonschema --with pyyaml python schemas/validate.py` in a batch SEPARATE from
the commit. Before each push: `git fetch -q && git merge-base --is-ancestor origin/main HEAD`,
`git add <explicit paths>`, commit, `git push origin main`, verify sync `0 0` + `git show --stat`.
Stop-gate: reconcile and explain any golden diff vs the prediction.

---

### Task 1: the `ForceSnapEngage` primitive (struct + variant + lower + gate) + example + golden

**Files:**
- Modify: `crates/rfl-core/src/skill_isa.rs` (Primitive enum; new struct near ForceWipe)
- Modify: `crates/rfl-core/src/translation.rs` (use import; check_capability; lower dispatch; new lower fn; tests)
- Create: `examples/03-screw-fasten/skill-snap.yaml`
- Modify: `examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml` (skills list)
- Create: `crates/rfl-conformance/tests/snap_engage.rs`

- [ ] **Step 1: Write the failing capability-gate test** — in `translation.rs` tests module,
near `wipe_capability_absent_when_not_declared`:

```rust
    const SNAP_SKILL: &str = "skill: t\nbody:\n  sequence:\n    - force.snap_engage: { mate_feature: clip, engage_direction: +z, force_budget: 25 N, confirm_held: true }\n";

    #[test]
    fn snap_engage_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(SNAP_SKILL).unwrap();
        let emb = load("allegro").1; // cable allegro lacks force.snap_engage
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: force.snap_engage"), "got {err}");
    }
```

- [ ] **Step 2: Run to verify it fails (unknown primitive)**

Run: `cargo test -p rfl-core snap_engage_capability_absent 2>&1 | grep -E "error\[|panicked|test result|FAILED" | head`
Expected: FAIL — `force.snap_engage` deserializes to no `Primitive` variant; `Skill::parse_yaml`
fails; `.unwrap()` panics.

- [ ] **Step 3: Add the `ForceSnapEngage` struct** — in `skill_isa.rs`, after the `ForceWipe`
struct:

```rust
/// `force.snap_engage` parameters (v0 subset of `$defs/ForceSnapEngageParams`, § 6.10).
/// `mate_feature` + `engage_direction` + `force_budget` are required. v0 lowers the force_budget
/// (the force-trajectory leg) + the detent snap-in marker (reusing press_button's detent) + the
/// confirm_held engagement-confirmation marker. `mate_feature` is carried symbolic; the
/// documented reverse path (snap_disengage) is deferred.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceSnapEngage {
    /// The bistable mechanism / receptacle to engage (carried symbolic in v0).
    pub mate_feature: serde_yaml::Value,
    /// Direction to drive engagement, in `frame`.
    pub engage_direction: Direction,
    /// Max engagement force (mechanism-break / over-force limit) — the force-trajectory bound.
    pub force_budget: Quantity,
    /// `ActuationSpec` — what marks snap-in: `detent` (default) | `effort_rise(force_threshold)`.
    #[serde(default)]
    pub snap_signature: Option<serde_yaml::Value>,
    /// Verify the bistable connection holds after engagement (release-test; default true).
    #[serde(default)]
    pub confirm_held: Option<bool>,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
}
```

- [ ] **Step 4: Add the Primitive variant** — in `skill_isa.rs` `enum Primitive`, after the
`ForceWipe` variant:

```rust
    /// `force.snap_engage`.
    #[serde(rename = "force.snap_engage")]
    ForceSnapEngage(ForceSnapEngage),
```

- [ ] **Step 5: Wire the import + capability gate + lower dispatch** — in `translation.rs`:

In the `use crate::skill_isa::{...}` block, add `ForceSnapEngage` to the import list.

In `check_capability`, after the `Primitive::ForceWipe(_)` arm:

```rust
        Primitive::ForceSnapEngage(_) => "force.snap_engage",
```

In the `lower` dispatch `match prim`, after the `Primitive::ForceWipe(p)` arm:

```rust
        Primitive::ForceSnapEngage(p) => (lower_force_snap_engage(p, e), "snap_engage"),
```

- [ ] **Step 6: Add the lowering fn** — in `translation.rs`, after `lower_force_wipe` (before
`actuation_is_detent`):

```rust
/// Lower `force.snap_engage` (`spec/01` § 6.10): a bistable engagement bounded by a force
/// trajectory (engagement force ≤ `force_budget`) and gated by a snap-in event (reusing
/// press_button's detent) + a confirm_held engagement-confirmation. v0 emits force_budget (the
/// ForceTrajectory leg), `force_profile.actuation = "detent"` (when snap_signature is detent /
/// default), and `force_profile.confirm_held = true` (default); the snap_signature lowers into a
/// Monitor. The held part's motion is along `engage_direction` (grasp frame, like force.screw);
/// `mate_feature` is carried symbolic; the snap_disengage reverse path is deferred.
fn lower_force_snap_engage(p: &ForceSnapEngage, e: &Embodiment) -> CanonicalAction {
    let is_detent = p.snap_signature.as_ref().map_or(true, actuation_is_detent);
    let do_confirm = p.confirm_held != Some(false);
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    if is_detent || do_confirm {
        let mut fp = serde_json::json!({});
        if is_detent {
            fp["actuation"] = serde_json::json!("detent");
        }
        if do_confirm {
            fp["confirm_held"] = serde_json::json!(true);
        }
        env.force_profile = Some(fp);
    }
    let sig = p
        .snap_signature
        .clone()
        .unwrap_or_else(|| serde_yaml::Value::String("detent".to_string()));
    let monitors = vec![Monitor { stop_condition: yaml_to_json(&sig) }];
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::AxisRelative {
            direction: yaml_to_json(&p.engage_direction),
            distance: Quantity("0 mm".to_string()),
        },
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
```

- [ ] **Step 7: Run the capability test (green)**

Run: `cargo test -p rfl-core snap_engage_capability_absent 2>&1 | grep -E "test result"`
Expected: PASS.

- [ ] **Step 8: Create the example skill** — `examples/03-screw-fasten/skill-snap.yaml`:

```yaml
# Example 03 — Snap engage (force.snap_engage variant)
# Engage a bistable clip / latch / snap-fit by driving the held part through the engagement
# force peak until the snap-in detent fires, within a force budget, then confirm the connection
# holds. Exercises the reversibility classification (spec/05 REV2, semi-reversible-persistent):
# the snap-in reuses press_button's detent, and confirm_held verifies the engagement actually
# holds (a false_engagement claiming success fails).
skill: surface-snap
description: >
  Engage a bistable clip by driving the held part to its snap-in detent within a force
  budget, then confirm the connection holds.

body:
  sequence:

    - force.snap_engage:
        mate_feature: clip
        engage_direction: +z
        force_budget: 25 N
        confirm_held: true
        compliance: active
```

- [ ] **Step 9: Declare the capability on the 3 descriptors** — in each of
`examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml`, add `force.snap_engage`
to the `skills:` array (read each line first; insert before `sense.locate`). For allegro the
line becomes:

```yaml
    skills: [grasp.pinch, grasp.lateral, grasp.envelope, transport, force.insert_fit, force.screw, force.unscrew, force.press_button, force.wipe, force.snap_engage, sense.locate]
```

- [ ] **Step 10: Add the positive lowering test** — in `translation.rs` tests module:

```rust
    #[test]
    fn snap_engage_lowers_actuation_confirm_held_and_force_budget() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill-snap.yaml")).unwrap()).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .unwrap();
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["snap_engage"]);
        let a = &out.actions[0];
        assert_eq!(a.force_budget.as_ref().map(|q| q.0.as_str()), Some("25 N"));
        let fp = serde_json::to_string(&a.safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"actuation\":\"detent\""), "got {fp}");
        assert!(fp.contains("\"confirm_held\":true"), "got {fp}");
    }
```

- [ ] **Step 11: Run both translation tests (green)**

Run: `cargo test -p rfl-core snap_engage 2>&1 | grep -E "test result|snap_engage"`
Expected: both PASS.

- [ ] **Step 12: Create the golden test** — `crates/rfl-conformance/tests/snap_engage.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for force.snap_engage: the retarget output is byte-deterministic,
//! matches a committed golden, and every line is a valid driver-interface execute message. (The
//! snap-in + engagement-confirmation verification is in envelope_conformance.rs.)

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-snap.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_snap_allegro() {
    insta::assert_snapshot!("snap_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_snap_leap() {
    insta::assert_snapshot!("snap_leap", jsonl_for("leap"));
}

#[test]
fn golden_snap_pneumatic() {
    insta::assert_snapshot!("snap_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn snap_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn snap_every_line_is_a_valid_execute_message() {
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

Run: `INSTA_UPDATE=always cargo test -p rfl-conformance --test snap_engage 2>&1 | grep -E "test result|error\["`
Then: `cat crates/rfl-conformance/tests/snapshots/snap_engage__snap_allegro.snap`
Expected (STOP-GATE): one execute line, `action_id` `surface-snap/wonik-allegro-v4/0001-snap_engage`,
`target_frame":"tcp_thumb"` (grasp frame, like screw), `target_pose":{"direction":"+z","distance":"0 mm"}`,
`force_budget":"25 N"`, `monitors":[{"stop_condition":"detent"}]`,
`force_profile":{"actuation":"detent","confirm_held":true}` (keys alphabetical), `compliance":"active"`,
`stop_time":"0.1 s"`. Three stems differ only by embodiment id / grasp frame / motion_bounds.
Reconcile if anything else appears.

- [ ] **Step 14: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `rfl-core` 79 (was 77, +2); new `snap_engage` golden binary 5 tests; existing
goldens byte-identical. `validate.py` PASS.

- [ ] **Step 15: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-core/src/skill_isa.rs crates/rfl-core/src/translation.rs examples/03-screw-fasten/skill-snap.yaml examples/03-screw-fasten/embodiments/allegro.yaml examples/03-screw-fasten/embodiments/leap.yaml examples/03-screw-fasten/embodiments/pneumatic-6f.yaml crates/rfl-conformance/tests/snap_engage.rs crates/rfl-conformance/tests/snapshots && \
git commit -m "feat(core): add force.snap_engage primitive (detent + confirm_held)

ForceSnapEngage struct + Primitive variant + capability gate + lowering:
reuse press_button's detent (force_profile.actuation) for snap-in + emit a
confirm_held marker + the force_budget (ForceTrajectory leg); engage along
engage_direction in the grasp frame (like force.screw). snap_disengage
reverse path / effort_rise / mate_feature geometry deferred. examples/03
skill-snap.yaml + the capability on 3 descriptors + a byte-deterministic golden.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head -15
```
Verify: sync `0 0`; only the listed files.

---

### Task 2: conformance — `check_engagement` + the `held_confirmed` echo + `SnapEngageDriver`

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (verdict echo; envelope_class_for; check_engagement;
  SnapEngageDriver; lib unit test)
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (4 tests + imports)

- [ ] **Step 1: Write the failing integration tests** — append to `envelope_conformance.rs`
(reuse `screw_dir()` + `suffix_of()`):

```rust
// --- force.snap_engage snap-in + engagement-confirmation (spec/01 § 6.10) --------------------

#[test]
fn nominal_snap_engage_passes_all_legs() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-snap.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "snap_engage");
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
    assert_eq!(check_actuation(goal, report), CheckOutcome::Pass); // reused detent leg
    assert_eq!(check_engagement(goal, report), CheckOutcome::Pass); // the new confirm_held leg
}

#[test]
fn snap_engage_no_snap_is_honest() {
    let dir = screw_dir();
    let pairs = drive(
        SnapEngageDriver::new(SnapEngageResponse::NoSnap),
        &dir.join("skill-snap.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // no snap fired, force held: not a success; the checks do not falsely fail it.
    assert!(!matches!(report.status.outcome, rfl_core::driver::Outcome::Succeeded));
    assert_eq!(check_engagement(goal, report), CheckOutcome::Pass);
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
}

#[test]
fn snap_engage_unconfirmed_hold_fails() {
    let dir = screw_dir();
    let pairs = drive(
        SnapEngageDriver::new(SnapEngageResponse::ClaimsHeld),
        &dir.join("skill-snap.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // claims success (snap detected) but the connection was never confirmed held -> the bite.
    assert!(matches!(check_engagement(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn snap_engage_over_force_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverForce),
        &dir.join("skill-snap.yaml"),
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

Update the `use rfl_conformance::{...}` block to add `check_engagement, SnapEngageDriver,
SnapEngageResponse`.

- [ ] **Step 2: Run to verify it fails (unresolved imports)**

Run: `cargo test -p rfl-conformance --test envelope_conformance snap_engage 2>&1 | grep -E "error\[|unresolved|test result" | head`
Expected: compile error — `check_engagement` / `SnapEngageDriver` / `SnapEngageResponse` not found.

- [ ] **Step 3: Echo `held_confirmed` in the ReferenceDriver verdict** — in `lib.rs`
`ReferenceDriver::execute`, replace the `let status = Status {` ... verdict block. Insert before
`let status = Status {`:

```rust
        // Engagement-confirmation evidence (force.snap_engage confirm_held, spec/01 § 6.10): the
        // nominal driver's release-test confirmed the bistable connection holds. AUD1 evidence.
        let mut evidence = vec!["nominal reference-driver execution".to_string()];
        if ca
            .safety_envelope
            .force_profile
            .as_ref()
            .and_then(|fp| fp.get("confirm_held"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        {
            evidence.push("held_confirmed".to_string());
        }
```
and in the `Status { ... verdict: Some(Verdict { ... }) ... }`, replace
`evidence: vec!["nominal reference-driver execution".to_string()],` with `evidence,`.

- [ ] **Step 4: Add `snap_engage` to `envelope_class_for`** — in `lib.rs`, the ForceTrajectory arm:

```rust
        "insert_fit" | "screw" | "unscrew" | "press_button" | "wipe" | "snap_engage" => {
            Some(EnvelopeClass::ForceTrajectory)
        }
```

- [ ] **Step 5: Add `check_engagement`** — in `lib.rs`, after `check_actuation`:

```rust
/// Verify the § 6.10 engagement-confirmation for `force.snap_engage`: a snap that succeeded
/// under a `confirm_held` contract MUST carry the held-confirmation evidence (the release-test
/// confirmed the bistable connection holds). Vacuous unless the action declares `confirm_held`.
/// Makes the AUD1 `verdict.evidence` channel falsifiable — a `false_engagement` claiming success
/// (snap detected but the connection does not hold) fails.
#[must_use]
pub fn check_engagement(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    let confirm = goal
        .canonical_action
        .safety_envelope
        .force_profile
        .as_ref()
        .and_then(|fp| fp.get("confirm_held"))
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    if !confirm {
        return CheckOutcome::Pass; // no confirm_held contract -> vacuous
    }
    if matches!(report.status.outcome, Outcome::Succeeded) {
        let held = report
            .status
            .verdict
            .as_ref()
            .is_some_and(|v| v.evidence.iter().any(|e| e == "held_confirmed"));
        if !held {
            return CheckOutcome::Fail(
                "snap_engage claimed success without confirming the connection holds (confirm_held)"
                    .to_string(),
            );
        }
    }
    CheckOutcome::Pass
}
```

- [ ] **Step 6: Add `SnapEngageResponse` + `SnapEngageDriver`** — in `lib.rs`, after the
`PressButtonDriver` impl:

```rust
/// How a driver reports a `force.snap_engage` (`spec/01` § 6.10). `Engages` is the nominal
/// detent + held-confirmed + success; `NoSnap` is the conformant `no_snap` (force rise, no
/// detent); `ClaimsHeld` is adversarial (success + detent but the hold was never confirmed).
#[derive(Debug, Clone, Copy)]
pub enum SnapEngageResponse {
    /// Conformant: the snap-in detent fired, the hold was confirmed, and the engagement succeeded.
    Engages,
    /// Conformant: no snap fired -> no_snap reported honestly (force kept within budget).
    NoSnap,
    /// Adversarial: claim success though the connection was never confirmed held.
    ClaimsHeld,
}

/// The `force.snap_engage` bench: models a driver's engagement outcome. Reuses the nominal
/// `ReferenceDriver` (which echoes the detent + held-confirmed evidence) and mutates it per
/// `response`. Non-snap actions pass through unchanged.
#[derive(Debug)]
pub struct SnapEngageDriver {
    inner: ReferenceDriver,
    response: SnapEngageResponse,
}

impl SnapEngageDriver {
    /// A snap-engage driver with the given engagement outcome.
    #[must_use]
    pub fn new(response: SnapEngageResponse) -> Self {
        SnapEngageDriver { inner: ReferenceDriver::default(), response }
    }
}

impl Driver for SnapEngageDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        match self.response {
            SnapEngageResponse::Engages => {} // nominal: detent + held_confirmed + Succeeded
            SnapEngageResponse::NoSnap => {
                for t in &mut report.telemetry {
                    t.events.clear(); // no snap detent fired
                }
                report.status.outcome = Outcome::Failed;
                report.status.failure_class = Some("blocked".to_string());
                report.status.failure_detail = Some("no_snap".to_string());
                if let Some(v) = report.status.verdict.as_mut() {
                    v.value = false;
                    v.evidence.retain(|e| e != "held_confirmed"); // nothing engaged to confirm
                }
            }
            SnapEngageResponse::ClaimsHeld => {
                // snap detected (detent kept), claims success, but the hold was never confirmed.
                if let Some(v) = report.status.verdict.as_mut() {
                    v.evidence.retain(|e| e != "held_confirmed");
                }
            }
        }
        report
    }
}
```

- [ ] **Step 7: Add the lib unit test** — in `lib.rs` tests module, after
`contact_band_rejects_loss_of_contact_and_over_force`:

```rust
    #[test]
    fn check_engagement_requires_held_confirmation_on_success() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode,
        };
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        let action = CanonicalAction {
            target_frame: "grasp".into(),
            target_pose: PoseExpr::Ref { r#ref: "clip".into() },
            force_budget: Some(rfl_core::quantity::Quantity("25 N".into())),
            timing: TimingHints {
                nominal_duration: None,
                timing_mode: TimingMode::TimeScalable,
                stop_at_goal: true,
            },
            tactile_target: None,
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: Some(serde_json::json!({ "actuation": "detent", "confirm_held": true })),
                station_keeping: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
        };
        let goal = ExecuteGoal::wrap("s/e/0001-snap_engage".to_string(), action);
        let report = |outcome: Outcome, evidence: Vec<String>| DriverReport {
            telemetry: vec![],
            status: Status {
                message: "status",
                action_id: "s/e/0001-snap_engage".to_string(),
                outcome,
                verdict: Some(Verdict { value: true, confidence: 1.0, evidence }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
            },
        };
        // Succeeded + held_confirmed -> Pass.
        assert_eq!(
            check_engagement(&goal, &report(Outcome::Succeeded, vec!["held_confirmed".to_string()])),
            CheckOutcome::Pass
        );
        // Succeeded + no held_confirmed -> Fail.
        assert!(matches!(
            check_engagement(&goal, &report(Outcome::Succeeded, vec![])),
            CheckOutcome::Fail(_)
        ));
        // Not succeeded -> vacuously Pass.
        assert_eq!(
            check_engagement(&goal, &report(Outcome::Failed, vec![])),
            CheckOutcome::Pass
        );
    }
```

- [ ] **Step 8: Run the new tests (green)**

Run: `cargo test -p rfl-conformance snap_engage 2>&1 | grep -E "test result|snap_engage"` then
`cargo test -p rfl-conformance check_engagement 2>&1 | grep -E "test result|check_engagement"`.
Expected: the 4 integration tests + the lib unit test PASS.

- [ ] **Step 9: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `rfl-conformance` lib 13 (was 12, +1), `envelope_conformance` 34 (was 30, +4).
Existing tests unaffected (check_engagement vacuous without confirm_held; the held_confirmed
evidence is only added when force_profile.confirm_held is set, so other actions' reports are
byte-identical → driver_protocol goldens unchanged).

- [ ] **Step 10: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs && \
git commit -m "feat(conformance): verify force.snap_engage engagement-confirmation

envelope_class_for(snap_engage)=>ForceTrajectory (reused) + ReferenceDriver
pushes held_confirmed into verdict.evidence when confirm_held is set +
check_engagement (Succeeded => held_confirmed present, vacuous otherwise).
SnapEngageDriver{Engages,NoSnap,ClaimsHeld}: ClaimsHeld (success without the
confirmation) fails check_engagement; the reused check_actuation bites a
no-detent success and OverForce the budget. First reader of verdict.evidence.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `lib.rs` + `envelope_conformance.rs`.

---

### Task 3: README status line

**Files:**
- Modify: `README.md` (~line 84)

- [ ] **Step 1: Extend the README status line** — add `force.snap_engage` to the examples /
force primitives listing and note it opens the reversibility classification (REV2 semi-reversible):
it reuses press_button's detent for snap-in and adds `confirm_held` engagement-confirmation (a
`false_engagement` claiming success fails), the first reader of the AUD1 `verdict.evidence`
channel. Match the surrounding phrasing; keep the existing sentence structure. (spec/05 REV1/REV2
already state the obligation, so no spec change.)

- [ ] **Step 2: Verify nothing regressed**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: unchanged green.

- [ ] **Step 3: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add README.md && \
git commit -m "docs(readme): note force.snap_engage engagement-confirmation (REV2)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `README.md`.

---

## Post-increment

- Final full-suite read: expected `rfl-core` 79, `rfl-conformance` lib 13, `driver_protocol` 9,
  `envelope_conformance` 34, new `snap_engage` 5, others unchanged.
- Update `project_rfl.md` "Implementation track" (18th increment, real hashes + test deltas) +
  the `MEMORY.md` RFL line. Move deferred items (effort_rise snap_signature, snap_disengage
  reverse path, held-part reaction, mate_feature geometry) into the parked list.

## Self-review (spec coverage)

- Reversibility REV2 entry (design § 1, § 3) → Task 2 (check_engagement + confirm_held). ✓
- Snap-in reuses check_actuation (design § 1, § 3) → Task 1 (force_profile.actuation=detent) +
  Task 2 (nominal test asserts check_actuation Pass). ✓
- ForceSnapEngage primitive + lowering + gate (design § 4) → Task 1. ✓
- held_confirmed via verdict.evidence + ClaimsHeld non-vacuity (design § 3, § 5) → Task 2. ✓
- README (design § 6) → Task 3. ✓
- No schema change (design § 2) → confirmed: no task touches schemas; descriptors gain a
  capability token (data). ✓
- Type consistency: `ForceSnapEngage{mate_feature: Value, engage_direction: Direction,
  force_budget: Quantity, snap_signature: Option<Value>, confirm_held: Option<bool>, compliance}`;
  `force_profile.actuation`/`confirm_held` written by the lowering, read by the ReferenceDriver
  echo + `check_actuation`/`check_engagement`; `held_confirmed` in verdict.evidence written by the
  ReferenceDriver, read by `check_engagement`; `SnapEngageResponse{Engages,NoSnap,ClaimsHeld}`
  used in lib.rs + envelope_conformance.rs; `envelope_class_for("snap_engage")=>ForceTrajectory`. ✓
