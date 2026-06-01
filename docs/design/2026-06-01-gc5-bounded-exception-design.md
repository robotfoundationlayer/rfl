# Design — GC5 bounded continuity-exception (in_hand.flip) (v0)

**Date:** 2026-06-01
**Track:** grasp-continuity — the bounded-exception subclass (GC2-6 sub-arc)
**Status:** approved (autonomous run — user directive "推奨順に全てのタスクを自動実行")
**Scope:** `rfl-core` (`lower_in_hand_flip` emits the bounded-window contract on the wire) + `rfl-conformance` (`ReferenceDriver` reports the measured window; `check_flip_bounded_window` in the battery; a `FlipWindowExceeded` adversary). No spec / schema change (the contract rides the open Envelope `force_profile` floor; the measured window rides `verdict.evidence`).

## 0. Why

GC5 (`spec/05` § Bounded continuity-exception) covers the **one** primitive that *suspends* grasp continuity: `in_hand.flip` momentarily releases the object (a 180° toss-and-recatch). Unlike the continuity-preserving modes, the suite verifies the exception is **bounded**: the unsecured window ≤ `max_release_time`, the re-catch occurs within `catch_envelope`, and on a failed catch the object lands within `safe_drop_zone`. GC5 is chosen as the next GC increment because `in_hand.flip` **already exists** (the AUD2 increment lowered it) — unlike GC3/GC4/GC6, which each need a brand-new primitive. The flip already parses `max_release_time` (required) and `safe_drop_zone` (required) but `lower_in_hand_flip` currently **drops them** — they never reach the wire, so no check can read the bound. This increment puts the contract on the wire and verifies the window.

## 1. T1 — `lower_in_hand_flip` emits the bounded-window contract

The flip action's `force_profile` gains the bound and the safe-drop region (symbolic, v0 — the authored strings, no reformat):
```rust
env.force_profile = Some(serde_json::json!({
    "max_release_time": p.max_release_time.0,     // the hard upper bound on the unsecured window
    "safe_drop_zone": <p.safe_drop_zone as json>, // the no-uncontrolled-drop landing region
}));
```
The `momentary_release` declaration is already carried by the driver's AUD2 evidence; this adds the *bound* the GC5 check measures against. Re-blesses the 3 `flip__flip_*` retarget goldens (the `0003-flip` action line gains the `force_profile`). No other golden (cable / screw / carry / scan) contains a flip.

## 2. T2 — the measured window + GC5 check

### 2.1 `ReferenceDriver` — report the measured unsecured window

When an action carries `force_profile.max_release_time` (the bounded-window contract — only a flip does), the nominal driver attests it completed a flip whose measured unsecured window stayed within the bound and the re-catch was confirmed:
```rust
if let Some(bound) = force_profile.max_release_time.parse() {
    // v0 models the measured window as a deterministic fraction of the bound (no physics):
    // 80% of the allotted release window, formatted via Quantity::from_si.
    let measured = Quantity::from_si(bound.0 * 0.8, bound.1);
    evidence.push(format!("unsecured_window:{}", measured.0));
    evidence.push("recatch_confirmed".to_string());
}
```
This models the response the way ENV3 models disturbance and GC2 models the hold test (v0 has no physics). Changes only the flip driver-report fixture + flip cert.

### 2.2 `check_flip_bounded_window(goal, report)` (`rfl-conformance`)

Added to `battery::verify_action` (vacuous-pass when the action carries no `max_release_time`):
```
let Some(bound) = force_profile.max_release_time.parse() else { return Pass };  // not a bounded-exception action
if outcome != Succeeded { return Pass };                                        // failure path is its own (recatch_failed -> safe drop)
let measured = evidence.find("unsecured_window:<m>").parse() else { return Fail("no measured window") };
if measured > bound { return Fail("unsecured window <m> exceeds max_release_time <bound>") };
if !evidence.contains("recatch_confirmed") { return Fail("no re-catch confirmation") };
Pass
```
Every cert action gains a `flip_bounded_window` check entry, so the 3 example certs are re-blessed (the established pattern). Quantities compared by magnitude (`Quantity::parse`), same unit (s) — consistent with the existing `securing_floor_violation` magnitude comparison.

### 2.3 The bite — `Fault::FlipWindowExceeded`

A new fault makes the driver report a measured window **over** the bound (`bound · 1.5`), violating GC5. `check_flip_bounded_window` rejects it. Non-circular: the nominal flip passes (window within bound + recatch), a flip that held the object unsecured too long fails.

## 3. Testing (TDD)

- `rfl-core` unit: `lower_in_hand_flip` on the flip example emits `force_profile.max_release_time == "0.3 s"` and `safe_drop_zone`.
- `rfl-conformance` lib unit `check_flip_bounded_window`: a flip goal with a measured window ≤ bound + recatch → Pass; a window > bound → Fail; no measured window → Fail; no recatch → Fail; a non-flip action (no max_release_time) → vacuous Pass; a non-Succeeded flip → vacuous Pass.
- integration on the flip example (`tests/flip.rs` / via the cert path): the nominal flip report passes; the `FlipWindowExceeded` driver's flip report fails.
- Re-bless: the 3 `flip__flip_*` retarget goldens (T1), the flip driver-report fixture + the 3 example certs (T2). Full `cargo test --workspace` (real exit + `grep -c FAILED`), fmt, clippy, validate.py(uv), `gh run` CI-green.

## 4. Out of scope (YAGNI / deferred)

The C2 recatch-failure → `safe_drop_zone` landing path (a geometric Region-containment check — needs concrete poses, the standard v0 symbolic-pose deferral; the safe_drop_zone is on the wire now for when it lands); `catch_envelope` geometric containment (same reason); the `continuity_alternative_exists` composition rejection (`spec/01` § flip admissibility — a class-1 `validate` check, a separate increment); the physical window measurement (v0 models it). GC3 (regrasp), GC4 (pivot), GC6 (handoff) each remain a fresh increment needing a new primitive.
