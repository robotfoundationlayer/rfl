# force.cut freeing disclosure (TM21c) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Checkbox steps.
> **LOCAL-ONLY** — never `git add` this file.

**Goal:** Generalize the increment-24 freed-part disposition disclosure to `force.cut` (a 2nd,
intent-less consumer): a successful cut must disclose `safety_flags.freed_part_disposition` with a
valid disposition (presence-only). Add an `outcome == Succeeded` guard.

**Architecture:** rfl-conformance only — extend `check_freed_part_disposition` (Succeeded guard +
an `irreversible` presence-only branch) and the `ReferenceDriver` `safety_flags` population
(`irreversible` → `retained`). Reuse `FreeingDriver`. Design:
`docs/design/2026-06-01-cut-freeing-disposition-design.md` (committed `78d0fe4`).

**Tech Stack:** Rust (rfl-conformance), validate.py.

**Discipline:** TDD red→green. After green, full `cargo test` (workspace, NOT `-p`) +
`uv run --with jsonschema --with pyyaml python schemas/validate.py` in a batch SEPARATE from the
commit. Before each push: `git fetch -q && git merge-base --is-ancestor origin/main HEAD`,
`git add <explicit paths>`, commit, `git push origin main`, verify sync `0 0` + `git show --stat`.

---

### Task 1: generalize the check + driver to `force.cut` + tests

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (check + ReferenceDriver + 1 unit test)
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (3 integration tests)

- [ ] **Step 1: Write the failing cut integration tests** — in `envelope_conformance.rs`, after the
existing freed-part (unscrew) tests:

```rust
#[test]
fn nominal_cut_discloses_disposition() {
    let dir = screw_dir();
    let pairs = drive(
        FreeingDriver::new(FreeingResponse::Discloses),
        &dir.join("skill-cut.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "cut");
    assert_eq!(check_freed_part_disposition(goal, report), CheckOutcome::Pass);
}

#[test]
fn cut_uncontrolled_drop_fails_disposition() {
    let dir = screw_dir();
    let pairs = drive(
        FreeingDriver::new(FreeingResponse::DropsUncontrolled),
        &dir.join("skill-cut.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "cut");
    assert!(matches!(check_freed_part_disposition(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn cut_accepts_either_disposition_presence_only() {
    // force.cut has no authored intent -> presence-only: either valid disposition passes
    // (contrast force.unscrew's expected-match, where the same flip fails).
    let dir = screw_dir();
    let pairs = drive(
        FreeingDriver::new(FreeingResponse::FalseDisposition),
        &dir.join("skill-cut.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(check_freed_part_disposition(goal, report), CheckOutcome::Pass);
}
```

- [ ] **Step 2: Write the failing unit cases** — in `crates/rfl-conformance/src/lib.rs` `mod tests`,
add after `check_freed_part_disposition_requires_a_disclosure_matching_intent`:

```rust
    #[test]
    fn check_freed_part_disposition_cut_is_presence_only_and_succeeded_gated() {
        use rfl_core::driver::{FreedPartDisposition, Outcome, RealizedPose, SafetyFlags, Status, Verdict};
        let mut action = sample_action();
        action.safety_envelope.force_profile = Some(serde_json::json!({ "irreversible": true }));
        let goal = ExecuteGoal::wrap("s/e/0001-cut".to_string(), action);
        let report = |outcome: Outcome, flags: Option<SafetyFlags>| {
            let status = Status {
                message: "status",
                action_id: "s/e/0001-cut".to_string(),
                outcome,
                verdict: Some(Verdict { value: true, confidence: 1.0, evidence: vec![] }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
                stop_latency: None,
                safety_flags: flags,
            };
            DriverReport { telemetry: vec![], status }
        };
        let disp = |d: &str| {
            Some(SafetyFlags {
                freed_part_disposition: Some(FreedPartDisposition {
                    disposition: d.to_string(),
                    zone: None,
                }),
            })
        };
        // presence-only: either valid disposition on a succeeded cut -> Pass.
        assert_eq!(check_freed_part_disposition(&goal, &report(Outcome::Succeeded, disp("retained"))), CheckOutcome::Pass);
        assert_eq!(check_freed_part_disposition(&goal, &report(Outcome::Succeeded, disp("safe_zone_release"))), CheckOutcome::Pass);
        // succeeded cut with no disclosure -> uncontrolled drop -> Fail.
        assert!(matches!(check_freed_part_disposition(&goal, &report(Outcome::Succeeded, None)), CheckOutcome::Fail(_)));
        // interrupted (Failed) cut froze nothing -> vacuous Pass even with no disclosure.
        assert_eq!(check_freed_part_disposition(&goal, &report(Outcome::Failed, None)), CheckOutcome::Pass);
    }
```

