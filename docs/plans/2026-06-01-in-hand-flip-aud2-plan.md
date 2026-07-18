# in_hand.flip (AUD2) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement
> this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> **LOCAL-ONLY** — never `git add` this file.

**Goal:** Implement `in_hand.flip` (the first in_hand primitive) and AUD2 cross-action
`momentary_release` propagation — the first sequence-level conformance check.

**Architecture:** Add the `InHandFlip` primitive (struct + variant + lower + gate); the flip
emits no floor (grasp-continuity vacuous — continuity suspended). The `ReferenceDriver` declares
`momentary_release` on the flip (verdict.evidence) and propagates it into every downstream
action; `check_momentary_release(&[(goal,report)])` verifies the declaration + propagation. No
schema change. Design: `docs/design/2026-06-01-in-hand-flip-aud2-design.md`.
**Reconciliation:** the example lives in `examples/03-screw-fasten` (not 01, as the design said)
so the capability-absent test reuses cable-01 (which lacks `in_hand.flip`), like every prior
force increment.

**Tech Stack:** Rust (rfl-core / rfl-conformance), insta goldens, boon, `schemas/validate.py`.

**Discipline (every task):** TDD red→green. After green, run `cargo test` (workspace, NOT `-p`)
+ `uv run --with jsonschema --with pyyaml python schemas/validate.py` in a batch SEPARATE from
the commit. Before each push: `git fetch -q && git merge-base --is-ancestor origin/main HEAD`,
`git add <explicit paths>`, commit, `git push origin main`, verify sync `0 0` + `git show --stat`.
Stop-gate: reconcile and explain any golden diff vs the prediction.

---

### Task 1: the `InHandFlip` primitive (struct + variant + lower + gate) + example + golden

**Files:**
- Modify: `crates/rfl-core/src/skill_isa.rs` (Primitive enum; new struct near ForceCut)
- Modify: `crates/rfl-core/src/translation.rs` (use import; check_capability; lower dispatch; new lower fn; tests)
- Create: `examples/03-screw-fasten/skill-flip.yaml`
- Modify: `examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml` (skills list)
- Create: `crates/rfl-conformance/tests/flip.rs`

- [ ] **Step 1: Write the failing capability-gate test** — in `translation.rs` tests module,
near `cut_capability_absent_when_not_declared`:

```rust
    const FLIP_SKILL: &str = "skill: t\nbody:\n  sequence:\n    - in_hand.flip: { flip_axis: +x, angle: 180 deg, max_release_time: 0.3 s, safe_drop_zone: tray }\n";

    #[test]
    fn flip_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(FLIP_SKILL).unwrap();
        let emb = load("allegro").1; // cable-01 allegro lacks in_hand.flip
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: in_hand.flip"), "got {err}");
    }
```

- [ ] **Step 2: Run to verify it fails (unknown primitive)**

Run: `cargo test -p rfl-core flip_capability_absent 2>&1 | grep -E "error\[|panicked|test result|FAILED" | head`
Expected: FAIL — `in_hand.flip` deserializes to no `Primitive` variant; `Skill::parse_yaml` fails;
`.unwrap()` panics.

- [ ] **Step 3: Add the `InHandFlip` struct** — in `skill_isa.rs`, after the `ForceCut` struct:

```rust
/// `in_hand.flip` parameters (v0 subset of `$defs/InHandFlipParams`, § 3.7). The only
/// continuity-suspending primitive. `flip_axis` + `angle` + `max_release_time` + `safe_drop_zone`
/// are required. v0 lowers a symbolic reorientation about `flip_axis`; angle / max_release_time /
/// safe_drop_zone are carried symbolic (the bounded-window envelope geometry is deferred). The
/// momentary_release audit (AUD2) is verified in conformance, not lowered.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InHandFlip {
    /// Reorientation axis, in the grasp frame.
    pub flip_axis: Direction,
    /// Reorientation magnitude (carried symbolic in v0).
    pub angle: Quantity,
    /// Hard upper bound on the unsecured window (carried symbolic in v0).
    pub max_release_time: Quantity,
    /// Region below the operation where an uncaught object lands safely (carried symbolic; mandatory).
    pub safe_drop_zone: serde_yaml::Value,
}
```

