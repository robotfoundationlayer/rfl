# Driver-protocol round-trip (Test Class 3 — C1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the driver-interface round-trip — define the driver-side report types + an in-process `Driver` trait, a nominal-echo reference driver, and a Class 3 test proving `execute → telemetry → status` is schema-valid, correlated, and deterministic.

**Architecture:** `rfl-core::driver` gains the protocol report types (`Telemetry`/`Status`/`Wrench`/`Verdict`/`RealizedPose`/`Outcome`/`DriverReport`) and a `Driver` trait, typed from the existing `driver-interface.schema.json` (minimal required fields; rich content validates against the open floors). `rfl-conformance` implements a nominal-echo `ReferenceDriver` + a Class 3 round-trip test (boon validity, `action_id` correlation, terminal status, fidelity-tier echo, generate-twice determinism, 3-hand golden). Envelope-class checkers + adversarial drivers are C2.

**Tech Stack:** Rust (workspace `rfl-core` / `rfl-conformance`, edition 2024, MSRV 1.85, cargo 1.96 via rustup), `serde`/`serde_json`, `insta` golden snapshots, `boon` JSON Schema validation, `anyhow` (conformance). Python `validate.py` via `uv`.

**Design doc:** `docs/design/2026-05-31-driver-protocol-roundtrip-c1-design.md` (committed, `20cf6eb`).

**Standing rules (every task):**
- Prefix every cargo command with `export PATH="$HOME/.cargo/bin:$PATH"`.
- **Validation read and `git commit` MUST be separate batches.** Run tests, READ `ok`/`PASS`/`EXIT=0`, then stage + commit in a later message.
- `git add` explicit paths only — never `-A` (keeps `docs/plans/` and other untracked files out).
- Before commit: branch == `main`. Before push: `git fetch -q origin && git merge-base --is-ancestor origin/main HEAD`. After push: `git rev-list --left-right --count origin/main...HEAD` == `0 0`. Rebase onto `origin/main` if the ff-check fails (parallel session). No `--force`, no `--no-verify`.
- Conventional Commits + `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- Keep `cargo test` warning-clean (rustc default lints); clippy `pedantic` noise is pre-existing and not gated.

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `crates/rfl-core/src/driver.rs` | protocol report types + `Driver` trait (fill the TODO stub) | 1 |
| `crates/rfl-conformance/src/lib.rs` | `ReferenceDriver` + `run_reference_driver` + `reports_to_jsonl` | 2 |
| `crates/rfl-conformance/tests/driver_protocol.rs` (new) | the Class 3 round-trip test + 3 goldens | 3 |
| `crates/rfl-conformance/tests/snapshots/driver_protocol__driver_*.snap` | generated goldens | 3 |

No `spec/` or `schemas/` edits. The cable example + the existing Class 2 / surface-scan tests stay green throughout.

---

## Task 1: Protocol report types + `Driver` trait (`rfl-core::driver`)

**Files:**
- Modify: `crates/rfl-core/src/driver.rs` (replace the TODO stub body)

- [ ] **Step 1: Write the failing tests** — replace the entire contents of `crates/rfl-core/src/driver.rs` with the module below (types + trait + tests together; the tests reference the types in the same file, so "red" is the pre-edit stub having none of them):

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Driver Interface protocol types.
//!
//! The report side of the Driver Interface (`spec/03` § Canonical driver messages,
//! `schemas/driver-interface.schema.json` TelemetryFeedback / StatusResult): the
//! `Telemetry` (Feedback) and `Status` (Result) a driver returns, and the in-process
//! `Driver` trait a reference / test driver implements. Representation-owned content
//! (pose, wrench, verdict) is concrete here but validates against the schema's open
//! floors, mirroring the execute side. Serialization is deterministic (RD1c): fixed
//! field order, optionals skipped when empty, no float reformatting beyond round6.
//!
//! See `spec/03-driver-interface.md` and `spec/05-conformance.md` § Four test classes.

use crate::canonical::ExecuteGoal;
use crate::quantity::Quantity;

/// A realized pose report (`Pose6DFloor`). v0 retarget targets are symbolic (the
/// concrete `Pose6D` representation is `spec/02`'s), so a reference driver emits the
/// deterministic placeholder; concrete tracking arrives with the pose representation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RealizedPose {
    /// Position in R^3 (metres).
    pub position: [f64; 3],
    /// Orientation unit quaternion `[x, y, z, w]`.
    pub orientation: [f64; 4],
}

impl RealizedPose {
    /// The deterministic placeholder pose (identity orientation at the origin).
    #[must_use]
    pub fn placeholder() -> Self {
        RealizedPose { position: [0.0, 0.0, 0.0], orientation: [0.0, 0.0, 0.0, 1.0] }
    }
}

/// A wrench reading (`WrenchFloor`): force (N) and torque (N·m) vectors.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Wrench {
    /// Force vector (N).
    pub force: [f64; 3],
    /// Torque vector (N·m).
    pub torque: [f64; 3],
}

/// A tactile feature reading.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TactileReading {
    /// The manifold feature name.
    pub feature: String,
    /// The reading (a unit-suffixed quantity or a qualitative token).
    pub reading: String,
}

/// A three-valued verdict with evidence (`VerdictFloor`; v0 carries a two-valued
/// `value` — the `outcome` enum already supplies the indeterminate case).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Verdict {
    /// The postcondition verdict.
    pub value: bool,
    /// Confidence in `[0, 1]`.
    pub confidence: f64,
    /// Evidence supporting the verdict.
    pub evidence: Vec<String>,
}

/// The terminal outcome of an action (`StatusResult.outcome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The action completed and its postcondition holds.
    Succeeded,
    /// The action failed (a `failure_class` accompanies it).
    Failed,
    /// The postcondition could not be determined.
    Indeterminate,
}

