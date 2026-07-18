# Envelope-class checkers + adversarial drivers (Test Class 3 — C2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Class 3 verification non-circular — implement the envelope-class checkers (ENV1) that judge a driver report against each primitive's envelope, and prove with adversarial drivers that each checker rejects a violation.

**Architecture:** All in `rfl-conformance`. A generic `drive<D: Driver>` helper returns `(ExecuteGoal, DriverReport)` pairs (the checker needs the commanded bounds + the report). An `EnvelopeClass` enum + ENV1 `envelope_class_for(suffix)` mapping; `check_envelope(class, goal, report) -> CheckOutcome` with three pure checkers (terminal structural, grasp-continuity GC1 numeric, force-trajectory ENV4 numeric). A `FaultyDriver` wraps the nominal `ReferenceDriver` and injects one schema-valid-but-violating fault. A conformance test where nominal passes every checker and each fault is rejected by its checker.

**Tech Stack:** Rust (workspace `rfl-conformance`, edition 2024, MSRV 1.85, cargo 1.96 via rustup), `serde_json`, `anyhow`. Python `validate.py` via `uv`.

**Design doc:** `docs/design/2026-05-31-envelope-checkers-c2-design.md` (committed, `e33959f`).

**Standing rules (every task):**
- Prefix every cargo command with `export PATH="$HOME/.cargo/bin:$PATH"`.
- **Validation read and `git commit` MUST be separate batches.** Run tests, READ `ok`/`PASS`, then stage + commit in a later message.
- `git add` explicit paths only — never `-A` (keeps `docs/plans/` out).
- Before commit: branch == `main`. Before push: `git fetch -q origin && git merge-base --is-ancestor origin/main HEAD`. After push: `git rev-list --left-right --count origin/main...HEAD` == `0 0`. Rebase onto `origin/main` if ff fails. No `--force`, no `--no-verify`.
- Conventional Commits + `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. Keep `cargo test` warning-clean.

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `crates/rfl-conformance/src/lib.rs` | `drive` helper + `EnvelopeClass`/`envelope_class_for` + `CheckOutcome`/`check_envelope` (T1); `FaultyDriver`/`Fault` (T2) | 1, 2 |
| `crates/rfl-conformance/tests/envelope_conformance.rs` (new) | nominal-pass + 3 adversarial-bite tests | 3 |

No `rfl-core`, `spec/`, or `schemas/` changes. No new goldens (pass/fail assertions). The C1 `driver_protocol.rs` + the other suites stay green.

---

## Task 1: `drive` helper + envelope-class mapping + the three checkers

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (add `drive`, refactor `run_reference_driver`, add the envelope types + checkers, add unit tests)

- [ ] **Step 1: Write the failing unit tests** — add to the existing `#[cfg(test)] mod tests` in `crates/rfl-conformance/src/lib.rs` (after `reference_driver_succeeds_and_echoes_fidelity`):

```rust
    #[test]
    fn envelope_class_mapping_follows_env1() {
        use rfl_core::driver::Outcome as _;
        assert_eq!(envelope_class_for("align"), Some(EnvelopeClass::TerminalPostcondition));
        assert_eq!(envelope_class_for("retract"), Some(EnvelopeClass::TerminalPostcondition));
        assert_eq!(envelope_class_for("pinch"), Some(EnvelopeClass::GraspContinuity));
        assert_eq!(envelope_class_for("transport"), Some(EnvelopeClass::GraspContinuity));
        assert_eq!(envelope_class_for("insert_fit"), Some(EnvelopeClass::ForceTrajectory));
        assert_eq!(envelope_class_for("locate"), None); // sense: perception
        assert_eq!(envelope_class_for("inspect"), None);
    }

    #[test]
    fn nominal_grasp_continuity_passes_and_under_secure_fails() {
        let dir = example_dir();
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // index 1 is grasp.pinch (force_profile.min_holding_force present).
        let (goal, report) = &pairs[1];
        assert_eq!(check_envelope(EnvelopeClass::GraspContinuity, goal, report), CheckOutcome::Pass);
        // Lower the securing force below the floor -> the checker must reject.
        let mut bad = report.clone();
        bad.telemetry[0].securing_force = Some(rfl_core::quantity::Quantity("0.1 N".to_string()));
        assert!(matches!(
            check_envelope(EnvelopeClass::GraspContinuity, goal, &bad),
            CheckOutcome::Fail(_)
        ));
    }

    #[test]
    fn force_trajectory_passes_nominal_and_fails_over_budget() {
        let dir = example_dir();
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // index 5 is force.insert_fit.
        let (goal, report) = &pairs[5];
        assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
        let mut bad = report.clone();
        if let Some(w) = bad.telemetry[0].wrench.as_mut() {
            w.force = [0.0, 0.0, 999.0];
        }
        assert!(matches!(
            check_envelope(EnvelopeClass::ForceTrajectory, goal, &bad),
            CheckOutcome::Fail(_)
        ));
    }

    #[test]
    fn terminal_postcondition_passes_nominal_and_fails_indeterminate() {
        let dir = example_dir();
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // index 4 is reach.align.
        let (goal, report) = &pairs[4];
        assert_eq!(check_envelope(EnvelopeClass::TerminalPostcondition, goal, report), CheckOutcome::Pass);
        let mut bad = report.clone();
        bad.status.outcome = rfl_core::driver::Outcome::Indeterminate;
        bad.status.final_pose = None;
        assert!(matches!(
            check_envelope(EnvelopeClass::TerminalPostcondition, goal, &bad),
            CheckOutcome::Fail(_)
        ));
    }
```

