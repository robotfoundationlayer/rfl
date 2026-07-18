# freed-part disposition (TM21c) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Checkbox steps.
> **LOCAL-ONLY** — never `git add` this file.

**Goal:** Enforce spec/04 TM21c: a freeing operation (force.unscrew, via its lowered
`force_profile.on_disengagement`) must disclose `status.safety_flags.freed_part_disposition`
matching the authored intent; an undisclosed freeing is an uncontrolled drop.

**Architecture:** The Rust `Status` gains a `safety_flags` struct matching the already-declared
schema (NO schema change). The `ReferenceDriver` populates it for freeing ops; a new
`check_freed_part_disposition` reads it; a `FreeingDriver` with two adversaries proves
non-circularity. Design: `docs/design/2026-06-01-freed-part-disposition-design.md` (committed
`378f543`).

**Tech Stack:** Rust (rfl-core + rfl-conformance), validate.py.

**Discipline (every task):** TDD red→green. After green, full `cargo test` (workspace, NOT `-p`)
+ `uv run --with jsonschema --with pyyaml python schemas/validate.py` in a batch SEPARATE from the
commit. Before each push: `git fetch -q && git merge-base --is-ancestor origin/main HEAD`,
`git add <explicit paths>`, commit, `git push origin main`, verify sync `0 0` + `git show --stat`.

**Exhaustive-init coupling:** adding `Status.safety_flags` breaks every `Status { … }` literal.
Drive the list with the compiler (`cargo test --no-run` → E0063); each gains `safety_flags: None`.

---

### Task 1: `Status.safety_flags` struct (green checkpoint, no behavior change)

**Files:**
- Modify: `crates/rfl-core/src/driver.rs` (3 structs + the 2 driver.rs Status literals + 1 unit test)
- Modify: `crates/rfl-conformance/src/lib.rs` (Status literals gain `safety_flags: None`)

- [ ] **Step 1: Add the structs + field** — in `crates/rfl-core/src/driver.rs`, in `pub struct
Status`, after `stop_latency`:

```rust
    /// Safety-critical facts the audit record propagates (`spec/05` AUD2 / AUD3, `spec/04`
    /// freed-part handling). Present only when a safety-relevant fact must be disclosed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_flags: Option<SafetyFlags>,
}

/// The `safety_flags` audit object (`driver-interface` schema `StatusResult.safety_flags`).
/// v0 carries the freed-part disposition; `momentary_release` (schema-declared) is currently
/// disclosed via `verdict.evidence` (increment 21) — reconciling the two is deferred.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SafetyFlags {
    /// The freed-part disposition disclosed by a freeing operation (`spec/04` TM21c).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freed_part_disposition: Option<FreedPartDisposition>,
}

/// A freeing operation's disposition (`spec/04` TM21c): `retained` or `safe_zone_release(zone)`,
/// never an uncontrolled drop.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FreedPartDisposition {
    /// `retained` or `safe_zone_release`.
    pub disposition: String,
    /// The declared safe zone (present for `safe_zone_release`; owned by `04`, floored).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<serde_json::Value>,
}
```

- [ ] **Step 2: Add `safety_flags: None` to the two driver.rs Status literals** — in `driver.rs`,
the production-builder field list ends with `failure_detail: None,` in the
`status_outcome_serializes_snake_case` test (~line 200) — add `safety_flags: None,` after the
`stop_latency: None,` line there.

- [ ] **Step 3: Add a serialization unit test** — in `driver.rs` `mod tests`, after
`status_outcome_serializes_snake_case`:

```rust
    #[test]
    fn status_safety_flags_serializes_freed_part_disposition() {
        let s = Status {
            message: "status",
            action_id: "a/b/0005-unscrew".into(),
            outcome: Outcome::Succeeded,
            verdict: None,
            fidelity_tier: None,
            final_pose: None,
            failure_class: None,
            failure_detail: None,
            stop_latency: None,
            safety_flags: Some(SafetyFlags {
                freed_part_disposition: Some(FreedPartDisposition {
                    disposition: "retained".into(),
                    zone: None,
                }),
            }),
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("\"freed_part_disposition\""));
        assert!(j.contains("\"disposition\":\"retained\""));
        assert!(!j.contains("\"zone\"")); // skipped when None
    }
```

