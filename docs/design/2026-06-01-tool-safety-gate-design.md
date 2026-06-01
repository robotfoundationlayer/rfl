# Design: tool_safety gate — enforce force.cut's hazardous-tool capability

Status: approved design, pre-implementation (2026-06-01)

This is the twenty-second reference-implementation increment, a *finish*: it closes a deferred
leg of increment 19 (`force.cut`). spec/01 § 6.7 requires a cut's embodiment to declare the
cutting tool's safety capability (`capabilities.aux.tool_safety: hazard_class`); the cut gate
today checks only `force.cut`, so the requirement is unenforced. This is force.cut's deferred
"next unit" (the *gate*; the dummy-hand validation *bench* stays deferred). The spec is
authoritative; this realizes the precondition gate.

## 1. The gap

`spec/01` § 6.7 preconditions: "`embodiment` declares `force` with `cut` support, the required
`compliance`, and the cutting tool's safety capability (per the deployment's safety standard —
cutting tools are hazardous)." The § 6.7 `capability_absent` failure mode: "no `cut` /
compliance / tool-safety capability → reject; no attempt." Today `lower`'s `check_capability`
gate for `Primitive::ForceCut` returns the single key `"force.cut"` — the `tool_safety`
requirement is never checked, so a hazardous cut on an embodiment that has *not* declared it
manages the hazard lowers fine. An unenforced M2 capability-completeness gap.

## 2. No schema change

The embodiment descriptor schema already defines `capabilities.aux.tool_safety { hazard_class,
standard? }` (`schemas/embodiment-descriptor.schema.json`). `tool_safety` is *input* (a declared
capability), not retarget output — so there is **no golden change** and **no conformance-check
change**. Pure `rfl-core` gate.

## 3. The contract

The `ForceCut` capability gate requires **both** `force.cut` and a declared `tool_safety`. A cut
on an embodiment with `force.cut` but no `tool_safety` is rejected (`capability_absent:
tool_safety`). `force.cut` is checked first, so an embodiment lacking `force.cut` entirely still
reports `capability_absent: force.cut`. This is the first *conjunctive* capability gate beyond
`grasp.release`'s "any `grasp.*`" — a primitive requiring more than one capability, the gate
naming which is missing.

## 4. Surface (rfl-core only)

- `src/embodiment.rs`: `Aux` gains `tool_safety: Option<ToolSafety>` (`#[serde(default)]`);
  new `struct ToolSafety { hazard_class: String, #[serde(default)] standard: Option<String> }`;
  `Embodiment::has_tool_safety(&self) -> bool` returning `self.capabilities.aux.tool_safety.is_some()`.
- `src/translation.rs`: replace the `Primitive::ForceCut(_) => "force.cut"` arm of
  `check_capability` with a custom `return` arm (like `GraspRelease`): `force.cut` absent →
  `Err("capability_absent: force.cut")`; else `tool_safety` absent →
  `Err("capability_absent: tool_safety")`; else `Ok(())`.
- `examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml`: add `tool_safety: {
  hazard_class: cut }` under `capabilities.aux` (so the existing cut tests keep passing).
- Test (`src/translation.rs`): `cut_requires_tool_safety_capability` — the existing `CUT_SKILL`
  on a minimal inline embodiment (`embodiment: { id: test-hand, capabilities: { skills:
  [force.cut] } }`, no `tool_safety`) → `retarget` errors with `capability_absent: tool_safety`.

## 5. Non-vacuity

The rejection test proves the gate bites (`force.cut` declared, `tool_safety` absent → rejected).
The existing cut suite (the `cut.rs` golden, the `envelope_conformance` cut tests,
`cut_lowers_irreversible_and_shear_budget`) keeps passing **because** the 3 descriptors now
declare `tool_safety` — confirming the gate does not over-reject a properly-equipped hand. The
existing `cut_capability_absent_when_not_declared` (cable-01, which lacks `force.cut`) still
reports `capability_absent: force.cut` (checked first). The two negative tests pin both arms.

## 6. Commit shape (executing-plans, inline)

Two commits, each red→green; full `cargo test` + `validate.py` read in a batch *separate* from
the commit; each ff-pushed with a `git show --stat` self-check:
1. **rfl-core gate** — `Aux.tool_safety` + `ToolSafety` + `has_tool_safety` + the `ForceCut`
   gate arm + `tool_safety` on the 3 descriptors + the rejection test. (The descriptors must be
   updated in this same commit or the existing cut suite goes red — the gate-tightening ⇒
   descriptor-update coupling.)
2. **README** status line (spec/01 § 6.7 + spec/05 already state the obligation; no spec change).

## 7. Faithfulness, determinism, and what stays deferred

- **Faithful:** the gate is § 6.7's `capability_absent` precondition (a hazardous cut requires a
  declared tool-safety capability); `hazard_class` is the schema's required field. The
  conjunctive form mirrors the spec's "declares cut support AND the tool's safety capability."
- **Deterministic:** no golden change (input-only capability); the cut output is byte-identical
  (the gate only rejects earlier when `tool_safety` is absent).
- **Deferred behind named prerequisites:** the **tool_safety validation bench** (the dummy-hand
  hazard-class validation, spec/05's deeper "Hazardous-operation conformance benches" unit —
  verifying the *declared* `hazard_class` actually matches the operation, not just that it is
  present); `human_collaboration_safety` (the sibling `aux` capability gating `place.hand_to`,
  which needs the `place` category); the `standard` field's semantics. The standing deferred list
  (force.scrub / push / pull, the rest of in_hand, place / sense, hover max_excursion, AUD1,
  AUD3 freed-part, Σ arc/path/volume, full held-interval GC1, GC2-6, per-skill ε-table, ROS 2,
  Class 4, {trajectory} MoveSpec, auto derivations, physics injection) stays parked.
