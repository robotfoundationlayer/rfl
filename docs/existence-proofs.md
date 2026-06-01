# RFL's two existence proofs: can a model emit it, can a robot run it?

RFL is a neutral layer between Vision-Language-Action foundation models and robotic
embodiments. The whitepaper argues the position; the `spec/` directory fixes the contract; the
Rust reference implementation shows the contract is internally coherent and constructible. But
a standard that sits *between* two populations has to answer two separate questions, one for
each side it touches:

1. **Supply side.** Can a real robot actually run a skill that RFL retargeted onto it?
2. **Demand side.** Can a real foundation model actually emit the RFL Skill ISA in the first
   place?

These are different claims, and a reference implementation that only talks to a placeholder
driver proves neither. So we built one existence proof for each side.

## Demand side: a foundation model emits valid Skill ISA (`cobel`)

[`masterleopold/cobel`](https://github.com/masterleopold/cobel) (public, Apache-2.0) tests the
demand-side claim directly. It defines a small planner-adapter contract,
`plan(task, scene) -> skill_yaml`, with two implementations: a deterministic offline mock
(the CI gate) and a real emitter backed by `claude-opus-4-8`. The model is given the task, a
structured scene, and the Skill ISA spec; it is **not** given the embodiment, because the
Skill ISA is embodiment-agnostic by construction (Principle 1). The descriptor enters only
afterward, at retarget.

Each emitted skill passes through two gates:

- **Schema (primary).** Does it conform to the full 50-primitive `skill-isa.schema.json`? This
  is the spec-grounded definition of "valid Skill ISA," and it does not depend on how much of
  the spec the reference engine has implemented yet.
- **Retarget (secondary).** Does the published `rfl.retarget` binding lower it onto a real
  embodiment descriptor? This is a stronger check, but it is bounded by engine coverage, so
  its outcome is four-valued: `retarget_ok`, `capability_rejected` (the embodiment lacks a
  capability the task needs, an honest mismatch), `beyond_engine` (valid Skill ISA the
  reference engine has not implemented yet, a coverage gap rather than a model error), or
  `malformed`.

Across six novel tasks (deliberately not the worked examples), `claude-opus-4-8` reaches
**5/5 schema-validity at pass@k in both spec-only and few-shot modes**. A real frontier model
emits valid, full-spec Skill ISA on every task, every sample.

Three findings are worth calling out, because they are the honest substance of the result:

- **Contract fidelity is the lever.** Given only a prose digest of the spec (no schema),
  spec-only validity fell to roughly one task in six: the model wrote plausible skills with
  slightly wrong parameter names and shapes. Supplying the full JSON schema as the contract,
  which is still "blind" (no worked example skill), took it to 6/6. The proof is honest about
  what you have to hand the model.
- **The retarget map spans embodiments.** The same embodiment-agnostic `relocate-part` skill
  retargets cleanly onto both a dexterous tendon-driven hand and a 600-dollar, 4-DOF,
  no-force-sensing arm, while force-controlled tasks are correctly rejected on the arm that
  lacks force sensing. One skill, many embodiments, with the abstraction declining to leak.
- **A single emission can exhibit every outcome.** In few-shot mode one Claude-emitted skill
  retargets to canonical actions on the force-capable hands and is correctly capability-
  rejected on the no-force arm.

Reproduce it offline with no API key: `python run.py --planner mock`. CI builds the binding
from this repository and runs the full suite hermetically.

## Supply side: a real robot runs a retargeted skill (in progress)

The matched half holds the planner trivial (a hand-authored skill) and makes the *embodiment*
real: a retarget proof on a 4-DOF arm, validated first in simulation, then on hardware, with
the measured integration cost published. Its design lives in
[`docs/design/`](design/2026-05-31-pincherx-existence-proof-design.md); the hardware bring-up
is underway. Together with `cobel` it brackets the roadmap's "(VLA, embodiment) pair"
milestone from both ends.

## The honest boundary

The demand-side claim is precise: a frontier model in the **planner / VLM tier** can target
the Skill ISA. It is **not** the claim that today's action-token VLAs (the ones that emit
low-level motor actions at tens of hertz) speak RFL. Those operate *below* the Skill ISA,
closer to the canonical-action and driver-interface layer. The Skill ISA's natural emitter is
the planner tier (the Code-as-Policies / SayCan / "System-2 VLM" archetype), and a general
frontier model is exactly that archetype. Stating where the boundary sits is part of the
result, not a footnote to it.

## Why this matters

RFL's whole bet is that the O(N x M) integration burden between models and robots collapses to
additive cost once a neutral, semantically-typed layer exists. Two independent repositories,
each depending on the published standard rather than on each other, are that thesis in
miniature: one demonstrates the standard is *emittable* by a real model, the other that it is
*runnable* on real hardware. The pattern they follow, an independent implementation depending
on the published contract, is the one any future driver or planner integration would reuse.