- [ ] **Step 4: Enumerate + fix the conformance Status literals** — run `cargo test --no-run 2>&1 |
grep -E "missing field .safety_flags|--> "` and add `safety_flags: None,` after the `stop_latency:
None,` line at every reported `crates/rfl-conformance/src/lib.rs` site (the same ~10 sites as the
`stop_latency` addition this session).

- [ ] **Step 5: Build + full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; rfl-core **85** (was 84, +1 the serialization test); conformance counts
UNCHANGED (lib 16, driver_protocol 9, envelope_conformance 45, goldens 5 each). validate.py PASS.
Goldens byte-identical (no driver emits safety_flags yet).

- [ ] **Step 6: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-core/src/driver.rs crates/rfl-conformance/src/lib.rs && \
git commit -m "feat(driver): add Status.safety_flags (freed-part disposition)

Adds the Rust Status.safety_flags struct (SafetyFlags + FreedPartDisposition)
to match the already-declared driver-interface schema (spec/04 TM21c freed-part
handling). No schema change; no driver populates it yet (next commit); all
literals None; goldens byte-identical.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only the 2 files.

---

### Task 2: `ReferenceDriver` population + `check_freed_part_disposition` + `FreeingDriver`

**Files:**
- Modify: `crates/rfl-core/src/driver.rs` (re-export — none needed; structs are pub already)
- Modify: `crates/rfl-conformance/src/lib.rs` (ReferenceDriver population; check fn; FreeingDriver)
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (3 integration tests; import)

- [ ] **Step 1: Write the failing unit test** — in `crates/rfl-conformance/src/lib.rs` `mod tests`,
after `check_settling_accepts_...`:

```rust
    #[test]
    fn check_freed_part_disposition_requires_a_disclosure_matching_intent() {
        use rfl_core::driver::{FreedPartDisposition, Outcome, RealizedPose, SafetyFlags, Status, Verdict};
        // a freeing action: force_profile.on_disengagement = "retain" -> expected "retained".
        let mut action = sample_action();
        action.safety_envelope.force_profile =
            Some(serde_json::json!({ "on_disengagement": "retain" }));
        let goal = ExecuteGoal::wrap("s/e/0005-unscrew".to_string(), action);
        let report = |flags: Option<SafetyFlags>| {
            let status = Status {
                message: "status",
                action_id: "s/e/0005-unscrew".to_string(),
                outcome: Outcome::Succeeded,
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
        let disp = |d: &str| Some(SafetyFlags {
            freed_part_disposition: Some(FreedPartDisposition { disposition: d.to_string(), zone: None }),
        });
        // discloses retained (matches retain) -> Pass.
        assert_eq!(check_freed_part_disposition(&goal, &report(disp("retained"))), CheckOutcome::Pass);
        // no disclosure on a freeing op -> uncontrolled drop -> Fail.
        assert!(matches!(check_freed_part_disposition(&goal, &report(None)), CheckOutcome::Fail(_)));
        // wrong disposition (released what should be retained) -> Fail.
        assert!(matches!(
            check_freed_part_disposition(&goal, &report(disp("safe_zone_release"))),
            CheckOutcome::Fail(_)
        ));
        // non-freeing action (no on_disengagement) -> vacuous Pass.
        let plain = ExecuteGoal::wrap("s/e/0001-align".to_string(), sample_action());
        assert_eq!(check_freed_part_disposition(&plain, &report(None)), CheckOutcome::Pass);
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p rfl-conformance --lib check_freed_part_disposition 2>&1 | grep -E "error\[|cannot find|test result|FAILED" | head`
Expected: RED — `check_freed_part_disposition` does not exist.

- [ ] **Step 3: Add `check_freed_part_disposition`** — in `crates/rfl-conformance/src/lib.rs`, after
`check_settling` (or beside the other `check_*` fns):

