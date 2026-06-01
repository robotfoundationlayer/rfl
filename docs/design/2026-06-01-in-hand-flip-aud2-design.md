# Design: in_hand.flip (AUD2) — cross-action momentary_release propagation

Status: approved design, pre-implementation (2026-06-01)

This is the twenty-first reference-implementation increment, and the heaviest of the session:
the first `in_hand`-category primitive, the only continuity-suspending primitive, and the
**first cross-action conformance obligation**. It realizes AUD2 (`spec/05`): `in_hand.flip`
sets `momentary_release = true` and the flag is **persisted and propagated downstream** so the
L4 / L8 loops can trace a continuity break several primitives later. The spec is authoritative;
this realizes it.

## 1. The gap

Every conformance check so far is a pure function of a single `(goal, report)` pair —
order-independent. AUD2 breaks that: a continuity break is meaningful only *relative to what
comes after it*. `spec/05` § momentary_release propagation: `in_hand.flip` "is the only
operation that breaks grasp continuity … it sets `momentary_release = true` in its result, and
the flag is persisted and propagated downstream … A continuity break that is
bounded-and-recovered is still a fact the audit trail must carry; suppressing it is a
transparency violation." AUD2 is the last major unexercised dimension.

## 2. No schema change

`in_hand.flip` is in the skill-isa PrimitiveId enum; the embodiment capability enum derives
`in_hand.flip` from PrimitiveId − reach.* + category gates (C1); `InHandFlipParams` is typed
(`flip_axis` + `angle` + `max_release_time` + `safe_drop_zone` required). `momentary_release`
rides `verdict.evidence` (the AUD channel, like `held_confirmed` / `partial_cut`); the
propagation is driver state. **No skill-isa / driver-interface schema change** — the descriptor
gains an `in_hand.flip` capability *token* (data).

## 3. The contract (v0)

- **`envelope_class_for("flip") => GraspContinuity`**, but `lower_in_hand_flip` emits **no**
  `min_holding_force` floor — continuity is intentionally suspended in the bounded unsecured
  window — so `securing_floor_violation` is vacuous for the flip. Correct: the floor is the
  thing being suspended; there is nothing to enforce during the exception.
- **AUD2 (the new dimension).** The flip's status declares `momentary_release` (a
  `verdict.evidence` entry), and **every downstream action's** status carries the propagated
  flag. A new **sequence-level** `check_momentary_release(&[(goal, report)])` — the first check
  over the whole `drive()` sequence rather than a single pair — verifies: the flip declares it,
  and all actions after the flip carry it. Vacuous if the sequence contains no flip.

## 4. Surface

**rfl-core (one commit — exhaustive-match):**
- `struct InHandFlip` (`src/skill_isa.rs`): `flip_axis` (`Direction`), `angle` (`Quantity`),
  `max_release_time` (`Quantity`), `safe_drop_zone` (`serde_yaml::Value`, carried symbolic),
  `target_mode` (`Option<serde_yaml::Value>`), `catch_envelope` (`Option<serde_yaml::Value>`).
- `Primitive::InHandFlip(InHandFlip)` + `check_capability` arm `"in_hand.flip"` + the lower
  dispatch arm `(lower_in_hand_flip(p, e), "flip")` — all this commit. The first `in_hand`
  primitive.
- `lower_in_hand_flip` (`src/translation.rs`): grasp-continuity, **no floor emitted**;
  `target_pose: PoseExpr::AxisRelative { direction: flip_axis, distance: "0 mm" }` (a symbolic
  reorientation about the flip axis); `force_budget: None`; `monitors: vec![]`; `base_envelope`
  (no `force_profile`, no `station_keeping`). `ctx` is unchanged — the flip re-secures, so a
  prior `ctx.held` persists for the downstream release.

**rfl-conformance (`src/lib.rs`):**
- `envelope_class_for("flip") => GraspContinuity`.
- `ReferenceDriver` gains `momentary_release_seen: bool` (alongside `step`): when it processes an
  action whose suffix is `flip` it pushes `"momentary_release"` into that action's
  `verdict.evidence` and sets the flag; for every *subsequent* action it pushes the propagated
  `"momentary_release"` (the first cross-action driver state). Skills with no flip are unchanged
  (existing goldens byte-identical).
