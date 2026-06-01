# Design — GC6 two-party co-grasp (transport.handoff) (v0)

**Date:** 2026-06-01
**Track:** grasp-continuity — the two-party mode (the LAST of GC2-6)
**Status:** approved (autonomous run — user directive "推奨順に全てのタスクを自動実行")
**Scope:** `rfl-core` (new `transport.handoff` primitive + a worked example + descriptor caps) + `rfl-conformance` (`envelope_class_for("handoff")`, `check_two_party_handoff`, a `CograspOverforce` adversary). No spec change (`TransportHandoffParams` already typed); no schema change (the markers ride the open `force_profile` floor; the attestations ride `verdict.evidence`).

## 0. Why

GC6 (`spec/05` § Two-party co-grasp) extends make-before-break to **two parties**: `transport.handoff` transfers a held object from the giver's grasp to a partner effector's grasp. It is the LAST grasp-continuity mode; after it, the GC1-6 group is complete. Two verifications:
- **at-least-one-secures continuity** — at every instant at least one party secures the object at ≥ `min_holding_force` (the receiver is confirmed securing *before* the giver releases — two-party make-before-break);
- **the combined-force ceiling** — during the dual-grasp window the *combined* force from both parties stays ≤ `cograsp_force_budget` (no crushing, no tug-of-war). This is a new dimension: a force **ceiling** (≤), distinct from GC1's per-party securing **floor** (≥).

`transport.handoff` is a brand-new primitive. Like `in_hand.regrasp` it **produces a new grasp** (the receiver's), so it emits `grasp_stability = for_mode(receiver_mode)` (GC2 hold-tests the receiver's new grasp) and `establishes_grasp` returns the receiver mode (STB3 tracks the post-handoff active grasp). It is **not** a free transport (`is_free_transport` excludes it — a handoff transfers ownership in place, it does not relocate freely). Two green commits: the primitive + a worked example, then the GC6 check + driver evidence + adversary.

## 1. T1 — `transport.handoff` primitive

### 1.1 `TransportHandoff` struct (`skill_isa`)
```rust
pub struct TransportHandoff {
    pub receiver: String,                       // EffectorRef — the partner effector frame (v0: a name)
    #[serde(default)] pub receiver_mode: Option<String>,     // GraspMode | auto (v0: a mode name)
    #[serde(default)] pub cograsp_force_budget: Option<Quantity>,  // the combined-force ceiling
    #[serde(default)] pub grasp_handle: Option<GraspHandle>,
}
impl TransportHandoff {
    /// The receiver's grasp mode (pin / platform / else pinch — auto/absent -> pinch).
    pub fn receiver_grasp_mode(&self) -> GraspMode { ... }
}
```
`Primitive::TransportHandoff(TransportHandoff)` (externally-tagged `transport.handoff`). `establishes_grasp` returns `Some(receiver_grasp_mode())`. **Not** added to `is_free_transport`.

### 1.2 `lower_transport_handoff(p, e, ctx: &GraspContext)`
- `grasp_stability = Some(for_mode(receiver_grasp_mode()))` — the receiver's new grasp (GC2 confirms it).
- `force_profile = { min_holding_force: <from ctx.held> (GC1 per-party floor), cograsp_force_budget: <p.cograsp_force_budget, if given> (the combined ceiling), transition: "two_party_handoff", receiver: <name> }`.
- `target_pose = PoseExpr::Ref { ref: "held" }` (symbolic — the object stays near the handoff pose).
- `target_frame = grasp_frame`; `force_budget: None`; capability gate `"transport.handoff"`.
- Read-only `ctx`.

### 1.3 Worked example
`examples/03-screw-fasten/skill-handoff.yaml`: `sense.locate → grasp.pinch → transport.handoff(receiver: tcp_index, cograsp_force_budget: 12 N) → grasp.release`. The part carries `estimated_mass` (the pinch sets `ctx.held`). The 3 example-03 descriptors gain `transport.handoff`.