- [ ] **Step 4: Add the Primitive variant** — in `skill_isa.rs` `enum Primitive`, after the
`ForceCut` variant:

```rust
    /// `in_hand.flip`.
    #[serde(rename = "in_hand.flip")]
    InHandFlip(InHandFlip),
```

- [ ] **Step 5: Wire the import + capability gate + lower dispatch** — in `translation.rs`:

In the `use crate::skill_isa::{...}` block, add `InHandFlip` to the import list.

In `check_capability`, after the `Primitive::ForceCut(_)` arm:

```rust
        Primitive::InHandFlip(_) => "in_hand.flip",
```

In the `lower` dispatch `match prim`, after the `Primitive::ForceCut(p)` arm:

```rust
        Primitive::InHandFlip(p) => (lower_in_hand_flip(p, e), "flip"),
```

- [ ] **Step 6: Add the lowering fn** — in `translation.rs`, after `lower_force_cut` (before
`actuation_is_detent`):

```rust
/// Lower `in_hand.flip` (`spec/01` § 3.7): the only continuity-suspending primitive — a large
/// reorientation through a bounded, recoverable unsecured window. v0 lowers a symbolic
/// reorientation about `flip_axis` in the grasp frame and emits **no** force_profile floor (the
/// securing floor is intentionally suspended during the window, so the grasp-continuity check is
/// vacuous for it). `ctx` is unchanged — the flip re-secures, so a prior held object persists for
/// the downstream release. The bounded-window envelope (max_release_time / safe_drop_zone) and
/// the momentary_release audit (AUD2) are handled elsewhere (deferred / conformance).
fn lower_in_hand_flip(p: &InHandFlip, e: &Embodiment) -> CanonicalAction {
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::AxisRelative {
            direction: yaml_to_json(&p.flip_axis),
            distance: Quantity("0 mm".to_string()),
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

- [ ] **Step 7: Run the capability test (green)**

Run: `cargo test -p rfl-core flip_capability_absent 2>&1 | grep -E "test result"`
Expected: PASS.

- [ ] **Step 8: Create the example skill** — `examples/03-screw-fasten/skill-flip.yaml`:

```yaml
# Example 03 — Object flip (in_hand.flip variant)
# A 180 deg reorientation requiring a momentary release (in_hand.flip is the only continuity-
# suspending primitive). Exercises AUD2 (spec/05): the flip declares momentary_release and the
# flag is propagated downstream so the audit trail traces the continuity break — here into the
# subsequent grasp.release.
skill: object-flip
description: >
  Grasp a part, flip it 180 deg (a momentary release), then release — the continuity break is
  declared and propagated to the downstream release.

objects:
  part: { ref: part, estimated_mass: 1.0 N }

body:
  sequence:

    - let: part_t
      from:
        sense.locate:
          target_ref: part
          modality: auto

    - grasp.pinch:
        target: part_t
        force_budget: 8 N
        tactile_target: auto

    - in_hand.flip:
        flip_axis: +x
        angle: 180 deg
        max_release_time: 0.3 s
        safe_drop_zone: tray

    - grasp.release:
        grasp_handle: active
```

- [ ] **Step 9: Declare the capability on the 3 descriptors** — in each of
`examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml`, add `in_hand.flip` to
the `skills:` array (read each line first; insert before `sense.locate`). For allegro the line
becomes:

```yaml
    skills: [grasp.pinch, grasp.lateral, grasp.envelope, transport, force.insert_fit, force.screw, force.unscrew, force.press_button, force.wipe, force.snap_engage, force.cut, in_hand.flip, sense.locate]
```

- [ ] **Step 10: Add the positive lowering test** — in `translation.rs` tests module:

```rust
    #[test]
    fn flip_lowers_no_floor() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill-flip.yaml")).unwrap()).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .unwrap();
        let out = retarget(&skill, &emb).expect("retarget");
        // locate(0), pinch(1), flip(2), release(3).
        assert_eq!(out.suffixes[2], "flip");
        // the flip suspends continuity -> no force_profile floor emitted.
        assert!(out.actions[2].safety_envelope.force_profile.is_none());
    }
