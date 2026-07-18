# force.wipe (contact-maintenance band) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement
> this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> **LOCAL-ONLY** — never `git add` this file.

**Goal:** Implement `force.wipe` (v0 = the two-sided normal-force band) to add the force *lower*
bound (contact-maintenance / loss-of-contact) the force-trajectory class lacks today.

**Architecture:** Add the `ForceWipe` primitive (struct + variant + lower + gate); lower a
`force_profile = {normal_force, normal_force_tolerance}` band; strengthen the `ForceTrajectory`
`check_envelope` arm with a vacuous-when-absent `contact_band_violation` leg (force ∈ [S−T,
S+T]); add `Fault::LoseContact` (force → 0). No schema change. Design:
`docs/design/2026-06-01-force-wipe-design.md`.

**Tech Stack:** Rust (rfl-core / rfl-conformance), insta goldens, boon, `schemas/validate.py`.

**Discipline (every task):** TDD red→green. After green, run `cargo test` (workspace, NOT `-p`)
+ `uv run --with jsonschema --with pyyaml python schemas/validate.py` in a batch SEPARATE from
the commit. Before each push: `git fetch -q && git merge-base --is-ancestor origin/main HEAD`,
`git add <explicit paths>`, commit, `git push origin main`, verify
`git rev-list --left-right --count HEAD...origin/main` == `0 0` and `git show --stat HEAD`.
Stop-gate: reconcile and explain any golden diff vs the prediction.

---

### Task 1: the `ForceWipe` primitive (struct + variant + lower + gate) + example + golden

**Files:**
- Modify: `crates/rfl-core/src/skill_isa.rs` (Primitive enum; new struct near ForcePressButton)
- Modify: `crates/rfl-core/src/translation.rs` (use import; check_capability; lower dispatch; new lower fn; tests)
- Create: `examples/03-screw-fasten/skill-wipe.yaml`
- Modify: `examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml` (skills list)
- Create: `crates/rfl-conformance/tests/wipe.rs`

- [ ] **Step 1: Write the failing capability-gate test** — in `translation.rs` tests module,
near `press_button_capability_absent_when_not_declared`:

```rust
    const WIPE_SKILL: &str = "skill: t\nbody:\n  sequence:\n    - force.wipe: { surface: panel, wipe_path: stroke_path, normal_force: 5 N, normal_force_tolerance: 1 N }\n";

    #[test]
    fn wipe_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(WIPE_SKILL).unwrap();
        let emb = load("allegro").1; // cable allegro lacks force.wipe
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: force.wipe"), "got {err}");
    }
```

- [ ] **Step 2: Run to verify it fails (unknown primitive)**

Run: `cargo test -p rfl-core wipe_capability_absent 2>&1 | grep -E "error\[|panicked|test result|FAILED" | head`
Expected: FAIL — `force.wipe` deserializes to no `Primitive` variant, so `Skill::parse_yaml`
fails and `.unwrap()` panics.

- [ ] **Step 3: Add the `ForceWipe` struct** — in `skill_isa.rs`, after the `ForcePressButton`
struct:

```rust
/// `force.wipe` parameters (v0 subset of `$defs/ForceWipeParams`, § 6.8). `surface` +
/// `wipe_path` + `normal_force` are required. v0 lowers the normal-force band (the
/// contact-maintenance leg); `wipe_path` is carried symbolic (the tangential position-tracking
/// leg is deferred). `normal_force_tolerance` auto defers (no band emitted).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceWipe {
    /// The surface / frame to wipe over (v0: a frame ref).
    pub surface: FrameRef,
    /// `Trajectory` — the tangential path (carried symbolic in v0; position-tracking deferred).
    pub wipe_path: serde_yaml::Value,
    /// The contact force to maintain normal to the surface (the band setpoint).
    pub normal_force: Quantity,
    /// Allowed deviation of the maintained normal force (`Force | auto`; explicit → the band).
    #[serde(default)]
    pub normal_force_tolerance: Option<serde_yaml::Value>,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
}
```

- [ ] **Step 4: Add the Primitive variant** — in `skill_isa.rs` `enum Primitive`, after the
`ForcePressButton` variant:

```rust
    /// `force.wipe`.
    #[serde(rename = "force.wipe")]
    ForceWipe(ForceWipe),
```

- [ ] **Step 5: Wire the import + capability gate + lower dispatch** — in `translation.rs`:

In the `use crate::skill_isa::{...}` block, add `ForceWipe` to the import list.