### 1.4 T1 tests
`rfl-core` translation unit: retarget the handoff example, assert the handoff action (index 2) carries `force_profile.transition == "two_party_handoff"`, `force_profile.cograsp_force_budget == "12 N"`, `force_profile.min_holding_force`, and `grasp_stability.closure == Force` (the receiver's new grasp). Full `cargo test` (the cap→sha256→flip-cert re-bless lesson).

## 2. T2 — two-party handoff check

### 2.1 `envelope_class_for("handoff") = GraspContinuity`
So the handoff gets GC1's per-party securing-floor envelope check for free.

### 2.2 `ReferenceDriver` — attest two-party continuity + combined force
For any action carrying `force_profile.transition == "two_party_handoff"`, push `at_least_one_secures` (continuity: ≥1 party secured at every instant) and, if a `cograsp_force_budget` is declared, `combined_force:<measured>` (v0 models the dual-grasp peak as 80% of the budget, the GC5 posture). Changes only the handoff driver report.

### 2.3 `check_two_party_handoff(goal, report)` (`rfl-conformance`)
Added to `battery::verify_action` (vacuous unless the action carries the marker):
```
let Some("two_party_handoff") = force_profile.transition else { return Pass };
if outcome != Succeeded { return Pass };                       // receiver-failed -> giver-retains is its own path
if !evidence.contains("at_least_one_secures") { Fail("the object was unsecured by both parties at some instant") }
// the combined-force ceiling (when a budget is declared):
if let Some(budget) = force_profile.cograsp_force_budget {
    let measured = evidence.find("combined_force:<m>")?;
    if measured > budget { Fail("combined co-grasp force <m> exceeds cograsp_force_budget <budget>") }
}
Pass
```
Verifies both GC6 halves: **at-least-one-secures** continuity and the **combined-force ceiling**. Every cert action gains a `two_party_handoff` check entry → the 3 example certs re-bless.

### 2.4 The bite — `Fault::CograspOverforce`
Drives the measured `combined_force` over the budget (`budget · 1.5` — a tug-of-war / crush). `check_two_party_handoff` rejects it. Non-circular: a nominal handoff passes (continuity + combined force within budget), a handoff whose two effectors fought the object fails.

## 3. Testing (TDD)
- `rfl-core` unit (T1, above).
- `rfl-conformance` lib unit `check_two_party_handoff`: a handoff goal with `at_least_one_secures` + combined force ≤ budget → Pass; combined force > budget → Fail; missing `at_least_one_secures` → Fail; a budgeted handoff with no `combined_force` evidence → Fail; a non-handoff action → vacuous Pass; a non-Succeeded handoff → vacuous Pass.
- integration on the handoff example: the nominal handoff report passes; the `CograspOverforce` driver's handoff report fails.
- `tests/handoff.rs`: retarget golden (3 hands) + generate-twice + boon schema validity.
- Re-bless: the flip cert (T1, cap sha256) + the 3 example certs (T2 battery entry). Full `cargo test --workspace` (real exit + `grep -c FAILED`), fmt, clippy, validate.py(uv), `gh run` CI-green.

## 4. Out of scope (YAGNI / deferred)
The inter-robot coordination protocol / two-party determinism (`spec/02` § Multi-embodiment-coordination — explicitly out of GC6's scope per spec/05; v0 models bimanual within one embodiment, symbolic); the `EffectorRef` resolution + receiver reachability (`spec/03` § Multi-embodiment addressing — needs concrete poses); the `receiver_grasp_failed` / `pose_drift` driver faults (the giver-retains paths — non-Succeeded, vacuous here); the `GraspRef` supersession composition check (a class-1 `validate` check, shared with regrasp, a separate increment); `cograsp_force_budget: auto` derivation (no second-party budget in v0). **After GC6, the grasp-continuity group GC1-6 is COMPLETE.** Remaining: the standing breadth (grasp modes power/hook/lateral/envelope_*, place/sense families, in_hand rotate/translate/roll/slide) + STB1 (ε-table + contact geometry, blocked).
