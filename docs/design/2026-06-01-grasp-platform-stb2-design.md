# Design — `grasp.platform` + STB2 + STB3 support row (v0)

**Date:** 2026-06-01
**Track:** grasp stability — the support-closure mode and its two obligations
**Status:** approved (autonomous run — user directive "推奨順に全てのタスクを自動実行")
**Scope:** `rfl-core` (`GraspMode::Platform`; `for_mode(Platform)`; `GraspPlatform` + `Primitive::GraspPlatform` + dispatch + `lower_grasp_platform`; STB3 support row in `Skill::validate`). `rfl-conformance` (`check_support_safe_state` in the per-action battery). No schema change (`GraspPlatformParams` typed; safe-state rides the open Envelope floor).

## 0. Why

`grasp.platform` (`spec/01` § 2.6) is the support-closure mode: an object rests in gravity-and-friction balance over a support polygon, neither gripped nor enclosed. It is the grasp mode the remaining two stability obligations read:
- **STB2** (`spec/05`): a `support` grasp's safe/abort response is a controlled lowering to the nearest surface, **never an open-release** — opening drops a balanced object (`spec/01` § 2.6 safety envelope, verbatim).
- **STB3 row 4**: `support` closure also **forbids free transport** (a balanced object is not freely transportable). This extends the 4b STB3 check (which keyed on `flags.surface_bound`) to also key on `closure == Support`.

Two green commits: **(T1)** `grasp.platform` primitive (closure=support, emits the controlled-lowering safe state), **(T2)** the STB2 check + the STB3 support-row extension.

## 1. T1 — `grasp.platform` primitive

### 1.1 `GraspMode::Platform`
Add the variant. `payload_key = "payload_support"`. `k_holding` / `k_reaction` are force-closure grip factors that do not apply to a borne support; v0 gives them `1.0` documented N/A (support emits no `min_holding_force` — the object is borne, not gripped), present only for the exhaustive match.

### 1.2 `for_mode(Platform)` (`spec/01` grasp-mode table: `platform | support | balance_held | —`)
- `closure: Support`
- `secured_dof: {"support_normal": BalanceHeld}`
- `stable_directions: Set(["support_normal"])` (§ 2.6 postcondition `stable_directions = {support_normal}`)
- `flags: default` (none — the transport-forbidding comes from `closure == Support`, not a flag; § table row has `—`)
- `residual_mobility: None`, `min_holding_force: None`

First `Support` closure and first `BalanceHeld` securing on the wire.

### 1.3 `GraspPlatform` struct
```rust
pub struct GraspPlatform {
    pub target: Ref,
    pub load_budget: Quantity,            // § 2.6: max supported weight, clamped to payload_support
    #[serde(default = "tactile_auto")]
    pub tactile_target: TactileTargetArg,
}
```
`Primitive::GraspPlatform(GraspPlatform)` (externally-tagged `grasp.platform`).

### 1.4 `lower_grasp_platform`
- `load_budget` = `clamp_force(&p.load_budget, "payload_support", e)` (the § 2.6 clamp).
- `tactile_target` = manifold/proxy resolution (auto = distributed load + CoM-in-polygon; proxy = measured load only).
- `target_pose` = symbolic `PoseExpr::Ref { ref: target }` (`support_pose: auto`, v0).
- `safety_envelope.force_profile` = `{ "load_budget": <clamped>, "safe_state": "controlled_lowering" }` — the § 2.6 support-specific safe state, the STB2 anchor.
- `grasp_stability: Some(for_mode(Platform))`.
- Does not set `ctx.held` (a supported object is not a freely-carried grip load; STB3 forbids transporting it).
- Dispatch: `check_capability` arm `=> "grasp.platform"`; `lower` arm `=> (lower_grasp_platform(p, e), "platform")`.

### 1.5 T1 tests
`for_mode(Platform)` (support / balance_held / no flags); lower a `GraspPlatform` → `grasp_stability.closure == Support`, wire contains `"closure":"support"` and `"safe_state":"controlled_lowering"`.

## 2. T2 — STB2 + STB3 support row

### 2.1 STB2 — `check_support_safe_state(goal)` (`rfl-conformance`)
Goal-only static check, added to `verify_action`'s battery (vacuous-pass for non-support actions):
```
let Some(st) = &goal.canonical_action.grasp_stability else { return Pass };   // non-grasp
if st.closure != Support { return Pass };                                     // force/form grasp
match force_profile.safe_state {
    Some("controlled_lowering") => Pass,
    other => Fail("a support grasp must declare a controlled-lowering safe state, not <other>"),
}
```
The check **bites** on a hand-built adversarial goal: a `Support`-closure action whose `safe_state` is `"open_withdraw"` → Fail (opening drops a balanced object). The real `grasp.platform` output passes.

### 2.2 STB3 support row (`Skill::validate`)
Extend the 4b transport-inadmissibility test from `flags.surface_bound` to also reject `closure == Support`:
```rust
let m = StabilityMetadata::for_mode(mode);
if m.flags.surface_bound || m.closure == Closure::Support { return Err(transport_inadmissible) }
```
A `grasp.platform → transport` skill is rejected; `grasp.platform → grasp.release → transport` is legal.

### 2.3 T2 tests
- `rfl-conformance`: `check_support_safe_state` — real platform goal passes; adversarial open-withdraw support goal fails; non-support (pinch) goal vacuous-passes. Battery integration: a platform action's verdict carries the `support_safe_state` check.
- `rfl-core` validate: `grasp.platform → transport` → `Err(transport_inadmissible)`; `grasp.platform → grasp.release → transport` → `Ok`.

## 3. Out of scope (YAGNI)
The support-polygon / CoM geometry (`support_pose`/`support_normal`/`com_unknown`/`unstable_placement` — needs concrete poses); the driver-level safe-state EXECUTION (Class 3/4 — STB2 here is the static declaration check); `load_not_confirmed`/`overload` driver faults; the tip-over transport margin (a `transport`-of-support accel clamp — deferred with support transport, which STB3 forbids anyway in v0); a dedicated platform example dir (inline/hand-built test goals suffice). STB1 stays blocked (ε-table + contact geometry).
