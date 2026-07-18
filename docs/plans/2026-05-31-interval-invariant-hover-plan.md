# Interval-Invariant Envelope Class (reach.hover) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce the interval-invariant envelope class (the 4th and last) by adding `reach.hover` and verifying it with interval sampling — a mid-interval violation fails even when the endpoint conforms (spec/05 ENV2).

**Architecture:** rfl-core gains a `ReachHover` primitive (baseline `reach.*`, symbolic standoff setpoint). rfl-conformance gains `EnvelopeClass::IntervalInvariant` + a structural interval check (every telemetry sample carries `realized_pose`), a multi-sample `ReferenceDriver` (N=3 for interval-invariant actions, gated on the envelope class), and a `MidIntervalDrop` fault. A `skill-hover.yaml` worked example demonstrates the ENV2 interval-vs-endpoint distinction.

**Tech Stack:** Rust (rfl-core / rfl-conformance), serde, insta (goldens), boon (schema), uv + jsonschema (Class-1).

**Process discipline (this repo):**
- Prefix every cargo command with `export PATH="$HOME/.cargo/bin:$PATH"`.
- Run validation in a batch **physically separate** from the `git commit`.
- `git add` explicit paths only (never `-A`); never stage `docs/plans/`. `git status` before each stage.
- Per commit: branch == `main`; before push `git merge-base --is-ancestor origin/main HEAD`; after push `git rev-list --left-right --count origin/main...HEAD` == `0 0`.
- Conventional Commits + `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. Real `git log` hashes.
- insta regen: `INSTA_UPDATE=always cargo test -p rfl-conformance --test <name>`, then `rm -f crates/rfl-conformance/tests/snapshots/*.snap.new`, then `git diff`.
- **Exhaustive-match traps:** (a) `translation.rs` `lower` + `check_capability` have no catch-all — `Primitive::ReachHover` + both arms land in Task 1's one commit. (b) `lib.rs` `check_envelope` matches `EnvelopeClass` exhaustively — the new `IntervalInvariant` variant + its arm land in Task 2's one commit.

**Facts:** `FrameRef = String`; `RealizedPose`/`Wrench`/`Telemetry`/`Status` all derive `Clone`; `RealizedPose::placeholder()` exists; `reach.*` is baseline (no descriptor capability); `e.control_frame()` exists; `PoseExpr::FrameRelative { frame: String, offset: serde_json::Value }`. The hover example has 2 actions (`hover`0, `inspect`1).

---

## Task 1: rfl-core — the `reach.hover` primitive

**Files:**
- Modify: `crates/rfl-core/src/skill_isa.rs` (`ReachHover` struct + `Primitive::ReachHover`)
- Modify: `crates/rfl-core/src/translation.rs` (import, baseline `check_capability`, `lower` arm, `lower_reach_hover`, 1 test)

- [ ] **Step 1: Write the failing translation test**

Append to the `mod tests` block in `crates/rfl-core/src/translation.rs`:

```rust
    #[test]
    fn hover_lowers_to_a_standoff_setpoint() {
        // reach.* is baseline (no capability gate). The hover lowers to a symbolic
        // standoff setpoint in the control frame.
        let yaml = "skill: t\nbody:\n  sequence:\n    - reach.hover: { target: panel, standoff: 50 mm, duration: 5 s }\n";
        let skill = Skill::parse_yaml(yaml).expect("parse");
        let emb = load("allegro").1;
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["hover"]);
        let crate::canonical::PoseExpr::FrameRelative { frame, offset } = &out.actions[0].target_pose
        else {
            panic!("expected FrameRelative");
        };
        assert_eq!(frame, "panel");
        assert_eq!(offset.get("distance").and_then(|v| v.as_str()), Some("50 mm"));
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core hover_lowers 2>&1 | tail -6`
Expected: FAIL — `reach.hover` has no matching `Primitive` variant (parse error).

- [ ] **Step 3a: Add the `ReachHover` struct + enum variant (`skill_isa.rs`)**

After the `ReachScan` struct (the one ending with `pub coverage_overlap: Option<f64>,` + `}`), add:

```rust
/// `reach.hover` parameters (v0 subset of `$defs/ReachHoverParams`). Sustained
/// station-keeping at a standoff over a bounded interval (`spec/01` § 1.5). v0 models
/// the target frame, the standoff, and the duration (carried, not lowered); the § 1.5
/// tolerances / tracking carry spec defaults and are not emitted.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReachHover {
    /// The surface / frame to hover over (v0: a frame name).
    pub target: FrameRef,
    /// Maintained distance from `target` along its outward normal.
    pub standoff: Quantity,
    /// Hold duration (`Duration | until`; carried, symbolic in v0).
    #[serde(default)]
    pub duration: Option<serde_yaml::Value>,
}
```

In `pub enum Primitive`, after the `ReachScan(ReachScan)` variant (it has `#[serde(rename = "reach.scan")]`):

```rust
    /// `reach.hover`.
    #[serde(rename = "reach.hover")]
    ReachHover(ReachHover),
```

- [ ] **Step 3b: Wire it into `translation.rs`**

In the `use crate::skill_isa::{...}` import, add `ReachHover` (e.g. after `ReachAlign,`).

In `check_capability`, the baseline reach arm currently reads:

```rust
        Primitive::ReachAlign(_) | Primitive::ReachRetract(_) | Primitive::ReachScan(_) => {
```

Add `ReachHover` to it:

```rust
        Primitive::ReachAlign(_) | Primitive::ReachRetract(_) | Primitive::ReachScan(_) | Primitive::ReachHover(_) => {
```

In `lower` (the `match prim`), after the `Primitive::ReachScan(p) => (lower_reach_scan(p, e), "scan"),` arm:

```rust
        Primitive::ReachHover(p) => (lower_reach_hover(p, e), "hover"),
```

After the `lower_reach_align` function, add:

```rust
/// Lower `reach.hover` (`spec/01` § 1.5): a sustained station-keeping action holding
/// the controlled frame at a standoff setpoint over a bounded interval. v0 emits a
/// symbolic standoff pose (`S = p + standoff·n`); the geometric station invariant is
/// checked structurally (interval-invariant, `05` ENV2). The duration is carried in the
/// skill but not lowered in v0 (the interval check is sample-structural, not timed).
fn lower_reach_hover(p: &ReachHover, e: &Embodiment) -> CanonicalAction {
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
        safety_envelope: base_envelope(e),
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core hover_lowers 2>&1 | tail -6`
Expected: PASS.

- [ ] **Step 5: Full rfl-core suite + warning check (validation batch, separate from commit)**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | grep "test result:" | head -1 && cargo build -p rfl-core 2>&1 | grep -i warning || echo "no warnings"`
Expected: all pass; no warnings. Read and confirm.

- [ ] **Step 6: Commit**

```bash
cd ~/Documents/GitHub/rfl && git fetch origin main --quiet && git rev-parse --abbrev-ref HEAD && git status --short
git add crates/rfl-core/src/skill_isa.rs crates/rfl-core/src/translation.rs
git commit -F - <<'EOF'
feat(core): add reach.hover primitive (sustained station-keeping)

reach.hover (spec/01 § 1.5) holds the controlled frame at a standoff setpoint
over a bounded interval. Lowers to a symbolic FrameRelative standoff pose;
reach.* baseline (no capability gate); the duration is carried but not lowered
in v0 (the interval check is sample-structural). Enum + lower + check_capability
arms land together (exhaustive match). No schema change (ReachHoverParams already
typed).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git show --stat --oneline HEAD | head -4
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```

Expected: 2 files; push succeeds; `0 0`. Record the real hash.

---

## Task 2: rfl-conformance — the interval-invariant class + multi-sample driver

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (`EnvelopeClass` variant, `envelope_class_for`, `check_envelope` arm, multi-sample `ReferenceDriver`, `Fault::MidIntervalDrop`, `FaultyDriver` arm, a private `suffix_of`, unit tests)

- [ ] **Step 1: Write the failing unit tests**

In the `lib.rs` `mod tests` block, add to the `envelope_class_for` test (next to the `insert_fit` assertion):

```rust
        assert_eq!(envelope_class_for("hover"), Some(EnvelopeClass::IntervalInvariant));
```

And add a new test (in the same `mod tests`):

```rust
    #[test]
    fn interval_invariant_checks_every_sample() {
        use rfl_core::driver::{Outcome, RealizedPose, Status, Telemetry, Verdict};
        let goal = ExecuteGoal::wrap("s/e/0001-hover".into(), sample_action());
        let sample = |pose: Option<RealizedPose>| Telemetry {
            message: "telemetry",
            action_id: "s/e/0001-hover".into(),
            t: 1.0,
            realized_pose: pose,
            wrench: None,
            securing_force: None,
            tactile: vec![],
            events: vec![],
            fidelity_tier: None,
        };
        let status = Status {
            message: "status",
            action_id: "s/e/0001-hover".into(),
            outcome: Outcome::Succeeded,
            verdict: Some(Verdict { value: true, confidence: 1.0, evidence: vec![] }),
            fidelity_tier: None,
            final_pose: Some(RealizedPose::placeholder()),
            failure_class: None,
            failure_detail: None,
        };
        let ok = DriverReport {
            telemetry: vec![sample(Some(RealizedPose::placeholder())); 3],
            status: status.clone(),
        };
        assert_eq!(check_envelope(EnvelopeClass::IntervalInvariant, &goal, &ok), CheckOutcome::Pass);
        // Drop the middle sample's pose: interval fails, but the endpoint (terminal) is fine.
        let mut bad = ok.clone();
        bad.telemetry[1].realized_pose = None;
        assert!(matches!(check_envelope(EnvelopeClass::IntervalInvariant, &goal, &bad), CheckOutcome::Fail(_)));
        assert_eq!(check_envelope(EnvelopeClass::TerminalPostcondition, &goal, &bad), CheckOutcome::Pass);
    }
```

This test references a `sample_action()` helper. Add it to the `mod tests` block (a minimal `CanonicalAction` for a hover-like goal):

```rust
    fn sample_action() -> rfl_core::canonical::CanonicalAction {
        use rfl_core::canonical::{CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode};
        CanonicalAction {
            target_frame: "control".into(),
            target_pose: PoseExpr::Ref { r#ref: "panel".into() },
            force_budget: None,
            timing: TimingHints { nominal_duration: None, timing_mode: TimingMode::Strict, stop_at_goal: true },
            tactile_target: None,
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
        }
    }
```

(If `mod tests` already imports some of these via `use super::*` or an existing helper, drop the duplicate `use`. The `mod tests` for lib.rs is near the bottom of the file.)

- [ ] **Step 2: Run to verify they fail (compile error: no IntervalInvariant variant)**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --lib interval 2>&1 | grep -E "error|cannot find|no variant|test result" | head`
Expected: FAIL — `EnvelopeClass::IntervalInvariant` does not exist yet.

- [ ] **Step 3a: Add the `IntervalInvariant` variant + `envelope_class_for` + the `check_envelope` arm**

In `EnvelopeClass`, replace the `ForceTrajectory` variant block (it ends the enum) — change the trailing comment and add the variant. Current:

```rust
    /// `force.*`: the force/torque-trajectory class.
    ForceTrajectory,
}
```

to:

```rust
    /// `force.*`: the force/torque-trajectory class.
    ForceTrajectory,
    /// `reach.hover` / `transport.carry`: a maintained invariant sampled over the
    /// interval (`05` ENV2). A mid-interval violation fails even when the endpoint conforms.
    IntervalInvariant,
}
```

In `envelope_class_for`, after the `"insert_fit" | "screw" | "unscrew" => ...` arm:

```rust
        "hover" => Some(EnvelopeClass::IntervalInvariant),
```

In `check_envelope`, before the final `}` that closes the `match class` (after the `EnvelopeClass::ForceTrajectory => { ... CheckOutcome::Pass }` arm), add:

```rust
        EnvelopeClass::IntervalInvariant => {
            // v0 structural interval check (concrete poses are spec/02's): the action
            // settled (Succeeded) and EVERY interval telemetry sample carries a
            // realized_pose — the station was maintained at each sampled instant. A
            // mid-interval sample missing its pose fails here even when the endpoint
            // (TerminalPostcondition) conforms — the ENV2 property.
            if !matches!(report.status.outcome, Outcome::Succeeded) {
                return CheckOutcome::Fail(format!("outcome not succeeded: {:?}", report.status.outcome));
            }
            for (i, t) in report.telemetry.iter().enumerate() {
                if t.realized_pose.is_none() {
                    return CheckOutcome::Fail(format!("interval sample {i} missing realized_pose"));
                }
            }
            CheckOutcome::Pass
        }
```

- [ ] **Step 3b: Add the private `suffix_of` helper + make the `ReferenceDriver` multi-sample**

Add a private helper near `envelope_class_for` in `lib.rs`:

```rust
/// The primitive suffix of an action id (`.../NNNN-<suffix>`; suffixes contain no `-`).
fn suffix_of(action_id: &str) -> &str {
    action_id.rsplit('-').next().unwrap_or(action_id)
}
```

In `ReferenceDriver::execute`: (a) delete the line `self.step += 1;` (the first statement). (b) Replace the single `let telemetry = Telemetry { ... };` block with the multi-sample build:

```rust
        // Interval-invariant actions (reach.hover) are sampled over the interval (ENV2,
        // spec/05); every other action emits a single terminal-ish sample. N is decided
        // by the envelope class so future interval-invariant primitives inherit it.
        let n_samples = if envelope_class_for(suffix_of(&goal.action_id))
            == Some(EnvelopeClass::IntervalInvariant)
        {
            3
        } else {
            1
        };
        let telemetry: Vec<Telemetry> = (0..n_samples)
            .map(|_| {
                self.step += 1;
                Telemetry {
                    message: "telemetry",
                    action_id: goal.action_id.clone(),
                    t: f64::from(self.step),
                    realized_pose: Some(RealizedPose::placeholder()),
                    wrench: wrench.clone(),
                    securing_force: securing_force.clone(),
                    tactile: vec![],
                    events: vec![],
                    fidelity_tier: fidelity_tier.clone(),
                }
            })
            .collect();
```

(c) Change the final `DriverReport { telemetry: vec![telemetry], status }` to `DriverReport { telemetry, status }`.

- [ ] **Step 3c: Add the `MidIntervalDrop` fault**

In the `Fault` enum, after the `NeverSettle` variant:

```rust
    /// Drop the middle telemetry sample's `realized_pose` (violates interval-invariant,
    /// ENV2 — a mid-interval gap while the endpoint still conforms).
    MidIntervalDrop,
```

In `FaultyDriver::execute`'s `match self.fault`, after the `Fault::NeverSettle => { ... }` arm:

```rust
            Fault::MidIntervalDrop => {
                let mid = report.telemetry.len() / 2;
                if let Some(t) = report.telemetry.get_mut(mid) {
                    t.realized_pose = None;
                }
            }
```

- [ ] **Step 4: Run the unit tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --lib interval 2>&1 | grep -E "test result|FAILED"`
Expected: PASS.

- [ ] **Step 5: CRITICAL guard — existing driver_protocol goldens unchanged (single-sample byte-identical)**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test driver_protocol 2>&1 | grep "test result:" && cd ~/Documents/GitHub/rfl && git diff --stat crates/rfl-conformance/tests/snapshots/driver_protocol__*.snap | tail -1 || echo "(driver_protocol unchanged)"`
Expected: driver_protocol all PASS and the snapshot diff is EMPTY — the multi-sample restructure leaves single-sample actions byte-identical (one `step += 1`, one sample), and no cable action is interval-invariant. If a golden changed, STOP — the restructure altered single-sample output.

- [ ] **Step 6: Full validation batch (separate from commit)**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | grep "test result:" | head -1 && cargo test -p rfl-conformance 2>&1 | grep -E "Running|test result:" && cargo build -p rfl-conformance 2>&1 | grep -i warning || echo "no warnings"`
Expected: all pass (envelope_conformance, screw_fasten_unscrew, etc. unaffected); no warnings. Read and confirm.

- [ ] **Step 7: Commit**

```bash
cd ~/Documents/GitHub/rfl && git fetch origin main --quiet && git rev-parse --abbrev-ref HEAD && git status --short
git add crates/rfl-conformance/src/lib.rs
git commit -F - <<'EOF'
feat(conformance): interval-invariant envelope class + multi-sample driver

Add EnvelopeClass::IntervalInvariant (the 4th class) + envelope_class_for("hover")
+ a structural check_envelope arm (every interval telemetry sample carries
realized_pose, action Succeeded). The ReferenceDriver now emits N=3 samples for an
interval-invariant action (gated on the envelope class so future interval primitives
inherit it); single-sample actions stay byte-identical (driver_protocol goldens
unchanged). Fault::MidIntervalDrop drops the middle sample's pose for the ENV2
interval-vs-endpoint demonstration.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git show --stat --oneline HEAD | head -3
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```

Expected: 1 file; push succeeds; `0 0`. Record the real hash.

---

## Task 3: worked example + the ENV2 demonstration

**Files:**
- Create: `examples/02-surface-scan/skill-hover.yaml`
- Create: `crates/rfl-conformance/tests/surface_scan_hover.rs` + 3 snapshots
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (a `surface_dir` helper + 2 tests)

- [ ] **Step 1: Create the hover worked example**

Create `examples/02-surface-scan/skill-hover.yaml`:

```yaml
# Example 02 — Surface scan (hover variant)
# Sustained station-keeping (reach.hover) over the panel at a standoff, then inspect.
# Demonstrates the interval-invariant envelope class (spec/05 ENV2): the station
# invariant is sampled over the whole hold interval, not just the endpoint.
skill: surface-hover
description: >
  Hold a fixed standoff over a rectangular panel for a bounded interval, then inspect
  the observed surface for defects.

body:
  sequence:

    - reach.hover:
        target: panel
        standoff: 50 mm
        duration: 5 s

    - sense.inspect:
        target: panel
        observe: [defect]
```

- [ ] **Step 2: Class-1 validate the new skill**

```bash
cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python -c '
import json, yaml
from jsonschema import Draft202012Validator
sk = Draft202012Validator(json.load(open("schemas/skill-isa.schema.json")))
doc = yaml.safe_load(open("examples/02-surface-scan/skill-hover.yaml"))
errs = list(sk.iter_errors(doc))
print("skill-hover.yaml:", "OK" if not errs else errs[0].message)
'
```

Expected: `skill-hover.yaml: OK` (reach.hover params validate against ReachHoverParams; target=panel is a FrameRef).

- [ ] **Step 3: Write the retarget golden test (snapshots missing -> fail)**

Create `crates/rfl-conformance/tests/surface_scan_hover.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for the surface-scan hover variant: the reach.hover retarget
//! output is byte-deterministic, matches a committed golden, and every line is a valid
//! driver-interface execute message. (The interval-invariant verification of the hover is
//! in envelope_conformance.rs.)

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/02-surface-scan")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-hover.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_hover_allegro() {
    insta::assert_snapshot!("hover_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_hover_leap() {
    insta::assert_snapshot!("hover_leap", jsonl_for("leap"));
}

#[test]
fn golden_hover_pneumatic() {
    insta::assert_snapshot!("hover_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn hover_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn hover_every_line_is_a_valid_execute_message() {
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

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test surface_scan_hover 2>&1 | grep -E "golden_hover|test result:"`
Expected: the three `golden_hover_*` FAIL (no snapshot); the other two PASS.

- [ ] **Step 4: Generate + review the goldens**

```bash
export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance --test surface_scan_hover
rm -f ~/Documents/GitHub/rfl/crates/rfl-conformance/tests/snapshots/*.snap.new
cd ~/Documents/GitHub/rfl && echo "--- allegro hover line ---" && grep -h "0001-hover" crates/rfl-conformance/tests/snapshots/surface_scan_hover__hover_allegro.snap | grep -o '"pattern":"[^"]*"\|"frame":"panel"\|"along":"outward_normal"\|"distance":"50 mm"' | head
```

Expected: 3 new snapshots; the `0001-hover` line shows `"frame":"panel"`, `"along":"outward_normal"`, `"distance":"50 mm"` (the standoff setpoint). 2 lines per file (hover + inspect).

- [ ] **Step 5: Write the ENV2 envelope tests**

In `crates/rfl-conformance/tests/envelope_conformance.rs`, add a `surface_dir` helper next to `screw_dir` (after the `screw_dir` fn):

```rust
fn surface_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/02-surface-scan")
}
```

Then append the two tests (at the end of the file; `ReferenceDriver`, `FaultyDriver`, `Fault`, `drive`, `check_envelope`, `EnvelopeClass`, `CheckOutcome`, `suffix_of` are in scope; `Fault::MidIntervalDrop` and `EnvelopeClass::IntervalInvariant` come with the existing `Fault`/`EnvelopeClass` imports):

```rust
#[test]
fn nominal_hover_passes_interval_invariant() {
    let dir = surface_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // reach.hover is action index 0; the driver samples the interval (3 samples).
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "hover");
    assert_eq!(report.telemetry.len(), 3, "the interval must be sampled (non-vacuous)");
    assert_eq!(check_envelope(EnvelopeClass::IntervalInvariant, goal, report), CheckOutcome::Pass);
}

#[test]
fn mid_interval_drop_fails_interval_but_passes_terminal() {
    // The ENV2 property, made executable: a mid-interval violation fails the interval
    // check even though the endpoint (terminal) conforms.
    let dir = surface_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::MidIntervalDrop),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "hover");
    assert!(matches!(
        check_envelope(EnvelopeClass::IntervalInvariant, goal, report),
        CheckOutcome::Fail(_)
    ));
    assert_eq!(
        check_envelope(EnvelopeClass::TerminalPostcondition, goal, report),
        CheckOutcome::Pass
    );
}
```

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test envelope_conformance hover 2>&1 | grep -E "test (nominal|mid).*|test result:"`
Expected: both PASS — nominal passes IntervalInvariant with 3 samples; MidIntervalDrop fails IntervalInvariant but passes TerminalPostcondition.

- [ ] **Step 6: Full validation batch (separate from commit)**

```bash
export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | grep "test result:" | head -1 && cargo test -p rfl-conformance 2>&1 | grep -E "Running|test result:" && cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1 && echo "--- other surface/driver goldens unchanged? ---" && git diff --stat crates/rfl-conformance/tests/snapshots/surface_scan__*.snap crates/rfl-conformance/tests/snapshots/driver_protocol__*.snap | tail -1 || echo "(unchanged)" && ls crates/rfl-conformance/tests/snapshots/*.snap.new 2>/dev/null && echo STALE || echo "no pending"
```

Expected: rfl-core all pass; every rfl-conformance suite passes (incl. new `surface_scan_hover` 5 + `envelope_conformance` now 12); validate.py C1–C7 PASS; `surface_scan__*` and `driver_protocol__*` snapshots NOT changed; no pending. Read and confirm before staging.

- [ ] **Step 7: Commit**

```bash
cd ~/Documents/GitHub/rfl && git fetch origin main --quiet && git rev-parse --abbrev-ref HEAD && git status --short
git add examples/02-surface-scan/skill-hover.yaml \
  crates/rfl-conformance/tests/surface_scan_hover.rs \
  crates/rfl-conformance/tests/snapshots/surface_scan_hover__hover_allegro.snap \
  crates/rfl-conformance/tests/snapshots/surface_scan_hover__hover_leap.snap \
  crates/rfl-conformance/tests/snapshots/surface_scan_hover__hover_pneumatic.snap \
  crates/rfl-conformance/tests/envelope_conformance.rs
git commit -F - <<'EOF'
test(conformance): hover worked example + ENV2 interval-vs-endpoint demonstration

Add examples/02 skill-hover.yaml (reach.hover over the panel -> sense.inspect) +
surface_scan_hover retarget goldens. Two envelope_conformance tests: nominal hover
passes IntervalInvariant with 3 interval samples (non-vacuous), and MidIntervalDrop
makes the same report FAIL IntervalInvariant while PASSING TerminalPostcondition —
the executable definition of ENV2 (a mid-interval violation fails even when the
endpoint conforms). Completes the four-class envelope taxonomy.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git show --stat --oneline HEAD | head -8
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```

Expected: 6 files (skill + test + 3 snapshots + envelope_conformance.rs), no `docs/plans/`; push succeeds; `0 0`. Record the real hash.

---

## Final verification gate

- [ ] Three feat/test commits on `main`, each pushed with `0 0`.
- [ ] `cargo test -p rfl-core` green; `cargo test -p rfl-conformance` green (9 suites incl. surface_scan_hover); `validate.py` C1–C7 EXIT 0.
- [ ] `driver_protocol` + `surface_scan` + screw/cable/unscrew goldens unchanged (multi-sample is hover-only).
- [ ] No `schemas/` change; new `Primitive::ReachHover` + `EnvelopeClass::IntervalInvariant` each landed with their exhaustive-match arms.
- [ ] **The 4-class envelope taxonomy is now complete** (terminal / grasp-continuity / force-trajectory / interval-invariant).
- [ ] Update README (example 02 hover variant) / `project_rfl.md` Implementation track / `MEMORY.md` with the three real hashes.

## Self-review (run after writing, fix inline)

**Spec coverage** (design §3-§5):
- reach.hover struct + enum + lower_reach_hover + baseline gate → Task 1. ✓
- EnvelopeClass::IntervalInvariant + envelope_class_for + check_envelope arm → Task 2 Step 3a. ✓
- multi-sample ReferenceDriver (N=3, gated on envelope class) → Task 2 Step 3b. ✓
- Fault::MidIntervalDrop → Task 2 Step 3c. ✓
- skill-hover.yaml + surface_scan_hover goldens → Task 3 Steps 1-4. ✓
- nominal + ENV2 (interval-fail/terminal-pass) envelope tests → Task 3 Step 5. ✓
- Deferred (ENV3, transport.carry, concrete geometry, duration lowering) → no task, out of scope per design §7. ✓

**Placeholder scan:** every code step shows full code; every run step has an exact command + expected output; values concrete (N=3, telemetry.len()==3, "50 mm", snapshot names). No TBD. ✓

**Type consistency:** `ReachHover { target: FrameRef, standoff: Quantity, duration: Option<serde_yaml::Value> }`; `lower_reach_hover(p, e)` matches the `lower` arm; `PoseExpr::FrameRelative { frame, offset }`; `EnvelopeClass::IntervalInvariant`; `envelope_class_for`/`suffix_of`/`check_envelope`/`Fault::MidIntervalDrop`/`RealizedPose::placeholder()` all match the read code; `Telemetry`/`Wrench` derive Clone (verified). ✓

**Golden scope:** Task 3 adds only `surface_scan_hover` snapshots; the multi-sample driver leaves `driver_protocol` byte-identical (Task 2 Step 5 guard); no descriptor change. ✓
