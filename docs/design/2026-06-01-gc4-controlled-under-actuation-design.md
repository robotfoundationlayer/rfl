# Design — GC4 controlled under-actuation (in_hand.pivot) (v0)

**Date:** 2026-06-01
**Track:** grasp-continuity — the controlled-under-actuation mode (GC2-6 sub-arc)
**Status:** approved (autonomous run — user directive "推奨順に全てのタスクを自動実行")
**Scope:** `rfl-core` (new `in_hand.pivot` primitive + a worked example + descriptor caps) + `rfl-conformance` (`envelope_class_for("pivot")`, `check_controlled_under_actuation`, a `DofNotResecured` adversary). No spec change (`InHandPivotParams` already typed); no schema change (the marker rides the open `force_profile` floor; the released-DOF attestation rides `verdict.evidence`).

## 0. Why

GC4 (`spec/05` § Controlled under-actuation) covers `in_hand.pivot`: it deliberately releases **exactly one** DOF — the pivot rotation — while the remaining DOF keep the object secured at ≥ `min_holding_force`, then **re-secures** the released DOF at completion (the object returns to fully held). It is the third continuity mode alongside base (GC1) and make-before-break (GC3): continuity is preserved through a deliberately *relaxed* — not swapped — contact constraint. The class invariant splits like GC3's:
- the **all-others-secure floor** is GC1's `securing_floor_violation` (reused — the pivot emits `min_holding_force`, `envelope_class_for("pivot") = GraspContinuity`);
- the **exactly-one-DOF-released + re-secured-at-completion** discipline is GC4's *new* content.

`in_hand.pivot` is a brand-new primitive. Unlike `in_hand.regrasp` (which changes grasp identity), the pivot **preserves the grasp** — so it emits **no `grasp_stability`** (GC2's hold test is vacuous; there is no new grasp to confirm) and is **not** in `establishes_grasp` (the active grasp is unchanged). Two green commits: the primitive + a worked example, then the GC4 check + driver evidence + adversary.

## 1. T1 — `in_hand.pivot` primitive

### 1.1 `InHandPivot` struct (`skill_isa`)
```rust
pub struct InHandPivot {
    pub pivot_axis: Direction,                 // the released rotational DOF
    pub angle: Quantity,                       // swing magnitude (carried symbolic)
    #[serde(default)] pub grasp_handle: Option<GraspHandle>,
    #[serde(default)] pub drive: Option<String>,   // actuated | gravity | external (v0: carried, not lowered)
}
```
`Primitive::InHandPivot(InHandPivot)` (externally-tagged `in_hand.pivot`). **Not** added to `establishes_grasp` (grasp identity preserved).

### 1.2 `lower_in_hand_pivot(p, e, ctx: &GraspContext)`
- `force_profile = { min_holding_force: <from ctx.held> (GC1 floor), released_dof: <pivot_axis as a string> (the single under-constrained DOF), transition: "controlled_under_actuation" (the GC4 marker) }`.
- **No `grasp_stability`** (grasp preserved — GC2 vacuous).
- `target_pose = PoseExpr::AxisRelative { direction: pivot_axis, distance: "0 mm" }` (symbolic, mirroring `in_hand.flip`'s rotation-axis pose).
- `target_frame = grasp_frame`; `force_budget: None`; capability gate `"in_hand.pivot"`.
- `released_dof` serialization: the `pivot_axis` Direction rendered to its string form (e.g. `"+x"`) via the existing `yaml_to_json`; stored as a JSON string in `force_profile.released_dof`.

### 1.3 Worked example
`examples/03-screw-fasten/skill-pivot.yaml`: `sense.locate → grasp.pinch → in_hand.pivot(pivot_axis: +x, angle: 90 deg) → grasp.release` (the part carries `estimated_mass` so the pinch sets `ctx.held` and the pivot emits the floor). The 3 example-03 descriptors gain `in_hand.pivot` in their `skills` lists.

### 1.4 T1 tests
`rfl-core` translation unit: retarget the pivot example, assert the pivot action (index 2) carries `force_profile.transition == "controlled_under_actuation"`, `force_profile.released_dof == "+x"`, `force_profile.min_holding_force`, and **no `grasp_stability`** (grasp preserved). Run full `cargo test` (the descriptor-cap-changes-the-sha256 lesson — the flip cert re-blesses).

## 2. T2 — controlled-under-actuation check

### 2.1 `envelope_class_for("pivot") = GraspContinuity`
So the pivot gets GC1's securing-floor envelope check (all-others-secure) for free.

### 2.2 `ReferenceDriver` — attest the DOF release + re-secure
For any action carrying `force_profile.transition == "controlled_under_actuation"`, read `force_profile.released_dof` and push evidence `released_dof:<axis>` (the driver attests it under-constrained exactly that DOF) + `dof_resecured` (re-secured at completion). Changes only the pivot driver report.

### 2.3 `check_controlled_under_actuation(goal, report)` (`rfl-conformance`)
Added to `battery::verify_action` (vacuous unless the action carries the marker):
```
let Some("controlled_under_actuation") = force_profile.transition else { return Pass };
if outcome != Succeeded { return Pass };                    // re-secure-and-revert failure path is its own
let declared = force_profile.released_dof;                  // exactly which DOF the goal released
let reported = evidence.find("released_dof:<d>");
if reported != Some(declared) { Fail("pivot released DOF <reported>, not the declared <declared>") }
else if !evidence.contains("dof_resecured") { Fail("the released DOF was not re-secured at completion") }
else { Pass }
```
Verifies both GC4 halves: **exactly the named DOF** (reported == declared) and **re-secured at completion** (`dof_resecured`). Every cert action gains a `controlled_under_actuation` check entry → the 3 example certs re-bless.

### 2.4 The bite — `Fault::DofNotResecured`
Drops the `dof_resecured` evidence (the object is left under-actuated, the released DOF never re-secured). `check_controlled_under_actuation` rejects it. Non-circular: a nominal pivot passes (DOF released and re-secured), a pivot that left the object under-constrained fails.

## 3. Testing (TDD)
- `rfl-core` unit (T1, above).
- `rfl-conformance` lib unit `check_controlled_under_actuation`: a pivot goal whose reported released_dof matches + `dof_resecured` → Pass; a mismatched released_dof → Fail; missing `dof_resecured` → Fail; a non-pivot action → vacuous Pass; a non-Succeeded pivot → vacuous Pass.
- integration on the pivot example: the nominal pivot report passes; the `DofNotResecured` driver's pivot report fails.
- `tests/pivot.rs`: retarget golden (3 hands) + generate-twice + boon schema validity.
- Re-bless: the flip cert (T1, the cap sha256 change) + the 3 example certs (T2 battery entry). Full `cargo test --workspace` (real exit + `grep -c FAILED`), fmt, clippy, validate.py(uv), `gh run` CI-green.

## 4. Out of scope (YAGNI / deferred)
The `pivot_inadmissible` admissibility check (`spec/01` § 3.5 — releasing `pivot_axis` would drop the object: a class-1 `validate` check against the grasp's `secured_dof`, a separate increment — the DOF-admissibility rule covers rotate/translate/roll/slide/pivot together); the `overshoot` / `wrong_drive_direction` driver faults (the C2 gravity-drive path — modeled when a failure driver lands); `drive` mode lowering (actuated/gravity/external — carried, not acted on in v0); the swing-rate / orientation-tolerance numerics (need concrete poses). GC6 (`transport.handoff`) is the last GC, a fresh new-primitive increment.
