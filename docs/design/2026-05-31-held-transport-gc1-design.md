# Design: held-transport GC1 propagation (min_holding_force on the carry)

Status: approved design, pre-implementation (2026-05-31)

This is the tenth reference-implementation increment. It closes the vacuous
transport grasp-continuity (GC1) hole: a held `transport.move_to_pose` now
declares its static securing floor (`min_holding_force`), so the conformance
grasp-continuity checker verifies the maintained grip stays `≥` that floor across
the carry, and an adversarial under-secure-mid-carry driver is rejected. The
specification under `spec/` is authoritative; this document describes how the
reference implementation realizes it.

## 1. Why this increment

`spec/05` § Grasp-continuity modes, Base continuity: the simplest mode is "a single
established grasp holds across an operation that does not change the contact topology
(`grasp.adjust`, **`transport.move_to_pose`**, `place.put_down`). The force trace
shows holding force never dropping below `min_holding_force` from the start of the
operation to its end." `spec/05` GC1: the class "verifies from the force trace that a
securing contact set maintains the object at `≥ min_holding_force` at every sampled
instant."

The reference checker (`check_envelope`, `GraspContinuity`) reads the floor from the
action's own `force_profile.min_holding_force`. Increment 3 (GF1c) emits that field
on `grasp.pinch` only; a held `transport` carries no floor, so the checker hits its
`None` early-return and **passes the transport vacuously** — it passes not because the
grip is safe but because there is nothing to check. This increment fills that gap so
the transport GC1 check has a real floor to enforce, and proves non-vacuity with an
adversarial driver.

`spec/02` § `min_holding_force` frames the encoding: the dynamic-stability `a_max`
"clamps `max_acceleration` in the `Envelope.motion_bounds` of every `transport`
primitive (**the dynamic counterpart of `min_holding_force`**)". GF2c already put
`a_max` on the held transport's `motion_bounds`; this increment puts the static
counterpart, `min_holding_force`, on the same transport's `force_profile` — the two
held-transport bounds side by side.

## 2. Scope

In scope (increment 10):

- **rfl-core** `lower_transport_move_to_pose`: when `ctx.held` is set, emit
  `force_profile.min_holding_force` (the static floor), mirroring `lower_grasp_pinch`.
- **rfl-conformance** `ReferenceDriver`: `securing_force` falls back to the
  `force_profile.min_holding_force` floor when no force budget is commanded, so the
  held transport reports a maintained grip. The `check_envelope` `GraspContinuity` arm
  and the `FaultyDriver` are **unchanged** — both already operate on
  `force_profile.min_holding_force` / `securing_force`.
- Tests: a translation test (the cable transport emits the floor) and a conformance
  test (nominal transport GC1 passes with `securing_force` present; `UnderSecure` makes
  the transport GC1 fail). Regenerated cable retarget, screw-fasten, and cable
  driver-protocol goldens.

Deferred (documented, each a future bite):

- **Full held-interval GC1** — extending the floor check across the *whole* held
  interval (`align` / `insert_fit` telemetry while the object is held), not just the
  transport. Base continuity for the transport is the bounded case here.
- **GC3 make-before-break / GC4 controlled under-actuation / GC5 bounded exception /
  GC6 two-party** continuity sub-modes — need `in_hand.*` / `transport.handoff`
  primitives not in the worked examples.
- **GC2 hold-test closure** (the calibrated sub-budget perturbation → retention test).
- **Interval-invariant ENV2 / ENV3** (disturbance injection) — needs `reach.hover` /
  `transport.carry`.

No `spec/` change. No `schemas/` change — the transport `force_profile.min_holding_force`
rides the same open `Envelope` floor the pinch's already does (the boon execute-schema
and driver-interface validation still pass). No new `Primitive` / enum variant.

## 3. The rfl-core change (`lower_transport_move_to_pose`)

The function already takes `ctx: &GraspContext` and, when `ctx.held` is set, computes
the GF2c dynamic `a_max` into `env.motion_bounds`. Add the static floor in the same
`if let Some(held) = &ctx.held` block, mirroring `lower_grasp_pinch` exactly:

```rust
let mhf = grasp_force::min_holding_force(held.weight_n, held.mode);
env.force_profile =
    Some(serde_json::json!({ "min_holding_force": Quantity::from_si(mhf, "N").0 }));
```