In `check_capability`, after the `Primitive::ForcePressButton(_)` arm:

```rust
        Primitive::ForceWipe(_) => "force.wipe",
```

In the `lower` dispatch `match prim`, after the `Primitive::ForcePressButton(p)` arm:

```rust
        Primitive::ForceWipe(p) => (lower_force_wipe(p, e), "wipe"),
```

- [ ] **Step 6: Add the lowering fn** — in `translation.rs`, after `lower_force_press_button`
(before `actuation_is_detent`):

```rust
/// Lower `force.wipe` (`spec/01` § 6.8): a hybrid force/position interval invariant. v0 emits
/// the two-sided normal-force band into `force_profile` (the contact-maintenance leg the
/// `ForceTrajectory` arm samples) when `normal_force_tolerance` is an explicit Force; an `auto`
/// tolerance emits no band (deferred). `wipe_path` is carried symbolic; the tangential
/// position-tracking + contour-following leg is deferred. `force_budget` is None — the band
/// owns both the upper and lower bounds.
fn lower_force_wipe(p: &ForceWipe, e: &Embodiment) -> CanonicalAction {
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    if let Some(tol) = p
        .normal_force_tolerance
        .as_ref()
        .and_then(serde_yaml::Value::as_str)
        .filter(|s| *s != "auto")
    {
        env.force_profile = Some(serde_json::json!({
            "normal_force": p.normal_force.0.clone(),
            "normal_force_tolerance": tol,
        }));
    }
    CanonicalAction {
        target_frame: e.control_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.surface.clone() },
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::TimeScalable,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: env,
    }
}
```

- [ ] **Step 7: Run the capability test (green) — variant now exists**

Run: `cargo test -p rfl-core wipe_capability_absent 2>&1 | grep -E "test result"`
Expected: PASS.

- [ ] **Step 8: Create the example skill** — `examples/03-screw-fasten/skill-wipe.yaml`:

```yaml
# Example 03 — Surface wipe (force.wipe variant)
# Maintain a controlled normal contact force against a surface while moving tangentially
# along a path. Exercises the hybrid force/position interval invariant (spec/05 § 6.8): the
# normal force must stay within a band (normal_force ± tolerance) throughout — loss of
# contact (force -> 0) is a violation, not just excess force.
skill: surface-wipe
description: >
  Wipe a controlled normal force across a panel along a stroke path, holding the contact
  force within a band (no loss of contact, no excess).

body:
  sequence:

    - force.wipe:
        surface: panel
        wipe_path: stroke_path
        normal_force: 5 N
        normal_force_tolerance: 1 N
        compliance: active
```

- [ ] **Step 9: Declare the capability on the 3 descriptors** — in each of
`examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml`, add `force.wipe` to the
`skills:` array (read each line first; insert before `sense.locate`). For allegro the line
becomes:

```yaml
    skills: [grasp.pinch, grasp.lateral, grasp.envelope, transport, force.insert_fit, force.screw, force.unscrew, force.press_button, force.wipe, sense.locate]
```

- [ ] **Step 10: Add the positive lowering test** — in `translation.rs` tests module:

```rust
    #[test]
    fn wipe_lowers_normal_force_band() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill-wipe.yaml")).unwrap()).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .unwrap();
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["wipe"]);
        let a = &out.actions[0];
        assert!(a.force_budget.is_none()); // the band owns both bounds
        let fp = serde_json::to_string(&a.safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"normal_force\":\"5 N\""), "got {fp}");
        assert!(fp.contains("\"normal_force_tolerance\":\"1 N\""), "got {fp}");
    }
```

- [ ] **Step 11: Run both translation tests (green)**

Run: `cargo test -p rfl-core wipe 2>&1 | grep -E "test result|wipe"`
Expected: both PASS.

