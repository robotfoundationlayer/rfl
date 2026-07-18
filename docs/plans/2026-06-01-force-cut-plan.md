# force.cut (REV3 irreversible) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement
> this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> **LOCAL-ONLY** — never `git add` this file.

**Goal:** Implement `force.cut` (REV3 irreversible), opening the irreversible-operation safety
class via the precise partial-state-on-interruption leg (`check_irreversible`), reusing
`ForceTrajectory` (shear budget) + `Fault::OverForce`.

**Architecture:** Add the `ForceCut` primitive (struct + variant + lower + gate); lower
`force_profile = {irreversible:true}` + `force_budget = shear_force_budget`; `envelope_class_for
=> ForceTrajectory`; add `check_irreversible` (interrupted ⟹ `verdict.evidence` has a
`partial_cut` marker) + a `CutDriver`. No schema change; the partial-state rides the AUD1
`verdict.evidence` channel. Design: `docs/design/2026-06-01-force-cut-design.md`.

**Tech Stack:** Rust (rfl-core / rfl-conformance), insta goldens, boon, `schemas/validate.py`.

**Discipline (every task):** TDD red→green. After green, run `cargo test` (workspace, NOT `-p`)
+ `uv run --with jsonschema --with pyyaml python schemas/validate.py` in a batch SEPARATE from
the commit. Before each push: `git fetch -q && git merge-base --is-ancestor origin/main HEAD`,
`git add <explicit paths>`, commit, `git push origin main`, verify sync `0 0` + `git show --stat`.
Stop-gate: reconcile and explain any golden diff vs the prediction.

---

### Task 1: the `ForceCut` primitive (struct + variant + lower + gate) + example + golden

**Files:**
- Modify: `crates/rfl-core/src/skill_isa.rs` (Primitive enum; new struct near ForceSnapEngage)
- Modify: `crates/rfl-core/src/translation.rs` (use import; check_capability; lower dispatch; new lower fn; tests)
- Create: `examples/03-screw-fasten/skill-cut.yaml`
- Modify: `examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml` (skills list)
- Create: `crates/rfl-conformance/tests/cut.rs`

- [ ] **Step 1: Write the failing capability-gate test** — in `translation.rs` tests module,
near `snap_engage_capability_absent_when_not_declared`:

```rust
    const CUT_SKILL: &str = "skill: t\nbody:\n  sequence:\n    - force.cut: { cut_path: seam_path, shear_force_budget: 30 N, completion: separation }\n";

    #[test]
    fn cut_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(CUT_SKILL).unwrap();
        let emb = load("allegro").1; // cable allegro lacks force.cut
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: force.cut"), "got {err}");
    }
```

- [ ] **Step 2: Run to verify it fails (unknown primitive)**

Run: `cargo test -p rfl-core cut_capability_absent 2>&1 | grep -E "error\[|panicked|test result|FAILED" | head`
Expected: FAIL — `force.cut` deserializes to no `Primitive` variant; `Skill::parse_yaml` fails;
`.unwrap()` panics.

- [ ] **Step 3: Add the `ForceCut` struct** — in `skill_isa.rs`, after the `ForceSnapEngage`
struct:

```rust
/// `force.cut` parameters (v0 subset of `$defs/ForceCutParams`, § 6.7). `cut_path` +
/// `shear_force_budget` + `completion` are required. v0 lowers the shear budget (the
/// force-trajectory leg) + an irreversible marker; `cut_path` is carried opaque (the
/// path-bounding leg is deferred); on_separation / tool_safety are deferred.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceCut {
    /// `Trajectory` — the path along which to cut (carried opaque in v0; path-bounding deferred).
    pub cut_path: serde_yaml::Value,
    /// Max shear force (tool-damage / over-cut / kickback limit) — the force-trajectory bound.
    pub shear_force_budget: Quantity,
    /// `CutStop` completion (path_complete / separation / depth), lowered into a Monitor.
    pub completion: serde_yaml::Value,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
}
```

- [ ] **Step 4: Add the Primitive variant** — in `skill_isa.rs` `enum Primitive`, after the
`ForceSnapEngage` variant:

```rust
    /// `force.cut`.
    #[serde(rename = "force.cut")]
    ForceCut(ForceCut),
```

- [ ] **Step 5: Wire the import + capability gate + lower dispatch** — in `translation.rs`:

In the `use crate::skill_isa::{...}` block, add `ForceCut` to the import list.

In `check_capability`, after the `Primitive::ForceSnapEngage(_)` arm:

```rust
        Primitive::ForceCut(_) => "force.cut",
```

In the `lower` dispatch `match prim`, after the `Primitive::ForceSnapEngage(p)` arm:

```rust
        Primitive::ForceCut(p) => (lower_force_cut(p, e), "cut"),
```

- [ ] **Step 6: Add the lowering fn** — in `translation.rs`, after `lower_force_snap_engage`
(before `actuation_is_detent`):

```rust
/// Lower `force.cut` (`spec/01` § 6.7): an irreversible tool-mediated cut bounded by a force
/// trajectory (shear force ≤ `shear_force_budget`). v0 emits the shear budget (the ForceTrajectory
/// leg) + an `irreversible` marker (the partial-state-on-interruption check reads it); the
/// `completion` lowers into a Monitor and `cut_path` is carried opaque (path-bounding deferred).
/// Tool-mediated, so the held cutting tool's grasp frame is the controlled frame (like screw).
/// The tool_safety regime, path-bounding, and on_separation are deferred.
fn lower_force_cut(p: &ForceCut, e: &Embodiment) -> CanonicalAction {
    let monitors = vec![Monitor { stop_condition: yaml_to_json(&p.completion) }];
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    env.force_profile = Some(serde_json::json!({ "irreversible": true }));
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::FrameRelative {
            frame: "task".to_string(),
            offset: yaml_to_json(&p.cut_path),
        },
        force_budget: Some(p.shear_force_budget.clone()),
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

Run: `cargo test -p rfl-core cut_capability_absent 2>&1 | grep -E "test result"`
Expected: PASS.

- [ ] **Step 8: Create the example skill** — `examples/03-screw-fasten/skill-cut.yaml`:

```yaml
# Example 03 — Cut (force.cut variant)
# Separate material along a bounded cut path with a held cutting tool, within a shear-force
# budget, until separation is detected. Exercises the irreversible-operation safety class
# (spec/05 REV3): a cut cannot be undone, so an interrupted cut must report the precise
# partial state (how far it progressed), never a binary success / failure.
skill: surface-cut
description: >
  Cut along a seam with a held tool within a shear-force budget until separation; an
  interrupted cut reports how far it progressed.

body:
  sequence:

    - force.cut:
        cut_path: seam_path
        shear_force_budget: 30 N
        completion: separation
        compliance: active
```

- [ ] **Step 9: Declare the capability on the 3 descriptors** — in each of
`examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml`, add `force.cut` to the
`skills:` array (read each line first; insert before `sense.locate`). For allegro the line
becomes:

```yaml
    skills: [grasp.pinch, grasp.lateral, grasp.envelope, transport, force.insert_fit, force.screw, force.unscrew, force.press_button, force.wipe, force.snap_engage, force.cut, sense.locate]
```

- [ ] **Step 10: Add the positive lowering test** — in `translation.rs` tests module:

```rust
    #[test]
    fn cut_lowers_irreversible_and_shear_budget() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill-cut.yaml")).unwrap()).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .unwrap();
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["cut"]);
        let a = &out.actions[0];
        assert_eq!(a.force_budget.as_ref().map(|q| q.0.as_str()), Some("30 N"));
        let fp = serde_json::to_string(&a.safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"irreversible\":true"), "got {fp}");
    }