/// A telemetry sample (`TelemetryFeedback`; required: message, action_id, t).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Telemetry {
    /// The message discriminant (`telemetry`).
    pub message: &'static str,
    /// The correlated action id.
    pub action_id: String,
    /// Timestamp on the `04` manifold timebase (seconds).
    pub t: f64,
    /// The realized pose at this sample.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realized_pose: Option<RealizedPose>,
    /// The contact wrench at this sample.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrench: Option<Wrench>,
    /// The securing force on the held object (grasp-continuity, GC1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub securing_force: Option<Quantity>,
    /// Tactile feature readings (manifold confirmation).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tactile: Vec<TactileReading>,
    /// Force events fired this sample (breakaway / detent).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<String>,
    /// The fidelity tier in effect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fidelity_tier: Option<String>,
}

/// The terminal action result (`StatusResult`; required: message, action_id, outcome).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Status {
    /// The message discriminant (`status`).
    pub message: &'static str,
    /// The correlated action id.
    pub action_id: String,
    /// The terminal outcome.
    pub outcome: Outcome,
    /// The postcondition verdict + evidence (audit record, `05` AUD1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
    /// The achieved fidelity tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fidelity_tier: Option<String>,
    /// The final pose at rest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_pose: Option<RealizedPose>,
    /// The protocol failure class (present only when `outcome != succeeded`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_class: Option<String>,
    /// The primitive-specific failure detail token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_detail: Option<String>,
}

/// One action's driver report: the telemetry samples plus the terminal status.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DriverReport {
    /// The telemetry samples emitted during the action.
    pub telemetry: Vec<Telemetry>,
    /// The terminal status.
    pub status: Status,
}

/// An in-process driver: consumes an `execute` message and reports back in-protocol.
/// The conformance test target (`spec/05` § Four test classes, the local in-process
/// path); a ROS 2 transport binding is a later increment.
pub trait Driver {
    /// Execute one canonical action and produce its telemetry samples + terminal status.
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_serializes_required_fields_and_skips_empty() {
        let t = Telemetry {
            message: "telemetry",
            action_id: "a/b/0001-x".into(),
            t: 1.0,
            realized_pose: Some(RealizedPose::placeholder()),
            wrench: None,
            securing_force: None,
            tactile: vec![],
            events: vec![],
            fidelity_tier: Some("manifold".into()),
        };
        let j = serde_json::to_string(&t).unwrap();
        assert!(j.contains("\"message\":\"telemetry\""));
        assert!(j.contains("\"action_id\":\"a/b/0001-x\""));
        assert!(j.contains("\"t\":1.0"));
        assert!(j.contains("\"fidelity_tier\":\"manifold\""));
        assert!(!j.contains("wrench")); // skipped when None
        assert!(!j.contains("tactile")); // skipped when empty
    }

