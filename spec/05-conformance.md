# Conformance — Specification

> **Status**: in progress (2026-05-31) — the four test classes, the three-tier regime, and the envelope-class taxonomy (terminal-postcondition / interval-invariant / grasp-continuity / force-torque-trajectory) are specified. Remaining before freeze: the verification units (grasp-continuity modes; closure / stability / composition; reversibility + irreversible-operation safety; hazardous-operation benches; audit propagation; the determinism floor + fidelity-tier → badge + trademark gate), tracked in `02-translation-layer.md` § Open issues (Owned by `05`).

## Scope

This chapter defines:

1. The four conformance test classes
2. The three-tier conformance regime (self-certification → steward-verified → notified-body-certified)
3. The trademark gate (which tier allows use of the `RFL™` trademark on packaging)

## Four test classes

| # | Class | What it tests |
|---|---|---|
| 1 | **Skill ISA parser conformance** | A YAML file conforms to the Skill ISA JSON schema and the compositional algebra parses without error |
| 2 | **Translation Layer determinism** | Given a fixed skill + fixed embodiment descriptor, `retarget()` produces identical output byte-for-byte across runs and platforms |
| 3 | **Driver Interface protocol compliance** | A driver implementation accepts the canonical actions, executes them within stated tolerances, and reports back via the protocol |
| 4 | **End-to-end execution conformance** | A skill expressed at the Skill ISA layer executes correctly on the target embodiment via the full Translation → Driver Interface chain |

Test classes 1 and 2 are pure-compute and can run in CI. Class 3 requires the driver binary. Class 4 requires the physical embodiment (or a high-fidelity simulator declared as conformant).

## The envelope-class taxonomy

Test classes 3 and 4 (driver-protocol and end-to-end) verify that a primitive *executed correctly within stated tolerances*. "Correctly" is not a single shape: a `reach` is judged at its endpoint, a `transport.carry` over its whole interval, a `grasp` by a held-state invariant, a `force.wipe` by its force profile. Rather than re-derive a bespoke check per primitive (the per-primitive conformance-test sketch of `01`), the suite verifies every primitive against one of **four envelope classes**, defined by *what is sampled and when*. The envelope class is the reusable verification shape; a primitive's conformance test is its envelope class instantiated with that primitive's tolerances.

| Envelope class | What is sampled | When | Primitives |
|---|---|---|---|
| **Terminal-postcondition** | the end state (pose at rest, no task contact) | endpoint only | `reach.*` |
| **Interval-invariant** | a maintained invariant (station-keeping, dynamic stability) | every sample over the interval | `reach.hover`, `transport.carry` |
| **Grasp-continuity** | the held-state invariant (holding force ≥ `min_holding_force`) | every sample, held → held | `grasp.*`, `in_hand.*`, `transport.*`, `place.*` (§ Grasp-continuity modes) |
| **Force/torque-trajectory** | the force (or torque) profile against per-axis budgets | every sample over the motion | `force.*` |

### Endpoint vs. interval sampling

The terminal-postcondition class checks only the final state — a `reach` that passes through a transient excursion but arrives correctly conforms. The other three are **interval** classes: the invariant must hold at *every* sample, so a mid-motion violation is a failure even if the endpoint is correct. A `force.wipe` whose force spikes mid-stroke fails the force-trajectory class though it ends within budget; a `transport.carry` that loses stability mid-path fails the interval-invariant class though it reaches the goal. Interval sampling runs at a rate tied to the embodiment's control rate and the manifold timebase (`04` § Temporal alignment across multi-rate features), so the sampled invariant is evaluated deterministically (binding for fixture reproducibility, Class 2).

### Disturbance injection for interval classes

An interval-invariant test of a *disturbance-rejecting* primitive must perturb it: `transport.carry` declares a `disturbance_budget`, and the bench injects calibrated disturbances up to that budget during the interval, verifying the invariant holds under perturbation — and that an over-budget disturbance degrades gracefully (halt with object secured) rather than dropping the object. Disturbance injection is the interval-class analogue of the terminal class's "present the object and command"; it is part of the bench, not the primitive.

### The force/torque-trajectory class

The `force` category bounds the force *profile over the whole motion*, not a single endpoint — the verification backbone for all ten `force` primitives. The bench interval-samples the contact force against the per-axis budgets; a mid-motion spike is a violation, not a success. The class **generalizes to torque**: `force.screw` / `force.unscrew` bound a *torque* trajectory about an axis (`Torque`, N·m) under the identical interval discipline. The force-event detections that terminate these primitives (breakaway, detent — `04` § Force events) are evaluated on this same sampled trajectory.

### Conformance obligations (envelope classes)

- **ENV1 — class assignment.** Every primitive is verified against exactly one envelope class, fixed by category: `reach` terminal (except `hover`); `reach.hover` / `transport.carry` interval-invariant; `grasp` / `in_hand` / `transport` / `place` grasp-continuity; `force` force/torque-trajectory.
- **ENV2 — interval coverage.** An interval class samples the invariant at every step over the whole interval at the declared rate; a mid-interval violation fails the test even when the endpoint conforms.
- **ENV3 — disturbance injection.** A disturbance-rejecting interval test injects calibrated disturbances up to the primitive's `disturbance_budget`, verifies the invariant under perturbation, and verifies graceful degradation (object secured) above budget.
- **ENV4 — trajectory bounding.** The force/torque-trajectory class bounds the force (or torque) profile interval-sampled against per-axis budgets; the identical discipline applies to linear force and to torque about an axis.

### Deferred and referenced

- **The per-primitive tolerances and safety envelopes** the classes instantiate — `01-skill-isa.md` (each primitive's safety-envelope + conformance-test sketch).
- **The interval sampling timebase** — `04-tactile-manifold.md` § Temporal alignment across multi-rate features (the common monotonic timebase the sampling rate references).
- **The grasp-continuity class's sub-modes** — § Grasp-continuity modes (next unit).

## Three-tier conformance regime

| Tier | Verifier | Cost to implementer | Permits use of RFL™ trademark? |
|---|---|---|---|
| 1 — Self-certification | Implementer publishes own results | Free | No |
| 2 — Steward-verified | RFL Inc. or RFL Foundation engineering staff runs the suite | Free for Lead Customers / Founding Members; fee otherwise | Yes |
| 3 — Notified-body | Independent body (TÜV or comparable, post-MoU) | Independent fee | Yes (and enables ISO 10218 / 13482 flow-through) |

## Open issues

- Determinism requirement floor (bit-identical vs. semantic-equivalence with epsilon)
- How a simulator earns "high-fidelity / conformant" status (recursive conformance)
- Trademark assignment after Stage 2 Foundation donation
