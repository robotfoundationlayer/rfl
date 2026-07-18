# Torque-trajectory checker — Class-3 ENV4 torque case (E3) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the Class-3 force-trajectory envelope checker to the torque case so a `force.screw` driver report is verified against its torque budget (`|wrench.torque| ≤ torque_budget`), with the `ReferenceDriver` echoing torque and an `OverTorque` adversarial proving the check rejects an over-budget torque.

**Architecture:** All in `rfl-conformance`. `check_envelope`'s `ForceTrajectory` arm gains a torque check (reads `force_profile.torque`, checks `wrench.torque`) alongside the existing force check. The `ReferenceDriver` echoes `force_profile.torque` into `wrench.torque`. A `Fault::OverTorque` injects an over-budget torque; two `envelope_conformance` tests drive `examples/03-screw-fasten` (nominal pass + adversarial bite).

**Tech Stack:** Rust (`rfl-conformance`, edition 2024, MSRV 1.85, cargo 1.96 via rustup), `serde_json`. Python `validate.py` via `uv`.

**Design doc:** `docs/design/2026-05-31-torque-trajectory-checker-e3-design.md` (committed, `33d8b77`).

**Standing rules (every task):**
- Prefix every cargo command with `export PATH="$HOME/.cargo/bin:$PATH"`.
- **Validation read and `git commit` MUST be separate batches.** Run tests, READ `ok`/`PASS`, then stage + commit later.
- `git add` explicit paths only — never `-A`. Before commit: branch == `main`. Before push: `git merge-base --is-ancestor origin/main HEAD`; after push: sync `0 0`. No `--force`/`--no-verify`.
- Conventional Commits + `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. Keep `cargo test` warning-clean.

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `crates/rfl-conformance/src/lib.rs` | `ForceTrajectory` torque check + `ReferenceDriver` torque echo (T1); `Fault::OverTorque` (T2) | 1, 2 |
| `crates/rfl-conformance/tests/envelope_conformance.rs` | nominal screw pass + OverTorque bite (T2) | 2 |

No `rfl-core`, `spec/`, or `schemas/` change. No goldens change (the cable `driver_protocol` goldens stay identical — cable has no `force.screw`; `screw_fasten` is a retarget golden not using the driver).

---

## Task 1: torque check + `ReferenceDriver` torque echo

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (`check_envelope` ForceTrajectory arm; `ReferenceDriver::execute` wrench; a lib unit test)

- [ ] **Step 1: Write the failing lib test** — in `crates/rfl-conformance/src/lib.rs` `mod tests`, add:

```rust
    #[test]
    fn nominal_screw_passes_torque_trajectory_and_echoes_torque() {
        let ex = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let pairs = drive(
            ReferenceDriver::default(),
            &ex.join("skill.yaml"),
            &ex.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // force.screw is action index 5 (locate, pinch, transport, locate, align, screw, ...).
        let (goal, report) = &pairs[5];
        // the driver echoes the clamped torque (0.2 N·m) into wrench.torque.
        assert!((report.telemetry[0].wrench.as_ref().unwrap().torque[2] - 0.2).abs() < 1e-9);
        assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
        // an over-budget torque is rejected.
        let mut bad = report.clone();
        bad.telemetry[0].wrench.as_mut().unwrap().torque = [0.0, 0.0, 999.0];
        assert!(matches!(
            check_envelope(EnvelopeClass::ForceTrajectory, goal, &bad),
            CheckOutcome::Fail(_)
        ));
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance nominal_screw_passes_torque_trajectory 2>&1 | grep -E "got |panicked|assertion|test result" | head`
Expected: FAIL — the driver does not echo torque yet, so `wrench.torque[2]` is `0.0` (the `0.2` assertion fails); and the checker would pass the over-budget report (no torque check), so the `Fail` assertion fails.

- [ ] **Step 3: Extend the `ForceTrajectory` checker** — in `crates/rfl-conformance/src/lib.rs`, replace the `EnvelopeClass::ForceTrajectory => { ... }` arm of `check_envelope` with:

```rust
        EnvelopeClass::ForceTrajectory => {
            // Force budget (linear) — when present (e.g. force.insert_fit).
            if let Some(budget) = goal.canonical_action.force_budget.as_ref().and_then(quantity_mag) {
                for t in &report.telemetry {
                    if let Some(w) = &t.wrench {
                        let mag = w.force.iter().map(|x| x * x).sum::<f64>().sqrt();
                        if mag > budget {
                            return CheckOutcome::Fail(format!("|wrench.force| {mag} > budget {budget}"));
                        }
                    }
                }
            }
            // Torque budget (rotational) — when present (e.g. force.screw, in force_profile.torque).
            let torque_budget = goal
                .canonical_action
                .safety_envelope
                .force_profile
                .as_ref()
                .and_then(|fp| fp.get("torque"))
                .and_then(serde_json::Value::as_str)
                .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v));
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
            CheckOutcome::Pass
        }