```

- [ ] **Step 11: Run both translation tests (green)**

Run: `cargo test -p rfl-core flip 2>&1 | grep -E "test result|flip"`
Expected: both PASS.

- [ ] **Step 12: Create the golden test** — `crates/rfl-conformance/tests/flip.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for in_hand.flip: the retarget output is byte-deterministic, matches
//! a committed golden, and every line is a valid driver-interface execute message. (The AUD2
//! momentary_release propagation is in envelope_conformance.rs.)

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-flip.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_flip_allegro() {
    insta::assert_snapshot!("flip_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_flip_leap() {
    insta::assert_snapshot!("flip_leap", jsonl_for("leap"));
}

#[test]
fn golden_flip_pneumatic() {
    insta::assert_snapshot!("flip_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn flip_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn flip_every_line_is_a_valid_execute_message() {
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

Run: `INSTA_UPDATE=always cargo test -p rfl-conformance --test flip 2>&1 | grep -E "test result|error\["`
Then: `cat crates/rfl-conformance/tests/snapshots/flip__flip_allegro.snap`
Expected (STOP-GATE): four execute lines (`0001-locate`, `0002-pinch`, `0003-flip`, `0004-release`).
The **`0003-flip`** line is the new one: `target_frame":"tcp_thumb"` (grasp frame),
`target_pose":{"direction":"+x","distance":"0 mm"}`, NO `force_budget`, NO `monitors`,
`safety_envelope` with `motion_bounds` + `stop_time` and **no `force_profile`** (no floor),
`timing_mode":"strict"`. The locate / pinch / release lines are the standard reused lowerings
(pinch on allegro = manifold tactile, carries its min_holding_force floor). Verify the flip line
matches; the other three are the known reused outputs. Reconcile anything unexpected.

- [ ] **Step 14: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `rfl-core` 83 (was 81, +2); new `flip` golden binary 5 tests; existing goldens
byte-identical. `validate.py` PASS.

- [ ] **Step 15: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-core/src/skill_isa.rs crates/rfl-core/src/translation.rs examples/03-screw-fasten/skill-flip.yaml examples/03-screw-fasten/embodiments/allegro.yaml examples/03-screw-fasten/embodiments/leap.yaml examples/03-screw-fasten/embodiments/pneumatic-6f.yaml crates/rfl-conformance/tests/flip.rs crates/rfl-conformance/tests/snapshots && \
git commit -m "feat(core): add in_hand.flip primitive (continuity-suspending)

InHandFlip struct + Primitive variant + capability gate + lowering: the first
in_hand primitive and the only continuity-suspending op. Lowers a symbolic
reorientation about flip_axis in the grasp frame and emits NO force_profile
floor (the securing floor is intentionally suspended). ctx unchanged (re-
secured). Window geometry / safe_drop_zone deferred. examples/03 skill-flip.yaml
(locate->pinch->flip->release) + the capability on 3 descriptors + a golden.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head -15
```
Verify: sync `0 0`; only the listed files.

---

### Task 2: conformance — `momentary_release` propagation + `check_momentary_release`

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (ReferenceDriver state + echo; envelope_class_for;
  check_momentary_release; FlipDriver; lib unit test)
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (3 tests + imports)

- [ ] **Step 1: Write the failing integration tests** — append to `envelope_conformance.rs`
(reuse `screw_dir()` + `suffix_of()`):

```rust
// --- AUD2 momentary_release propagation (in_hand.flip, spec/05) ------------------------------

#[test]
fn nominal_flip_declares_and_propagates() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-flip.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // locate(0), pinch(1), flip(2), release(3).
    assert_eq!(suffix_of(&pairs[2].0.action_id), "flip");
    assert_eq!(suffix_of(&pairs[3].0.action_id), "release");
    assert_eq!(check_momentary_release(&pairs), CheckOutcome::Pass);
}

#[test]
fn flip_suppressed_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FlipDriver::new(FlipResponse::SuppressesFlip),
        &dir.join("skill-flip.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // the flip omits momentary_release -> transparency violation.
    assert!(matches!(check_momentary_release(&pairs), CheckOutcome::Fail(_)));
}

#[test]
fn flip_propagation_dropped_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FlipDriver::new(FlipResponse::DropsDownstream),
        &dir.join("skill-flip.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // the flip declares it but the downstream release drops the propagated flag -> the
    // sequence-level bite (each report looks fine in isolation).
    assert!(matches!(check_momentary_release(&pairs), CheckOutcome::Fail(_)));
}
```

Update the `use rfl_conformance::{...}` block to add `check_momentary_release, FlipDriver,
FlipResponse`.

- [ ] **Step 2: Run to verify it fails (unresolved imports)**

Run: `cargo test -p rfl-conformance --test envelope_conformance flip 2>&1 | grep -E "error\[|unresolved|test result" | head`
Expected: compile error — `check_momentary_release` / `FlipDriver` / `FlipResponse` not found.

- [ ] **Step 3: Add `momentary_release_seen` to the ReferenceDriver + propagate** — in `lib.rs`:

Change the struct:
```rust
#[derive(Debug, Default)]
pub struct ReferenceDriver {
    step: u32,
    momentary_release_seen: bool,
}
```

In `ReferenceDriver::execute`, in the `evidence` block (after the `held_confirmed` push), add:
```rust
        // AUD2 (in_hand.flip, spec/05): the flip declares momentary_release and the flag
        // propagates into every downstream action's audit record (the first cross-action state).
        let is_flip = suffix_of(&goal.action_id) == "flip";
        if is_flip || self.momentary_release_seen {
            evidence.push("momentary_release".to_string());
        }
        if is_flip {
            self.momentary_release_seen = true;
        }
```

- [ ] **Step 4: Add `flip` to `envelope_class_for`** — in `lib.rs`, the GraspContinuity arm:

```rust
        "pinch" | "release" | "transport" | "flip" => Some(EnvelopeClass::GraspContinuity),
```

- [ ] **Step 5: Add `check_momentary_release`** — in `lib.rs`, after `check_audit_honesty`:

```rust
/// Verify the AUD2 obligation (`spec/05`): the continuity-suspending `in_hand.flip` declares
/// `momentary_release`, and the flag is **propagated** to every downstream action's audit record
/// (so the L4 / L8 loops trace the continuity break several primitives later). The FIRST
/// sequence-level check — a pure function of the whole `drive()` sequence, not a single pair.
/// Vacuous when the sequence contains no flip.
#[must_use]
pub fn check_momentary_release(pairs: &[(ExecuteGoal, DriverReport)]) -> CheckOutcome {
    let declares = |r: &DriverReport| {
        r.status.verdict.as_ref().is_some_and(|v| v.evidence.iter().any(|e| e == "momentary_release"))
    };
    let Some(flip_idx) = pairs.iter().position(|(g, _)| suffix_of(&g.action_id) == "flip") else {
        return CheckOutcome::Pass; // no continuity break -> nothing to trace
    };
    if !declares(&pairs[flip_idx].1) {
        return CheckOutcome::Fail(
            "in_hand.flip did not declare momentary_release (transparency violation)".to_string(),
        );
    }
    for (g, r) in &pairs[flip_idx + 1..] {
        if !declares(r) {
            return CheckOutcome::Fail(format!(
                "momentary_release not propagated to downstream action {}",
                g.action_id
            ));
        }
    }
    CheckOutcome::Pass
}
```

- [ ] **Step 6: Add `FlipResponse` + `FlipDriver`** — in `lib.rs`, after the `CutDriver` impl:

```rust
/// How a driver reports an `in_hand.flip` sequence (`spec/05` AUD2). `Propagates` is conformant
/// (the flip declares momentary_release and it propagates downstream); `SuppressesFlip` omits the
/// flag on the flip; `DropsDownstream` keeps it on the flip but strips it from later actions.
#[derive(Debug, Clone, Copy)]
pub enum FlipResponse {
    /// Conformant: declared on the flip and propagated downstream.
    Propagates,
    /// Adversarial: the flip omits momentary_release (claims continuity preserved).
    SuppressesFlip,
    /// Adversarial: declared on the flip but dropped from every downstream action.
    DropsDownstream,
}

/// The `in_hand.flip` AUD2 bench: reuses the nominal `ReferenceDriver` (which declares +
/// propagates momentary_release) and mutates the audit trail per `response`.
#[derive(Debug, Default)]
pub struct FlipDriver {
    inner: ReferenceDriver,
    response_set: bool,
    response: Option<FlipResponse>,
}

impl FlipDriver {
    /// A flip driver with the given audit-trail response.
    #[must_use]
    pub fn new(response: FlipResponse) -> Self {
        FlipDriver { inner: ReferenceDriver::default(), response_set: true, response: Some(response) }
    }
}

impl Driver for FlipDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let _ = self.response_set;
        let mut report = self.inner.execute(goal);
        let is_flip = suffix_of(&goal.action_id) == "flip";
        let strip = |report: &mut DriverReport| {
            if let Some(v) = report.status.verdict.as_mut() {
                v.evidence.retain(|e| e != "momentary_release");
            }
        };
        match self.response {
            Some(FlipResponse::Propagates) | None => {} // passthrough
            Some(FlipResponse::SuppressesFlip) => {
                if is_flip {
                    strip(&mut report);
                }
            }
            Some(FlipResponse::DropsDownstream) => {
                if !is_flip {
                    strip(&mut report); // pre-flip actions have nothing to strip; post-flip lose the propagated flag
                }
            }
        }
        report
    }
}
```

(Note: the `response_set` field is unnecessary — simplify `FlipDriver` to `{ inner, response:
FlipResponse }` and `new(response)` setting it directly; drop the `let _ = self.response_set;`
line and the `Option`. Use:)
```rust
#[derive(Debug)]
pub struct FlipDriver {
    inner: ReferenceDriver,
    response: FlipResponse,
}
impl FlipDriver {
    #[must_use]
    pub fn new(response: FlipResponse) -> Self {
        FlipDriver { inner: ReferenceDriver::default(), response }
    }
}
impl Driver for FlipDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        let is_flip = suffix_of(&goal.action_id) == "flip";
        let strip = |report: &mut DriverReport| {
            if let Some(v) = report.status.verdict.as_mut() {
                v.evidence.retain(|e| e != "momentary_release");
            }
        };
        match self.response {
            FlipResponse::Propagates => {}
            FlipResponse::SuppressesFlip => {
                if is_flip {
                    strip(&mut report);
                }
            }
            FlipResponse::DropsDownstream => {
                if !is_flip {
                    strip(&mut report);
                }
            }
        }
        report
    }
}
```
(Use the simplified form; the first block is superseded.)

- [ ] **Step 7: Add the lib unit test** — in `lib.rs` tests module, after
`check_audit_honesty_rejects_undisclosed_degradation`:

```rust
    #[test]
    fn check_momentary_release_requires_declaration_and_propagation() {
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        let pair = |suffix: &str, momentary: bool| {
            let mut evidence = vec!["nominal".to_string()];
            if momentary {
                evidence.push("momentary_release".to_string());
            }
            let id = format!("s/e/0001-{suffix}");
            let goal = ExecuteGoal::wrap(id.clone(), super::tests::sample_action());
            let report = DriverReport {
                telemetry: vec![],
                status: Status {
                    message: "status",
                    action_id: id,
                    outcome: Outcome::Succeeded,
                    verdict: Some(Verdict { value: true, confidence: 1.0, evidence }),
                    fidelity_tier: None,
                    final_pose: Some(RealizedPose::placeholder()),
                    failure_class: None,
                    failure_detail: None,
                },
            };
            (goal, report)
        };
        // flip declares + downstream release carries -> Pass.
        let ok = vec![pair("pinch", false), pair("flip", true), pair("release", true)];
        assert_eq!(check_momentary_release(&ok), CheckOutcome::Pass);
        // flip omits the flag -> Fail.
        let suppressed = vec![pair("flip", false), pair("release", false)];
        assert!(matches!(check_momentary_release(&suppressed), CheckOutcome::Fail(_)));
        // flip declares but downstream dropped -> Fail (the sequence-level bite).
        let dropped = vec![pair("flip", true), pair("release", false)];
        assert!(matches!(check_momentary_release(&dropped), CheckOutcome::Fail(_)));
        // no flip -> vacuous Pass.
        let no_flip = vec![pair("pinch", false), pair("release", false)];
        assert_eq!(check_momentary_release(&no_flip), CheckOutcome::Pass);
    }