    #[test]
    fn status_outcome_serializes_snake_case() {
        let s = Status {
            message: "status",
            action_id: "a/b/0001-x".into(),
            outcome: Outcome::Succeeded,
            verdict: None,
            fidelity_tier: None,
            final_pose: None,
            failure_class: None,
            failure_detail: None,
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("\"outcome\":\"succeeded\""));
        assert!(!j.contains("failure_class")); // skipped when None
    }
}
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core driver`
Expected: PASS (`telemetry_serializes_required_fields_and_skips_empty`, `status_outcome_serializes_snake_case`). (No separate "red" run — this fills a stub; the prior `driver.rs` had no types, so nothing referenced them.)

- [ ] **Step 3: Confirm the workspace builds warning-clean**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "warning:|test result|FAILED|error" | tail -12`
Expected: all `test result: ok`; no `warning:` from `driver.rs`. (rfl-core gains 2 tests.)

- [ ] **Step 4: Commit** (separate batch from Step 3)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-core/src/driver.rs && git commit -m "feat(core): add Driver Interface report types + Driver trait

Telemetry / Status / Wrench / Verdict / RealizedPose / Outcome / DriverReport
+ the in-process Driver trait, typed from driver-interface.schema.json
(minimal required fields; rich content validates against the open floors).
Deterministic serialization (fixed field order, optionals skipped).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: nominal-echo `ReferenceDriver` + run helper (`rfl-conformance`)

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (append the driver + helpers + a unit test)

- [ ] **Step 1: Write the failing test** — append to `crates/rfl-conformance/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn example_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
    }

    #[test]
    fn reference_driver_succeeds_and_echoes_fidelity() {
        let dir = example_dir();
        let reports = run_reference_driver(
            &dir.join("skill.yaml"),
            &dir.join("embodiments/pneumatic-6f.yaml"),
        )
        .expect("drive");
        assert_eq!(reports.len(), 8); // 8 cable-insertion actions
        for r in &reports {
            assert!(matches!(r.status.outcome, rfl_core::driver::Outcome::Succeeded));
            for t in &r.telemetry {
                assert_eq!(t.action_id, r.status.action_id); // correlation
            }
        }
        // pneumatic has no tactile sensing -> grasp.pinch confirmation degrades to proxy
        assert_eq!(reports[1].status.fidelity_tier.as_deref(), Some("proxy"));
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance reference_driver_succeeds_and_echoes_fidelity`
Expected: FAIL — `cannot find function 'run_reference_driver'`.

- [ ] **Step 3: Implement the driver + helpers** — insert into `crates/rfl-conformance/src/lib.rs` (after the existing `retarget_example_to_jsonl` function, before the `#[cfg(test)]` module added in Step 1):

