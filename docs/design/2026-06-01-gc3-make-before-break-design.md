# Design — GC3 make-before-break (in_hand.regrasp) (v0)

**Date:** 2026-06-01
**Track:** grasp-continuity — the make-before-break mode (GC2-6 sub-arc)
**Status:** approved (autonomous run — user directive "推奨順に全てのタスクを自動実行")
**Scope:** `rfl-core` (new `in_hand.regrasp` primitive + a worked example + descriptor caps) + `rfl-conformance` (`envelope_class_for("regrasp")`, `check_make_before_break`, a `BreakBeforeMake` adversary). No spec change (`InHandRegraspParams` already typed); no schema change (the transition marker rides the open Envelope `force_profile` floor; the ordering rides `verdict.evidence`).

## 0. Why

GC3 (`spec/05` § Make-before-break and gaiting) covers the contact-set transition: `in_hand.regrasp` swaps one stable grasp for another **without releasing the object**, using make-before-break — the new grasp is confirmed *before* the old is released, so the two overlap rather than gap. The class invariant ("a securing contact set maintains the object at ≥ `min_holding_force` at every instant") splits cleanly:
- the **union-securing floor** ("the union of old ∪ new contacts secures at every instant") is exactly what GC1's `securing_floor_violation` already verifies across the trace — so emitting `min_holding_force` on the regrasp gets that coverage for free;
- the **make-before-break ordering** ("new confirmed before old released") is GC3's *new* content — a discipline GC1 cannot see, because GC1 only checks the floor's magnitude, not the ordering of the contact handover.

`in_hand.regrasp` is a brand-new primitive (unlike GC5's flip reuse). Two green commits: **(T1)** the primitive + a worked example, **(T2)** the make-before-break check + driver evidence + adversary.

## 1. T1 — `in_hand.regrasp` primitive

### 1.1 `InHandRegrasp` struct (`skill_isa`)
```rust
pub struct InHandRegrasp {
    pub target_mode: String,                        // GraspMode to transition to (v0: a mode name)
    #[serde(default)] pub grasp_handle: Option<GraspHandle>,
    #[serde(default)] pub target_contacts: Option<serde_yaml::Value>,
    #[serde(default)] pub target_force_budget: Option<serde_yaml::Value>,
}
impl InHandRegrasp {
    /// Map the target_mode name to a known v0 GraspMode (pin / platform / else pinch).
    pub fn target_grasp_mode(&self) -> GraspMode { ... }
}
```
`Primitive::InHandRegrasp(InHandRegrasp)` (externally-tagged `in_hand.regrasp`). The `Primitive::establishes_grasp` arm returns `Some(target_grasp_mode())` — so STB3 correctly tracks the post-regrasp active grasp (a regrasp to a `surface_bound`/`support` mode would then forbid a free-transport successor).

### 1.2 `lower_in_hand_regrasp(p, e, ctx: &GraspContext)`
- `grasp_stability = Some(for_mode(p.target_grasp_mode()))` — the new grasp's stability class (so the regrasp's hold test, GC2, confirms the *new* grasp, faithful to "the new grasp is confirmed").
- `force_profile = { min_holding_force: <from ctx.held>, transition: "make_before_break" }` — the continuity floor (read by GC1) and the make-before-break transition marker (the GC3 contract on the wire). `min_holding_force` from `grasp_force::min_holding_force(ctx.held.weight_n, ctx.held.mode)` (the held object's floor, mirroring `lower_transport_move_to_pose`'s held-floor emit); absent if no held mass is known.
- `target_pose = PoseExpr::Ref { ref: "held" }` (symbolic — the regrasp preserves the object pose; poses are presence-only in conformance).
- `target_frame = grasp_frame`; `force_budget: None`; capability gate `"in_hand.regrasp"`.
- Read-only `ctx` (v0 `target_mode` equals the current mode in the worked example, so no mode mutation; the ctx mode-change is deferred with a differing target_mode).

### 1.3 Worked example
`examples/03-screw-fasten/skill-regrasp.yaml`: `sense.locate → grasp.pinch → in_hand.regrasp(target_mode: pinch) → grasp.release` (the part carries `estimated_mass` so the pinch sets `ctx.held` and the regrasp emits the floor). The 3 example-03 descriptors gain `in_hand.regrasp` in their `skills` lists (Class-1 valid — `in_hand.regrasp ∈ PrimitiveId`).

### 1.4 T1 tests
`rfl-core` translation unit: retarget the regrasp example, assert the regrasp action (index 2) carries `force_profile.transition == "make_before_break"`, `force_profile.min_holding_force`, and `grasp_stability.closure == Force`. Run full `cargo test` to confirm the descriptor-cap additions break nothing (the transport.carry lesson — a cap addition once broke a cap-absence test).

## 2. T2 — make-before-break check

### 2.1 `envelope_class_for("regrasp") = GraspContinuity`
So the regrasp gets GC1's securing-floor envelope check (the union-securing floor) for free.

### 2.2 `ReferenceDriver` — report the ordering
For any action carrying `force_profile.transition == "make_before_break"`, push evidence `make_before_break:confirmed` (the nominal driver confirmed the new grasp before releasing the old). Changes only the regrasp driver report (no committed cable/screw/flip action carries the transition marker).

### 2.3 `check_make_before_break(goal, report)` (`rfl-conformance`)
Added to `battery::verify_action` (vacuous unless the action carries the transition marker):
```
let Some("make_before_break") = force_profile.transition else { return Pass };  // not a regrasp transition
if outcome != Succeeded { return Pass };                                        // failure path = abort-to-original
if evidence.contains("make_before_break:confirmed") { Pass } else { Fail("regrasp released the old grasp before confirming the new (break-before-make gap)") }
```
Every cert action gains a `make_before_break` check entry → the 3 example certs re-bless (the established battery pattern).

### 2.4 The bite — `Fault::BreakBeforeMake`
Replaces `make_before_break:confirmed` with `make_before_break:gap` (the old grasp released before the new confirmed → an unsecured instant). `check_make_before_break` rejects it. Non-circular: a nominal regrasp passes (confirmed overlap), a break-before-make regrasp fails.

## 3. Testing (TDD)
- `rfl-core` unit (T1, above).
- `rfl-conformance` lib unit `check_make_before_break`: a transition goal with `confirmed` → Pass; with `gap` → Fail; with no ordering evidence → Fail; a non-transition action → vacuous Pass; a non-Succeeded transition → vacuous Pass.
- integration on the regrasp example: the nominal regrasp report passes (make_before_break + the GC1 floor); the `BreakBeforeMake` driver's regrasp report fails.
- `tests/regrasp.rs`: retarget golden (3 hands) + generate-twice + boon schema validity.
- Re-bless: the 3 example certs (T2 battery entry). Full `cargo test --workspace` (real exit + `grep -c FAILED`), fmt, clippy, validate.py(uv), `gh run` CI-green.

## 4. Out of scope (YAGNI / deferred)
The C2 failed-target fallback (`target_grasp_failed` → retain original grasp — a non-Succeeded path, vacuous here; the abort-to-original is the driver's behavior, modeled when a failure driver lands); the `GraspRef` supersession / stale-handle composition check (`spec/01` § 3.3 — a class-1 `validate` check, a separate increment); gaiting via `in_hand.rotate`/`translate` (the same union-securing applies, deferred with those primitives); `preserve_pose` drift (`position_tolerance` — needs concrete poses); a differing `target_mode` mutating `ctx.held.mode` (v0 uses an equal mode). GC4 (pivot), GC6 (handoff) remain fresh new-primitive increments.
