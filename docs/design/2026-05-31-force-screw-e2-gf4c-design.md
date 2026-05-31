# Design: GF4c tool-mediated force — reaction-torque limit + coupling (E2)

Status: approved design, pre-implementation (2026-05-31)

This is the seventh reference-implementation increment, and the second half of the
screw/fasten work (E1 added `force.screw` structurally). It implements GF4c
(`spec/02` § Tool-mediated force and coupled motion), completing the grasp-force arc
GF1c–GF4c: the driving torque loads the *tool's* grasp, so `force.screw`'s torque
budget is clamped to the tool-grasp rotational holding capacity, and the
`thread_pitch` rotation↔advance coupling is expanded into the canonical action. The
specification under `spec/` is authoritative; this document describes how the
reference implementation realizes it.

## 1. Why this increment

Increment 3 derived GF1c (`min_holding_force`), GF2c (dynamic `a_max`), and GF3c
(linear reaction-load); E1 added `force.screw` but left its tool-mediated reaction
and coupling symbolic. GF4c is the last grasp-force obligation: `force.screw`
transmits torque to the fastener *through a held tool* (the driver), and
(`spec/02`:150) "the reaction-load limit applies to the **tool's grasp**, not the
target's — the grip on the driver must resist the driving torque, or the driver
slips in-hand." The reaction here is a **torque** about the tool axis
(`spec/02`:143), resisted by the grasp's rotational holding capacity. This increment
derives that limit (the torque counterpart of GF3c) and clamps the torque budget to
it, plus expands the `thread_pitch` coupling (`spec/02`:151) into the canonical
action. With it, GF1c–GF4c are all numeric and the grasp-force arc is complete.

## 2. Scope

In scope (E2 — GF4c):

