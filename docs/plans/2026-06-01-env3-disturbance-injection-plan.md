# ENV3 disturbance injection (transport.carry C2) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline, per
> the RFL main-branch direct-commit discipline). Steps use checkbox (`- [ ]`) syntax.
> LOCAL-ONLY: never `git add` this file.

**Goal:** Verify ENV3 for `transport.carry` — inject calibrated disturbances up to the
emitted `disturbance_budget` (invariant holds under perturbation) and verify graceful
degradation (halt with object secured) above budget (`spec/05` ENV3, `spec/01` § 4.4 C2).

**Architecture:** Pure `rfl-conformance` addition (like the C2 envelope checkers, increment
5; no rfl-core/spec/schema change). A `DisturbanceDriver` parameterized by an injected
magnitude reads `disturbance_budget` from the goal and models the conformant response
(maintain under / halt-secured over); two adversarial response variants (`Drops`,
`ClaimsSuccess`) prove a new `check_graceful_degradation` bites. The over-budget report
yields opposite verdicts: `IntervalInvariant → Fail` (did not succeed) and
`graceful_degradation → Pass` (object secured).

**Tech Stack:** Rust (rustup stable, `export PATH="$HOME/.cargo/bin:$PATH"` per cargo call),
serde_json, `uv run --with jsonschema --with pyyaml python schemas/validate.py`. No goldens
(pass/fail asserts).