(The `use rfl_core::driver::Outcome as _;` line in the mapping test is unused noise — remove it; it is left out of the implementation. The other three tests reference `drive` / `EnvelopeClass` / `check_envelope` / `CheckOutcome`, none of which exist yet → red.)

Correction: do **not** include that `use` line. Use exactly the four test functions above with the `use` line omitted from `envelope_class_mapping_follows_env1`.

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance envelope 2>&1 | grep -E "cannot find|error\[|test result" | head`
Expected: FAIL — `cannot find function 'drive'` / `cannot find ... EnvelopeClass` / `check_envelope` / `CheckOutcome`.

- [ ] **Step 3: Add the `drive` helper + refactor `run_reference_driver`** — in `crates/rfl-conformance/src/lib.rs`, replace the existing `run_reference_driver` function with:

```rust
/// Retarget the skill onto the embodiment and drive every `execute` message through
/// `driver`, returning the `(goal, report)` pair per action. Generic over any
/// `Driver` (the nominal `ReferenceDriver` or a `FaultyDriver`). Action ids match the
/// `{skill}/{embodiment_id}/{NNNN}-{suffix}` form `canonical::to_jsonl` emits.
///
/// # Errors
/// Propagates parse / retarget errors.
pub fn drive<D: Driver>(
    mut driver: D,
    skill_path: &Path,
    embodiment_path: &Path,
) -> anyhow::Result<Vec<(ExecuteGoal, DriverReport)>> {
    let skill = rfl_core::skill_isa::Skill::parse_yaml(&std::fs::read_to_string(skill_path)?)?;
    let emb =
        rfl_core::embodiment::Embodiment::parse_yaml(&std::fs::read_to_string(embodiment_path)?)?;
    let out = rfl_core::translation::retarget(&skill, &emb)?;
    let mut pairs = Vec::new();
    for (i, (action, suffix)) in out.actions.iter().zip(&out.suffixes).enumerate() {
        let action_id = format!("{}/{}/{:04}-{}", skill.skill, emb.id, i + 1, suffix);
        let goal = ExecuteGoal::wrap(action_id, action.clone());
        let report = driver.execute(&goal);
        pairs.push((goal, report));
    }
    Ok(pairs)
}

/// Drive the example with the nominal `ReferenceDriver`, returning the reports
/// (goals dropped) for golden / JSONL rendering.
///
/// # Errors
/// Propagates parse / retarget errors.
pub fn run_reference_driver(
    skill_path: &Path,
    embodiment_path: &Path,
) -> anyhow::Result<Vec<DriverReport>> {
    Ok(drive(ReferenceDriver::default(), skill_path, embodiment_path)?
        .into_iter()
        .map(|(_, report)| report)
        .collect())
}
```

- [ ] **Step 4: Add the envelope-class mapping + checkers** — in `crates/rfl-conformance/src/lib.rs`, insert after `reports_to_jsonl` (before the `#[cfg(test)]` module):