`held.weight_n` / `held.mode` are already in scope. For the cable connector
(`estimated_mass 1.45 N`, pinch) `min_holding_force = 1.45 · 2.0 = 2.9 N`; for the
screw driver (`estimated_mass 1.0 N`, pinch) `= 2.0 N`. `ctx.held` is set at
`grasp.pinch` (GF1c plumbing) and cleared at `grasp.release`, so the floor lands on
the held transport and **not** on `release` — `release` is the controlled let-go and
must not be floor-checked.

## 4. The conformance change (`ReferenceDriver`)

The driver currently sets `securing_force = ca.force_budget.clone()` — `None` for a
transport (no commanded budget). Fall back to the declared floor so a held transport
reports its maintained grip:

```rust
let securing_force = ca.force_budget.clone().or_else(|| {
    ca.safety_envelope
        .force_profile
        .as_ref()
        .and_then(|fp| fp.get("min_holding_force"))
        .and_then(serde_json::Value::as_str)
        .map(|s| rfl_core::quantity::Quantity(s.to_string()))
});
```

Per-action, no statefulness. `grasp.pinch` (force budget present) and `force.insert_fit`
(budget present) are unaffected; the held `transport` now reports
`securing_force = "2.9 N"` (the floor, i.e. the grip maintained at the declared
minimum); `release` (no budget, no floor) still reports `None`.

The `check_envelope` `GraspContinuity` arm needs **no change**: it already reads
`goal.canonical_action.safety_envelope.force_profile.min_holding_force` and checks
every telemetry `securing_force ≥ floor`. With the transport now carrying the floor and
the driver reporting `securing_force`, the transport is verified automatically
(`2.9 ≥ 2.9` passes). `FaultyDriver::UnderSecure` needs **no change**: it lowers any
present `securing_force` to `0.1 N`, so it now drives the transport GC1 to fail
(`0.1 < 2.9`).

## 5. Tests

- **rfl-core translation** (`scan`-test neighbourhood): retarget the cable skill onto
  allegro; assert `out.actions[2]` (the `transport`) carries
  `safety_envelope.force_profile.min_holding_force == "2.9 N"`. (Index 2: `locate`(0),
  `pinch`(1), `transport`(2).)
- **conformance `envelope_conformance`** driving `examples/01-cable-insertion`:
  - *nominal*: the transport's GC1 check passes **and** its telemetry `securing_force`
    is `Some` (non-vacuous — the floor is actually exercised).
  - *adversarial*: `FaultyDriver(UnderSecure)` makes the transport (index 2) GC1
    **Fail** (`securing_force 0.1 < min_holding_force 2.9`) — the bite that proves the
    gap closed, mirroring the existing pinch `UnderSecure` test.

## 6. Conformance and goldens

`validate.py` (C1–C7) is unchanged (no schema edit). Golden churn is mechanical and
each regenerated snapshot is eyeballed:

- **Commit B (rfl-core)**: the cable retarget goldens (`retarget_determinism`, 3 hands)
  and the `screw_fasten` goldens (3 hands) gain `force_profile.min_holding_force` on
  their `transport` execute line. The `driver_protocol` golden is **unchanged at this
  commit** (the driver is not yet changed — explicitly re-run to confirm), since
  `force_profile` is execute-message input, not driver-report output.
- **Commit C (conformance)**: the cable `driver_protocol` goldens (3 hands) gain
  `securing_force` on the `transport` telemetry line.

`surface_scan` / `surface_scan_spiral` are unaffected (no transport).

## 7. Sections to transcribe during implementation

- `spec/05-conformance.md` § Grasp-continuity modes, Base continuity + GC1 — already
  read for this design.
- `spec/02-translation-layer.md` § `min_holding_force` (the static-floor / dynamic-
  `a_max` counterpart framing) — already read.
- `lower_grasp_pinch`'s `force_profile.min_holding_force` emit + `grasp_force::min_holding_force`
  + the existing `lower_transport_move_to_pose` `ctx.held` block (`crates/rfl-core/src/translation.rs`).
- The `ReferenceDriver::execute` `securing_force` line, the `check_envelope`
  `GraspContinuity` arm, the `Fault::UnderSecure` injection, and the existing
  `envelope_conformance.rs` test shape (`crates/rfl-conformance/src/lib.rs`,
  `tests/envelope_conformance.rs`).