```rust
/// Verify the `spec/04` TM21c freed-part disposition contract: a freeing operation (an action
/// whose `force_profile.on_disengagement` is present — `force.unscrew`) must disclose
/// `status.safety_flags.freed_part_disposition` matching the authored intent (`retain` →
/// `retained`, `drop_safe` → `safe_zone_release`). An undisclosed freeing is an uncontrolled
/// drop (forbidden). Vacuous for any action without `on_disengagement`.
#[must_use]
pub fn check_freed_part_disposition(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    let on_diseng = goal
        .canonical_action
        .safety_envelope
        .force_profile
        .as_ref()
        .and_then(|fp| fp.get("on_disengagement"))
        .and_then(serde_json::Value::as_str);
    let Some(on_diseng) = on_diseng else {
        return CheckOutcome::Pass; // not a freeing operation
    };
    let expected = match on_diseng {
        "drop_safe" => "safe_zone_release",
        _ => "retained", // retain (and the lowering default)
    };
    let disclosed = report
        .status
        .safety_flags
        .as_ref()
        .and_then(|sf| sf.freed_part_disposition.as_ref());
    match disclosed {
        None => CheckOutcome::Fail(
            "uncontrolled drop: a freeing operation disclosed no freed_part_disposition".to_string(),
        ),
        Some(d) if d.disposition != expected => CheckOutcome::Fail(format!(
            "freed_part_disposition {} does not match the authored intent {expected}",
            d.disposition
        )),
        Some(_) => CheckOutcome::Pass,
    }
}
```

- [ ] **Step 4: Populate `safety_flags` in the `ReferenceDriver`** — in
`crates/rfl-conformance/src/lib.rs` `ReferenceDriver::execute`, just before the `let status = Status
{` literal (~line 172), compute the disclosure from `on_disengagement`:

```rust
        // Freed-part disposition (spec/04 TM21c): a freeing operation (force.unscrew, carrying
        // force_profile.on_disengagement) discloses where the freed part went — retained, or
        // released into a declared safe zone. Absent for every non-freeing action.
        let safety_flags = ca
            .safety_envelope
            .force_profile
            .as_ref()
            .and_then(|fp| fp.get("on_disengagement"))
            .and_then(serde_json::Value::as_str)
            .map(|od| {
                let (disposition, zone) = if od == "drop_safe" {
                    ("safe_zone_release", Some(serde_json::json!({ "zone": "discard_bin" })))
                } else {
                    ("retained", None)
                };
                SafetyFlags {
                    freed_part_disposition: Some(FreedPartDisposition {
                        disposition: disposition.to_string(),
                        zone,
                    }),
                }
            });
```

and set `safety_flags,` in the `Status { … }` literal (replacing `safety_flags: None,`). Add
`SafetyFlags, FreedPartDisposition` to the `use rfl_core::driver::{…}` import at the top of lib.rs.

- [ ] **Step 5: Run the unit test (green)**

Run: `cargo test -p rfl-conformance --lib check_freed_part_disposition 2>&1 | grep -E "test result"`
Expected: PASS.

- [ ] **Step 6: Add the `FreeingDriver` adversary** — in `crates/rfl-conformance/src/lib.rs`, beside
the other bench drivers (e.g. after `CutDriver`):

```rust
/// How a driver discloses a freeing operation's disposition (`spec/04` TM21c). `Discloses` is
/// conformant; `DropsUncontrolled` omits the disclosure (an uncontrolled drop); `FalseDisposition`
/// reports the opposite disposition (e.g. releases a part that should have been retained).
#[derive(Debug, Clone, Copy)]
pub enum FreeingResponse {
    /// Conformant: discloses the freed-part disposition (the nominal driver already does).
    Discloses,
    /// Adversarial: a freeing op with no disclosure — an uncontrolled drop.
    DropsUncontrolled,
    /// Adversarial: discloses the opposite disposition (released what should be retained).
    FalseDisposition,
}

/// The freed-part bench: reuses the nominal `ReferenceDriver` (which discloses the disposition for
/// a freeing op) and mutates that disclosure per `response`. Non-freeing actions have no
/// disclosure, so the mutations are no-ops on them.
#[derive(Debug)]
pub struct FreeingDriver {
    inner: ReferenceDriver,
    response: FreeingResponse,
}

impl FreeingDriver {
    /// A freeing driver with the given disclosure policy.
    #[must_use]
    pub fn new(response: FreeingResponse) -> Self {
        FreeingDriver { inner: ReferenceDriver::default(), response }
    }
}

impl Driver for FreeingDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        match self.response {
            FreeingResponse::Discloses => {} // nominal: the disclosure stands
            FreeingResponse::DropsUncontrolled => {
                report.status.safety_flags = None; // freeing with no disposition recorded
            }
            FreeingResponse::FalseDisposition => {
                if let Some(d) = report
                    .status
                    .safety_flags
                    .as_mut()
                    .and_then(|sf| sf.freed_part_disposition.as_mut())
                {
                    d.disposition = if d.disposition == "retained" {
                        "safe_zone_release".to_string()
                    } else {
                        "retained".to_string()
                    };
                }
            }
        }
        report
    }
}
```