```

Note: `sample_action()` is the existing test helper in the `tests` module (used by other lib
unit tests); reference it as `sample_action()` directly (same module) rather than
`super::tests::sample_action()`. If the borrow/scope differs, inline a minimal `CanonicalAction`
as the other unit tests do.

- [ ] **Step 8: Run the new tests (green)**

Run: `cargo test -p rfl-conformance flip 2>&1 | grep -E "test result|flip|momentary"` then
`cargo test -p rfl-conformance check_momentary_release 2>&1 | grep -E "test result|momentary"`.
Expected: the 3 integration tests + the lib unit test PASS.

- [ ] **Step 9: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `rfl-conformance` lib 16 (was 15, +1), `envelope_conformance` 44 (was 41, +3).
Existing goldens/tests unaffected (no other skill contains a flip, so `momentary_release_seen`
stays false and no evidence changes; driver_protocol goldens unchanged).

- [ ] **Step 10: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs && \
git commit -m "feat(conformance): verify in_hand.flip momentary_release propagation (AUD2)

envelope_class_for(flip)=>GraspContinuity (vacuous floor — continuity
suspended) + ReferenceDriver declares momentary_release on the flip and
propagates it into every downstream action's audit record (momentary_release_
seen — the first cross-action driver state) + check_momentary_release, the
first SEQUENCE-level check. FlipDriver{Propagates,SuppressesFlip,Drops
Downstream}: DropsDownstream is the bite only a sequence-level check catches.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `lib.rs` + `envelope_conformance.rs`.

---

### Task 3: README status line

**Files:**
- Modify: `README.md` (~line 84)

- [ ] **Step 1: Extend the README status line** — add `in_hand.flip` to the examples listing and
note it opens AUD2: the only continuity-suspending primitive declares `momentary_release` and the
flag is propagated downstream into the audit trail (the first cross-action / sequence-level
conformance check). Match the surrounding phrasing. (spec/05 AUD2 already states the obligation,
so no spec change.)

- [ ] **Step 2: Verify nothing regressed**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: unchanged green.

- [ ] **Step 3: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add README.md && \
git commit -m "docs(readme): note in_hand.flip momentary_release propagation (AUD2)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `README.md`.

---

## Post-increment

- Final full-suite read: expected `rfl-core` 83, `rfl-conformance` lib 16, `driver_protocol` 9,
  `envelope_conformance` 44, new `flip` 5, others unchanged.
- Update `project_rfl.md` "Implementation track" (21st increment, real hashes + test deltas) +
  the `MEMORY.md` RFL line. Move deferred items (continuity-exception window/safe_drop_zone
  geometry, catch_envelope, target_mode, continuity_alternative_exists, AUD1, AUD3 freed-part,
  the rest of the in_hand category) into the parked list.

## Self-review (spec coverage)

- AUD2 cross-action propagation (design § 1, § 3) → Task 2 (`check_momentary_release` +
  ReferenceDriver propagation). ✓
- in_hand.flip primitive + no-floor lowering (design § 3, § 4) → Task 1. ✓
- DropsDownstream sequence-level non-vacuity (design § 5) → Task 2. ✓
- README (design § 6) → Task 3. ✓
- No schema change (design § 2) → confirmed: descriptors gain a capability token (data). ✓
- Type consistency: `InHandFlip{flip_axis: Direction, angle: Quantity, max_release_time:
  Quantity, safe_drop_zone: Value}`; `momentary_release` in verdict.evidence written by the
  ReferenceDriver (flip + propagation) and FlipDriver, read by `check_momentary_release`;
  `FlipResponse{Propagates,SuppressesFlip,DropsDownstream}` used in lib.rs + envelope_conformance.rs;
  `envelope_class_for("flip")=>GraspContinuity`. ✓
```