```rust
use rfl_core::canonical::{ExecuteGoal, TactileTargetOut};
use rfl_core::driver::{Driver, DriverReport, Outcome, RealizedPose, Status, Telemetry, Verdict, Wrench};

/// A nominal-echo reference driver: it does not simulate physics; it returns the
/// in-protocol report a conformant driver would produce on a nominal execution,
/// deterministically from the `execute` message. The in-process Class 3 target
/// (`spec/05` § Four test classes). One telemetry sample + one terminal status per
/// action; `t` is the 1-based step counter.
#[derive(Debug, Default)]
pub struct ReferenceDriver {
    step: u32,
}

impl Driver for ReferenceDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        self.step += 1;
        let ca = &goal.canonical_action;
        let fidelity_tier = match &ca.tactile_target {
            Some(TactileTargetOut::Auto) => Some("manifold".to_string()),
            Some(TactileTargetOut::Proxy { .. }) => Some("proxy".to_string()),
            _ => None,
        };
        // Echo any commanded force budget as plausible in-protocol content (carries
        // increment 3's bounds through; C2's envelope checkers will sample it).
        let (wrench, securing_force) = match &ca.force_budget {
            Some(q) => {
                let mag = q.parse().map_or(0.0, |(v, _)| v);
                (
                    Some(Wrench { force: [0.0, 0.0, mag], torque: [0.0, 0.0, 0.0] }),
                    Some(q.clone()),
                )
            }
            None => (None, None),
        };
        let telemetry = Telemetry {
            message: "telemetry",
            action_id: goal.action_id.clone(),
            t: f64::from(self.step),
            realized_pose: Some(RealizedPose::placeholder()),
            wrench,
            securing_force,
            tactile: vec![],
            events: vec![],
            fidelity_tier: fidelity_tier.clone(),
        };
        let status = Status {
            message: "status",
            action_id: goal.action_id.clone(),
            outcome: Outcome::Succeeded,
            verdict: Some(Verdict {
                value: true,
                confidence: 1.0,
                evidence: vec!["nominal reference-driver execution".to_string()],
            }),
            fidelity_tier,
            final_pose: Some(RealizedPose::placeholder()),
            failure_class: None,
            failure_detail: None,
        };
        DriverReport { telemetry: vec![telemetry], status }
    }
}

/// Retarget the skill onto the embodiment and drive every `execute` message through a
/// fresh `ReferenceDriver`, returning the per-action reports. Action ids match the
/// `{skill}/{embodiment_id}/{NNNN}-{suffix}` form `canonical::to_jsonl` emits.
///
/// # Errors
/// Propagates parse / retarget errors.
pub fn run_reference_driver(
    skill_path: &std::path::Path,
    embodiment_path: &std::path::Path,
) -> anyhow::Result<Vec<DriverReport>> {
    let skill = rfl_core::skill_isa::Skill::parse_yaml(&std::fs::read_to_string(skill_path)?)?;
    let emb =
        rfl_core::embodiment::Embodiment::parse_yaml(&std::fs::read_to_string(embodiment_path)?)?;
    let out = rfl_core::translation::retarget(&skill, &emb)?;
    let mut driver = ReferenceDriver::default();
    let mut reports = Vec::new();
    for (i, (action, suffix)) in out.actions.iter().zip(&out.suffixes).enumerate() {
        let action_id = format!("{}/{}/{:04}-{}", skill.skill, emb.id, i + 1, suffix);
        let goal = ExecuteGoal::wrap(action_id, action.clone());
        reports.push(driver.execute(&goal));
    }
    Ok(reports)
}

/// Render a report stream as JSON Lines (each telemetry sample, then the status, per
/// action) for golden snapshots and determinism checks.
#[must_use]
pub fn reports_to_jsonl(reports: &[DriverReport]) -> String {
    let mut out = String::new();
    for r in reports {
        for t in &r.telemetry {
            out.push_str(&serde_json::to_string(t).expect("serialize telemetry"));
            out.push('\n');
        }
        out.push_str(&serde_json::to_string(&r.status).expect("serialize status"));
        out.push('\n');
    }
    out
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance reference_driver_succeeds_and_echoes_fidelity`
Expected: PASS.

- [ ] **Step 5: Confirm warning-clean**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance 2>&1 | grep -E "warning:|test result|FAILED|error" | tail`
Expected: `test result: ok` for the lib unit test + the existing integration tests; no warnings.

- [ ] **Step 6: Commit** (separate batch)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-conformance/src/lib.rs && git commit -m "feat(conformance): add nominal-echo ReferenceDriver + run/render helpers

ReferenceDriver implements the in-process Driver trait: one telemetry + one
status per execute message, deterministic, echoing fidelity tier from
tactile_target and any commanded force budget. run_reference_driver drives the
retargeted cable stream; reports_to_jsonl renders it for goldens.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Class 3 round-trip test + goldens (`tests/driver_protocol.rs`)

**Files:**
- Create: `crates/rfl-conformance/tests/driver_protocol.rs`
- Create (generated): `crates/rfl-conformance/tests/snapshots/driver_protocol__driver_{allegro,leap,pneumatic}.snap`

- [ ] **Step 1: Write the test file** — create `crates/rfl-conformance/tests/driver_protocol.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 3 (`spec/05` § Four test classes): the driver-protocol
//! round-trip. A driver consumes the `execute` messages and reports back in-protocol
//! (`telemetry` + `status`), schema-valid, action-correlated, and deterministic.
//! C1 verifies the round-trip; the envelope-class checkers + adversarial drivers are
//! C2.

