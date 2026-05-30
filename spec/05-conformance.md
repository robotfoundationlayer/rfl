# Conformance — Specification

> **Status**: in progress (2026-05-31) — the four test classes, the three-tier regime, and the envelope-class taxonomy (terminal-postcondition / interval-invariant / grasp-continuity / force-torque-trajectory) are specified. Remaining before freeze: the verification units (closure / stability / composition; reversibility + irreversible-operation safety; hazardous-operation benches; audit propagation; the determinism floor + fidelity-tier → badge + trademark gate), tracked in `02-translation-layer.md` § Open issues (Owned by `05`).

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

## Grasp-continuity modes

The grasp-continuity envelope class (§ The envelope-class taxonomy) is the most elaborate, because the `grasp` / `in_hand` / `transport` / `place` categories hold an object across operations that *change the contact set* — gaiting migrates contacts, regrasp swaps them, a pivot releases one DOF, a flip releases entirely, a handoff transfers between two parties. The class's invariant is uniform — **a securing contact set maintains the object at ≥ `min_holding_force` at every sampled instant, verified from the force trace** — but *what counts as the securing set* and *how continuity survives a transition* differ by mode. This section defines the operational closure test and the five continuity modes.

### The hold test and closure branching

A grasp's success is operationally defined by a **hold test**: apply a calibrated perturbation below the grasp's force budget and verify the object is retained. The hold test is what every grasp primitive's C1 means by "the grasp held." It branches on the closure type (`01` `StabilityMetadata.closure`):

| Closure | Perturbation | Retention criterion |
|---|---|---|
| `force` | **omnidirectional** | the object resists a sub-budget perturbation from any direction |
| `form` | **`load_direction`-only** | the object resists along the form-held load direction(s); reverse / lateral are free by design |
| `support` | **level, gentle** | a balanced object resists a gentle perturbation; it cannot be released by opening, only set down |

The closure branch is what makes the hold test a single canonical procedure across ten grasp modes: the perturbation profile is selected by `closure`, not re-specified per mode.

### Base continuity — held → held

The simplest mode: a single established grasp holds across an operation that does not change the contact topology (`grasp.adjust`, `transport.move_to_pose`, `place.put_down`). The force trace shows holding force never dropping below `min_holding_force` from the start of the operation to its end. Every other mode relaxes "the *same* contacts secure throughout" into "*some* securing set exists throughout."

### Make-before-break and gaiting

Finger gaiting (`in_hand.rotate` / `in_hand.translate`) and regrasp (`in_hand.regrasp`, and the single-party part of `transport.handoff`) pass through **intermediate contact sets** — a finger lifts and replaces, a grasp is swapped. Continuity holds iff at every instant the **union** of engaged contacts (old, new, or both) satisfies `min_holding_force`: a securing set exists at all times even though no single contact persists. For a regrasp the discipline is stricter — **make-before-break**: the new grasp is confirmed *before* the old is released, so the two overlap rather than gap. The suite verifies both "a securing contact set exists at every instant" and the make-before-break ordering from the force trace. This is the most general form of the class; base continuity is its degenerate case (the union is a single unchanging contact set).

### Controlled under-actuation

`in_hand.pivot` deliberately releases **exactly one** DOF — the pivot rotation — while the remaining DOF keep the object secured at ≥ `min_holding_force` (a controlled under-actuation, not a release). The suite verifies that exactly the named DOF is under-constrained while all others secure, and that the released DOF is **re-secured** at completion (the object returns to fully held). A third continuity mode alongside base and make-before-break: continuity is preserved, but through a deliberately relaxed — not swapped — contact constraint.

### Bounded continuity-exception

`in_hand.flip` is the **only** primitive that *suspends* grasp continuity: a momentary release tosses and recatches the object. The suite verifies the exception is **bounded** rather than continuity-preserving:

- the unsecured window ≤ `max_release_time`;
- the re-catch occurs within `catch_envelope`;
- on a failed catch, the object lands within `safe_drop_zone` (the no-uncontrolled-drop guarantee shared with the freed-part disposition contract, `04` § Freed-part handling at constraint release).

This is an explicit, bounded **subclass** of the grasp-continuity class, distinct from the continuity-preserving modes above: continuity *is* broken, but only within a verified bound with a safe-landing fallback.

### Two-party co-grasp

`transport.handoff` (inter-party) extends make-before-break to **two parties**: at every instant at least one party secures the object at ≥ `min_holding_force`, and during the dual-grasp window the **combined** force stays ≤ `cograsp_force_budget` (no crushing, no tug-of-war). The suite verifies both — at-least-one-secures continuity and the combined-force ceiling — from the two-party force trace. Bimanual handoff within one embodiment is fully verifiable here; the inter-robot coordination protocol and its two-party determinism are owned by `02-translation-layer.md` (the multi-embodiment-coordination open issue).

### Conformance obligations (grasp continuity)

- **GC1 — continuous securing.** The grasp-continuity class verifies from the force trace that a securing contact set maintains the object at ≥ `min_holding_force` at every sampled instant.
- **GC2 — hold-test closure.** A successful closure is operationally defined by the hold test (calibrated sub-budget perturbation → retention), with the perturbation profile branched on `closure ∈ {force: omnidirectional, form: load-direction, support: level-gentle}`.
- **GC3 — make-before-break.** Through a contact-set transition (gaiting, regrasp) the union of engaged contacts satisfies `min_holding_force` at every instant; a regrasp confirms the new grasp before releasing the old.
- **GC4 — controlled under-actuation.** A pivot releases exactly the named DOF while all others secure at ≥ `min_holding_force`, and re-secures the released DOF at completion.
- **GC5 — bounded exception.** A flip's continuity suspension is bounded: unsecured window ≤ `max_release_time`, re-catch within `catch_envelope`, and on failure the object lands within `safe_drop_zone`.
- **GC6 — two-party continuity.** In a co-grasp handoff, at every instant at least one party secures the object at ≥ `min_holding_force`, and the combined force stays ≤ `cograsp_force_budget` during the dual-grasp window.

### Deferred and referenced

- **The lifecycle transition table and `StabilityMetadata`** the modes read (`closure`, `secured_dof`, the held → manipulated → held FSM) — `01-skill-isa.md` § Grasp state model.
- **The `min_holding_force` derivation** (from mass / mode / friction / load direction) — `02-translation-layer.md`.
- **The force trace and slip classification** the verification reads — `04-tactile-manifold.md`.
- **The `momentary_release` audit propagation** a flip raises — § Audit and transparency (later unit).
- **The inter-robot handoff coordination protocol** (two-party determinism) — `02-translation-layer.md`.

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
