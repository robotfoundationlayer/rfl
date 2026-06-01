# Design — force.cut freeing disclosure (TM21c generalized to a 2nd consumer)

**Date:** 2026-06-01
**Increment:** 25
**Status:** approved
**Scope:** rfl-conformance only (extend `check_freed_part_disposition` + the `ReferenceDriver`
population + cut tests). No schema, no struct, no golden, no spec, no rfl-core change.

## 0. Why this increment

Increment 24 gave `force.unscrew` a freed-part disposition disclosure (TM21c) keyed on its authored
`on_disengagement`. `force.cut` is the spec's *other* freeing operation (TM20c: "the freeing
`ForceEvent` is breakaway for `force.unscrew`, cut-completion for `force.cut`"), and the most
safety-critical (an irreversible cut frees a piece). Generalizing the disclosure contract to a
2nd, structurally different consumer — one with **no authored disposition intent** — proves the
`safety_flags` freed-part abstraction is not unscrew-specific (the same generalization-proof role
`force.snap_engage` played for press_button's detent). No new dimension; a contained gap-closure.

## 1. The contract (v0)

Extend `check_freed_part_disposition` so a **freeing operation** is either:

- an action whose lowered `force_profile.on_disengagement` is present (`force.unscrew`) — the
  existing **expected-match** check (`retain` → `retained`, `drop_safe` → `safe_zone_release`); or
- an action whose lowered `force_profile.irreversible == true` (`force.cut`) — a **presence-only**
  check: the disclosure must be present and a valid disposition (`retained` or
  `safe_zone_release`), but no intent is authored so either is acceptable.

Add a **`outcome == Succeeded` guard**: a freeing completes only on success. An interrupted cut or
unscrew froze nothing, so the disposition check is vacuous on a non-`Succeeded` report. This is a
small refinement of the increment-24 check; every existing freed-part test is `Succeeded`, so it
changes no existing verdict, and it correctly keeps an interrupted cut (`CutResponse::PartialReported`
/ `BinaryHalt`, which are `Failed`) out of the disposition check.

`force.cut` lowering already emits `force_profile.irreversible: true` (increment 19), so the marker
needs no new plumbing.

## 2. Schema / struct / golden delta: NONE

`safety_flags` / `SafetyFlags` / `FreedPartDisposition` were added in increment 24;
`force_profile.irreversible` in increment 19. This increment only reads them. No retarget golden
changes (the cut retarget output is unchanged); no driver-report golden changes (`driver_protocol`
goldens are cable-only — no `irreversible`). `validate.py` C1–C7 untouched.

## 3. Implementation points

- **`crates/rfl-conformance/src/lib.rs` `ReferenceDriver::execute`** — extend the `safety_flags`
  computation: when `force_profile.on_disengagement` is present, keep the existing
  retain/drop_safe mapping; else when `force_profile.irreversible == true`, disclose
  `{ disposition: "retained" }` (the v0 conservative default — the cut-off piece is kept).
- **`crates/rfl-conformance/src/lib.rs` `check_freed_part_disposition`** — restructure:
  1. `if report.status.outcome != Succeeded { return Pass }` (no freeing on a failed op).
  2. Read `on_disengagement` and `irreversible` from the goal's `force_profile`.
  3. The expected disposition is `Some("safe_zone_release")` for `drop_safe`, `Some("retained")`
     for any other `on_disengagement`, `None` (presence-only) for `irreversible`, and "not a
     freeing op → `Pass`" otherwise.
  4. Require the disclosure present (`None` → `Fail` "uncontrolled drop"); for an expected, require
     a match (`Fail` "mismatch"); for presence-only, require a valid disposition value
     (`retained` / `safe_zone_release`, else `Fail` "unknown disposition").

## 4. Adversarial drivers (reuse `FreeingDriver`)

- **`Discloses`** (cut nominal, the ReferenceDriver discloses `retained`) → `Pass`.
- **`DropsUncontrolled`** (cut succeeds, `safety_flags` stripped) → the uncontrolled-drop `Fail`.
  This proves `safety_flags` is load-bearing for cut too.
- **`FalseDisposition`** (`retained` → `safe_zone_release`) → **`Pass` for cut** (presence-only: both
  are valid dispositions; no authored intent to violate). Asserting this documents the
  presence-only semantics and contrasts with unscrew's expected-match (where the same mutation
  fails). This is the meaningful difference between the two consumers.

## 5. Tests

- `envelope_conformance.rs` on `examples/03-screw-fasten/skill-cut.yaml` (the cut, `pairs[0]`):
  `nominal_cut_discloses_disposition` (Discloses → Pass); `cut_uncontrolled_drop_fails`
  (DropsUncontrolled → Fail); `cut_accepts_either_disposition` (FalseDisposition → Pass,
  presence-only).
- Extend the `check_freed_part_disposition` unit test with presence-only cases: irreversible +
  disclosure present → Pass; irreversible + absent → Fail; irreversible + `Failed` outcome →
  vacuous Pass.

**Expected deltas:** rfl-core unchanged; conformance `lib` unchanged or +0 (the unit test is
extended in place); `envelope_conformance` +3. All 12 golden binaries byte-identical. `validate.py`
C1–C7 PASS.

## 6. Deferred (each behind a named prerequisite)

- **TM20c full freeing-detection** — the explicit constrained → free transition event (the freeing
  `ForceEvent`), beyond the disposition disclosure.
- **`grasp.release` freeing disclosure** — release frees the held object (a controlled release).
- **`force.cut` authored `on_separation`** — give cut an authored disposition intent (so it gets an
  expected-match like unscrew, not just presence-only).
- **`zone` geometry verification** — the safe-release region is a placeholder.
- **`safety_flags.momentary_release` reconciliation** — still routed through `verdict.evidence`
  (increment 21); the latent drift remains.