```

- [ ] **Step 4: Extend the `ReferenceDriver` to echo torque** — in `crates/rfl-conformance/src/lib.rs` `impl Driver for ReferenceDriver`, replace the `let (wrench, securing_force) = match &ca.force_budget { ... };` block with:

```rust
        // Echo any commanded force budget into wrench.force / securing_force and any
        // commanded torque (force.screw, in force_profile.torque) into wrench.torque
        // (within budget) — so C2/E3's envelope checkers have something to sample.
        let force_mag = ca.force_budget.as_ref().and_then(|q| q.parse().map(|(v, _)| v));
        let torque_mag = ca
            .safety_envelope
            .force_profile
            .as_ref()
            .and_then(|fp| fp.get("torque"))
            .and_then(serde_json::Value::as_str)
            .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v));
        let wrench = if force_mag.is_some() || torque_mag.is_some() {
            Some(Wrench {
                force: [0.0, 0.0, force_mag.unwrap_or(0.0)],
                torque: [0.0, 0.0, torque_mag.unwrap_or(0.0)],
            })
        } else {
            None
        };
        let securing_force = ca.force_budget.clone();
```

- [ ] **Step 5: Run the lib test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance nominal_screw_passes_torque_trajectory 2>&1 | grep -E "test result|FAILED|got " | head`
Expected: PASS.

- [ ] **Step 6: Confirm the cable `driver_protocol` goldens are unchanged + full suite green**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance 2>&1 | grep -E "warning:|test result|FAILED" | head`
Expected: all `ok` — crucially the 7 `driver_protocol` tests (the 3 goldens included) still pass with **no** snapshot update: cable has no `force.screw`, so `force_profile.torque` is absent and the driver's `wrench.torque` stays `[0,0,0]` exactly as before. If a `driver_protocol` golden fails, STOP — the torque echo wrongly affected the cable reports.

- [ ] **Step 7: Commit** (separate batch from Steps 5-6)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-conformance/src/lib.rs && git commit -m "feat(conformance): force-trajectory checker torque case + ReferenceDriver torque echo (ENV4)

check_envelope ForceTrajectory now also bounds |wrench.torque| <= force_profile.torque
(force.screw), alongside the existing force check. ReferenceDriver echoes
force_profile.torque into wrench.torque. Cable driver_protocol goldens unchanged
(no force.screw in cable).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: the `OverTorque` adversarial + screw envelope-conformance tests

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (`Fault::OverTorque`)
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (2 screw tests)

- [ ] **Step 1: Write the failing screw tests** — in `crates/rfl-conformance/tests/envelope_conformance.rs`, add (after the existing tests):

```rust
fn screw_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten")
}

#[test]
fn nominal_screw_passes_force_trajectory() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // force.screw is action index 5.
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "screw");
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
}

#[test]
fn over_torque_driver_fails_screw_force_trajectory() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverTorque),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // force.screw (index 5) -> torque-trajectory must reject the over-budget wrench torque.
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "screw");
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test envelope_conformance over_torque 2>&1 | grep -E "no variant|cannot find|error\[|test result" | head`
Expected: FAIL — `Fault::OverTorque` does not exist (`no variant ... OverTorque`). (The nominal test compiles but cannot run until the crate compiles.)

- [ ] **Step 3: Add `Fault::OverTorque`** — in `crates/rfl-conformance/src/lib.rs`:

Add the variant to the `Fault` enum (after `OverForce`):

```rust
    /// `wrench.torque` above any torque budget (violates the force-trajectory bound, ENV4).
    OverTorque,