use rfl_conformance::{reports_to_jsonl, run_reference_driver};
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

fn reports_jsonl(stem: &str) -> String {
    let dir = example_dir();
    let reports = run_reference_driver(
        &dir.join("skill.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("drive");
    reports_to_jsonl(&reports)
}

#[test]
fn golden_allegro() {
    insta::assert_snapshot!("driver_allegro", reports_jsonl("allegro"));
}

#[test]
fn golden_leap() {
    insta::assert_snapshot!("driver_leap", reports_jsonl("leap"));
}

#[test]
fn golden_pneumatic() {
    insta::assert_snapshot!("driver_pneumatic", reports_jsonl("pneumatic-6f"));
}

#[test]
fn report_stream_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(reports_jsonl(stem), reports_jsonl(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn all_actions_correlate_and_succeed() {
    let dir = example_dir();
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        let reports = run_reference_driver(
            &dir.join("skill.yaml"),
            &dir.join(format!("embodiments/{stem}.yaml")),
        )
        .expect("drive");
        assert_eq!(reports.len(), 8, "stem {stem}");
        for r in &reports {
            assert!(matches!(r.status.outcome, rfl_core::driver::Outcome::Succeeded), "stem {stem}");
            for t in &r.telemetry {
                assert_eq!(t.action_id, r.status.action_id, "stem {stem}");
            }
        }
    }
}

#[test]
fn fidelity_tier_echoes_tactile_degradation() {
    let dir = example_dir();
    // allegro + leap declare tactile sensing -> manifold; pneumatic-6f does not -> proxy.
    for (stem, tier) in [("allegro", "manifold"), ("leap", "manifold"), ("pneumatic-6f", "proxy")] {
        let reports = run_reference_driver(
            &dir.join("skill.yaml"),
            &dir.join(format!("embodiments/{stem}.yaml")),
        )
        .expect("drive");
        // action index 1 is grasp.pinch (the tactile-confirmed action).
        assert_eq!(reports[1].status.fidelity_tier.as_deref(), Some(tier), "stem {stem}");
    }
}

#[test]
fn every_report_message_is_schema_valid() {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/driver-interface.schema.json");
    let schema: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&schema_path).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource("driver-interface.schema.json", schema).unwrap();
    let idx = compiler.compile("driver-interface.schema.json", &mut schemas).unwrap();

    let dir = example_dir();
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        let reports = run_reference_driver(
            &dir.join("skill.yaml"),
            &dir.join(format!("embodiments/{stem}.yaml")),
        )
        .expect("drive");
        for r in &reports {
            for t in &r.telemetry {
                let v = serde_json::to_value(t).unwrap();
                schemas.validate(&v, idx).unwrap_or_else(|e| panic!("{stem} telemetry: {e}"));
            }
            let v = serde_json::to_value(&r.status).unwrap();
            schemas.validate(&v, idx).unwrap_or_else(|e| panic!("{stem} status: {e}"));
        }
    }
}
```

- [ ] **Step 2: Run the non-golden tests first to confirm the round-trip + schema validity**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test driver_protocol -- all_actions_correlate_and_succeed fidelity_tier_echoes_tactile_degradation every_report_message_is_schema_valid report_stream_is_byte_identical`
Expected: all PASS. **If `every_report_message_is_schema_valid` fails**, the emitted `fidelity_tier` string is not in the schema's `FidelityTier` enum — inspect `$defs/FidelityTier` in `driver-interface.schema.json` and adjust the echoed strings (`manifold`/`proxy`) to match.

- [ ] **Step 3: Generate + eyeball the goldens**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test driver_protocol` (expect the 3 `golden_*` tests to FAIL — no snapshot yet)
Then: `export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance --test driver_protocol`
Then eyeball: `git status --short crates/rfl-conformance/tests/snapshots/ && sed -n '1,12p' crates/rfl-conformance/tests/snapshots/driver_protocol__driver_pneumatic.snap`
Expected: three new `.snap` files; the pneumatic snapshot shows `"message":"telemetry"`/`"status"` lines with `"fidelity_tier":"proxy"` on the pinch action, `"outcome":"succeeded"`, placeholder poses, and `securing_force`/`wrench` echoing the commanded budgets.

- [ ] **Step 4: Re-run to confirm green**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test driver_protocol 2>&1 | grep "test result"`
Expected: `test result: ok. 6 passed`.

- [ ] **Step 5: Gating validation batch** (separate from the commit)

Run: `cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1 && export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "test result|FAILED|error|warning:" | tail -12`
Expected: validate.py `PASS`; every crate green (rfl-core + rfl-conformance incl. the new `driver_protocol` 6 + the existing `retarget_determinism` 5 + `surface_scan` 5); no warnings.

- [ ] **Step 6: Commit** (separate batch — after reading PASS above)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-conformance/tests/driver_protocol.rs crates/rfl-conformance/tests/snapshots/driver_protocol__driver_allegro.snap crates/rfl-conformance/tests/snapshots/driver_protocol__driver_leap.snap crates/rfl-conformance/tests/snapshots/driver_protocol__driver_pneumatic.snap && git commit -m "test(conformance): add Class 3 driver-protocol round-trip (C1)

Drives the retargeted cable stream through the in-process ReferenceDriver and
verifies the execute->telemetry->status round-trip: boon schema validity,
action_id correlation, terminal succeeded status, fidelity-tier echo
(manifold/manifold/proxy), generate-twice determinism, and 3 goldens. The
envelope-class checkers + adversarial drivers are C2.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Full verification + push + memory

**Files:** none (verification + integration).

- [ ] **Step 1: Final full suite + validate.py**

Run: `cd ~/Documents/GitHub/rfl && export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "test result|FAILED|error" && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: all green; validate.py `PASS`.

- [ ] **Step 2: Push any unpushed commits** (Tasks 1-3 should each have pushed per the standing rules; this confirms `0 0`)

Run: `cd ~/Documents/GitHub/rfl && git rev-parse --abbrev-ref HEAD && git fetch -q origin && git merge-base --is-ancestor origin/main HEAD && git push -q origin main; git rev-list --left-right --count origin/main...HEAD && git log --oneline -5`
Expected: branch `main`; final count `0 0`; the 3 C1 commits + the design-doc commit visible. (If a per-task push was skipped, this pushes them; rebase onto `origin/main` first if the ff-check fails.)

- [ ] **Step 3: Update the memory** (`~/.claude/.../memory/project_rfl.md` "## Implementation track") with the real commit hashes from `git log --oneline -6`, recording this as the 4th increment (Test Class 3 C1 — driver-protocol round-trip), and note C2 (envelope-class checkers + adversarial drivers) as the planned follow-up. Not a repo commit — memory only.

---

## Self-review (completed during planning)

- **Spec coverage:** design §3 protocol types → Task 1; §4 `Driver` trait + nominal `ReferenceDriver` → Tasks 1 (trait) + 2 (driver); §5 round-trip test (boon / correlation / terminal / fidelity / determinism / golden) → Task 3; §6 determinism + placement → enforced across Tasks 1-3; §2 deferrals (envelope checkers, adversarial drivers, ε-table, ROS 2) → explicitly out of scope, no task. No gaps.
- **Placeholder scan:** no "TBD/TODO" in plan steps (the one `driver.rs` "TODO stub" reference is the existing code state being replaced); every code step shows complete code; every run step shows the command + expected output.
- **Type consistency:** `Telemetry`/`Status`/`Wrench`/`Verdict`/`RealizedPose`/`Outcome`/`DriverReport`/`Driver`/`ReferenceDriver`/`run_reference_driver`/`reports_to_jsonl` are used identically across Tasks 1-3; `Outcome::Succeeded` (snake_case "succeeded"), `fidelity_tier` strings `manifold`/`proxy`, and `message` discriminants `telemetry`/`status` are consistent between the driver impl (Task 2) and the assertions (Task 3).
- **Determinism:** `t` = step counter; poses are fixed placeholders; verdict/confidence fixed; field order fixed with empty-skip — byte-stable, golden + generate-twice covered.
