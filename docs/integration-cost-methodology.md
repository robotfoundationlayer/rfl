# Integration-cost methodology (the nine-cell matrix)

The README claims RFL collapses the O(N×M) VLA-×-embodiment integration burden to
**additive** cost, and commits (footnote 1) to publishing a per-pair benchmark
against a **nine-cell support matrix** during the 2026 Q3 – 2027 Q1 review
period. This document fixes the **methodology** so the eventual numbers are
reproducible and falsifiable; it does not pre-judge the results.

## The nine cells

The v0.1 matrix is a 3×3 representative slice of the ~9-VLA × ~30-embodiment
space — three VLA-tier *emitters* against the three structurally distinct
embodiment classes already committed as reference descriptors:

| | Allegro (tactile, anthropomorphic) | LEAP (ft-class) | Pneumatic-6F (no-tactile) |
|---|---|---|---|
| **Emitter A** | cell 1 | cell 2 | cell 3 |
| **Emitter B** | cell 4 | cell 5 | cell 6 |
| **Emitter C** | cell 7 | cell 8 | cell 9 |

The three embodiments are chosen to span the capability axes that stress
retargeting: tactile vs force/position-proxy confirmation, and force-closure vs
no-tactile grasp. The three emitters span planner/VLM tiers (a frontier
general model, a robotics-tuned VLA's planner layer, a minimal prompted
baseline). The exact model roster is fixed at publication.

## What each cell measures

For a (emitter, embodiment) pair, three instruments — all already in the repo —
produce the cell's numbers:

1. **Emit (demand side)** — does the emitter produce valid, schema-conformant
   Skill ISA for the task set? Measured as `schema_ok` pass@k, exactly as the
   demand-side existence proof does (see [existence-proofs.md](existence-proofs.md);
   the `cobel` reference reports `claude-opus-4-8` pass@5 across novel tasks).
2. **Retarget (the contract)** — does `rfl retarget <skill> --embodiment <desc>`
   lower the emitted skill onto that embodiment, or report `capability_absent`?
   A `capability_absent` is a *capability* gap (an honest negotiation outcome),
   distinct from an emission error or an engine-coverage gap (`beyond_engine`).
3. **Certify (conformance)** — does the lowered skill, run against the
   embodiment's driver, pass `rfl certify` (Class-3 driver-protocol)? The
   content-hashed certificate is the per-cell evidence artifact.

## The integration-cost metric

Per-pair integration cost is decomposed so the **additive-vs-multiplicative**
claim is measurable, not rhetorical:

- **Without RFL** — `C_pair`: the bespoke effort (engineer-hours and LOC) to wire
  one VLA to one embodiment directly. The total burden is `Σ C_pair` over all
  N×M pairs (multiplicative).
- **With RFL** — two one-time costs:
  - `C_emit(VLA)` — make a VLA emit Skill ISA once (the demand-side adapter);
  - `C_driver(embodiment)` — implement the Driver Interface once (the supply-side
    driver).
  Every (VLA, embodiment) pair then costs only the *negotiation* (retarget +
  certify), which is automated. The total is `Σ C_emit + Σ C_driver + O(N·M)·c`
  where `c` is the near-zero automated per-pair cost (additive in N and M).

The benchmark reports, per cell: the emit pass-rate, the retarget outcome, the
certify result, and the marginal engineer-effort to *add* that cell once both
its row's emitter and column's driver exist — which the additive model predicts
is approximately `c`, not a fresh `C_pair`.

## Honesty constraints

- **No silent capping.** If a cell is not measured (e.g. an emitter is
  unavailable), the matrix marks it explicitly, never blank-as-pass.
- **Engine-coverage gaps are disclosed**, not folded into model error: a valid
  emission that fails retarget because the reference engine doesn't implement a
  primitive is `beyond_engine`, reported separately from a `capability_absent`
  (a genuine embodiment limitation) and from an invalid emission.
- The instruments are deterministic (`retarget`, `certify` content hashes), so a
  third party can reproduce any cell from the published skill + descriptor +
  driver report.

## Status

Methodology fixed (this document). The populated nine-cell matrix and its
numbers are the 2026 Q3 – 2027 Q1 deliverable; the supply-side (real-hardware
4-DOF arm) and demand-side (VLA → Skill ISA) existence proofs that anchor the
emitter and driver columns are tracked in [existence-proofs.md](existence-proofs.md).