```rust
/// The conformance envelope class a primitive is verified against (`spec/05` ENV1).
/// Interval-invariant is omitted in v0 (no cable primitive — `reach.hover` /
/// `transport.carry` — exercises it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeClass {
    /// `reach.*` (except hover): the end state, pose at rest, endpoint only.
    TerminalPostcondition,
    /// `grasp.*` / `in_hand.*` / `transport.*` / `place.*`: securing force >= floor.
    GraspContinuity,
    /// `force.*`: the force/torque profile against per-axis budgets.
    ForceTrajectory,
}

/// Map an action-id suffix to its envelope class (`spec/05` ENV1). `sense.*`
/// (locate / inspect) has no motion envelope; the interval-invariant primitives
/// (hover / carry) are not in the v0 worked example.
#[must_use]
pub fn envelope_class_for(suffix: &str) -> Option<EnvelopeClass> {
    match suffix {
        "align" | "retract" | "scan" => Some(EnvelopeClass::TerminalPostcondition),
        "pinch" | "release" | "transport" => Some(EnvelopeClass::GraspContinuity),
        "insert_fit" => Some(EnvelopeClass::ForceTrajectory),
        _ => None, // locate / inspect: perception, no envelope
    }
}

/// The result of an envelope-class check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    /// The report conforms to the envelope.
    Pass,
    /// The report violates the envelope, with a human-readable reason.
    Fail(String),
}

/// Magnitude of a `"<x> <unit>"` quantity.
fn quantity_mag(q: &rfl_core::quantity::Quantity) -> Option<f64> {
    q.parse().map(|(v, _)| v)
}

/// Verify a driver report against the action's envelope class (`spec/05` ENV1–ENV4,
/// GC1). A pure function of the commanded `execute` goal and the returned report.
#[must_use]
pub fn check_envelope(
    class: EnvelopeClass,
    goal: &ExecuteGoal,
    report: &DriverReport,
) -> CheckOutcome {
    match class {
        EnvelopeClass::TerminalPostcondition => {
            if !matches!(report.status.outcome, Outcome::Succeeded) {
                return CheckOutcome::Fail(format!(
                    "outcome not succeeded: {:?}",
                    report.status.outcome
                ));
            }
            if report.status.final_pose.is_none() {
                return CheckOutcome::Fail("no final_pose at rest".to_string());
            }
            CheckOutcome::Pass
        }
        EnvelopeClass::GraspContinuity => {
            // GF1c floor from the execute message's force_profile, if present.
            let floor = goal
                .canonical_action
                .safety_envelope
                .force_profile
                .as_ref()
                .and_then(|fp| fp.get("min_holding_force"))
                .and_then(serde_json::Value::as_str)
                .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v));
            let Some(floor) = floor else {
                return CheckOutcome::Pass; // transport / release carry no floor in v0
            };
            for t in &report.telemetry {
                if let Some(sf) = t.securing_force.as_ref().and_then(quantity_mag) {
                    if sf < floor {
                        return CheckOutcome::Fail(format!(
                            "securing_force {sf} < min_holding_force {floor}"
                        ));
                    }
                }
            }
            CheckOutcome::Pass
        }
        EnvelopeClass::ForceTrajectory => {
            let Some(budget) = goal.canonical_action.force_budget.as_ref().and_then(quantity_mag)
            else {
                return CheckOutcome::Pass; // no budget claimed
            };
            for t in &report.telemetry {
                if let Some(w) = &t.wrench {
                    let mag = w.force.iter().map(|x| x * x).sum::<f64>().sqrt();
                    if mag > budget {
                        return CheckOutcome::Fail(format!("|wrench.force| {mag} > budget {budget}"));
                    }
                }
            }
            CheckOutcome::Pass
        }
    }
}
```