- [ ] **Step 7: Add the integration tests** — in `crates/rfl-conformance/tests/envelope_conformance.rs`,
add the imports `check_freed_part_disposition, FreeingDriver, FreeingResponse` to the
`use rfl_conformance::{…}` block, then add (mirroring the existing unscrew tests, index 5):

```rust
#[test]
fn nominal_unscrew_discloses_freed_part_disposition() {
    let dir = screw_dir();
    let pairs = drive(
        FreeingDriver::new(FreeingResponse::Discloses),
        &dir.join("skill-unscrew.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "unscrew");
    assert_eq!(check_freed_part_disposition(goal, report), CheckOutcome::Pass);
}

#[test]
fn unscrew_uncontrolled_drop_fails_disposition() {
    let dir = screw_dir();
    let pairs = drive(
        FreeingDriver::new(FreeingResponse::DropsUncontrolled),
        &dir.join("skill-unscrew.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "unscrew");
    assert!(matches!(check_freed_part_disposition(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn unscrew_false_disposition_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FreeingDriver::new(FreeingResponse::FalseDisposition),
        &dir.join("skill-unscrew.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[5];
    assert!(matches!(check_freed_part_disposition(goal, report), CheckOutcome::Fail(_)));
}
```

- [ ] **Step 8: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; rfl-core 85; conformance `lib` 17 (+1), `envelope_conformance` 48 (+3);
driver_protocol 9; all 12 golden binaries byte-identical (the unscrew safety_flags is in the
DRIVER report, and `driver_protocol` goldens are cable-only); `screw_fasten_unscrew` retarget
goldens unchanged (`on_disengagement` was already emitted). validate.py PASS.

- [ ] **Step 9: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs && \
git commit -m "feat(conformance): enforce freed-part disposition (TM21c)

A freeing operation (force.unscrew, via force_profile.on_disengagement) must
disclose status.safety_flags.freed_part_disposition matching the authored
intent; an undisclosed freeing is an uncontrolled drop (spec/04 TM21c). The
ReferenceDriver populates it; check_freed_part_disposition reads it; a
FreeingDriver with DropsUncontrolled (omits disclosure) and FalseDisposition
(opposite disposition) adversaries proves safety_flags is load-bearing. First
use of the safety_flags channel.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only the 2 files.

---

### Task 3: README status line

**Files:** Modify: `README.md` (~line 84)

- [ ] **Step 1: Extend the README** — note that a freeing operation (force.unscrew) must now
disclose `safety_flags.freed_part_disposition` (`retained` / `safe_zone_release`) matching intent,
an undisclosed freeing being an uncontrolled drop (`spec/04` TM21c); the first use of the
`safety_flags` audit channel. Match the surrounding phrasing.

- [ ] **Step 2: Verify nothing regressed**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: unchanged green.

- [ ] **Step 3: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add README.md && \
git commit -m "docs(readme): note freed-part disposition disclosure (TM21c)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only README.md.

---

## Post-increment

- Final full-suite read: rfl-core 85; conformance `lib` 17, `envelope_conformance` 48; all 12
  golden binaries byte-identical; validate.py PASS.
- Update `project_rfl.md` "Implementation track" (24th increment, real hashes + deltas) +
  the `MEMORY.md` RFL line (now compacted — append a short note to the hook, do NOT re-bloat).

## Self-review (spec coverage)

- TM21c disposition contract (design § 1) → Task 2 check + ReferenceDriver. ✓
- `safety_flags` Rust struct matching the schema (design § 2/3) → Task 1. ✓
- DropsUncontrolled + FalseDisposition non-circularity (design § 4) → Task 2 Step 6/7. ✓
- No schema / golden change (design § 2) → confirmed: Task 1 = Rust struct only; Task 2 =
  conformance only; goldens byte-identical. ✓
- Type consistency: `Status.safety_flags: Option<SafetyFlags>`;
  `SafetyFlags { freed_part_disposition: Option<FreedPartDisposition> }`;
  `FreedPartDisposition { disposition: String, zone: Option<Value> }`;
  `check_freed_part_disposition(goal, report)`; `FreeingDriver`/`FreeingResponse`. ✓