- [ ] **Step 12: Create the golden test** — `crates/rfl-conformance/tests/wipe.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for force.wipe: the retarget output is byte-deterministic, matches
//! a committed golden, and every line is a valid driver-interface execute message. (The
//! contact-maintenance band verification is in envelope_conformance.rs.)

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-wipe.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_wipe_allegro() {
    insta::assert_snapshot!("wipe_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_wipe_leap() {
    insta::assert_snapshot!("wipe_leap", jsonl_for("leap"));
}

#[test]
fn golden_wipe_pneumatic() {
    insta::assert_snapshot!("wipe_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn wipe_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn wipe_every_line_is_a_valid_execute_message() {
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

Run: `INSTA_UPDATE=always cargo test -p rfl-conformance --test wipe 2>&1 | grep -E "test result|error\["`
Then: `cat crates/rfl-conformance/tests/snapshots/wipe__wipe_allegro.snap`
Expected (STOP-GATE): one execute line, `action_id` `surface-wipe/wonik-allegro-v4/0001-wipe`,
`target_frame":"tcp_index"`, `target_pose":{"ref":"panel"}`, NO `force_budget` (None), NO
`monitors` (empty), `force_profile":{"normal_force":"5 N","normal_force_tolerance":"1 N"}`
(keys alphabetical: `normal_force` before `normal_force_tolerance`), `compliance":"active"`,
`stop_time":"0.1 s"`. Three stems differ only by embodiment id / frame / motion_bounds.
Reconcile if anything else appears.

- [ ] **Step 14: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `rfl-core` 77 (was 75, +2); new `wipe` golden binary 5 tests; existing
goldens byte-identical. `validate.py` PASS.

- [ ] **Step 15: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-core/src/skill_isa.rs crates/rfl-core/src/translation.rs examples/03-screw-fasten/skill-wipe.yaml examples/03-screw-fasten/embodiments/allegro.yaml examples/03-screw-fasten/embodiments/leap.yaml examples/03-screw-fasten/embodiments/pneumatic-6f.yaml crates/rfl-conformance/tests/wipe.rs crates/rfl-conformance/tests/snapshots && \
git commit -m "feat(core): add force.wipe primitive (normal-force band)

ForceWipe struct + Primitive variant + capability gate + lowering: emit a
two-sided normal-force band (normal_force +/- tolerance) into force_profile
when the tolerance is explicit; force_budget is None (the band owns both
bounds). wipe_path carried symbolic; position-tracking / auto-tolerance
deferred. examples/03 skill-wipe.yaml + the capability on 3 descriptors + a
byte-deterministic golden.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head -15
```
Verify: sync `0 0`; only the listed files.

---

### Task 2: conformance — the contact-maintenance band leg + `Fault::LoseContact`

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (wrench echo; envelope_class_for; band helper +
  ForceTrajectory arm; Fault enum + FaultyDriver; lib unit test)
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (3 tests)

- [ ] **Step 1: Write the failing integration tests** — append to `envelope_conformance.rs`
(reuse `screw_dir()` + `suffix_of()`):

```rust
// --- force.wipe contact-maintenance band (spec/01 § 6.8) -------------------------------------

#[test]
fn nominal_wipe_holds_the_contact_band() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-wipe.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "wipe");
    // nominal wrench echoes the 5 N setpoint -> in [4, 6] band.
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
}

#[test]
fn wipe_loss_of_contact_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::LoseContact),
        &dir.join("skill-wipe.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // wrench.force -> 0, below the band lower edge -> the new lower-bound bite.
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn wipe_over_force_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverForce),
        &dir.join("skill-wipe.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // wrench.force -> 999, above the band upper edge.
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}
```

Update the `use rfl_conformance::{...}` block to add `Fault` if not already imported (it is, from
the press_button/hover work — verify and only add if missing).

- [ ] **Step 2: Run to verify it fails (no `Fault::LoseContact`)**

Run: `cargo test -p rfl-conformance --test envelope_conformance wipe 2>&1 | grep -E "error\[|no variant|test result" | head`
Expected: compile error — `Fault` has no variant `LoseContact`.

- [ ] **Step 3: Add `Fault::LoseContact`** — in `lib.rs` `enum Fault`, after `MidIntervalDrop`:

```rust
    /// `wrench.force` driven to zero (loss of contact — violates the force.wipe band's lower
    /// edge; the mirror of `OverForce`).
    LoseContact,
```

- [ ] **Step 4: Add the `FaultyDriver` match arm** — in `lib.rs` `impl Driver for FaultyDriver`,
after the `Fault::MidIntervalDrop` arm:

```rust
            Fault::LoseContact => {
                for t in &mut report.telemetry {
                    if let Some(w) = t.wrench.as_mut() {
                        w.force = [0.0, 0.0, 0.0];
                    }
                }
            }
```

- [ ] **Step 5: Extend the `ReferenceDriver` wrench echo** — in `lib.rs`
`ReferenceDriver::execute`, replace the `force_mag` binding:

```rust
        let force_mag = ca.force_budget.as_ref().and_then(|q| q.parse().map(|(v, _)| v));
