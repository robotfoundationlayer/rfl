# Design — GC2 hold-test closure (v0)

**Date:** 2026-06-01
**Track:** grasp-continuity — the operational closure test (first of the GC2-6 sub-arc)
**Status:** approved (autonomous run — user directive "推奨順に全てのタスクを自動実行")
**Scope:** `rfl-conformance` only — `ReferenceDriver` emits a closure-branched hold-test evidence entry; `check_hold_test` joins the per-action battery; a `HoldTestFault` adversary proves the bite. No `rfl-core` / spec / schema change (the hold-test rides `verdict.evidence`, the open `VerdictFloor`, the established home for continuity signals).

## 0. Why

GC2 (`spec/05` § The hold test and closure branching) is the operational definition of a successful grasp: a calibrated sub-budget perturbation is applied and the object is retained, with the **perturbation profile selected by the closure type** — `force → omnidirectional`, `form → load_direction`, `support → level, gentle`. The closure is now on the wire (`grasp_stability.closure`, the 4a foundation), so the hold test is verifiable from the report. GC1 (continuous securing) already checks `securing_force ≥ min_holding_force` across telemetry; GC2 adds the perturbation dimension — that the securing held under the *closure-appropriate* perturbation, not that it merely held undisturbed.

This is the demand-side counterpart to the ENV3 disturbance increment (#14): the perturbation is part of the conformance bench, not the primitive, and v0 *models* the driver's response (no physics, same posture as symbolic poses). The hold test is reported by the driver and the check verifies the profile matches the closure.

## 1. `ReferenceDriver` — emit the hold-test evidence

For any grasp-establishing action (`ca.grasp_stability` is `Some` — pinch / pin / platform; `grasp.release` and non-grasp actions have none), push one evidence entry recording the closure-appropriate perturbation profile the nominal driver applied and retained under:
```rust
if let Some(st) = &ca.grasp_stability {
    let profile = match st.closure {
        Closure::Force => "omnidirectional",
        Closure::Form => "load_direction",
        Closure::Support => "level_gentle",
    };
    evidence.push(format!("hold_test:{profile}"));
}
```
This changes only the cable `driver_protocol` goldens (3) — the `grasp.pinch` action's verdict gains `"hold_test:omnidirectional"`; every other cable action has no `grasp_stability` so its verdict is unchanged. The retarget-output snapshots (flip / screw / carry / scan) are driver-independent and unchanged.

## 2. `check_hold_test(goal, report)` (`rfl-conformance`)

```rust
let Some(st) = &goal.canonical_action.grasp_stability else { return Pass };   // non-grasp
if report.status.outcome != Outcome::Succeeded { return Pass };              // failure has its own path
let expected = match st.closure { Force => "omnidirectional", Form => "load_direction", Support => "level_gentle" };
match report.status.verdict.evidence.find("hold_test:<p>") {
    Some(p) if p == expected => Pass,
    Some(p) => Fail("hold test used <p> but <expected> is required for <closure> closure"),
    None    => Fail("a successful grasp must report a hold test"),
}
```
Added to `battery::verify_action` after `support_safe_state` (the per-action battery; vacuous-pass for non-grasp). Every cert action gains a `hold_test` check entry, so the 3 example certs are re-blessed (the established pattern, as AUD1 / STB2 did).

## 3. The bite — `HoldTestFault`

A new `Fault::WrongHoldTest` (or a small dedicated wrapper) mutates the nominal grasp report to report the **wrong** perturbation profile — e.g. `omnidirectional` replaced by `level_gentle` on a force-closure pinch (a profile that would not certify a force grasp), or strips the `hold_test` evidence entirely. `check_hold_test` rejects it. This is the non-circular proof: the nominal driver passes, a driver that ran the wrong hold test fails. The simplest faithful injection: replace the `hold_test:*` evidence entry with `hold_test:level_gentle` (wrong for a force closure) on grasp actions.

## 4. Testing (TDD)

- `rfl-conformance` lib unit `check_hold_test`: a force grasp with `hold_test:omnidirectional` → Pass; with `hold_test:level_gentle` → Fail; with no hold_test → Fail; a support grasp with `level_gentle` → Pass; a non-grasp action → vacuous Pass; a non-Succeeded grasp → vacuous Pass.
- integration on the cable example: the nominal `ReferenceDriver` grasp.pinch report passes `check_hold_test`; the `WrongHoldTest` driver's pinch report fails it.
- battery: the nominal pinch action's verdict now carries the `hold_test` check (passing).
- Re-bless: cable `driver_protocol` (3 goldens) + the 3 example certs. Full `cargo test --workspace` (real exit + `grep -c FAILED`), fmt, clippy, validate.py(uv), `gh run` CI-green.

## 5. Out of scope (YAGNI / deferred)

GC3 (make-before-break — `in_hand.regrasp` + contact-set/union force trace), GC4 (controlled under-actuation — `in_hand.pivot` + per-DOF securing), GC5 (bounded exception — `in_hand.flip` exists, but the unsecured-window *timing* ≤ `max_release_time` + catch_envelope + safe_drop_zone needs a timing report), GC6 (two-party — `transport.handoff` + combined-force ceiling). The physical perturbation injection (v0 models it). The `form → load_direction` profile has no emitter yet (no form-closure mode lowered — hook/envelope deferred); the branch is present and unit-tested for when a form grasp lands.
