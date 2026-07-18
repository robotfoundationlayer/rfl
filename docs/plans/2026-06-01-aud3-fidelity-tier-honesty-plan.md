# AUD3 fidelity-tier honesty — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement
> this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> **LOCAL-ONLY** — never `git add` this file.

**Goal:** Make `fidelity_tier` honesty falsifiable (spec/05 AUD3): a degraded (`proxy`) execution
must disclose it — claiming `manifold` when the action lowered to proxy is malformed.

**Architecture:** Pure `rfl-conformance` (no rfl-core / schema / example / golden, like the ENV3
increment). Add `check_audit_honesty` (expected tier from the action's `tactile_target`, claimed
tier from `status.fidelity_tier`; over-claim = violation) + `Fault::FalseTier` (over-claims
manifold). Design: `docs/design/2026-06-01-aud3-fidelity-tier-honesty-design.md`.

**Tech Stack:** Rust (rfl-conformance), `schemas/validate.py`.

**Discipline (every task):** TDD red→green. After green, run `cargo test` (workspace, NOT `-p`)
+ `uv run --with jsonschema --with pyyaml python schemas/validate.py` in a batch SEPARATE from
the commit. Before each push: `git fetch -q && git merge-base --is-ancestor origin/main HEAD`,
`git add <explicit paths>`, commit, `git push origin main`, verify sync `0 0` + `git show --stat`.

---

### Task 1: `check_audit_honesty` + `Fault::FalseTier`

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (Fault enum + FaultyDriver match; new check; lib unit test)
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (3 tests + import)

- [ ] **Step 1: Write the failing integration tests** — append to `envelope_conformance.rs`
(reuse `example_dir()` [cable-insertion] + `suffix_of()`):

```rust
// --- AUD3 fidelity-tier honesty (degradation disclosure, spec/05 AUD3) ----------------------

#[test]
fn nominal_proxy_tier_is_disclosed() {
    let dir = example_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/pneumatic-6f.yaml"),
    )
    .expect("drive");
    // grasp.pinch (index 1) degrades to proxy on the no-tactile pneumatic hand.
    let (goal, report) = &pairs[1];
    assert_eq!(suffix_of(&goal.action_id), "pinch");
    assert_eq!(report.status.fidelity_tier.as_deref(), Some("proxy"));
    assert_eq!(check_audit_honesty(goal, report), CheckOutcome::Pass);
}

#[test]
fn false_manifold_claim_on_proxy_fails() {
    let dir = example_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::FalseTier),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/pneumatic-6f.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[1];
    // claims manifold on a proxy-degraded action -> undisclosed degradation (the bite).
    assert!(matches!(check_audit_honesty(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn manifold_tier_passes() {
    let dir = example_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[1];
    // allegro declares tactile -> manifold; honest.
    assert_eq!(report.status.fidelity_tier.as_deref(), Some("manifold"));
    assert_eq!(check_audit_honesty(goal, report), CheckOutcome::Pass);
}
```

Update the `use rfl_conformance::{...}` block to add `check_audit_honesty`.

- [ ] **Step 2: Run to verify it fails (unresolved import / no Fault::FalseTier)**

Run: `cargo test -p rfl-conformance --test envelope_conformance _tier 2>&1 | grep -E "error\[|unresolved|no variant|test result" | head`
Expected: compile error — `check_audit_honesty` not found and `Fault` has no variant `FalseTier`.

- [ ] **Step 3: Add `Fault::FalseTier`** — in `lib.rs` `enum Fault`, after `LoseContact`:

```rust
    /// `status.fidelity_tier` over-claimed as `manifold` (undisclosed degradation, AUD3).
    FalseTier,
```

- [ ] **Step 4: Add the `FaultyDriver` match arm** — in `lib.rs` `impl Driver for FaultyDriver`,
after the `Fault::LoseContact` arm:

```rust
            Fault::FalseTier => {
                report.status.fidelity_tier = Some("manifold".to_string());
            }
```

- [ ] **Step 5: Add `check_audit_honesty`** — in `lib.rs`, after `check_irreversible`:

```rust
/// Verify the AUD3 fidelity-tier honesty obligation (`spec/05`): a degraded execution must
/// disclose it — a result reported at full `manifold` tier when the action lowered to `proxy`
/// is malformed. The expected tier is the lowering's `tactile_target` degradation decision
/// (`Auto` => manifold, `Proxy` => proxy); the claimed tier is `status.fidelity_tier`.
/// Over-claiming (manifold claimed when proxy was the truth) is the violation; under-claiming is
/// conservative. Vacuous when the action carries no auto-confirmation (`Explicit` / none).
#[must_use]
pub fn check_audit_honesty(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    let expected = match &goal.canonical_action.tactile_target {
        Some(TactileTargetOut::Auto) => "manifold",
        Some(TactileTargetOut::Proxy { .. }) => "proxy",
        _ => return CheckOutcome::Pass, // no auto-confirmation -> nothing to disclose
    };
    if report.status.fidelity_tier.as_deref() == Some("manifold") && expected == "proxy" {
        return CheckOutcome::Fail(
            "degraded (proxy) execution claimed manifold tier — undisclosed degradation (AUD3)"
                .to_string(),
        );
    }
    CheckOutcome::Pass
}
```

(`TactileTargetOut` is already imported at the top of `lib.rs`.)

- [ ] **Step 6: Add the lib unit test** — in `lib.rs` tests module, after
`check_irreversible_requires_partial_state_on_interruption`:

```rust
    #[test]
    fn check_audit_honesty_rejects_undisclosed_degradation() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, ProxySpec, TactileTargetOut,
            TimingHints, TimingMode,
        };
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        let action = |tt: Option<TactileTargetOut>| CanonicalAction {
            target_frame: "tcp".into(),
            target_pose: PoseExpr::Ref { r#ref: "obj".into() },
            force_budget: None,
            timing: TimingHints {
                nominal_duration: None,
                timing_mode: TimingMode::Strict,
                stop_at_goal: true,
            },
            tactile_target: tt,
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: None,
                station_keeping: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
        };
        let report = |tier: &str| DriverReport {
            telemetry: vec![],
            status: Status {
                message: "status",
                action_id: "s/e/0001-pinch".to_string(),
                outcome: Outcome::Succeeded,
                verdict: Some(Verdict { value: true, confidence: 1.0, evidence: vec![] }),
                fidelity_tier: Some(tier.to_string()),
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
            },
        };
        let proxy_goal = ExecuteGoal::wrap(
            "s/e/0001-pinch".to_string(),
            action(Some(TactileTargetOut::Proxy {
                proxy: ProxySpec { tier: "proxy", criterion: "force_position" },
            })),
        );
        let manifold_goal =
            ExecuteGoal::wrap("s/e/0001-pinch".to_string(), action(Some(TactileTargetOut::Auto)));
        // proxy action + manifold claim -> Fail (undisclosed degradation).
        assert!(matches!(check_audit_honesty(&proxy_goal, &report("manifold")), CheckOutcome::Fail(_)));
        // proxy action + proxy claim -> Pass (honest).
        assert_eq!(check_audit_honesty(&proxy_goal, &report("proxy")), CheckOutcome::Pass);
        // manifold action + manifold claim -> Pass.
        assert_eq!(check_audit_honesty(&manifold_goal, &report("manifold")), CheckOutcome::Pass);
    }
```

- [ ] **Step 7: Run the new tests (green)**

Run: `cargo test -p rfl-conformance _tier 2>&1 | grep -E "test result|_tier|_proxy"` then
`cargo test -p rfl-conformance check_audit_honesty 2>&1 | grep -E "test result|check_audit"`.
Expected: `nominal_proxy_tier_is_disclosed`, `false_manifold_claim_on_proxy_fails`,
`manifold_tier_passes`, `check_audit_honesty_rejects_undisclosed_degradation` PASS.

- [ ] **Step 8: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `rfl-conformance` lib 15 (was 14, +1), `envelope_conformance` 41 (was 38, +3).
Existing goldens / tests unaffected (check_audit_honesty is new; `Fault::FalseTier` only used in
the new test; the ReferenceDriver is unchanged).

- [ ] **Step 9: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs && \
git commit -m "feat(conformance): verify AUD3 fidelity-tier honesty

check_audit_honesty: a degraded (proxy) execution must disclose it —
claiming manifold when the action lowered to proxy (tactile_target=Proxy) is
malformed. Reuses tactile_target (expected tier) + status.fidelity_tier
(claimed). Fault::FalseTier (over-claims manifold) bites on a proxy-degraded
action but is vacuous when the truth is manifold — pinned to the lowered
truth. Opens the audit/transparency class; pure conformance.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `lib.rs` + `envelope_conformance.rs`.

---

### Task 2: README status line

**Files:**
- Modify: `README.md` (~line 84)

- [ ] **Step 1: Extend the README status line** — add a clause that the suite now also checks
AUD3 fidelity-tier honesty: a degraded (`proxy`) execution must disclose it — claiming `manifold`
when the action degraded to proxy is malformed (an undisclosed degradation), opening the
audit/transparency class. Match the surrounding phrasing; keep the existing sentence structure.
(spec/05 AUD3 already states the obligation, so no spec change.)

- [ ] **Step 2: Verify nothing regressed**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: unchanged green.

- [ ] **Step 3: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add README.md && \
git commit -m "docs(readme): note AUD3 fidelity-tier honesty (degradation disclosure)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `README.md`.

---

## Post-increment

- Final full-suite read: expected `rfl-core` 81 (unchanged), `rfl-conformance` lib 15,
  `driver_protocol` 9, `envelope_conformance` 41, others unchanged.
- Update `project_rfl.md` "Implementation track" (20th increment, real hashes + test deltas) +
  the `MEMORY.md` RFL line. Move deferred items (AUD2 momentary_release / in_hand.flip, AUD3
  freed-part disposition, AUD1 audit record, proxy_reactive tier) into the parked list.

## Self-review (spec coverage)

- AUD3 fidelity-tier honesty (design § 1, § 3) → Task 1 (`check_audit_honesty`). ✓
- Fault::FalseTier non-vacuity / asymmetry (design § 5) → Task 1 (pneumatic Fail, allegro
  vacuous). ✓
- README (design § 6) → Task 2. ✓
- No rfl-core / schema / example / golden change (design § 2) → confirmed: Task 1 touches only
  `lib.rs` + `envelope_conformance.rs`. ✓
- Type consistency: `check_audit_honesty(goal, report)` reads `tactile_target` (TactileTargetOut)
  + `status.fidelity_tier`; `Fault::FalseTier` sets `status.fidelity_tier = "manifold"`; both
  used identically in lib.rs + envelope_conformance.rs. ✓
