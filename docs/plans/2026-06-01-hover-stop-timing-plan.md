# hover stop_time abort-timing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Checkbox steps.
> **LOCAL-ONLY** — never `git add` this file.

**Goal:** Enforce spec/01 § 1.5 C2 (line 685): an over-envelope `reach.hover` abort must reach a
safe state within `embodiment.limits.stop_time`. The first latency/timing conformance check.

**Architecture:** Additive optional `Status.stop_latency` (Duration) measurement (mirrors
increment 15's `station_error`); `Envelope.stop_time` already emitted. `check_settling` gains the
goal + a timing leg (non-vacuous for the `station_exceeded` abort). One adversarial driver.
Design: `docs/design/2026-06-01-hover-stop-timing-design.md` (committed `0cd4f5f`).

**Tech Stack:** Rust (rfl-core + rfl-conformance), `schemas/driver-interface.schema.json`, validate.py.

**Discipline (every task):** TDD red→green. After green, full `cargo test` (workspace, NOT `-p`)
+ `uv run --with jsonschema --with pyyaml python schemas/validate.py` in a batch SEPARATE from the
commit. Before each push: `git fetch -q && git merge-base --is-ancestor origin/main HEAD`,
`git add <explicit paths>`, commit, `git push origin main`, verify sync `0 0` + `git show --stat`.

**Exhaustive-init coupling:** adding `Status.stop_latency` breaks every `Status { … }` literal
(struct has no `..Default`). The 11 sites (Task 1 Step 2) MUST all gain `stop_latency: None` in
the same commit or the build is red.

---

### Task 1: additive `Status.stop_latency` field + schema `Duration` def (green checkpoint, no behavior change)

**Files:**
- Modify: `crates/rfl-core/src/driver.rs` (Status field + the one rfl-core test literal :187)
- Modify: `schemas/driver-interface.schema.json` (Duration $def + StatusResult.stop_latency)
- Modify: `crates/rfl-conformance/src/lib.rs` (10 Status literals gain `stop_latency: None`)

- [ ] **Step 1: Add the field** — in `crates/rfl-core/src/driver.rs`, in `pub struct Status`,
after `failure_detail`:

```rust
    /// The externally measured time from envelope breach to the at-rest safe state, bounded by
    /// `embodiment.limits.stop_time` (`spec/01` § 1.5 C2 abort timing). Present only on an
    /// aborted action (the bench measures it); a nominal success omits it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_latency: Option<Quantity>,
```

(`Quantity` is already imported in driver.rs — `securing_force: Option<Quantity>` uses it.)

- [ ] **Step 2: Add `stop_latency: None` to every Status literal** — these sites (compile error
lists them if any missed): `crates/rfl-core/src/driver.rs:187`; `crates/rfl-conformance/src/lib.rs`
lines `172` (the ReferenceDriver production builder — nominal success, `None`), `1328`, `1360`,
`1406`, `1464`, `1532`, `1587`, `1644`, `1698`, `1737`. Add `stop_latency: None,` after the
`failure_detail` line in each.

- [ ] **Step 3: Add the schema `Duration` $def + StatusResult property** — in
`schemas/driver-interface.schema.json`, add a `Duration` $def after the `Length` $def (a realized
wire value — concrete `s`/`ms`, no Ref/Auto, parallel to this schema's `Length`/`Force`):

```json
    "Duration": {
      "description": "A duration value (canonical wire unit s): a unit-suffixed quantity string (s / ms). A realized wire value carries no let-reference (post-execution measurement).",
      "type": "string",
      "pattern": "^-?[0-9]+(\\.[0-9]+)?\\s+(s|ms)$"
    },
```

and in `StatusResult.properties` (after `final_pose`):

```json
        "stop_latency": { "$ref": "#/$defs/Duration", "description": "Externally measured time from envelope breach to the at-rest safe state, bounded by embodiment.limits.stop_time (01 § 1.5 C2); present only on an aborted action. Read by check_settling." },
```

- [ ] **Step 4: Build + full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green, all counts UNCHANGED from increment 22 (additive optional field, every literal
`None`, no check reads it yet). validate.py PASS (additive optional property + new $def, C1–C7
untouched). Goldens byte-identical.

- [ ] **Step 5: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-core/src/driver.rs schemas/driver-interface.schema.json crates/rfl-conformance/src/lib.rs && \
git commit -m "feat(driver): add Status.stop_latency abort-timing measurement

Additive optional Duration field on StatusResult: the externally measured
time from envelope breach to the at-rest safe state (spec/01 § 1.5 C2),
bounded by embodiment.limits.stop_time. Mirrors increment 15's station_error.
No check reads it yet (next commit); all literals None; goldens byte-identical.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only the 3 files.

---

### Task 2: `check_settling` timing leg + `AbortsTooSlow` adversary + tests

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (check_settling signature+leg; HoverResponse variant;
  Aborts arm emits stop_latency; AbortsTooSlow arm; unit tests + signature follow at call sites)
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (2 call-site updates + 1 new test)

- [ ] **Step 1: Write the failing unit test** — in `crates/rfl-conformance/src/lib.rs`, REPLACE the
existing `check_settling_accepts_station_exceeded_abort_rejects_pretended_success` test body to use
the new `(goal, report)` signature and add the timing assertions. The goal carries
`stop_time = "0.1 s"`; the report's abort sets `stop_latency`:

```rust
    #[test]
    fn check_settling_accepts_station_exceeded_abort_rejects_pretended_success() {
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        use rfl_core::quantity::Quantity;
        let mut action = sample_action();
        action.safety_envelope.stop_time = Some(Quantity("0.1 s".into()));
        let goal = ExecuteGoal::wrap("s/e/0001-hover".to_string(), action);
        let status = |outcome: Outcome, detail: Option<&str>, lat: Option<&str>| Status {
            message: "status",
            action_id: "s/e/0001-hover".to_string(),
            outcome,
            verdict: Some(Verdict { value: false, confidence: 1.0, evidence: vec![] }),
            fidelity_tier: None,
            final_pose: Some(RealizedPose::placeholder()),
            failure_class: detail.map(|_| "blocked".to_string()),
            failure_detail: detail.map(str::to_string),
            stop_latency: lat.map(|s| Quantity(s.to_string())),
        };
        let report = |o, d, l| DriverReport { telemetry: vec![], status: status(o, d, l) };
        // abort within stop_time -> Pass.
        assert_eq!(
            check_settling(&goal, &report(Outcome::Failed, Some("station_exceeded"), Some("0.05 s"))),
            CheckOutcome::Pass
        );
        // abort too slow (> stop_time) -> Fail (the new timing leg).
        assert!(matches!(
            check_settling(&goal, &report(Outcome::Failed, Some("station_exceeded"), Some("0.5 s"))),
            CheckOutcome::Fail(_)
        ));
        // abort with no measured latency -> Fail (malformed abort, non-vacuous timing leg).
        assert!(matches!(
            check_settling(&goal, &report(Outcome::Failed, Some("station_exceeded"), None)),
            CheckOutcome::Fail(_)
        ));
        // pretended success -> Fail (outcome leg, unchanged).
        assert!(matches!(
            check_settling(&goal, &report(Outcome::Succeeded, None, None)),
            CheckOutcome::Fail(_)
        ));
        // wrong halt reason -> Fail (outcome leg, unchanged).
        assert!(matches!(
            check_settling(&goal, &report(Outcome::Failed, Some("blocked"), None)),
            CheckOutcome::Fail(_)
        ));
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p rfl-conformance --lib check_settling 2>&1 | grep -E "error\[|cannot|test result|FAILED" | head`
Expected: RED — `check_settling` takes 1 arg not 2 (and the timing branches don't exist yet).

- [ ] **Step 3: Change `check_settling` to take the goal + add the timing leg** — in
`crates/rfl-conformance/src/lib.rs`, replace the function (currently `pub fn check_settling(report:
&DriverReport)`, ~line 979) with:

```rust
#[must_use]
pub fn check_settling(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    if matches!(report.status.outcome, Outcome::Succeeded) {
        return CheckOutcome::Fail(
            "claimed success under a station-exceeding disturbance".to_string(),
        );
    }
    if report.status.failure_detail.as_deref() != Some("station_exceeded") {
        return CheckOutcome::Fail(format!(
            "expected failure_detail station_exceeded, got {:?}",
            report.status.failure_detail
        ));
    }
    // Timing leg (spec/01 § 1.5 C2): the abort must reach a safe state within stop_time.
    // Non-vacuous for the station_exceeded abort — an abort that omits its measured time, or
    // that overruns stop_time, is malformed.
    let bound = goal
        .canonical_action
        .safety_envelope
        .stop_time
        .as_ref()
        .and_then(quantity_mag);
    let Some(bound) = bound else {
        return CheckOutcome::Fail("envelope missing stop_time for an aborting hover".to_string());
    };
    match report.status.stop_latency.as_ref().and_then(quantity_mag) {
        None => CheckOutcome::Fail("station_exceeded abort reported no stop_latency".to_string()),
        Some(lat) if lat > bound => {
            CheckOutcome::Fail(format!("abort stop_latency {lat} s exceeds stop_time {bound} s"))
        }
        Some(_) => CheckOutcome::Pass,
    }
}
```

(`quantity_mag` is the existing helper at lib.rs:760; both `stop_time` and `stop_latency` parse to
SI seconds.)

- [ ] **Step 4: Add the `AbortsTooSlow` HoverResponse variant + arm; make `Aborts` emit
stop_latency** — in `enum HoverResponse` (line ~358), add after `Aborts`:

```rust
    /// Adversarial (over-envelope): aborts honestly (station_exceeded) but overruns stop_time.
    AbortsTooSlow,
```

In `HoverSettlingDriver::execute` `match self.response`, replace the `Aborts` arm and add the new
arm (the helper reads the envelope stop_time so the latency is descriptor-relative):

```rust
            HoverResponse::Aborts | HoverResponse::AbortsTooSlow => {
                report.status.outcome = Outcome::Failed;
                report.status.failure_class = Some("blocked".to_string());
                report.status.failure_detail = Some("station_exceeded".to_string());
                if let Some(v) = report.status.verdict.as_mut() {
                    v.value = false; // honest: the station was not held
                }
                let stop = goal
                    .canonical_action
                    .safety_envelope
                    .stop_time
                    .as_ref()
                    .and_then(|q| q.parse().map(|(v, _)| v))
                    .unwrap_or(0.1);
                let factor = if matches!(self.response, HoverResponse::AbortsTooSlow) { 5.0 } else { 0.5 };
                report.status.stop_latency = Some(rfl_core::quantity::Quantity::from_si(stop * factor, "s"));
            }
```

- [ ] **Step 5: Run the unit test (green)**

Run: `cargo test -p rfl-conformance --lib check_settling 2>&1 | grep -E "test result"`
Expected: PASS.

- [ ] **Step 6: Follow the signature change at the `envelope_conformance.rs` call sites + add a
too-slow integration test** — in `crates/rfl-conformance/tests/envelope_conformance.rs`, the two
existing `check_settling(report)` calls (lines ~432, ~449) become `check_settling(goal, report)`.
The hover ENV3 test already binds the `(goal, report)` pair from `drive(...)`; use that goal. Then
add a new test mirroring the existing `Aborts` test but with `HoverResponse::AbortsTooSlow`,
asserting `check_settling(goal, report)` is `Fail` while the outcome itself is still
`station_exceeded` (the non-circular proof). Match the existing test's harness exactly (same
skill-hover.yaml drive setup).

- [ ] **Step 7: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; rfl-core unchanged; conformance `lib` unchanged count (the unit test was replaced
in place, +0) — UNLESS a separate too-slow lib test is added; `envelope_conformance` +1 (the new
AbortsTooSlow integration test). All 12 golden binaries byte-identical. validate.py PASS.

- [ ] **Step 8: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs && \
git commit -m "feat(conformance): enforce hover abort within stop_time

check_settling gains the goal and a timing leg: an over-envelope hover that
aborts (station_exceeded) must report a stop_latency <= envelope stop_time
(spec/01 § 1.5 C2). The first latency/timing conformance check. Non-vacuous
for the abort case. AbortsTooSlow adversary (honest station_exceeded but
overruns stop_time) fails the timing leg while the outcome leg passes,
proving stop_latency is load-bearing.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only the 2 files.

---

### Task 3: README status line

**Files:** Modify: `README.md` (~line 84)

- [ ] **Step 1: Extend the README** — note that the hover over-envelope abort is now timing-checked:
the abort must reach a safe state within `stop_time` (`spec/01` § 1.5 C2), the first
latency/timing conformance check. Match the surrounding phrasing; keep the existing structure.

- [ ] **Step 2: Verify nothing regressed**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: unchanged green.

- [ ] **Step 3: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add README.md && \
git commit -m "docs(readme): note hover abort stop_time timing check

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only README.md.

---

## Post-increment

- Final full-suite read: rfl-core unchanged; conformance `envelope_conformance` +1; the `lib`
  check_settling unit test rewritten in place (timing assertions added); all 12 golden binaries
  byte-identical; validate.py PASS.
- Update `project_rfl.md` "Implementation track" (23rd increment, real hashes + test deltas) + the
  `MEMORY.md` RFL line. Park `max_excursion` (not a spec concept) and non-hover abort timing.

## Self-review (spec coverage)

- spec/01 § 1.5 C2 abort-within-stop_time (design § 1) → Task 2 timing leg. ✓
- `stop_latency` measurement channel (design § 2) → Task 1 (field + schema). ✓
- AbortsTooSlow non-circularity (design § 4) → Task 2 Step 4/6. ✓
- No envelope class / golden / spec change (design scope) → confirmed: Task 1 = additive field +
  schema $def; Task 2 = conformance only; goldens byte-identical. ✓
- Type consistency: `Status.stop_latency: Option<Quantity>`; `check_settling(goal, report)`;
  `quantity_mag` for both bound and measured; `HoverResponse::AbortsTooSlow`. ✓