- [ ] **Step 3: Run to verify red**

Run: `cargo test -p rfl-conformance check_freed_part_disposition_cut 2>&1 | grep -E "test result|FAILED|panicked" | head`
Run: `cargo test -p rfl-conformance --test envelope_conformance cut_uncontrolled_drop 2>&1 | grep -E "test result|FAILED" | head`
Expected: RED — the current check keys only on `on_disengagement`, so a cut (irreversible, no
on_disengagement) returns `Pass` vacuously: `cut_uncontrolled_drop_fails_disposition` expects
`Fail` but gets `Pass`, and the unit's "succeeded cut with no disclosure -> Fail" assertion fails.

- [ ] **Step 4: Extend `check_freed_part_disposition`** — in `crates/rfl-conformance/src/lib.rs`,
replace the current function body with:

```rust
#[must_use]
pub fn check_freed_part_disposition(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    // A freeing completes only on success; an interrupted op froze nothing.
    if !matches!(report.status.outcome, Outcome::Succeeded) {
        return CheckOutcome::Pass;
    }
    let fp = goal.canonical_action.safety_envelope.force_profile.as_ref();
    let on_diseng = fp
        .and_then(|fp| fp.get("on_disengagement"))
        .and_then(serde_json::Value::as_str);
    let irreversible = fp
        .and_then(|fp| fp.get("irreversible"))
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    // Expected disposition: Some for an authored on_disengagement (force.unscrew, expected-match),
    // None for a presence-only freeing (force.cut, no authored intent). Not a freeing op -> Pass.
    let expected: Option<&str> = match on_diseng {
        Some("drop_safe") => Some("safe_zone_release"),
        Some(_) => Some("retained"),
        None if irreversible => None,
        None => return CheckOutcome::Pass,
    };
    let disclosed = report
        .status
        .safety_flags
        .as_ref()
        .and_then(|sf| sf.freed_part_disposition.as_ref());
    match (disclosed, expected) {
        (None, _) => CheckOutcome::Fail(
            "uncontrolled drop: a freeing operation disclosed no freed_part_disposition".to_string(),
        ),
        (Some(d), Some(e)) if d.disposition != e => CheckOutcome::Fail(format!(
            "freed_part_disposition {} does not match the authored intent {e}",
            d.disposition
        )),
        (Some(d), None)
            if d.disposition != "retained" && d.disposition != "safe_zone_release" =>
        {
            CheckOutcome::Fail(format!("unknown freed_part_disposition {}", d.disposition))
        }
        (Some(_), _) => CheckOutcome::Pass,
    }
}
```

- [ ] **Step 5: Extend the `ReferenceDriver` `safety_flags` population** — in
`ReferenceDriver::execute`, replace the current `let safety_flags = ca.safety_envelope.force_profile
.as_ref().and_then(|fp| fp.get("on_disengagement"))...map(...)` block with:

```rust
        let safety_flags = ca.safety_envelope.force_profile.as_ref().and_then(|fp| {
            if let Some(od) = fp.get("on_disengagement").and_then(serde_json::Value::as_str) {
                let (disposition, zone) = if od == "drop_safe" {
                    ("safe_zone_release", Some(serde_json::json!({ "zone": "discard_bin" })))
                } else {
                    ("retained", None)
                };
                Some(SafetyFlags {
                    freed_part_disposition: Some(FreedPartDisposition {
                        disposition: disposition.to_string(),
                        zone,
                    }),
                })
            } else if fp.get("irreversible").and_then(serde_json::Value::as_bool) == Some(true) {
                // force.cut: the cut-off piece is retained (v0 conservative default).
                Some(SafetyFlags {
                    freed_part_disposition: Some(FreedPartDisposition {
                        disposition: "retained".to_string(),
                        zone: None,
                    }),
                })
            } else {
                None
            }
        });
```

- [ ] **Step 6: Run the new tests (green)**

Run: `cargo test -p rfl-conformance check_freed_part_disposition 2>&1 | grep -E "test result"`
Run: `cargo test -p rfl-conformance --test envelope_conformance cut 2>&1 | grep -E "test result"`
Expected: PASS (the cut nominal discloses retained; DropsUncontrolled strips it; FalseDisposition
flips to a still-valid safe_zone_release; the unit presence-only + Succeeded-guard cases hold).

- [ ] **Step 7: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; rfl-core 85 (unchanged); conformance `lib` 18 (+1 the cut unit test),
`envelope_conformance` 51 (+3); driver_protocol 9; all 12 golden binaries byte-identical.
validate.py PASS. (Confirm the existing unscrew freed-part tests + the existing cut envelope tests
still pass — the Succeeded guard changes no Succeeded verdict, and the cut envelope tests do not
call check_freed_part_disposition.)

- [ ] **Step 8: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs && \
git commit -m "feat(conformance): generalize freed-part disposition to force.cut

A successful force.cut frees a piece (spec/04 TM20c) and must disclose
safety_flags.freed_part_disposition; cut has no authored intent, so the check
is presence-only (any valid disposition), contrasting force.unscrew's
expected-match. Adds an outcome==Succeeded guard (a freeing completes only on
success). Proves the safety_flags freed-part contract generalizes beyond
unscrew. No schema / struct / golden / rfl-core change.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only the 2 files.

---

### Task 2: README status line

**Files:** Modify: `README.md` (~line 84)

- [ ] **Step 1: Extend the README** — note the freed-part disposition now also covers `force.cut`
(a successful cut discloses its freed piece's disposition; presence-only since the cut authors no
intent), generalizing the `safety_flags` contract to a 2nd consumer. Match the surrounding phrasing.

- [ ] **Step 2: Verify nothing regressed**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: unchanged green.

- [ ] **Step 3: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add README.md && \
git commit -m "docs(readme): note force.cut freed-part disposition disclosure

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only README.md.

---

## Post-increment

- Final full-suite read: rfl-core 85; conformance `lib` 18, `envelope_conformance` 51; all 12
  golden binaries byte-identical; validate.py PASS.
- Update `project_rfl.md` "Implementation track" (25th increment) + the `MEMORY.md` RFL hook
  (short — count 24→25, latest line; keep compact).

## Self-review (spec coverage)

- TM21c generalized to force.cut presence-only (design § 1) → Task 1 check branch. ✓
- Succeeded guard (design § 1) → Task 1 Step 4 first line. ✓
- ReferenceDriver discloses for irreversible (design § 3) → Task 1 Step 5. ✓
- FreeingDriver reuse; FalseDisposition passes for cut (design § 4) → Task 1 Step 1 third test. ✓
- No schema / struct / golden / rfl-core change (design § 2) → confirmed: only lib.rs +
  envelope_conformance.rs. ✓
- Type consistency: reuses `SafetyFlags` / `FreedPartDisposition` / `FreeingDriver` /
  `FreeingResponse` from increment 24. ✓
