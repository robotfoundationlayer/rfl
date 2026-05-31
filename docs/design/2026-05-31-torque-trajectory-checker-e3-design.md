# Design: torque-trajectory checker — Class-3 ENV4 torque case (E3)

Status: approved design, pre-implementation (2026-05-31)

This is the eighth reference-implementation increment. It extends the Class-3
force-trajectory envelope checker (`spec/05` ENV4) to the **torque** case, so a
`force.screw` driver report is verified against its torque budget the same way a
`force.insert_fit` report is verified against its force budget. With it, the
screw-fasten example gains the non-circular Class-3 verification the cable example
already has. The specification under `spec/` is authoritative; this document
describes how the reference implementation realizes it.

## 1. Why this increment

`spec/05` ENV4: the force/torque-trajectory class "bounds the force (or torque)
profile interval-sampled against per-axis budgets … the class generalizes to torque
… under the identical interval discipline." The C2 checker only implemented the
force case (it reads `force_budget` and checks `wrench.force`). `force.screw` carries
its budget as a *torque* in `force_profile.torque` (not `force_budget`), so the
current ForceTrajectory checker early-returns `Pass` for a screw action — it passes
vacuously, with nothing verified. E3 closes that: the checker also reads the torque
budget and checks `wrench.torque`, the `ReferenceDriver` echoes a within-budget
torque, and an adversarial `OverTorque` driver proves the check rejects an
over-budget torque (the non-circular proof, mirroring C2's `OverForce`).

## 2. Scope

In scope (E3), all in `rfl-conformance`:

- Extend `check_envelope`'s `ForceTrajectory` arm from force-only to force **and**
  torque.
- Extend the `ReferenceDriver` to echo `force_profile.torque` into `wrench.torque`.
- Add a `Fault::OverTorque` and two `envelope_conformance` tests (nominal pass +
  adversarial bite) driving the screw example.

Deferred (documented):

- Runtime **decoupling detection** (advance-without-turn / turn-without-advance as a
  fault) — a driver/Class-4 concern; E3 verifies the torque envelope, not the
  coupling.
- The **interval-invariant** class (ENV2/ENV3) — still no exercising primitive
  (hover/carry).
- `force.unscrew` and the other category-6 primitives.

No `rfl-core`, `spec/`, or `schemas/` change.

## 3. The checker extension (`check_envelope` ForceTrajectory)

Restructure the `ForceTrajectory` arm so the force and torque checks are independent
(each applies when its budget is present); fail on the first violation; pass if
neither budget is claimed or both samples are within:

- **Force** (when `goal.canonical_action.force_budget` is present): every
  `|wrench.force| ≤ force_budget` (the existing check).
- **Torque** (when `goal.canonical_action.safety_envelope.force_profile.torque` is
  present — the `"X N·m"` string the screw lowering emits): every
  `|wrench.torque| ≤ torque_budget`. Read via the same `force_profile` →
  `get("torque")` → `as_str` → `Quantity::parse` path the GraspContinuity checker
  uses for `min_holding_force`.

A `force.insert_fit` action (force budget, no torque) exercises only the force
check; a `force.screw` action (torque budget, no force budget) exercises only the
torque check — but both paths live in the one arm.

## 4. The `ReferenceDriver` torque echo

The driver currently echoes `force_budget` into `wrench.force` (and `securing_force`).
Extend it so a `force_profile.torque` is echoed into `wrench.torque`:

- Compute `force_mag` from `force_budget` (if any) and `torque_mag` from
  `force_profile.torque` (if any). Emit `wrench = Some(Wrench { force: [0,0,force_mag],
  torque: [0,0,torque_mag] })` when *either* is present; `securing_force` stays the
  `force_budget` echo as before.

For the screw action (`force_budget` `None`, `force_profile.torque` `"0.2 N·m"`), the
nominal report carries `wrench.torque = [0,0,0.2]` — exactly at the budget, so the
torque check passes (`0.2 ≤ 0.2`). **The cable `driver_protocol` goldens do not
change**: cable has no `force.screw`, so `force_profile.torque` is never present and
the torque component stays `[0,0,0]` exactly as today (re-run to confirm).

## 5. The adversarial fault + tests

- **`Fault::OverTorque`** — set every telemetry `wrench.torque = [0,0,999]` (over any
  torque budget; schema-valid but envelope-violating, like `OverForce`).
- **`envelope_conformance.rs`** — two tests driving `examples/03-screw-fasten`
  (where `force.screw` is action index 5):
  - *nominal*: `check_envelope(ForceTrajectory, screw_goal, screw_report)` **passes**
    (echoed torque `0.2 N·m ≤ 0.2 N·m` budget).
  - *adversarial*: `FaultyDriver(OverTorque)` makes the screw action's ForceTrajectory
    check **fail** (`999 > 0.2`).

No new goldens — the checks are pass/fail assertions.

## 6. Conformance

`validate.py` (C1–C7), the rfl-core suite, and all existing conformance suites stay
green; the cable `driver_protocol` goldens are unchanged (verified by re-running),
and `screw_fasten` (Class 2 retarget golden) is untouched (it does not use the
`ReferenceDriver`). The new `envelope_conformance` screw tests are pass/fail. No
schema change — the echoed `wrench.torque` rides the open `WrenchFloor`.

## 7. Sections to transcribe during implementation

- `spec/05-conformance.md` § The force/torque-trajectory class (ENV4, the torque
  generalization) — already read for this design.
- The existing C2 `check_envelope` (the `ForceTrajectory` arm + the `quantity_mag` /
  `force_profile` read pattern), the `ReferenceDriver` wrench/`securing_force` match,
  the `Fault` enum + `OverForce` injection, and the `envelope_conformance.rs` test
  shape — all in the current `rfl-conformance` tree.