- A `reaction_torque_limit` derivation in `grasp_force.rs` (the rotational
  counterpart of GF3c's `reaction_limit`).
- `lower_force_screw` reads the held tool grasp (`GraspContext`) and clamps
  `torque_budget` to that limit when `tool_mediated`; the `thread_pitch` coupling
  becomes a structured `force_profile.coupling`.
- Regenerated `screw_fasten` goldens (the per-hand torque clamp + the coupling field).

Deferred (documented — not part of GF4c):

- The **torque-trajectory checker** (`|wrench.torque| ≤ torque_budget`, the ENV4
  torque case) + the `ReferenceDriver` torque echo + an `OverTorque` adversarial +
  a screw envelope-conformance test. That is Class-3 *verification* of a driver
  report, not the grasp-force derivation; it is a clean follow-on.
- The runtime **decoupling detection** (advance-without-turn / turn-without-advance
  as a fault) — a driver/telemetry concern (Class-3/4); E2 emits the coupling
  *directive*, not its runtime check.
- `force.unscrew` and the other category-6 primitives.

## 3. The reaction-torque limit (`rfl-core::grasp_force`)

The grasp's **rotational holding capacity** is the torque it resists before the
held tool twists in-grasp. The capacity model stays schematic and provider-neutral
(Principle 4, as in GF1c–GF3c): capacity ≈ grip force × an effective lever, reduced
by the same per-mode friction factor. A documented effective grip-radius constant
collapses the lever:

```
pub const R_GRIP: f64 = 0.02; // m — schematic effective grip radius (v0 reference choice)

/// GF4c — the reaction-torque limit: a tool-mediated force primitive's reaction is a
/// torque about the tool axis; the held tool's grasp must resist it with rotational
/// holding capacity (≈ grip force × lever ÷ the reaction factor), or the tool spins
/// in-grasp. Returns the smaller of the requested torque budget and that capacity (N·m).
pub fn reaction_torque_limit(torque_budget_nm: f64, grip_force_max_n: f64, mode: GraspMode) -> f64 {
    torque_budget_nm.min(grip_force_max_n * R_GRIP / mode.k_reaction())
}
```

`R_GRIP` and the reuse of `k_reaction` are explicit v0 reference choices, pinned by
golden, non-normative — RFL fixes the *dependency* (capacity is a function of grip,
geometry, friction), not a vendor's law.

## 4. The lowering change (`rfl-core::translation`)

`lower_force_screw` gains a `ctx: &GraspContext` parameter (the dispatcher's
`force.screw` arm passes it, like `force.insert_fit`). The held tool grasp is what
`grasp.pinch` recorded in `ctx.held` (the driver pinch):

- **Reaction-torque clamp**: when the screw is `tool_mediated` (param truthy) **and**
  a tool grasp is held (`ctx.held.is_some()`), clamp the torque budget:
  `torque = reaction_torque_limit(parsed torque_budget, e.scalar_limit("grip_force_max"),
  held.mode)`, emitted in `force_profile.torque` (round6 + unit string via
  `Quantity::from_si`, byte-deterministic). When no tool is held (a bare
  `force.screw` with no preceding grasp), the budget passes through unchanged —
  tool-mediated reaction presupposes a held tool.
- **Coupling expansion**: replace the symbolic `force_profile.thread_pitch` marker
  with a structured `force_profile.coupling = { advance_per_turn: <thread_pitch> }`
  (the linked DOF, `spec/02`:151). `tool_mediated` stays a recorded flag.

Everything else (`ScrewStop` monitor, compliance, `target_pose`) is unchanged.

## 5. Per-hand demonstration

In `examples/03-screw-fasten`, `grasp.pinch` holds the driver before `force.screw`,
so `ctx.held` is set and the clamp fires. The same 2 N·m commanded budget yields a
different limit per hand (`grip_force_max · R_GRIP / k_reaction`, `R_GRIP = 0.02`,
`k_reaction = 2.0`):

| hand | grip_force_max | reaction-torque limit | emitted torque |
|---|---|---|---|
| allegro | 20 N | 20·0.02/2 = 0.2 | **0.2 N·m** |
| leap | 15 N | 15·0.02/2 = 0.15 | **0.15 N·m** |
| pneumatic | 12 N | 12·0.02/2 = 0.12 | **0.12 N·m** |

The physical reading: a precision pinch on a driver shank resists only a fraction of
the commanded driving torque before it would spin in-grasp — a real limit (a power
grasp would resist more). Same skill, three hands, three derived torque ceilings —
the Principle-1 point again, now for tool-mediated torque.

## 6. Conformance

The 3 `screw_fasten` goldens regenerate: the `0006-screw` action's
`force_profile.torque` changes `2 N·m` → `0.2`/`0.15`/`0.12 N·m` per hand, and
`thread_pitch` becomes `coupling: {advance_per_turn: "0.8 mm"}`. The existing
`screw_fasten` determinism + boon tests stay green (the clamped torque + coupling
object ride the open `Envelope`/`canonical_action` floor — no schema change). The
bare-`force.screw` unit test in `translation.rs` (no preceding grasp) is unaffected:
with no held tool its torque stays `2 N·m`. A new `translation.rs` test retargets
the full `examples/03` skill onto allegro and asserts the clamped `0.2 N·m`.
`grasp_force.rs` gains unit tests for `reaction_torque_limit`. `validate.py` (C1–C7),
the 51 rfl-core tests (minus the unaffected ones), and the other conformance suites
stay green. No `spec/` or `schemas/` change.

## 7. Sections to transcribe during implementation

- `spec/02-translation-layer.md` § Reaction-load limit (the rotational-capacity
  generalization, `:143`) and § Tool-mediated force and coupled motion (`:146-151`)
  + GF4c (`:158`) — already read for this design.
- The existing `grasp_force::reaction_limit` (the GF3c template the torque limit
  mirrors) and `lower_force_screw` / `lower_force_insert_fit` (the lowering + ctx
  pattern) in the current tree.