```

- [ ] **Step 11: Run both translation tests (green)**

Run: `cargo test -p rfl-core "cut" 2>&1 | grep -E "test result|cut_"`
Expected: both `cut_capability_absent_when_not_declared` and `cut_lowers_irreversible_and_shear_budget` PASS.

- [ ] **Step 12: Create the golden test** — `crates/rfl-conformance/tests/cut.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for force.cut: the retarget output is byte-deterministic, matches a
//! committed golden, and every line is a valid driver-interface execute message. (The
//! irreversibility / partial-state verification is in envelope_conformance.rs.)

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-cut.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_cut_allegro() {
    insta::assert_snapshot!("cut_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_cut_leap() {
    insta::assert_snapshot!("cut_leap", jsonl_for("leap"));
}

#[test]
fn golden_cut_pneumatic() {
    insta::assert_snapshot!("cut_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn cut_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn cut_every_line_is_a_valid_execute_message() {
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

Run: `INSTA_UPDATE=always cargo test -p rfl-conformance --test cut 2>&1 | grep -E "test result|error\["`
Then: `cat crates/rfl-conformance/tests/snapshots/cut__cut_allegro.snap`
Expected (STOP-GATE): one execute line, `action_id` `surface-cut/wonik-allegro-v4/0001-cut`,
`target_frame":"tcp_thumb"` (grasp frame, tool-mediated), `target_pose":{"frame":"task","offset":"seam_path"}`
(cut_path carried opaque), `force_budget":"30 N"`, `monitors":[{"stop_condition":"separation"}]`,
`force_profile":{"irreversible":true}`, `compliance":"active"`, `stop_time":"0.1 s"`. Three stems
differ only by embodiment id / grasp frame / motion_bounds. Reconcile if anything else appears.

- [ ] **Step 14: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `rfl-core` 81 (was 79, +2); new `cut` golden binary 5 tests; existing goldens
byte-identical. `validate.py` PASS.

- [ ] **Step 15: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-core/src/skill_isa.rs crates/rfl-core/src/translation.rs examples/03-screw-fasten/skill-cut.yaml examples/03-screw-fasten/embodiments/allegro.yaml examples/03-screw-fasten/embodiments/leap.yaml examples/03-screw-fasten/embodiments/pneumatic-6f.yaml crates/rfl-conformance/tests/cut.rs crates/rfl-conformance/tests/snapshots && \
git commit -m "feat(core): add force.cut primitive (irreversible marker + shear budget)

ForceCut struct + Primitive variant + capability gate + lowering: emit the
shear_force_budget (ForceTrajectory leg) + an irreversible marker in
force_profile; completion -> Monitor; cut_path carried opaque (path-bounding
deferred); tool-mediated grasp frame (like screw). tool_safety regime /
on_separation deferred. examples/03 skill-cut.yaml + the capability on 3
descriptors + a byte-deterministic golden.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head -15
```
Verify: sync `0 0`; only the listed files.

---

### Task 2: conformance — `check_irreversible` + `CutDriver`

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (envelope_class_for; check_irreversible; CutDriver; lib unit test)
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (4 tests + imports)

- [ ] **Step 1: Write the failing integration tests** — append to `envelope_conformance.rs`
(reuse `screw_dir()` + `suffix_of()`):

```rust
// --- force.cut irreversibility / partial-state on interruption (spec/01 § 6.7) --------------

#[test]
fn nominal_cut_passes() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-cut.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "cut");
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
    assert_eq!(check_irreversible(goal, report), CheckOutcome::Pass); // Succeeded -> vacuous
}

#[test]
fn cut_partial_state_reported_is_honest() {
    let dir = screw_dir();
    let pairs = drive(
        CutDriver::new(CutResponse::PartialReported),
        &dir.join("skill-cut.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // interrupted, but the precise partial state is reported -> honest.
    assert!(!matches!(report.status.outcome, rfl_core::driver::Outcome::Succeeded));
    assert_eq!(check_irreversible(goal, report), CheckOutcome::Pass);
}

#[test]
fn cut_binary_halt_fails() {
    let dir = screw_dir();
    let pairs = drive(
        CutDriver::new(CutResponse::BinaryHalt),
        &dir.join("skill-cut.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // interrupted irreversible op reported a binary failure without the partial state -> the bite.
    assert!(matches!(check_irreversible(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn cut_over_force_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverForce),
        &dir.join("skill-cut.yaml"),
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

Update the `use rfl_conformance::{...}` block to add `check_irreversible, CutDriver, CutResponse`.

- [ ] **Step 2: Run to verify it fails (unresolved imports)**

Run: `cargo test -p rfl-conformance --test envelope_conformance cut 2>&1 | grep -E "error\[|unresolved|test result" | head`
Expected: compile error — `check_irreversible` / `CutDriver` / `CutResponse` not found.

- [ ] **Step 3: Add `cut` to `envelope_class_for`** — in `lib.rs`, the ForceTrajectory arm:

```rust
        "insert_fit" | "screw" | "unscrew" | "press_button" | "wipe" | "snap_engage" | "cut" => {
            Some(EnvelopeClass::ForceTrajectory)
        }
```

- [ ] **Step 4: Add `check_irreversible`** — in `lib.rs`, after `check_engagement`:

```rust
/// Verify the § 175 / § 6.7 irreversibility obligation for `force.cut`: an irreversible op that
/// is INTERRUPTED (outcome != Succeeded) MUST report the precise partial state (how far it
/// progressed) in `verdict.evidence`, never a bare binary failure. Vacuous unless the action
/// declares `irreversible`; a completed (Succeeded) cut is vacuous (nothing partial to report).
/// The mirror of `check_actuation`/`check_engagement`: those bite the success path, this bites
/// the failure path.
#[must_use]
pub fn check_irreversible(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    let irreversible = goal
        .canonical_action
        .safety_envelope
        .force_profile
        .as_ref()
        .and_then(|fp| fp.get("irreversible"))
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    if !irreversible {
        return CheckOutcome::Pass; // not an irreversible op -> vacuous
    }
    if !matches!(report.status.outcome, Outcome::Succeeded) {
        let reported = report
            .status
            .verdict
            .as_ref()
            .is_some_and(|v| v.evidence.iter().any(|e| e.starts_with("partial_cut")));
        if !reported {
            return CheckOutcome::Fail(
                "interrupted irreversible cut reported a binary failure without the precise partial state"
                    .to_string(),
            );
        }
    }
    CheckOutcome::Pass
}
```

- [ ] **Step 5: Add `CutResponse` + `CutDriver`** — in `lib.rs`, after the `SnapEngageDriver`
impl:

```rust
/// How a driver reports a `force.cut` (`spec/01` § 6.7). `Completes` is the nominal success;
/// `PartialReported` is the conformant interruption (the precise partial state is reported);
/// `BinaryHalt` is adversarial (an interrupted cut hides the partial state behind a bare failure).
#[derive(Debug, Clone, Copy)]
pub enum CutResponse {
    /// Conformant: the cut completed and separated (Succeeded).
    Completes,
    /// Conformant: interrupted, but reports the precise partial state (how far it progressed).
    PartialReported,
    /// Adversarial: interrupted, reports a bare binary failure with no partial state.
    BinaryHalt,
}

/// The `force.cut` bench: models a driver's cut outcome. Reuses the nominal `ReferenceDriver`
/// (a Succeeded cut) and mutates it per `response`. Non-cut actions pass through unchanged.
#[derive(Debug)]
pub struct CutDriver {
    inner: ReferenceDriver,
    response: CutResponse,
}

impl CutDriver {
    /// A cut driver with the given outcome.
    #[must_use]
    pub fn new(response: CutResponse) -> Self {
        CutDriver { inner: ReferenceDriver::default(), response }
    }
}

impl Driver for CutDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        match self.response {
            CutResponse::Completes => {} // nominal: Succeeded
            CutResponse::PartialReported => {
                report.status.outcome = Outcome::Failed;
                report.status.failure_class = Some("blocked".to_string());
                report.status.failure_detail = Some("incomplete_cut".to_string());
                if let Some(v) = report.status.verdict.as_mut() {
                    v.value = false;
                    v.evidence.push("partial_cut: 0.6".to_string()); // the precise irreversible state
                }
            }
            CutResponse::BinaryHalt => {
                report.status.outcome = Outcome::Failed;
                report.status.failure_class = Some("blocked".to_string());
                report.status.failure_detail = Some("incomplete_cut".to_string());
                if let Some(v) = report.status.verdict.as_mut() {
                    v.value = false; // no partial_cut evidence -> a binary halt (adversarial)
                }
            }
        }
        report
    }
}
```

- [ ] **Step 6: Add the lib unit test** — in `lib.rs` tests module, after
`check_engagement_requires_held_confirmation_on_success`:

```rust
    #[test]
    fn check_irreversible_requires_partial_state_on_interruption() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode,
        };
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        let action = CanonicalAction {
            target_frame: "grasp".into(),
            target_pose: PoseExpr::Ref { r#ref: "seam".into() },
            force_budget: Some(rfl_core::quantity::Quantity("30 N".into())),
            timing: TimingHints {
                nominal_duration: None,
                timing_mode: TimingMode::TimeScalable,
                stop_at_goal: true,
            },
            tactile_target: None,
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: Some(serde_json::json!({ "irreversible": true })),
                station_keeping: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
        };
        let goal = ExecuteGoal::wrap("s/e/0001-cut".to_string(), action);
        let report = |outcome: Outcome, evidence: Vec<String>| DriverReport {
            telemetry: vec![],
            status: Status {
                message: "status",
                action_id: "s/e/0001-cut".to_string(),
                outcome,
                verdict: Some(Verdict { value: true, confidence: 1.0, evidence }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
            },
        };
        // Succeeded -> vacuously Pass (completed; nothing partial).
        assert_eq!(check_irreversible(&goal, &report(Outcome::Succeeded, vec![])), CheckOutcome::Pass);
        // Interrupted + partial state reported -> Pass.
        assert_eq!(
            check_irreversible(&goal, &report(Outcome::Failed, vec!["partial_cut: 0.6".to_string()])),
            CheckOutcome::Pass
        );
        // Interrupted + binary halt (no partial state) -> Fail.
        assert!(matches!(
            check_irreversible(&goal, &report(Outcome::Failed, vec![])),
            CheckOutcome::Fail(_)
        ));
    }
```

- [ ] **Step 7: Run the new tests (green)**

Run: `cargo test -p rfl-conformance cut 2>&1 | grep -E "test result|cut_|nominal_cut"` then
`cargo test -p rfl-conformance check_irreversible 2>&1 | grep -E "test result|check_irreversible"`.
Expected: the 4 integration tests + the lib unit test PASS.

- [ ] **Step 8: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `rfl-conformance` lib 14 (was 13, +1), `envelope_conformance` 38 (was 34, +4).
Existing tests unaffected (check_irreversible vacuous without `irreversible`; the ReferenceDriver
is unchanged, so all other reports are byte-identical).

- [ ] **Step 9: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs && \
git commit -m "feat(conformance): verify force.cut irreversibility (partial-state)

envelope_class_for(cut)=>ForceTrajectory (reused) + check_irreversible: an
interrupted irreversible cut MUST report the precise partial state in
verdict.evidence, never a binary failure (vacuous otherwise; a completed cut
is vacuous). CutDriver{Completes,PartialReported,BinaryHalt}: BinaryHalt (a
bare failure hiding the partial state) fails; OverForce fails the shear
budget. Bites the failure path, the mirror of check_actuation/engagement.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `lib.rs` + `envelope_conformance.rs`.

---

### Task 3: README status line

**Files:**
- Modify: `README.md` (~line 84)

- [ ] **Step 1: Extend the README status line** — add `force.cut` to the examples / force
primitives listing and note it opens the irreversible-operation safety class (REV3): an
interrupted cut must report the precise partial state (how far it progressed) in
`verdict.evidence`, never a binary failure — the mirror of the success-path checks. Match the
surrounding phrasing; keep the existing sentence structure. (spec/05 REV3 already states the
obligation, so no spec change.)

- [ ] **Step 2: Verify nothing regressed**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: unchanged green.

- [ ] **Step 3: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add README.md && \
git commit -m "docs(readme): note force.cut irreversibility / partial-state (REV3)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `README.md`.

---

## Post-increment

- Final full-suite read: expected `rfl-core` 81, `rfl-conformance` lib 14, `driver_protocol` 9,
  `envelope_conformance` 38, new `cut` 5, others unchanged.
- Update `project_rfl.md` "Implementation track" (19th increment, real hashes + test deltas) +
  the `MEMORY.md` RFL line. Move deferred items (tool_safety regime, path-bounding,
  pre-execution confirmation, on_separation, feed_rate) into the parked list.

## Self-review (spec coverage)

- Irreversibility REV3 partial-state leg (design § 1, § 3) → Task 2 (`check_irreversible`). ✓
- ForceCut primitive + lowering + gate + irreversible marker (design § 4) → Task 1. ✓
- partial-state via verdict.evidence + BinaryHalt non-vacuity (design § 3, § 5) → Task 2. ✓
- README (design § 6) → Task 3. ✓
- No schema change (design § 2) → confirmed: no task touches schemas; descriptors gain a
  capability token (data). ✓
- Type consistency: `ForceCut{cut_path: Value, shear_force_budget: Quantity, completion: Value,
  compliance}`; `force_profile.irreversible` written by the lowering, read by `check_irreversible`;
  `partial_cut…` evidence written by `CutDriver`, read by `check_irreversible`;
  `CutResponse{Completes,PartialReported,BinaryHalt}` used in lib.rs + envelope_conformance.rs;
  `envelope_class_for("cut")=>ForceTrajectory`. ✓
