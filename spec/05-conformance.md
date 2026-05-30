# Conformance — Specification

> **Status**: in progress (2026-05-31) — the four test classes, the three-tier regime, and the envelope-class taxonomy (terminal-postcondition / interval-invariant / grasp-continuity / force-torque-trajectory) are specified. Remaining before freeze: the verification units (audit propagation; the determinism floor + fidelity-tier → badge + trademark gate), tracked in `02-translation-layer.md` § Open issues (Owned by `05`).

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

## Closure, stability, and composition verification

Where the grasp-continuity modes verify a held state *over time*, this unit verifies a grasp's **static** properties — the geometry that makes a closure what it claims to be, the safe state a closure type demands, and the legality of a primitive *sequence* given a grasp's stability class. These are structural checks, decidable from the grasp's `StabilityMetadata` (`01`) without a time trace.

### Non-degenerate tripod

`grasp.precision_tripod` claims `rotation_constrained` (`01` `StabilityMetadata.flags`) — it resists rotation about the grasp axis that a two-point pinch cannot — but only if its three contacts are genuinely spread. Three near-collinear contacts form a line, not a triangle, and provide no rotation resistance. The suite confirms a tripod only when its three contacts form a **non-collinear triangle above a minimum-area threshold** (relative to the object's cross-section); a near-collinear triple is not reported as a successful tripod, because the claimed rotation constraint would be absent.

### Support-grasp safe state

A `support`-closure grasp (`grasp.platform`, `balance_held`) holds a balanced object by resting it over a support polygon — it **cannot be released by opening**, because opening drops a balanced object. Its safe / abort state is therefore special: **a controlled lowering to the nearest surface, minimizing fall height**, never an open-release. The suite verifies that a support grasp's safe response is this controlled set-down (gated by the supported-state predicate, `01`), distinct from the open-and-withdraw safe state of a force-closure grasp.

### Composition validity by stability class

The lifecycle transition table (`01` § Grasp state model) fixes which state transitions are legal; this unit adds the full **stability-class → permitted-successor** table — a static composition check that rejects an illegal sequence at validation, the mechanically-checkable basis of composition conformance (test class 1 + the `01` composition-validity algebra).

| Stability property | Forbidden successor | Reason |
|---|---|---|
| `surface_bound` (pin) | free `transport.*` | the grasp is invalid if the supporting surface is lost (`transport_inadmissible`) |
| `form_held` DOF *d* | an `in_hand.*` that moves *d* | only `friction_held` DOF are movable (`*_inadmissible`) |
| `rotation_constrained` (tripod) | `in_hand.rotate` about the constrained axis | the rotation is resisted by form, not free |
| `support` / `balance_held` | free `transport.*`; release-by-opening | a balanced object is not freely transportable and cannot be open-released |

The table is the conformance complement of the `01` DOF-admissibility rule: `01` states the rule per primitive (`rotation_inadmissible`, `translation_inadmissible`, …); `05` enumerates it as a successor table the suite checks across a composed plan.

### Conformance obligations (stability and composition)

- **STB1 — non-degenerate tripod.** A `grasp.precision_tripod` is confirmed only when its three contacts form a non-collinear triangle above the minimum-area threshold; a near-collinear triple is not reported as a successful tripod.
- **STB2 — support safe state.** A `support` / `balance_held` grasp's abort / safe response is a controlled lowering to the nearest surface minimizing fall height, never an open-release.
- **STB3 — stability-class composition.** The suite enforces the stability-class → permitted-successor table — `surface_bound` forbids free transport; a `form_held` / `rotation_constrained` DOF forbids the corresponding `in_hand` operation; `support` closure forbids free transport and open-release — as a static validation atop the `01` lifecycle transition table.

### Deferred and referenced

- **`StabilityMetadata`** (`closure`, `secured_dof`, `flags`), the **lifecycle transition table**, the **DOF-admissibility rule**, and the **supported-state predicate** these checks read — `01-skill-isa.md` § Grasp state model and § World-state model.
- **The composed-plan algebra** (where the successor table is evaluated) — `01-skill-isa.md` § Composition validity.

## Reversibility and irreversible-operation safety

Primitives differ in how undoable their effect is, and the safety-conformance weight a primitive carries **rises with irreversibility**: a `reach` that can simply be retracted needs little extra scrutiny, while a `force.cut` that severs a part permanently needs the strictest treatment. This unit positions primitives on a **reversibility spectrum** and defines the safety class for its strict end.

### The reversibility spectrum

| Class | Effect | Examples | Safety treatment |
|---|---|---|---|
| **Reversible** | undoable by a reverse operation | `reach.*` (retractable), most manipulation | the primitive's own envelope class suffices |
| **Semi-reversible (persistent)** | persists, but a reverse operation can undo it | `force.snap_engage` (bistable; `snap_disengage` reverses) | engagement-confirmation + a documented reverse path |
| **Irreversible** | cannot be undone by any operation | `force.cut` (a cut is permanent) | the irreversible-operation safety class (below) |

The spectrum is open at both ends for extensions (`06`): a `weld` or an `adhesive` bond slots into **irreversible**; a `snap_disengage` is the **reverse** that makes snap semi-reversible rather than irreversible.

### Semi-reversible persistent state change

`force.snap_engage` produces a **bistable engagement** that persists after the operation completes — unlike a grasp (held only while the effector holds), the engagement stays without continued effort. This persistence is *intended*, not a continuity break: it is confirmed via engagement-confirmation (`confirm_held`) and carries a documented **reverse-operation path** (`snap_disengage`) so the persistent state can be undone. The suite verifies the engagement was confirmed (not merely commanded) and that a reverse path exists — the discriminator from a truly irreversible operation, which has none.

### The irreversible-operation safety class

`force.cut` is the first **irreversible** primitive: a cut cannot be undone. Irreversible operations get a dedicated safety-conformance treatment, distinct from the envelope classes, because there is no recovery from an error:

- **Strictly bounded action path.** The operation cannot exceed a pre-declared path (a `cut_path`); the bench verifies the realized action stayed within it.
- **Pre-execution confirmation of the path and the material beyond it.** Before any irreversible action, the path *and what lies beyond the cut plane* are confirmed — so the operation does not sever something behind the intended target.
- **Precise partial-state reporting on failure.** If an irreversible operation is interrupted, it reports exactly how far it progressed (a partially-cut state), never a binary success / failure — downstream recovery and the audit loop need the precise irreversible state.

This sets the precedent for every future irreversible extension (`weld`, `adhesive`): each must declare a bounded action path, confirm before acting, and report precise partial state on failure. A hazardous tool wielded by an irreversible operation additionally requires the `tool_safety` regime (§ Hazardous-operation benches, next unit).

### Conformance obligations (reversibility)

- **REV1 — reversibility classification.** Every primitive is positioned on the reversibility spectrum (reversible / semi-reversible-persistent / irreversible); its safety-conformance weight rises with irreversibility.
- **REV2 — semi-reversible confirmation.** A semi-reversible persistent change (`force.snap_engage`) is confirmed via engagement-confirmation (`confirm_held`) and carries a documented reverse-operation path (`snap_disengage`); the persistence is intended, distinct from a continuity break.
- **REV3 — irreversible-operation safety class.** An irreversible operation (`force.cut`) runs only under a strictly bounded action path, pre-execution confirmation of the path and the material beyond it, and precise partial / irreversible-state reporting on failure; the treatment is the precedent for future irreversible extensions.

### Deferred and referenced

- **The engagement-confirmation parameter (`confirm_held`)** and the `snap_engage` / `snap_disengage` / `cut` primitive definitions — `01-skill-isa.md`.
- **The `tool_safety` regime** an irreversible hazardous-tool operation also requires — § Hazardous-operation benches (next unit) and `03` § Safety capabilities.
- **Future irreversible extensions** (`weld`, `adhesive`) and the reverse `snap_disengage` — `06-extension-registry.md`.

## Hazardous-operation conformance benches

Two operations interact with the world in ways that can cause harm — a hazardous *tool* (`force.cut` wields a cutter) and a *human* (`place.hand_to` hands an object to a person). Their capability *declarations* are owned by `03` § Safety capabilities (`tool_safety`, `human_collaboration_safety`); this unit defines the instrumented **test benches** that verify the declared safety actually holds. Both are physical, instrumented benches (test class 4).

### Tool-safety bench

A primitive that wields a hazardous tool does not run without the declared `tool_safety` capability — the gate is enforced at validation (`03`); the bench here verifies that the declared safety holds in execution, with **instrumented test material**:

- **Shear measurement** — the cut is a controlled shear within the declared bound, not an uncontrolled tear or crush.
- **Separation detection** — the part actually separated (the cut completed), distinguishing a true cut from a stalled or partial one.
- **Hard-inclusion injection** — an unexpected hard inclusion is introduced in the cut path; the operation must **arrest safely** (no uncontrolled follow-through, no tool shatter), exercising the irreversible-operation safety class's bounded-path and partial-state reporting (§ Reversibility and irreversible-operation safety) under an adverse condition.

### Human-collaboration / handover bench

`place.hand_to` releases an object to a human and is safety-critical (ISO 10218 / 13482 context, tied to the L4 certification loop). The bench uses an **instrumented dummy-hand recipient** to verify:

- **Release only after weight transfer** — the object is not released until the recipient is measurably bearing it (no premature drop into an unready hand);
- **Exchanged force ≤ `max_interaction_force`** — the force exchanged with the human stays under the declared limit (never crush the hand);
- **Compliant yielding to a human tug** — the embodiment yields compliantly when the human pulls, rather than resisting rigidly (the force semantics tie to `01` / `06`).

The capability declaration (`human_collaboration_safety`: `standard` + `max_interaction_force` / `weight_transfer_threshold`) is `03`'s; the dummy-hand bench that verifies it is here.

### Conformance obligations (hazardous operations)

- **HAZ1 — hazardous-tool gate.** A primitive wielding a hazardous tool (`force.cut`) does not run without the declared `tool_safety` capability (`03`); the gate is enforced at validation, and the tool-safety bench verifies the declared safety in execution.
- **HAZ2 — tool-safety bench.** The bench uses instrumented test material to verify controlled shear (shear measurement), actual separation (separation detection), and safe handling of an unexpected hard inclusion (hard-inclusion injection → arrest, no uncontrolled follow-through).
- **HAZ3 — human-handover bench.** `place.hand_to` is verified with an instrumented dummy-hand recipient: release only after weight transfer, exchanged force ≤ `max_interaction_force`, and compliant yielding to a human tug (ISO 10218 / 13482, L4 certification).

### Deferred and referenced

- **The capability declarations** (`tool_safety`: `hazard_class` + `standard`; `human_collaboration_safety`: `standard` + `max_interaction_force` / `weight_transfer_threshold`) and their validation gates — `03-driver-interface.md` § Safety capabilities.
- **The ISO 10218 / 13482 flow-through** to the Tier-3 notified-body regime — § Three-tier conformance regime (and the strategy documents).
- **The compliant-yield force semantics** a human tug invokes — `01-skill-isa.md` / `06-extension-registry.md`.

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