**Process discipline (every task):** TDD red → green → run the FULL `cargo test` + validate.py
and **READ the result in a batch PHYSICALLY SEPARATE from the git commit** → only if green,
stage explicit files (no `-A`, never `docs/plans/`) and commit (Conventional Commits +
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`) → before push
`git merge-base --is-ancestor origin/main HEAD`, after push
`git rev-list --left-right --count origin/main...HEAD` == `0 0`, branch == `main`.
`cargo test` warning-clean (clippy pedantic pre-existing, not gated). The design doc commit
`aeb9cf9` already landed. Run the FULL `cargo test` (not just `-p rfl-conformance`) before
each commit — increment 13 showed a cross-crate test break that the scoped run missed.

---

## File Structure

- **Modify** `crates/rfl-conformance/src/lib.rs` — add `check_graceful_degradation` (pub fn),
  `DisturbanceResponse` (pub enum), `DisturbanceDriver` (pub struct + `Driver` impl), and a
  lib unit test for the check.
- **Modify** `crates/rfl-conformance/tests/envelope_conformance.rs` — 4 ENV3 integration tests
  on the `skill-carry.yaml` carry action.
- **Modify** `README.md` (Task 3) — note ENV3 / disturbance injection.

No new files. `securing_floor_violation` (added increment 13) is reused for the "object
secured" clause. `Outcome` is already imported in `lib.rs`.

---

## Task 1: `check_graceful_degradation`

**Files:** Modify `crates/rfl-conformance/src/lib.rs`

- [ ] **Step 1: Write the failing lib unit test** — add to the `tests` module in `lib.rs`
(near `interval_invariant_checks_every_sample`, reusing its `sample_action()` helper, which has
no `force_profile` so the object-secured clause is vacuously satisfied — clause 3 is exercised
end-to-end by the Task 2 `Drops` test):

```rust
    #[test]
    fn graceful_degradation_accepts_secured_halt_rejects_pretended_success() {
        use rfl_core::driver::{Outcome, RealizedPose, Status, Telemetry, Verdict};
        let goal = ExecuteGoal::wrap("s/e/0003-carry".to_string(), sample_action());
        let report = |outcome: Outcome, detail: Option<&str>| {
            let status = Status {
                message: "status",
                action_id: "s/e/0003-carry".to_string(),
                outcome,
                verdict: Some(Verdict { value: false, confidence: 1.0, evidence: vec![] }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: detail.map(|_| "blocked".to_string()),
                failure_detail: detail.map(str::to_string),
            };
            DriverReport {
                telemetry: vec![Telemetry {
                    message: "telemetry",
                    action_id: "s/e/0003-carry".to_string(),
                    t: 1.0,
                    realized_pose: Some(RealizedPose::placeholder()),
                    wrench: None,
                    securing_force: None,
                    tactile: vec![],
                    events: vec![],
                    fidelity_tier: None,
                }],
                status,
            }
        };
        // graceful halt: Failed + disturbance_exceeded -> Pass.
        assert_eq!(
            check_graceful_degradation(&goal, &report(Outcome::Failed, Some("disturbance_exceeded"))),
            CheckOutcome::Pass
        );
        // pretended success -> Fail (clause 1).
        assert!(matches!(
            check_graceful_degradation(&goal, &report(Outcome::Succeeded, None)),
            CheckOutcome::Fail(_)
        ));
        // wrong halt reason -> Fail (clause 2).
        assert!(matches!(
            check_graceful_degradation(&goal, &report(Outcome::Failed, Some("blocked"))),
            CheckOutcome::Fail(_)
        ));
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance check_graceful 2>&1 | grep -E "error|cannot find" | head`
Expected: FAIL — `cannot find function check_graceful_degradation in this scope`.

- [ ] **Step 3: Implement the check** — add after `check_envelope` in `lib.rs`:

```rust
/// Verify the § 4.4 C2 graceful-degradation contract on an OVER-budget disturbance report:
/// the carry must NOT claim success, must report the `disturbance_exceeded` halt reason, and
/// must keep the object secured (`securing_force >= min_holding_force` at every sample — a
/// controlled halt, not a drop). Distinct from `check_envelope`: this judges the *failure
/// shape* of an over-budget injection (`spec/05` ENV3), not correct execution.
#[must_use]
pub fn check_graceful_degradation(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    if matches!(report.status.outcome, Outcome::Succeeded) {
        return CheckOutcome::Fail("claimed success under an over-budget disturbance".to_string());
    }
    if report.status.failure_detail.as_deref() != Some("disturbance_exceeded") {
        return CheckOutcome::Fail(format!(
            "expected failure_detail disturbance_exceeded, got {:?}",
            report.status.failure_detail
        ));
    }
    if let Some(reason) = securing_floor_violation(goal, report) {
        return CheckOutcome::Fail(format!("object not secured during halt: {reason}"));
    }
    CheckOutcome::Pass
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance check_graceful 2>&1 | tail -5`
Expected: PASS (the new unit test).

- [ ] **Step 5: Full suite + validate.py (separate from commit) and READ**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "FAILED|error\[|warning:" || echo ALL GREEN`
Then: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1; echo EXIT=$?`
Expected: `ALL GREEN`; validate.py PASS `EXIT=0`. Read both before staging.

- [ ] **Step 6: Commit**

```bash
cd ~/Documents/GitHub/rfl
git add crates/rfl-conformance/src/lib.rs
git commit -F - <<'EOF'
feat(conformance): check_graceful_degradation (ENV3 over-budget contract)

spec/05 ENV3 + spec/01 § 4.4 C2. Verifies an over-budget disturbance report
is a controlled halt with the object secured: not-Succeeded + failure_detail
disturbance_exceeded + securing_force >= min_holding_force at every sample
(reusing securing_floor_violation). Distinct from check_envelope (failure
shape, not correct execution).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```
Expected: push ok, `0 0`.

---

## Task 2: `DisturbanceDriver` + ENV3 integration tests

**Files:** Modify `crates/rfl-conformance/src/lib.rs`, `tests/envelope_conformance.rs`

- [ ] **Step 1: Write the failing integration tests** — append to `tests/envelope_conformance.rs`
(add `DisturbanceDriver` and `DisturbanceResponse` to the `use rfl_conformance::{...}` import,
and `check_graceful_degradation`):

```rust
#[test]
fn carry_invariant_holds_under_sub_budget_disturbance() {
    // Injected 0.3 N <= the emitted 0.4 N budget: the held interval invariant still holds.
    let dir = example_dir();
    let pairs = drive(
        DisturbanceDriver::new(0.3, DisturbanceResponse::Graceful),
        &dir.join("skill-carry.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[2]; // locate, pinch, carry
    assert_eq!(suffix_of(&goal.action_id), "carry");
    assert_eq!(report.telemetry.len(), 3, "interval-sampled under perturbation");
    assert_eq!(check_envelope(EnvelopeClass::IntervalInvariant, goal, report), CheckOutcome::Pass);
}

#[test]
fn over_budget_disturbance_degrades_gracefully() {
    // Injected 0.6 N > 0.4 N budget: NOT IntervalInvariant (did not succeed), but graceful
    // (halt with object secured) — the opposite-verdicts property.
    let dir = example_dir();
    let pairs = drive(
        DisturbanceDriver::new(0.6, DisturbanceResponse::Graceful),
        &dir.join("skill-carry.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[2];
    assert_eq!(check_graceful_degradation(goal, report), CheckOutcome::Pass);
    assert!(matches!(
        check_envelope(EnvelopeClass::IntervalInvariant, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn over_budget_drop_fails_graceful_degradation() {
    // Adversarial: over budget AND drops the object -> not graceful (a loss). The non-circular bite.
    let dir = example_dir();
    let pairs = drive(
        DisturbanceDriver::new(0.6, DisturbanceResponse::Drops),
        &dir.join("skill-carry.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[2];
    assert!(matches!(
        check_graceful_degradation(goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn over_budget_false_success_fails_graceful_degradation() {
    // Adversarial: over budget but claims success (ignores the disturbance) -> rejected.
    let dir = example_dir();
    let pairs = drive(
        DisturbanceDriver::new(0.6, DisturbanceResponse::ClaimsSuccess),
        &dir.join("skill-carry.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[2];
    assert!(matches!(
        check_graceful_degradation(goal, report),
        CheckOutcome::Fail(_)
    ));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test envelope_conformance 2>&1 | grep -E "error|cannot find" | head`
Expected: FAIL — `cannot find ... DisturbanceDriver` / `DisturbanceResponse`.

- [ ] **Step 3: Implement the bench** — add after the `FaultyDriver` impl in `lib.rs`:

```rust
/// How a driver responds to an OVER-budget injected disturbance (under budget it always
/// returns the nominal maintained invariant). `Graceful` is the conformant § 4.4 C2 response;
/// the two adversarial responses prove `check_graceful_degradation` bites.
#[derive(Debug, Clone, Copy)]
pub enum DisturbanceResponse {
    /// Conformant: halt to a stable config with the object still secured.
    Graceful,
    /// Adversarial: the object is dropped (securing_force below floor) — a loss, not a halt.
    Drops,
    /// Adversarial: claim success despite the over-budget disturbance.
    ClaimsSuccess,
}

/// The ENV3 disturbance bench (`spec/05` § Disturbance injection): a driver parameterized by
/// the bench-injected disturbance magnitude (N). It reads the action's `disturbance_budget`
/// from the execute message and models a driver's response — maintain the nominal invariant
/// when `injected <= budget`, otherwise respond per `DisturbanceResponse`. Disturbance
/// injection is the bench's input; the response is what the checks judge. (Non-carry actions,
/// which carry no `disturbance_budget`, pass through unchanged.)
#[derive(Debug)]
pub struct DisturbanceDriver {
    inner: ReferenceDriver,
    injected_n: f64,
    response: DisturbanceResponse,
}

impl DisturbanceDriver {
    /// A disturbance driver injecting `injected_n` newtons with the given response policy.
    #[must_use]
    pub fn new(injected_n: f64, response: DisturbanceResponse) -> Self {
        DisturbanceDriver { inner: ReferenceDriver::default(), injected_n, response }
    }
}

impl Driver for DisturbanceDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        let budget = goal
            .canonical_action
            .safety_envelope
            .force_profile
            .as_ref()
            .and_then(|fp| fp.get("disturbance_budget"))
            .and_then(serde_json::Value::as_str)
            .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v));
        // No budget (non-carry) or injected within budget: the invariant holds (nominal).
        let Some(budget) = budget else { return report };
        if self.injected_n <= budget {
            return report;
        }
        // Over budget. ClaimsSuccess returns the nominal Succeeded report unchanged.
        if matches!(self.response, DisturbanceResponse::ClaimsSuccess) {
            return report;
        }
        // Graceful / Drops: a controlled halt (Failed, blocked, disturbance_exceeded).
        report.status.outcome = Outcome::Failed;
        report.status.failure_class = Some("blocked".to_string());
        report.status.failure_detail = Some("disturbance_exceeded".to_string());
        if let Some(v) = report.status.verdict.as_mut() {
            v.value = false; // honest: the postcondition did not hold
        }
        if matches!(self.response, DisturbanceResponse::Drops) {
            // The object was lost — securing_force falls below the floor (mirrors UnderSecure).
            for t in &mut report.telemetry {
                if t.securing_force.is_some() {
                    t.securing_force = Some(rfl_core::quantity::Quantity("0.1 N".to_string()));
                }
            }
        }
        report
    }
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test envelope_conformance 2>&1 | grep -E "disturbance|graceful|sub_budget|test result"`
Expected: the 4 new tests `ok`; envelope_conformance `test result: ok.` (was 18 after Task-13
+ Task-1 lib unit; this binary gains 4 → check the count is `+4`).

- [ ] **Step 5: Full suite + validate.py (separate from commit) and READ**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "FAILED|error\[|warning:" || echo ALL GREEN`
Then: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1; echo EXIT=$?`
Then: `git status --porcelain crates/rfl-conformance/tests/snapshots/`
Expected: `ALL GREEN`; validate.py PASS `EXIT=0`; no snapshot churn (pass/fail asserts only).

- [ ] **Step 6: Commit**

```bash
cd ~/Documents/GitHub/rfl
git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs
git commit -F - <<'EOF'
feat(conformance): ENV3 disturbance injection for transport.carry

spec/05 ENV3. DisturbanceDriver reads the emitted disturbance_budget and models
the response: maintain the held interval invariant under budget, halt-with-object-
secured (Graceful) over budget. Sub-budget injection passes IntervalInvariant;
over-budget Graceful passes check_graceful_degradation AND fails IntervalInvariant
(opposite verdicts). Adversarial Drops (object lost) and ClaimsSuccess (pretended
success) both fail check_graceful_degradation — the non-circular proof. No
rfl-core/spec/schema change; pass/fail asserts, no goldens.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```
Expected: push ok, `0 0`.

---

## Task 3: README note + memory update (wrap-up)

**Files:** Modify `README.md` (commit); update `~/.claude/.../memory/project_rfl.md` + `MEMORY.md`
(NOT git; real hashes after Tasks 1–2 push).

- [ ] **Step 1: Read the README envelope-class prose** (it currently ends each example/envelope
clause with "each verified against adversarial drivers"; the parallel session rebuilt it):

Run: `cd ~/Documents/GitHub/rfl && grep -n "adversarial drivers\|interval-invariant\|disturbance" README.md`
Then read that region.

- [ ] **Step 2: Edit the README** — extend the envelope-class clause to note ENV3, e.g. append
to the "interval-invariant ... transport.carry held-under-disturbance" phrase: "(with ENV3
disturbance injection: invariant held up to `disturbance_budget`, graceful degradation above)".
Keep it accurate; do not add new em-dashes (de-AI rule for public docs).

- [ ] **Step 3: Verify + commit the README**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | tail -3` (confirm still green).
```bash
cd ~/Documents/GitHub/rfl
git add README.md
git commit -F - <<'EOF'
docs(readme): note ENV3 disturbance injection for transport.carry

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```
Expected: push ok, `0 0`.

- [ ] **Step 4: Update the auto-memory with REAL hashes** (not git): append the 14th-increment
milestone to the `## Implementation track` of `project_rfl.md` (design `aeb9cf9` + the real
Task 1–3 hashes), and update the `MEMORY.md` RFL line: ENV3 DONE, the conformance story is
COMPLETE (all four envelope classes + ENV2 + ENV3 + held-transport, all adversarially verified),
**in-process incrementing ENDS** per the agreement; the remaining deferred list stays parked.

---

## Self-Review

**Spec coverage** (design doc § by §): § 3 disturbance bench → Task 2 (`DisturbanceDriver` +
`DisturbanceResponse`); § 4 graceful check → Task 1 (`check_graceful_degradation`); § 5 tests →
Task 1 (lib unit) + Task 2 (4 integration tests, one per design bullet); § 2 scope (carry only,
no rfl-core/spec/schema change) → honored (only `lib.rs` + `envelope_conformance.rs` + README). ✓

**Placeholder scan:** every code step has complete code; run steps have exact commands +
expected output; no goldens (pass/fail asserts). No TBD/TODO. ✓

**Type consistency:** `check_graceful_degradation(goal, report)` defined Task 1, used Task 2
tests. `DisturbanceDriver::new(injected_n: f64, response: DisturbanceResponse)` defined Task 2
Step 3, called identically in Task 2 Step 1 tests. `DisturbanceResponse::{Graceful, Drops,
ClaimsSuccess}` consistent. `securing_floor_violation` (increment 13) reused unchanged.
`Outcome::{Succeeded, Failed}` per `driver.rs`. The over-budget `Graceful` report sets
`outcome = Failed` → `check_envelope(IntervalInvariant)` returns `Fail` on its first
`Succeeded` check (consistent with the Task-13 arm), so the opposite-verdicts assertion holds. ✓