```
with:
```rust
        // Echo the commanded force_budget, or — on a wipe with no budget — the normal_force
        // band setpoint (force.wipe), so the ForceTrajectory band leg has an in-band sample.
        let force_mag = ca.force_budget.as_ref().and_then(|q| q.parse().map(|(v, _)| v)).or_else(|| {
            ca.safety_envelope
                .force_profile
                .as_ref()
                .and_then(|fp| fp.get("normal_force"))
                .and_then(serde_json::Value::as_str)
                .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v))
        });
```

- [ ] **Step 6: Add the `contact_band_violation` helper** — in `lib.rs`, after
`station_keeping_violation`:

```rust
/// If the action carries a `normal_force` band (`force.wipe`, `spec/01` § 6.8) and any
/// `wrench` sample's `|force|` falls outside `[normal_force − tol, normal_force + tol]`, the
/// failure reason (below = loss of contact, above = over-force); else `None`. Vacuous when no
/// `normal_force` band (insert_fit / screw / press_button) — the two-sided counterpart of the
/// securing floor, on contact wrench.
fn contact_band_violation(goal: &ExecuteGoal, report: &DriverReport) -> Option<String> {
    let fp = goal.canonical_action.safety_envelope.force_profile.as_ref()?;
    let setpoint = fp
        .get("normal_force")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v))?;
    let tol = fp
        .get("normal_force_tolerance")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v))?;
    let (lo, hi) = (setpoint - tol, setpoint + tol);
    for t in &report.telemetry {
        if let Some(w) = &t.wrench {
            let mag = w.force.iter().map(|x| x * x).sum::<f64>().sqrt();
            if mag < lo {
                return Some(format!("contact lost: |wrench.force| {mag} < {lo} (normal_force {setpoint} − tol {tol})"));
            }
            if mag > hi {
                return Some(format!("over-force: |wrench.force| {mag} > {hi} (normal_force {setpoint} + tol {tol})"));
            }
        }
    }
    None
}
```

- [ ] **Step 7: Add `press_button` / `wipe` to `envelope_class_for` + wire the band leg** —

First, in `envelope_class_for`, add `"wipe"` to the ForceTrajectory arm:
```rust
        "insert_fit" | "screw" | "unscrew" | "press_button" | "wipe" => Some(EnvelopeClass::ForceTrajectory),
```

Then in `check_envelope`, in the `EnvelopeClass::ForceTrajectory` arm, replace the trailing
torque-budget block's closing `CheckOutcome::Pass` with the band leg before it:
```rust
            if let Some(tb) = torque_budget {
                for t in &report.telemetry {
                    if let Some(w) = &t.wrench {
                        let mag = w.torque.iter().map(|x| x * x).sum::<f64>().sqrt();
                        if mag > tb {
                            return CheckOutcome::Fail(format!("|wrench.torque| {mag} > torque budget {tb}"));
                        }
                    }
                }
            }
            // Contact-maintenance band (force.wipe, spec/01 § 6.8): the two-sided lower+upper
            // edge. Vacuous without a normal_force band (insert_fit / screw / press_button).
            if let Some(reason) = contact_band_violation(goal, report) {
                return CheckOutcome::Fail(reason);
            }
            CheckOutcome::Pass