```

Add the injection arm to `FaultyDriver::execute`'s `match self.fault` (after the `Fault::OverForce` arm):

```rust
            Fault::OverTorque => {
                for t in &mut report.telemetry {
                    if let Some(w) = t.wrench.as_mut() {
                        w.torque = [0.0, 0.0, 999.0];
                    }
                }
            }
```

- [ ] **Step 4: Run the screw tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test envelope_conformance 2>&1 | grep -E "test .* (ok|FAILED)|test result" | head`
Expected: all `ok` (the existing 4 C2 tests + the 2 new screw tests = 6) — nominal screw passes ForceTrajectory; OverTorque makes it fail.

- [ ] **Step 5: Gating validation batch** (separate from the commit)

Run: `cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1 && export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "test result|FAILED|error|warning:" | tail -12`
Expected: validate.py `PASS`; every crate green (rfl-core 54 + conformance lib + driver_protocol 7 + envelope_conformance 6 + retarget_determinism 5 + surface_scan 5 + screw_fasten 5); no warnings.

- [ ] **Step 6: Commit** (separate batch — after reading PASS)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs && git commit -m "test(conformance): screw torque-trajectory checks + OverTorque adversarial (E3)

The nominal ReferenceDriver's force.screw passes the torque-trajectory check
(echoed 0.2 N·m <= budget); FaultyDriver(OverTorque) is REJECTED (999 > budget) --
the non-circular proof for the torque case, mirroring OverForce.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Full verification + push + memory

**Files:** none.

- [ ] **Step 1: Final full suite + validate.py**

Run: `cd ~/Documents/GitHub/rfl && export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "test result|FAILED|error" && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: all green; validate.py `PASS`.

- [ ] **Step 2: Confirm pushes** (Tasks 1-2 each pushed)

Run: `cd ~/Documents/GitHub/rfl && git rev-parse --abbrev-ref HEAD && git fetch -q origin && git merge-base --is-ancestor origin/main HEAD && git push -q origin main; git rev-list --left-right --count origin/main...HEAD && git log --oneline -4`
Expected: branch `main`; final `0 0`; the 2 E3 commits + the design-doc commit visible.

- [ ] **Step 3: Update the memory** (`~/.claude/.../memory/project_rfl.md` "## Implementation track") with the real commit hashes from `git log --oneline -5`, recording this as the 8th increment (torque-trajectory checker E3 — Class-3 ENV4 torque case), and note that the screw line's Class-3 verification is now complete (force + torque envelope; round-trip via the existing driver_protocol path). Update the `MEMORY.md` RFL index line (8 increments, new HEAD). Not a repo commit — memory only.

---

## Self-review (completed during planning)

- **Spec coverage:** design §3 checker extension → Task 1 (Step 3); §4 driver torque echo → Task 1 (Step 4); §5 OverTorque + tests → Task 2; §6 conformance (cable goldens unchanged) → Task 1 (Step 6) + Task 2 (Step 5); §2 deferrals (decoupling detection, interval-invariant, unscrew) → out of scope, no task. No gaps.
- **Placeholder scan:** every code step shows complete code; commands show expected output.
- **Type consistency:** `check_envelope`/`EnvelopeClass::ForceTrajectory`/`CheckOutcome`/`quantity_mag`, `ReferenceDriver`/`Wrench`/`drive`, `Fault::OverTorque`/`FaultyDriver`, and `suffix_of` match the existing C2 definitions; the torque read uses the same `force_profile`→`get("torque")`→`as_str`→`Quantity::parse` path as the GraspContinuity checker and the screw lowering's emit.
- **No golden churn:** the `ReferenceDriver` torque echo only fires when `force_profile.torque` is present (screw only); cable `driver_protocol` reports are byte-identical, so no snapshot update (Step 6 verifies). `screw_fasten` is a retarget golden, not driver output — untouched.