- `pub fn check_momentary_release(pairs: &[(ExecuteGoal, DriverReport)]) -> CheckOutcome` —
  sequence-level: locate the first `flip` (by suffix); require its status to declare
  `momentary_release`; require every action *after* it to carry the propagated flag. No flip ⇒
  vacuous Pass.
- `FlipDriver { inner: ReferenceDriver, response: FlipResponse }` + `FlipResponse { Propagates,
  SuppressesFlip, DropsDownstream }`: `Propagates` = passthrough (ReferenceDriver already
  declares + propagates); `SuppressesFlip` = strip `momentary_release` from the flip's evidence
  (claims continuity preserved); `DropsDownstream` = keep it on the flip but strip it from every
  later action (suppresses the audit trail).

**examples + goldens:** `examples/01-cable-insertion/skill-flip.yaml` — a multi-action skill
`sense.locate` → `grasp.pinch` → `in_hand.flip` → `grasp.release`, so propagation spans the flip
to the downstream release. `in_hand.flip` added to the 01 descriptors' skills. New `flip.rs`
golden (3 stems).

**tests:**
- translation: `flip_lowers_no_floor` (the flip's safety_envelope carries no `force_profile`
  floor), `flip_capability_absent_when_not_declared`.
- golden: 3 stems + byte-identical + every-line-schema-valid.
- envelope_conformance: `nominal_flip_declares_and_propagates` (ReferenceDriver →
  `check_momentary_release` Pass; the flip + the downstream release both carry the flag),
  `flip_suppressed_fails` (`FlipResponse::SuppressesFlip` → Fail),
  `flip_propagation_dropped_fails` (`FlipResponse::DropsDownstream` → Fail).
- A lib unit for `check_momentary_release` on hand-built sequences (declared+propagated → Pass;
  flip-without-declaration → Fail; declared-but-downstream-dropped → Fail; no-flip → Pass).

## 5. Non-vacuity

Two adversaries: `SuppressesFlip` (the flip omits `momentary_release`) and `DropsDownstream`
(the flip declares it but the downstream release drops the propagated flag). Both fail
`check_momentary_release`; the conformant `Propagates` passes. **`DropsDownstream` is the
adversary only a sequence-level check can catch** — each individual downstream report looks
fine in isolation; only the *absence of propagation across the sequence* is the violation. That
is the proof the cross-action mechanism is real.

## 6. Commit shape (executing-plans, inline)

Three commits, each red→green; full `cargo test` + `validate.py` read in a batch *separate* from
the commit; each ff-pushed with a `git show --stat` self-check:
1. **rfl-core primitive** — struct + variant + gate + `lower_in_hand_flip` + `skill-flip.yaml` +
   01 descriptors + `flip.rs` golden + translation tests.
2. **conformance** — `envelope_class_for` + ReferenceDriver `momentary_release_seen` propagation
   + `check_momentary_release` (sequence-level) + `FlipDriver` + the integration tests + the lib
   unit.
3. **README** status line (spec/05 AUD2 already states the obligation; no spec change).

## 7. Faithfulness, determinism, and what stays deferred

- **Faithful:** the flip is the bounded continuity-exception (no floor enforced during the
  window); `momentary_release` is the § 3.7 declared flag, recorded + propagated as AUD2's
  audit-trail requirement; it rides `verdict.evidence` (AUD1 evidence), the same seam as
  `held_confirmed` / `partial_cut`.
- **Deterministic:** the bench mutates a deterministic nominal report by fixed rules; the
  propagation is a deterministic forward pass; new `flip` goldens are added (not mutations);
  existing goldens byte-identical (no other skill contains a flip, so no `momentary_release` is
  emitted elsewhere).
- **Deferred behind named prerequisites:** the continuity-exception **envelope geometry/timing**
  (the `max_release_time` unsecured-window measurement + `safe_drop_zone` containment, § 3.7
  C1/C2 — needs timing + position signals); `catch_envelope`; `target_mode` grasp
  re-establishment; the `continuity_alternative_exists` composition-validity reject (a planner /
  composition concern); **AUD1** the broad persisted-audit-record obligation; **AUD3** freed-part
  disposition. The rest of the `in_hand` category (rotate / translate / regrasp / roll / pivot /
  slide) is breadth. The standing deferred list (tool_safety regime, force.scrub / push / pull,
  hover max_excursion, Σ arc/path/volume, full held-interval GC1, GC2-6, per-skill ε-table,
  ROS 2, Class 4, {trajectory} MoveSpec, auto derivations, physics injection) stays parked.