```

- [ ] **Step 8: Add the lib unit test** — in `lib.rs` tests module, after
`check_actuation_requires_a_detent_on_success`:

```rust
    #[test]
    fn contact_band_rejects_loss_of_contact_and_over_force() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode,
        };
        use rfl_core::driver::{Outcome, RealizedPose, Status, Telemetry, Verdict, Wrench};
        let action = CanonicalAction {
            target_frame: "control".into(),
            target_pose: PoseExpr::Ref { r#ref: "panel".into() },
            force_budget: None,
            timing: TimingHints {
                nominal_duration: None,
                timing_mode: TimingMode::TimeScalable,
                stop_at_goal: true,
            },
            tactile_target: None,
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: Some(serde_json::json!({ "normal_force": "5 N", "normal_force_tolerance": "1 N" })),
                station_keeping: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
        };
        let goal = ExecuteGoal::wrap("s/e/0001-wipe".to_string(), action);
        let report = |fz: f64| {
            let t = Telemetry {
                message: "telemetry",
                action_id: "s/e/0001-wipe".to_string(),
                t: 1.0,
                realized_pose: Some(RealizedPose::placeholder()),
                wrench: Some(Wrench { force: [0.0, 0.0, fz], torque: [0.0, 0.0, 0.0] }),
                securing_force: None,
                station_error: None,
                tactile: vec![],
                events: vec![],
                fidelity_tier: None,
            };
            DriverReport {
                telemetry: vec![t],
                status: Status {
                    message: "status",
                    action_id: "s/e/0001-wipe".to_string(),
                    outcome: Outcome::Succeeded,
                    verdict: Some(Verdict { value: true, confidence: 1.0, evidence: vec![] }),
                    fidelity_tier: None,
                    final_pose: Some(RealizedPose::placeholder()),
                    failure_class: None,
                    failure_detail: None,
                },
            }
        };
        // in band (5 N) -> Pass.
        assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, &goal, &report(5.0)), CheckOutcome::Pass);
        // loss of contact (0 N, below 4) -> Fail.
        assert!(matches!(
            check_envelope(EnvelopeClass::ForceTrajectory, &goal, &report(0.0)),
            CheckOutcome::Fail(_)
        ));
        // over-force (9 N, above 6) -> Fail.
        assert!(matches!(
            check_envelope(EnvelopeClass::ForceTrajectory, &goal, &report(9.0)),
            CheckOutcome::Fail(_)
        ));
    }
```

- [ ] **Step 9: Run the new tests (green)**

Run: `cargo test -p rfl-conformance wipe 2>&1 | grep -E "test result|wipe"` then
`cargo test -p rfl-conformance contact_band 2>&1 | grep -E "test result|contact_band"`.
Expected: the 3 integration tests + the lib unit test PASS.

- [ ] **Step 10: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `rfl-conformance` lib 12 (was 11, +1), `envelope_conformance` 30 (was 27, +3).
Existing force goldens / tests unaffected (band leg vacuous without `normal_force`).

- [ ] **Step 11: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs && \
git commit -m "feat(conformance): verify force.wipe contact-maintenance band

envelope_class_for(wipe)=>ForceTrajectory + ReferenceDriver echoes the
normal_force setpoint into wrench + contact_band_violation strengthens the
ForceTrajectory arm with a two-sided band (vacuous without normal_force, so
insert_fit/screw/press_button untouched) + Fault::LoseContact (force->0, the
mirror of OverForce). LoseContact fails the lower edge, OverForce the upper.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `lib.rs` + `envelope_conformance.rs`.

---

### Task 3: README status line

**Files:**
- Modify: `README.md` (~line 84)

- [ ] **Step 1: Extend the README status line** — add `force.wipe` to the examples / force
primitives listing and note its contact-maintenance band (the force class is now two-sided: a
force *lower* bound / loss-of-contact detection, not just `≤ budget`). Match the surrounding
phrasing; keep the existing sentence structure. (spec/05 already states the hybrid invariant, so
no spec change.)

- [ ] **Step 2: Verify nothing regressed**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: unchanged green.

- [ ] **Step 3: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add README.md && \
git commit -m "docs(readme): note force.wipe contact-maintenance band

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `README.md`.

---

## Post-increment

- Final full-suite read: expected `rfl-core` 77, `rfl-conformance` lib 12, `driver_protocol` 9,
  `envelope_conformance` 30, new `wipe` 5, others unchanged.
- Update `project_rfl.md` "Implementation track" (17th increment, real hashes + test deltas) +
  the `MEMORY.md` RFL line. Move deferred items (auto-tolerance, position-tracking /
  contour-following, feed_rate, held-tool reaction; force.scrub can reuse the band) into the
  parked list.

## Self-review (spec coverage)

- The force lower bound / band (design § 1, § 3) → Task 2 (`contact_band_violation` + band leg). ✓
- ForceWipe primitive + lowering + gate (design § 4) → Task 1. ✓
- `Fault::LoseContact` non-vacuity (design § 5) → Task 2. ✓
- README (design § 6) → Task 3. ✓
- No schema change (design § 2) → confirmed: no task touches schemas; descriptors gain a
  capability token (data). ✓
- Type consistency: `ForceWipe{surface: FrameRef, wipe_path: Value, normal_force: Quantity,
  normal_force_tolerance: Option<Value>, compliance}`; `force_profile.normal_force` /
  `normal_force_tolerance` written by the lowering, echoed by the ReferenceDriver, and read by
  `contact_band_violation`; `Fault::LoseContact` used in lib.rs + envelope_conformance.rs;
  `envelope_class_for("wipe")=>ForceTrajectory`. ✓
