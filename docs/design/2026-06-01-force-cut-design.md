# Design: force.cut (REV3 irreversible) — precise partial-state on interruption

Status: approved design, pre-implementation (2026-06-01)

This is the nineteenth reference-implementation increment. It implements `force.cut`
(`spec/01` § 6.7), opening the **irreversible-operation safety class** (`spec/05` § 175,
REV3) — "distinct from the envelope classes" and entirely unexercised. v0 realizes its
distinctive obligation: **on interruption, report exactly how far the cut progressed (a
partially-cut state), never a binary success / failure**. It completes the reversibility
spectrum the previous increment opened (REV2 `force.snap_engage` → REV3 `force.cut`). The spec
is authoritative; this realizes it.

## 1. The gap

`force.cut` is the first irreversible primitive: a cut cannot be undone. `spec/05` § 175 gives
it a dedicated safety treatment, of which the conformance-falsifiable v0 leg is the
partial-state rule (§ 179): "If an irreversible operation is interrupted, it reports exactly
how far it progressed (a partially-cut state), never a binary success / failure — downstream
recovery and the audit loop need the precise irreversible state." § 6.7's `incomplete_cut`
failure mode: "stop; report (partial, irreversible — state reported)." Nothing exercises this.

## 2. No schema change

`force.cut` is in the skill-isa PrimitiveId enum and the embodiment capability enum;
`ForceCutParams` is typed (`cut_path` + `shear_force_budget` + `completion` required). The
partial-state rides `verdict.evidence` (the AUD1 audit channel, exactly as
`force.snap_engage`'s `held_confirmed` does); the shear budget reuses `ForceTrajectory`. So
**pure rfl-core + rfl-conformance + an example**.

## 3. The contract (v0)

- **`envelope_class_for("cut") => ForceTrajectory`.** Shear force ≤ `shear_force_budget` (the
  existing arm, reused; `Fault::OverForce` is the § 6.7 `shear_exceeded` C2 case — a spike at a
  hard inclusion).
- **Irreversibility (the new leg).** The lowering emits `force_profile.irreversible = true`. A
  new `check_irreversible(goal, report)` is vacuous unless `irreversible` is set; else for an
  **interrupted** cut (outcome ≠ Succeeded) `verdict.evidence` MUST carry a `partial_cut…`
  marker (the precise partial state). A binary failure that hides the partial state fails. A
  *completed* cut (Succeeded) is vacuous — nothing partial to report.

The obligation flips with the outcome, the mirror of `check_actuation`/`check_engagement`
(which bite on the *success* path): cut bites on the *failure* path — "don't hide failure
behind a binary halt."

## 4. Surface

**rfl-core (one commit — exhaustive-match):**
- `struct ForceCut` (`src/skill_isa.rs`): `cut_path` (`serde_yaml::Value`, carried opaque),
  `shear_force_budget` (`Quantity`), `completion` (`serde_yaml::Value`), `compliance`
  (`Option<Compliance>`).
- `Primitive::ForceCut(ForceCut)` + `check_capability` arm `"force.cut"` (the `tool_safety`
  gate-extension is **deferred** — spec/05's "next unit") + the lower dispatch arm
  `(lower_force_cut(p, e), "cut")` — all this commit.
- `lower_force_cut` (`src/translation.rs`): `force_budget: Some(p.shear_force_budget)`;
  `force_profile = {"irreversible": true}`; `completion → Monitor`; `target_pose:
  PoseExpr::FrameRelative { frame: "task", offset: yaml_to_json(&p.cut_path) }` (carry the
  trajectory opaque, exactly as `lower_transport_carry` does for a non-`to_pose` MoveSpec);
  `target_frame: e.grasp_frame()` (tool-mediated, like `force.screw`); compliance mapped.

**rfl-conformance (`src/lib.rs`):**
- `envelope_class_for("cut") => ForceTrajectory`.
- `pub fn check_irreversible(goal, report) -> CheckOutcome` — vacuous unless
  `force_profile.irreversible` is true; else `outcome != Succeeded ⟹ verdict.evidence` has an
  entry beginning `"partial_cut"`.
- `CutDriver { inner: ReferenceDriver, response: CutResponse }` + `CutResponse { Completes,
  PartialReported, BinaryHalt }`: `Completes` = passthrough (Succeeded); `PartialReported` =
  `Failed` + `failure_class "blocked"` + `failure_detail "incomplete_cut"` + push
  `"partial_cut: 0.6"` into `verdict.evidence`; `BinaryHalt` = same `Failed` + `incomplete_cut`
  but **no** `partial_cut` evidence (a binary halt — adversarial). No `ReferenceDriver` change
  (a nominal cut is `Succeeded`, vacuous for the irreversible leg).

**examples + goldens:** `examples/03-screw-fasten/skill-cut.yaml` (a `force.cut` with `cut_path`,
`shear_force_budget`, `completion`, `compliance`) + `force.cut` on the 3 descriptors; new
`cut.rs` golden (3 stems).

**tests:** translation (`cut_lowers_irreversible_and_shear_budget`,
`cut_capability_absent_when_not_declared`); golden (3 stems); envelope_conformance
(`nominal_cut_passes`: ForceTrajectory Pass + check_irreversible vacuous-Pass on Succeeded;
`cut_partial_state_reported_is_honest`: PartialReported → check_irreversible Pass;
`cut_binary_halt_fails`: BinaryHalt → check_irreversible Fail; `cut_over_force_fails`:
`Fault::OverForce` → ForceTrajectory Fail) + a lib unit for `check_irreversible`.

## 5. Non-vacuity

`BinaryHalt` (an interrupted cut reporting a bare `Failed` with no `partial_cut` evidence) →
`check_irreversible` Fail — proves the precise-partial-state obligation is load-bearing (an
irreversible op that hides how far it got is a real audit/recovery failure). `Fault::OverForce`
(reused) → `ForceTrajectory` Fail. Conformant `Completes` / `PartialReported` give the contrast
on reports that differ only by the partial-state evidence + outcome.

## 6. Commit shape (executing-plans, inline)

Three commits, each red→green; full `cargo test` + `validate.py` read in a batch *separate* from
the commit; each ff-pushed with a `git show --stat` self-check:
1. **rfl-core primitive** — struct + variant + gate + `lower_force_cut` + `skill-cut.yaml` + 3
   descriptors + `cut.rs` golden + translation tests.
2. **conformance** — `envelope_class_for` + `check_irreversible` + `CutDriver` + the integration
   tests + the lib unit.
3. **README** status line (spec/05 REV3 already states the obligation; no spec change).

## 7. Faithfulness, determinism, and what stays deferred

- **Faithful:** the partial-state is § 6.7's `incomplete_cut` "state reported" recorded as AUD1
  `verdict.evidence`; the shear budget drives the existing ForceTrajectory leg; the cut is
  tool-mediated (grasp frame, like screw).
- **Deterministic:** the bench mutates a deterministic nominal report by fixed rules; new `cut`
  goldens are added (not mutations); existing goldens byte-identical (no other action emits an
  `irreversible` marker, and the ReferenceDriver is unchanged).
- **Deferred behind named prerequisites:** the **`tool_safety` hazardous-tool regime** (the
  capability gate-extension + the dummy-hand validation bench — spec/05's explicit "next unit");
  **path-bounding** (the cut stays within `cut_path` — needs resolved geometry + a position
  signal); **pre-execution confirmation** of the path and material beyond; **`on_separation`**
  freed-part handling (retain / drop_safe); `feed_rate`. `force.weld` / adhesive (future
  irreversible extensions) reuse `check_irreversible`. The standing deferred list (force.scrub /
  push / pull, in_hand.flip AUD1, hover over-envelope max_excursion, Σ arc/path/volume, full
  held-interval GC1, GC2-6, per-skill ε-table, ROS 2, Class 4, {trajectory} MoveSpec, the
  various `auto` derivations, physics injection) stays parked.