- [ ] **Step 5: Run the unit tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance 2>&1 | grep -E "warning:|test result|FAILED|error" | head`
Expected: `test result: ok` (the 4 new lib tests + the C1 `reference_driver_succeeds_and_echoes_fidelity` + the integration suites); no warnings.

- [ ] **Step 6: Commit** (separate batch from Step 5)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-conformance/src/lib.rs && git commit -m "feat(conformance): add envelope-class checkers + drive helper (Class 3 C2)

EnvelopeClass + ENV1 envelope_class_for mapping + check_envelope (terminal
structural, grasp-continuity GC1 securing_force >= min_holding_force,
force-trajectory |wrench| <= budget). Generic drive<D: Driver> returns
(goal, report) pairs; run_reference_driver refactored onto it.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: the adversarial `FaultyDriver`

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (add `Fault` + `FaultyDriver` + a unit test)

- [ ] **Step 1: Write the failing test** — add to `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn faulty_under_secure_lowers_securing_force() {
        let dir = example_dir();
        let pairs = drive(
            FaultyDriver::new(Fault::UnderSecure),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // grasp.pinch (index 1) has a securing_force -> lowered to the violating value.
        assert_eq!(pairs[1].1.telemetry[0].securing_force.as_ref().unwrap().0, "0.1 N");
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance faulty_under_secure 2>&1 | grep -E "cannot find|error\[|test result" | head`
Expected: FAIL — `cannot find ... FaultyDriver` / `Fault`.

- [ ] **Step 3: Add `Fault` + `FaultyDriver`** — in `crates/rfl-conformance/src/lib.rs`, insert after the `ReferenceDriver` `impl Driver` block (before `run_reference_driver` or `drive`):

```rust
/// A fault a `FaultyDriver` injects into the nominal report — schema-valid but
/// envelope-violating, to verify the checkers reject violations (the non-circular
/// proof of Test Class 3).
#[derive(Debug, Clone, Copy)]
pub enum Fault {
    /// `securing_force` below any derived floor (violates grasp-continuity GC1).
    UnderSecure,
    /// `wrench.force` above any budget (violates the force-trajectory bound, ENV4).
    OverForce,
    /// `outcome = indeterminate`, no `final_pose` (violates terminal-postcondition).
    NeverSettle,
}

/// Wraps the nominal `ReferenceDriver` and injects one `Fault` into every report it
/// returns. The report still validates against the driver-interface schema; the
/// violation is semantic (caught by `check_envelope`, not the schema).
#[derive(Debug)]
pub struct FaultyDriver {
    inner: ReferenceDriver,
    fault: Fault,
}

impl FaultyDriver {
    /// A faulty driver injecting `fault`.
    #[must_use]
    pub fn new(fault: Fault) -> Self {
        FaultyDriver { inner: ReferenceDriver::default(), fault }
    }
}

impl Driver for FaultyDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        match self.fault {
            Fault::UnderSecure => {
                for t in &mut report.telemetry {
                    if t.securing_force.is_some() {
                        t.securing_force =
                            Some(rfl_core::quantity::Quantity("0.1 N".to_string()));
                    }
                }
            }
            Fault::OverForce => {
                for t in &mut report.telemetry {
                    if let Some(w) = t.wrench.as_mut() {
                        w.force = [0.0, 0.0, 999.0];
                    }
                }
            }
            Fault::NeverSettle => {
                report.status.outcome = Outcome::Indeterminate;
                report.status.final_pose = None;
            }
        }
        report
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance faulty_under_secure 2>&1 | grep -E "test result|FAILED" | head`
Expected: PASS.

- [ ] **Step 5: Confirm warning-clean**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance 2>&1 | grep -E "warning:|test result|FAILED" | tail`
Expected: all `ok`, no warnings.

- [ ] **Step 6: Commit** (separate batch)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-conformance/src/lib.rs && git commit -m "feat(conformance): add fault-injecting FaultyDriver (Class 3 C2)

FaultyDriver wraps the nominal ReferenceDriver and injects one schema-valid but
envelope-violating fault: UnderSecure (securing_force 0.1 N), OverForce (wrench
999 N), NeverSettle (indeterminate / no final_pose). Used to prove the checkers
reject violations.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Class 3 C2 conformance test (nominal pass + adversarial bite)

**Files:**
- Create: `crates/rfl-conformance/tests/envelope_conformance.rs`

- [ ] **Step 1: Create the test file** — `crates/rfl-conformance/tests/envelope_conformance.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 3, part 2 (`spec/05` § The envelope-class taxonomy): the
//! envelope-class checkers JUDGE a driver report against each primitive's envelope.
//! The nominal driver conforms to every check; the adversarial `FaultyDriver`s are
//! REJECTED by the matching checker — the non-circular proof that the suite bites.

use rfl_conformance::{
    check_envelope, drive, envelope_class_for, CheckOutcome, EnvelopeClass, Fault, FaultyDriver,
    ReferenceDriver,
};
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

/// The primitive suffix of an action id (`.../NNNN-<suffix>`; suffixes contain no `-`).
fn suffix_of(action_id: &str) -> &str {
    action_id.rsplit('-').next().unwrap_or(action_id)
}

#[test]
fn nominal_driver_passes_every_envelope_check() {
    let dir = example_dir();
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join(format!("embodiments/{stem}.yaml")),
        )
        .expect("drive");
        for (goal, report) in &pairs {
            if let Some(class) = envelope_class_for(suffix_of(&goal.action_id)) {
                assert_eq!(
                    check_envelope(class, goal, report),
                    CheckOutcome::Pass,
                    "{stem} {}",
                    goal.action_id
                );
            }
        }
    }
}

#[test]
fn under_secure_driver_fails_grasp_continuity() {
    let dir = example_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::UnderSecure),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // grasp.pinch (index 1) carries min_holding_force -> GC1 must reject the lowered securing_force.
    let (goal, report) = &pairs[1];
    assert_eq!(suffix_of(&goal.action_id), "pinch");
    assert!(matches!(
        check_envelope(EnvelopeClass::GraspContinuity, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn over_force_driver_fails_force_trajectory() {
    let dir = example_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverForce),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // force.insert_fit (index 5) -> force-trajectory must reject the over-budget wrench.
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "insert_fit");
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn never_settle_driver_fails_terminal_postcondition() {
    let dir = example_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::NeverSettle),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // reach.align (index 4) -> terminal-postcondition must reject indeterminate / no final pose.
    let (goal, report) = &pairs[4];
    assert_eq!(suffix_of(&goal.action_id), "align");
    assert!(matches!(
        check_envelope(EnvelopeClass::TerminalPostcondition, goal, report),
        CheckOutcome::Fail(_)
    ));
}
```

- [ ] **Step 2: Run the test**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test envelope_conformance 2>&1 | grep -E "test .* (ok|FAILED)|test result|panic" | head`
Expected: `test result: ok. 4 passed` — nominal passes every check; each faulty driver is rejected by its checker.

- [ ] **Step 3: Gating validation batch** (separate from the commit)

Run: `cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1 && export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "test result|FAILED|error|warning:" | tail -12`
Expected: validate.py `PASS`; every crate green (rfl-core 48 + conformance lib + `driver_protocol` 7 + `envelope_conformance` 4 + `retarget_determinism` 5 + `surface_scan` 5); no warnings.

- [ ] **Step 4: Commit** (separate batch — after reading PASS)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-conformance/tests/envelope_conformance.rs && git commit -m "test(conformance): Class 3 C2 envelope checks (nominal pass + adversarial bite)

The nominal ReferenceDriver passes every primitive's envelope-class check; the
UnderSecure / OverForce / NeverSettle FaultyDrivers are each REJECTED by the
matching checker (grasp-continuity / force-trajectory / terminal) -- the
non-circular proof that Class 3 verification bites.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Full verification + push + memory

**Files:** none (verification + integration).

- [ ] **Step 1: Final full suite + validate.py**

Run: `cd ~/Documents/GitHub/rfl && export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "test result|FAILED|error" && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: all green; validate.py `PASS`.

- [ ] **Step 2: Confirm pushes** (Tasks 1-3 each pushed per the standing rules)

Run: `cd ~/Documents/GitHub/rfl && git rev-parse --abbrev-ref HEAD && git fetch -q origin && git merge-base --is-ancestor origin/main HEAD && git push -q origin main; git rev-list --left-right --count origin/main...HEAD && git log --oneline -5`
Expected: branch `main`; final count `0 0`; the 3 C2 commits + the design-doc commit visible. (Rebase onto `origin/main` first if the ff-check fails.)

- [ ] **Step 3: Update the memory** (`~/.claude/.../memory/project_rfl.md` "## Implementation track") with the real commit hashes from `git log --oneline -6`, recording this as the 5th increment (Test Class 3 C2 — envelope-class checkers + adversarial drivers), and note that Class 3 (C1 + C2) is now complete for the cable example; remaining Class 3 work = interval-invariant + GC3/4/6 + transport propagation (await exercising primitives). Not a repo commit — memory only.

---

## Self-review (completed during planning)

- **Spec coverage:** design §3 mapping → Task 1 (`envelope_class_for`); §4 three checkers → Task 1 (`check_envelope`); §5 adversarial driver → Task 2 (`FaultyDriver`/`Fault`); §6 tests (nominal pass + adversarial bite) → Tasks 1 (unit) + 3 (integration); §2 deferrals (interval-invariant, transport propagation, GC3/4/6, ε-table) → explicitly out of scope, no task. No gaps.
- **Placeholder scan:** every code step shows complete code; commands show expected output. (The Step 1 note about the stray `use` line is a correction instruction, not a placeholder — the implementation omits it.)
- **Type consistency:** `EnvelopeClass{TerminalPostcondition,GraspContinuity,ForceTrajectory}`, `envelope_class_for`, `CheckOutcome{Pass,Fail}`, `check_envelope`, `drive`, `Fault{UnderSecure,OverForce,NeverSettle}`, `FaultyDriver::new`, `suffix_of` used identically across Tasks 1-3. The checkers read `goal.canonical_action.safety_envelope.force_profile`/`force_budget` and `report.status`/`telemetry` — fields defined in C1 (`rfl-core::driver`) and increment 3.
- **Determinism / correctness:** the nominal force magnitude equals the budget (both parsed from the same `"<x> N"` string), so `mag > budget` is false → nominal passes; the faulty `999 N` / `0.1 N` / indeterminate values cross the bounds → rejected.
