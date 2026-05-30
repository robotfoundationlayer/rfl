# TactileManifold — Specification

> **Status**: foundation + degradation + slip + force-events specified (2026-05-31) — the feature-field model, the field-set discipline, the feature taxonomy, the contact-sensor descriptor, the site model, the `TactileTarget` type, the graceful-degradation proxy discipline, the slip discrimination model, and the breakaway / detent force-event detection with its temporal-alignment timebase. Remaining before freeze: the per-feature semantic units (deformation; freed-part safety; sensing-scope contracts), tracked in `02-translation-layer.md` § Open issues (Owned by `04`). The formal mathematical specification is in the white paper Appendix B; this chapter is the implementation-facing version.

## Scope

TactileManifold is the formal abstraction RFL uses to represent contact feedback across heterogeneous tactile sensors — the **feedback counterpart** of the Skill ISA's embodiment-agnostic action vocabulary. Where the Skill ISA lets a primitive *command* a contact pattern without naming an actuator, the TactileManifold lets that same primitive *sense* the contact without naming a sensor.

This chapter owns:

1. The feature-field model and the closed-core feature **taxonomy** (this unit)
2. The contact-sensor **descriptor** `embodiment.tactile[tactile_frame]` — the contact field set deferred from `03-driver-interface.md` § Sensor descriptor
3. The **`TactileTarget`** type referenced from `01-skill-isa.md` § Tactile + grasp reference types
4. The **graceful-degradation proxy** discipline that backs every `tactile_sensing`-preferred primitive (Principle 5)
5. The per-feature **semantics** — slip, force-events (breakaway / detent), deformation (bend / crease), freed-part handling — and the two **sensing-scope contracts** (measurement non-disturbance, observation-capturability)

Items 4–5 are specified in the units that follow; this unit fixes 1–3 and the structure the rest instantiate.

The four representative sensor classes the abstraction spans:

- Discrete force sensors at fingertips (**FT-class**)
- Distributed force / pressure arrays (**XELA-class**)
- Visuotactile sensors (**GelSight / DIGIT-class**)
- Pneumatic pressure sensors (McKibben / bellows-style hands)

These are illustrative, not normative (Principle 4): the abstraction admits any sensor that maps onto the feature taxonomy, and the registry path (§ Field-set discipline) admits modalities not yet enumerated.

## The manifold as a feature field

A TactileManifold is **not a sensor model**; it is a mapping from heterogeneous physical sensors onto a common, embodiment-agnostic **feature field**. A Skill ISA primitive expresses its contact criterion purely in features — `normal_force ≥ ε at ≥ 2 antipodal sites` — and never references a taxel index, a visuotactile image region, or a pneumatic pressure line. Per-sensor-class **adapters** (in `schemas/tactile-manifold/`, referenced from the embodiment descriptor) translate raw sensor output into manifold features; the primitive sees only the features.

A manifold instance is the triple introduced in the skeleton, now sharpened:

```
TactileManifold := (
    sites:      Set<Site>,                  # abstract contact-capable regions (§ Site model)
    features:   Site -> Set<FeatureType>,   # which features each site reports (§ Feature taxonomy)
    resolution: Site -> Resolution,         # spatial / temporal / value resolution per site
)
```

`FeatureType` ranges over the closed-core vocabulary (§ Feature taxonomy); `Site` and `Resolution` are defined below. The adapter populates this triple from a concrete sensor; the embodiment **declares** it through the contact-sensor descriptor (§ The contact-sensor descriptor).

**Principle 1 (embodiment-agnostic).** Two embodiments with structurally different sensors — a parallel-jaw gripper with two fingertip load cells, and a multi-fingered hand with a 200-taxel dense skin — present the *same* feature vocabulary to a primitive. They differ only in which features they report, at how many sites, and at what resolution. A primitive's contact criterion is reviewable by someone whose only sensor is a single pneumatic pressure line, expressible in their world without anthropomorphic intuition.

| Sensor class | Typical sites | Native features (illustrative) | Typically absent |
|---|---|---|---|
| FT-class (discrete) | few (≈ 1 per fingertip) | `contact`, `normal_force` | `shear`, `pressure` distribution, `contact_area` |
| XELA-class (array) | many (dense grid) | `contact`, `normal_force`, `pressure`, `contact_centroid`, `load_distribution` | high-rate `shear` (sensor-dependent) |
| Visuotactile (GelSight / DIGIT) | dense (image-derived) | `contact`, `pressure`, `shear`, `contact_area`, `deformation_rate`, `slip` | absolute `normal_force` calibration (geometry-derived) |
| Pneumatic (bellows) | few (per chamber) | `contact`, aggregate `normal_force` (pressure-proxied) | site-resolved `shear`, `contact_centroid` |

The "typically absent" column is the source of graceful degradation (§ the proxy unit): a primitive whose `TactileTarget` references an absent feature degrades to a proxy rather than failing.

## Field-set discipline — closed core, registry extension

**Decision (resolves the chapter's field-set open issue).** The feature vocabulary is a **closed core enumeration in v1.0**, extended only through the namespaced registry of `06-extension-registry.md`.

The two constitutional principles pull in opposite directions, and the spec resolves the tension the same way it did for the `StopCondition` family (`01`) and the capability key space (`03`):

- **Principle 3 (verifiable)** requires a *closed* core: `retarget` must be able to decide whether a `TactileTarget` is checkable against a given embodiment, and the conformance suite must freeze fixtures over a fixed feature set. An open-ended vocabulary makes both undecidable.
- **Principle 5 (forward-compatible)** requires an *open* path: a tactile modality introduced after v1.0 (e.g. a thermal or a vibrotactile spectral feature) must enter without a breaking change.

A core feature is referenced by primitive preconditions and `TactileTarget` defaults directly and carries no prefix. A registered extension feature carries a namespace prefix (`<vendor>.<feature>`) and is admissible in a `TactileTarget` only when both the embodiment's descriptor and the target declare it. The closed core is exactly the set enumerated in § Feature taxonomy; no other unprefixed feature name is valid in v1.x.

## Feature taxonomy

The closed-core vocabulary, grouped by kind. This unit **enumerates** the vocabulary and fixes each feature's type and unit; the **discrimination semantics** of the dynamic features (how a slip is told from intended migration, a detent from bottoming-out) are specified in the units noted, so the vocabulary is complete and stable before any primitive's feedback logic is written.

Quantity types follow the Skill ISA's conventions (`01` type system): `Force`, `Pose6D`, `Direction`, and `UncertaintyBound` are `01`'s; the manifold adds the contact-specific quantities (`Pressure` [Pa], `Area` [m²], `Probability`, the rate types) as simple SI-typed scalars / vectors. `SlipState` and `BendState` are manifold-local composite types, fully defined in their owning unit.

**Contact-state features** (instantaneous, per site):

| Feature | Type | Unit | Meaning |
|---|---|---|---|
| `contact` | `Probability` | — | contact present at the site (0–1; a hard switch reports {0, 1}) |
| `normal_force` | `Force` | N | force normal to the contact surface at the site |
| `shear` | `Vector2` | N | in-plane tangential force (2-vector in the site's contact tangent plane) |
| `pressure` | `Pressure` | Pa | distributed normal stress (dense sensors; integrates to `normal_force` over `contact_area`) |
| `contact_area` | `Area` | m² | area in contact at the site |

**Derived-spatial features** (over a site set):

| Feature | Type | Unit | Meaning | Consumed by |
|---|---|---|---|---|
| `contact_centroid` | `Pose6D` position | m | centroid of the contact-pressure distribution | grasp recentre / `grasp.adjust` |
| `load_distribution` | distribution summary | — | how borne load spreads over a support area; yields CoM-over-area | `grasp.platform` load / CoM confirmation |

**Dynamic / event features** (temporal):

| Feature | Type | Unit | Meaning | Semantics owned by |
|---|---|---|---|---|
| `slip` | `SlipState` | — | incipient / gross relative motion at the contact, with a direction | § Slip feature family |
| `force_derivative` | `Force/Time` | N/s | signed rate of change of `normal_force` | § Force-event feature family |
| `deformation_rate` | `Strain/Time` | 1/s | rate of object deformation under contact (crush indicator) | § Deformation feature |
| `bending` | `BendState` | — | thin-object flexure about a crease line, distinct from compression | § Deformation feature |

**Per-feature uncertainty.** Every feature reading is a value paired with an `UncertaintyBound` (the bound type the Skill ISA attaches to targets, `01` § type system): a reading is `(value, uncertainty)`. A threshold comparison in a `TactileTarget` therefore yields a three-valued outcome — confidently met, confidently unmet, or `indeterminate` — feeding the `Verdict` confidence model the algebra defines (`01` § Predicates, verdicts, and three-valued control flow). This resolves the *representation* half of the chapter's calibration-uncertainty open issue; **propagation into the conformance epsilon** (bit-identical vs. semantic-equivalence with tolerance) is owned by `05-conformance.md`.

## The contact-sensor descriptor

`03-driver-interface.md` § Sensor descriptor defines `embodiment.sensors[sensor_frame]` for **non-contact** (visual / range) sensors and explicitly defers the **contact** field set here (`03` § Scope boundary — non-contact vs. contact sensors). This section defines that field set. It is the contact counterpart of the non-contact sensor map, sharing only the **frame-keyed-map structure**, not the fields.

`embodiment.tactile` is a map keyed by frame name, with one entry per control frame carrying the `tactile` role (`03` § Role frames and defaults). An embodiment declaring the `tactile_sensing` auxiliary capability (`03` § Auxiliary capabilities) MUST populate it; an embodiment that does not declare `tactile_sensing` has no entry, and every `tactile_sensing`-preferred primitive degrades to its proxy (§ the proxy unit).

| Field | Type | Meaning |
|---|---|---|
| `sites` | `SiteLayout` | discrete site count + per-site pose, or a dense grid (extent + spacing), in the tactile frame (§ Site model) |
| `features` | `Set<FeatureType>` | the subset of the closed-core vocabulary this frame reports (extension features carry their namespace prefix) |
| `resolution` | `Resolution` | `(spatial: Length, temporal: Frequency, value: per-feature least-count)` — the triple's `bits_per_feature` made physical |

**Scope boundary.** This descriptor declares feature *availability and resolution*. It does not declare *that* the embodiment has tactile sensing (the boolean `tactile_sensing` capability — `03`), the tactile *frame* or *role* (`03` frame model), or `max_probe_force` (target-derived — a fraction below the *target's* disturb threshold, per `03` § Sense capabilities and the sensing-scope unit). It does not define how a feature is *computed* from raw sensor data — that is the adapter's (`schemas/tactile-manifold/`).

## Site model

A **site** is an embodiment-agnostic contact-capable region — not a taxel. It carries a pose (in a tactile frame) and participates in **relational roles** that a `TactileTarget`'s site-quantifiers reference:

```
Site := { pose: Pose6D, role_tags: subset of {antipodal, enclosure, support, probe} }
```

The quantifiers in a `TactileTarget` — `≥ 2 antipodal sites`, `≥ enclosure_completeness · N enclosure sites`, `3 non-collinear sites` — refer to these **abstract** sites and their relational roles, resolved per embodiment by the adapter. This is the Principle-1 crux of the chapter: the *count and arrangement semantics* are abstract and shared, while the physical layout (two load cells vs. a 200-taxel skin) is the adapter's concern. A dense-skin embodiment resolves `≥ 2 antipodal sites` by clustering its taxels into two opposing contact patches; a two-fingered gripper resolves it by its two fingertips. Both satisfy the identical target.

`SiteLayout` in the descriptor is the embodiment's declaration of its sites and the relational roles they can serve; `auto` site resolution — the planner choosing which sites a quantifier binds to — MUST be deterministic given identical inputs (binding for `retarget` determinism).

## The `TactileTarget` type

`01-skill-isa.md` § Tactile + grasp reference types reserves `TactileTarget` as "a contact criterion expressed in TactileManifold terms (sites, feature thresholds)" and points here for its definition. A `TactileTarget` is a **predicate over the feature field** — a conjunction of clauses, each a feature comparison quantified over a site set:

```
TactileTarget := all_of( Set<TactileClause> )

TactileClause  := ( feature:    FeatureType,
                    comparator: {≥, ≤, ==, within},
                    threshold:  Value,            # typed to the feature's quantity
                    quantifier: SiteQuantifier )

SiteQuantifier := at_least(n, role) | all(role) | exactly(n, role)
```

A clause evaluates against the per-site feature readings; with per-feature uncertainty, the conjunction yields a three-valued `Verdict` (`01` algebra), so a `TactileTarget` is *confidently satisfied*, *confidently unsatisfied*, or *indeterminate* — never silently coerced.

The `auto` spellings already carried by the Skill ISA grasp primitives are the canonical defaults this type produces:

| Primitive | `tactile_target = auto` resolves to |
|---|---|
| `grasp.pinch` | `normal_force ≥ ε` at `at_least(2, antipodal)` |
| `grasp.power` | `normal_force ≥ ε` at `at_least(⌈enclosure_completeness · N⌉, enclosure)` |
| `grasp.tripod` | `normal_force ≥ ε` at `at_least(3, antipodal)` ∧ non-collinear (§ Site model) |
| `grasp.lateral` | `normal_force ≥ ε` across the clamp dimension |
| `grasp.platform` | `load_distribution` borne with CoM inside the support polygon |
| `grasp.pin` | `contact == true` ∧ confirmed reaction `normal_force` from the surface |

(The full per-primitive defaults live in `01`; this table records the manifold-term expansion, not new behaviour.)

**Satisfiability and the hand-off to degradation.** A `TactileTarget` is **satisfiable** on an embodiment iff every clause's `feature` is in the addressed frame's declared `features` and the `threshold` lies within the declared `resolution`'s value least-count. An unsatisfiable target does not fail the primitive: it degrades to the force / position **proxy** specified in § the proxy unit (Principle 5). The satisfiability check is a deterministic validation-phase test — parallel to `03`'s `capability_absent` gate, but resolving to *degrade*, not *reject*.

## Conformance obligations (manifold foundation)

- **TM1c — vocabulary closure.** Every `feature` named in a `TactileTarget` or a primitive precondition is either a closed-core feature (§ Feature taxonomy, unprefixed) or a registered extension feature (namespace-prefixed, `06`). No other name validates.
- **TM2c — descriptor completeness.** Every embodiment declaring `tactile_sensing = true` has an `embodiment.tactile` entry, per `tactile`-role frame, carrying `sites`, `features`, and `resolution`.
- **TM3c — uncertainty presence.** Every feature reading the adapter produces carries an `UncertaintyBound`; a threshold comparison missing an uncertainty is malformed.
- **TM4c — satisfiability determinism.** The satisfiability check (`TactileTarget` clauses vs. declared `features` / `resolution`) is deterministic and resolves each target to *satisfiable* or *degrade-to-proxy*, never to a nondeterministic outcome.

The conformance *test classes and regime* are owned by `05-conformance.md`; these obligations are the declarative requirements that suite checks.

## Graceful degradation and the force/position proxy

The Skill ISA marks `tactile_sensing` **preferred, never required** for every `grasp` mode (`01` § Category 2; `03` § Tactile sensing is preferred, never required). When the capability is undeclared — or when a `TactileTarget` clause references a feature the embodiment does not report (§ The `TactileTarget` type, satisfiability) — confirmation cannot use the manifold. Principle 5 requires the primitive to **still run**, confirming through a substitute, and never to either fail silently or report `success` without confirmation. This section defines that substitute once, at the manifold level, so it is not re-derived per primitive.

### The degradation invariant

A confirmation degrades along three honesty rules, in priority order:

1. **Never fake.** A degraded confirmation never reports `success` on an unconfirmed contact. The empty-close case (`no_contact_confirmation`, `01` grasp failure modes) MUST be caught by the proxy as surely as by the manifold.
2. **Never silently fail.** Absence of a feature degrades the confirmation to a lower-fidelity substitute and lowers the declared tier; it does not turn the primitive off — the capability stays declarable and passes its C1 test (`03` G4c).
3. **Always disclose.** A degraded confirmation marks itself proxy-derived in its `evidence` and declares the lower fidelity tier, so the certification and audit loops (L4 / L8) can trace that confirmation was degraded — parallel to the `momentary_release` audit-propagation requirement (`02` / `05`).

### The force/position proxy

The manifold confirms *"contact achieved at sites meeting feature thresholds."* The proxy substitutes a **force/position signature** built from two embodiment-internal quantities every actuated effector already has, with no contact sensor required:

- **Position convergence** — the commanded contact closed to the target's expected cross-section (the `grasp_width` derived from `target.geometry`), **not** past it. Closing *past* the cross-section to the effector's own minimum is the empty-close signature: the contact met nothing.
- **Force rise-and-hold** — the actuator effort rose to the commanded `force_budget` and **held** it over a `confirm_window` — the resistance of an object pushing back. Free-space closure shows no such sustained resistance.

```
proxy_confirm(target, force_budget) :=
      position_converged( grasp_width ≈ cross_section(target.geometry) ± tol )
    ∧ force_held( actuator_effort ≥ force_budget over confirm_window )
```

The conjunction is the proxy for *"object present ∧ force closure."* Either half alone is insufficient: position alone cannot tell an object from a jam; force alone cannot tell a grasped object from a collision. This is the force/position proxy referenced throughout `01`'s grasp preconditions and named in `03` § Tactile sensing is preferred, never required.

### Proxy expansions of the grasp targets

Each grasp mode's `auto` `TactileTarget` (§ The `TactileTarget` type) has a deterministic proxy expansion. The manifold target and its proxy are the two confirmation paths a mode may take; which path runs is fixed at validation by the satisfiability check.

| Mode | Manifold target | Proxy expansion |
|---|---|---|
| `grasp.pinch` | `normal_force ≥ ε` at `at_least(2, antipodal)` | `grasp_width ≈ cross_section ∧ force_held` |
| `grasp.power` | `normal_force ≥ ε` at `at_least(⌈enclosure_completeness · N⌉, enclosure)` | enclosure `grasp_width ≈ cross_section ∧ force_held` (no per-site completeness) |
| `grasp.tripod` | `normal_force ≥ ε` at `at_least(3, antipodal)` ∧ non-collinear | `grasp_width ≈ cross_section ∧ force_held` (non-collinearity unverifiable — tier drop) |
| `grasp.lateral` | `normal_force ≥ ε` across clamp dim | `grasp_width ≈ thickness ∧ force_held` |
| `grasp.platform` | `load_distribution` borne, CoM in polygon | measured borne load `≈` target weight only (CoM-in-polygon unverifiable; `01` "proxy via measured load only") |
| `grasp.pin` | `contact == true` ∧ reaction `normal_force` | reaction `force_held` against the surface |

The proxy column is *coarser* than the manifold column: it loses per-site distribution (power's completeness), arrangement (tripod's non-collinearity), and CoM placement (platform's polygon). That loss is precisely the fidelity-tier drop disclosed under § Fidelity tier.

### Proxy-degradable vs. proxy-irreducible confirmations

Not every confirmation has a force/position proxy. The manifold partitions confirmations:

- **Proxy-degradable** — force-closure confirmation (all grasp modes above) and `sense.probe`'s contact / location / normal (a force-threshold touch needs only force sensing). These always have a proxy; a non-tactile embodiment runs them at a lower tier.
- **Proxy-irreducible** — a confirmation that *intrinsically* needs a feature with no position/force surrogate. **Slip** (incipient relative motion needs `shear`; position cannot see motion that has not yet displaced the object) and **deformation_rate** (crush onset needs distributed deformation) are irreducible.

For an irreducible guard the degradation rule is **tiered, reactive-only — never a hard reject**:

- The primitive still runs (consistent with "tactile preferred, never required"); the irreducible guard degrades from *preemptive* to *reactive-only*. Without `slip`, `grasp.adjust`'s slip-recovery and the grasp envelopes' preemptive slip response cannot fire on incipient slip; they fall back to reacting to *gross* slip detected as an object-pose change (a position-observable event).
- The result declares the lower tier and discloses the unavailable guard in `evidence` (§ Fidelity tier).
- **Risk-acceptance lives at the certification layer, not here.** RFL does not refuse a primitive for lack of a tactile guard. The genuinely unsafe cases are gated by their *own orthogonal* hard capabilities — `tool_safety` for `force.cut`, `human_collaboration_safety` for `place.hand_to` (`03` § Safety capabilities) — independent of `tactile_sensing`. The tactile guard degrades uniformly to a tier; whether that tier is acceptable for a deployment (a fragile heavy carry without slip sensing) is the L4 decision, not an RFL-layer reject.

This is what makes `in_hand.slide`'s "closed-loop slip sensing **or** a force / position proxy" (`01`) precise: slide's *displacement tracking* is proxy-degradable (position observes the slide), but its *drop-slip safety guard* is irreducible and degrades to reactive-only (gross escape detected as an orthogonal-DOF pose change, not incipient shear).

### Fidelity tier and audit honesty

A confirmation path carries a **fidelity tier**: `manifold` (full feature confirmation) > `proxy` (force/position substitute) > `proxy_reactive` (an irreducible guard in reactive-only fallback). The tier is recorded in the result's `evidence` (`01` § Predicates, verdicts, and three-valued control flow) and is the value an embodiment may claim through `03`'s reserved per-capability `tier` attribute. The mapping from a claimed tier to a conformance class and badge is owned by `05-conformance.md`; this chapter fixes only what each tier *means* in confirmation terms.

A `proxy`-tier `Verdict` is still a confident `true` / `false` when its proxy signature is decisive (an object held at budget over the confirm window is a confident hold); it is `indeterminate` when the proxy cannot decide (a force rise that neither converged in position nor held). The three-valued honesty of `01`'s verdict model is preserved across degradation — the proxy lowers *fidelity*, never the *honesty* of the confidence report.

#### Conformance obligations (degradation)

- **TM5c — empty-close detection under proxy.** With `tactile_sensing` undeclared, a grasp commanded on no object returns `no_contact_confirmation` via the proxy (position closed past the cross-section), never a false `success` (`01` grasp C2, generalized).
- **TM6c — proxy tier passes C1.** Each asserted grasp mode passes its C1 nominal-grasp-and-hold test via the proxy with `tactile_sensing` undeclared (the realization of `03` G4c — tactile independence).
- **TM7c — disclosure.** A proxy- or reactive-tier confirmation marks its tier and the unavailable guards in `evidence`; a degraded confirmation reported at `manifold` tier is malformed.
- **TM8c — no tactile hard-reject.** No `grasp` primitive returns `capability_absent` for absent `tactile_sensing` alone; hard rejection comes only from a primitive's own orthogonal safety capability (`tool_safety` / `human_collaboration_safety`), never from the tactile manifold.

#### Deferred to other chapters

- **The tier → conformance-class / badge mapping** — `05-conformance.md` (this chapter fixes tier *meaning*, not its certification weight).
- **The canonical-action encoding of proxy monitoring** — how `retarget` emits the position-convergence and force-hold monitors into the canonical action — `02-translation-layer.md`.
- **`tool_safety` / `human_collaboration_safety`** as the orthogonal hard gates — `03` § Safety capabilities; their conformance test benches — `05`.

## Slip — controlled migration vs. loss of control

`slip` is the manifold's contact-motion feature (§ Feature taxonomy). It is the hardest feedback to use correctly, because several primitives create relative contact motion *by design*: `in_hand.roll` migrates the contact point as it rolls, and `in_hand.slide` permits slip along one DOF. A raw slip reading therefore cannot be a blanket abort trigger — it would abort the very primitives whose intent is motion. This section defines the `slip` feature and the **intended-slip model** that separates controlled migration from loss of control.

### The `SlipState` feature

The manifold-local `SlipState` type promised in § Feature taxonomy:

```
SlipState := {
  stage:     {none, incipient, gross},   # partial-slip onset vs. full relative sliding
  direction: Direction | None,           # slip-velocity direction at the contact, in the tactile frame
  rate:      Velocity,                    # slip-speed magnitude (0 when stage = none)
}
```

- **`incipient`** is partial slip — the stick-slip onset a contact shows *before* the object visibly displaces. Detecting it requires `shear` (the tangential-force feature); it is the early warning a tactile sensor gives and a force/position proxy cannot (§ Proxy-degradable vs. proxy-irreducible confirmations).
- **`gross`** is full relative sliding — the object is moving against the effector. Because the object's pose changes, gross slip *is* position-observable, which is what lets the reactive-only proxy fallback (§ Graceful degradation) catch gross slip but not incipient slip.
- **`direction`** is the discriminator the directional primitives turn on.

### The intended-slip model

A primitive that commands relative motion supplies an **intended-slip model** — the slip the command is expected to produce. The manifold classifies an observed `SlipState` against it:

```
classify(observed, intended_model) :=
  | controlled       if observed is consistent with intended_model
  | loss_of_control  otherwise   (a securing DOF slips, or magnitude / direction exceeds the model)
```

A primitive with **no** commanded motion (every `grasp` mode, `transport`) supplies the **empty** intended-slip model, under which *any* slip is `loss_of_control` — the blanket-abort base case that `01`'s grasp `slip` failure mode and `slip_response` already assume. The two motion primitives supply non-empty models, defined next. The classification is the single construct; roll and slide are its two instances, so the discrimination is not re-derived per primitive.

### Rolling discriminator — `in_hand.roll`

`in_hand.roll` rolls the object about `roll_axis` over its declared rolling surface (the `GeometryRef` rolling aspect, `01`). The intended-slip model is the **rolling kinematics**: at the commanded angular velocity ω, rolling contact migrates the contact point at the tangential rate the rolling radius predicts, along the roll tangent. An observed `SlipState` is `controlled` iff its direction lies along the roll tangent and its rate matches the kinematic migration rate within the primitive's `orientation_tolerance`-derived band. It is `loss_of_control` (the `gross_slip` failure, `01`) when:

- the rate **exceeds** the kinematic migration (the object skids faster than it rolls), or
- the direction has a component **off** the roll tangent (a lateral slide or along-axis creep the rolling model does not predict), or
- slip **persists while ω = 0** (the object is moving when it should be still).

The "rolling degenerates toward line / point contact" envelope concern (`01` safety envelope) is the manifold concurrently checking that the migrating contact stays a *securing* contact — `normal_force ≥ min_holding_force` at the moving contact — so a roll that is kinematically correct but losing normal force is still arrested.

### Sliding discriminator — `in_hand.slide`

`in_hand.slide` permits slip along `slide_direction` (a `friction_held` translational DOF) while the orthogonal DOF keep securing (`01`). The intended-slip model is **directional**: decompose the observed slip velocity into its `slide_direction` component and its orthogonal component.

- the `slide_direction` component is **intended feed** — it advances the slide toward the stop condition;
- an orthogonal component beyond a tolerance band is **drop-slip** — a securing DOF is slipping and the object is escaping the grasp (the `drop_slip` failure, `01`).

Direction, not magnitude, is the discriminator: a fast slide along `slide_direction` is fine, while a slow slip orthogonal to it is an abort. This is the precise content of `01`'s "distinguish intended slip (along `slide_direction`) from unintended drop-slip (object escaping)."

### Closed-loop slip sensing and its degradation

`in_hand.slide` names **closed-loop slip sensing** a requirement (`01`): the `slip` feature must be available *and* fed back within the control loop's temporal resolution, both to arrest at the stop condition and to detect drop-slip as it begins. Per § Proxy-degradable vs. proxy-irreducible confirmations, slip's *incipient* stage has no force/position surrogate, so when slip sensing is absent the degradation is the irreducible rule:

- the slide's **displacement tracking** still runs — position observes the advance, so the primitive is not hard-rejected (proxy-degradable);
- **drop-slip detection** degrades to **reactive-only** — only *gross* escape is caught, as an orthogonal-DOF object-pose change, not incipient orthogonal slip;
- the result discloses the `proxy_reactive` tier and the unavailable incipient-slip guard (§ Fidelity tier and audit honesty).

The same degradation governs `grasp.adjust(reason = slip_recovery)` (`01`): preemptive incipient-slip recovery needs the feature; without it, recovery can only react to gross slip.

#### Conformance obligations (slip)

- **TM9c — slip staging.** The manifold reports `SlipState` with `stage ∈ {none, incipient, gross}`, a `direction`, and a `rate`; the `incipient` stage is reported only from a shear-bearing feature, never inferred from position.
- **TM10c — rolling consistency.** During `in_hand.roll`, slip consistent with the rolling kinematics at the commanded ω is classified `controlled`; slip that skids beyond the migration rate, runs off the roll tangent, or persists at ω = 0 is classified `gross_slip`.
- **TM11c — slide directionality.** During `in_hand.slide`, the slip component along `slide_direction` is intended feed; an orthogonal-DOF slip component beyond the tolerance band is `drop_slip`. The discrimination is by direction, not by rate.
- **TM12c — closed-loop degradation.** With slip sensing absent, `in_hand.slide` still tracks displacement (position proxy) but its drop-slip guard degrades to reactive-only gross-escape detection, reported at `proxy_reactive` tier.

#### Deferred to other chapters

- **The rolling-surface geometry** (`GeometryRef` rolling aspect) and the DOF-admissibility that gates which DOF a primitive may move — `01-skill-isa.md`.
- **The canonical-action encoding** of the slip monitor and the intended-slip model — `02-translation-layer.md`.
- **The continuity-verification test classes** (gaiting, make-before-break, the slide / roll C-tests) that consume the slip classification — `05-conformance.md`.

## Force events — breakaway and detent

The `StopCondition` family (`01`) carries two *event* variants — `effort_drop` and `detent` — and fixes only their event class, deferring their physical detection here (`01`: "the type fixes the event class; the manifold fixes how it is sensed"). Both are **temporal signatures** over the resisting-effort signal (`Force`, tension, or `Torque` — the `Effort` quantity of the family), computed from the `force_derivative` feature (§ Feature taxonomy) over a short history window. This section defines the two signatures and the timebase that makes their detection deterministic.

### The `ForceEvent` feature

```
ForceEvent := {
  kind:      {breakaway, detent},
  at:        Timestamp,        # the aligned event time (§ Temporal alignment across multi-rate features)
  magnitude: Effort,           # breakaway: the drop depth; detent: the peak height above baseline
}
```

A `ForceEvent` is the manifold's resolution of a `StopCondition` event variant: `effort_drop` resolves to a `breakaway` event, `detent` to a `detent` event. The detection runs over the effort trajectory, reusing the `force_derivative` feature rather than minting a new raw quantity.

### Breakaway — the `effort_drop` event

A **breakaway** is a sudden, sustained collapse of resisting effort: `force_derivative` falls past a declared negative-rate threshold **and** the effort settles to a markedly lower level (a real loss of resistance, not derivative noise). It marks a `force.pull` extraction completing or a fastener pulling free, and recurs as the thread-disengagement / `torque_drop` of `force.unscrew` (`01` `PullStop` / `ScrewStop`).

Detection must fire on the **leading edge** — the derivative collapse — not after the effort has fully settled, because a force-controlled effort that loses its reaction would otherwise **lurch** into the freed space (a follow-through that can damage the part or the surroundings). Firing on the derivative is what lets the consumer arrest within tolerance. The freed part a breakaway exposes is handed to § Freed-part handling (next unit).

### Detent vs. bottoming-out — the `detent` event

A **detent** is a **rise-then-drop** signature *while motion continues*: the effort climbs to a local peak, then drops as the mechanism gives — the "click" of a button actuation (`force.press_button`) or a bistable snap (`force.snap_engage`) (`01` `ActuationSpec`). The critical discrimination is from **bottoming-out**:

| Signature | Effort profile | Verdict |
|---|---|---|
| detent (click / snap) | rise to a peak, **then drop**, motion continues | actuation succeeded |
| bottoming-out | monotone rise to a **plateau**, motion arrests | hard stop, **not** an actuation — never reported as success |

The drop *after* the peak is the whole signal: an effort that rises and stays high hit a hard stop, not a detent. Reporting a bottoming-out as a successful click is the false-positive this discrimination exists to prevent. (This is distinct from the seating / jam discrimination of `force.insert_fit`, which is `01`'s force-at-state rule — `effort_rise` *at* the expected depth is seated, without depth is jam; the detent signature is the click, the seating rule is the depth conjunction, and the two compose where a primitive needs both.)

### Temporal alignment across multi-rate features

A `ForceEvent` is computed over a time window, and its inputs (force, tactile, proprioceptive features) may sample at different rates, each declared in the descriptor's `resolution.temporal` (§ The contact-sensor descriptor). For the event's `at` timestamp to be deterministic — and `StopCondition` evaluation with it (`01`: "`StopCondition` evaluation is deterministic … event variants evaluate from declared thresholds / signatures per the `04` feature definitions") — the manifold fixes:

- **A common timebase.** Every feature reading is stamped on one monotonic manifold timebase; the descriptor's per-feature `resolution.temporal` declares each feature's rate against it.
- **The slowest-grid rule.** When a signature combines features at different rates, it is evaluated on the **slowest contributing feature's** sample grid — an event cannot be detected faster than its slowest required input — and its `at` is the boundary sample of the signature on that grid.
- **Detection determinism.** Given an identical timestamped feature trace, a `ForceEvent`'s `(kind, at, magnitude)` is identical. This is the manifold-side guarantee `retarget` determinism (`02`) and conformance fixtures (`05`) build on. The *realized* trace of a contact-dynamics primitive is **not** byte-reproducible (the determinism boundary owned by `02`); the **detection function over a given trace** is.

This resolves the chapter's multi-rate temporal-alignment open issue.

#### Conformance obligations (force events)

- **TM13c — breakaway signature.** `effort_drop` is detected as a `force_derivative` fall past a declared negative-rate threshold together with a settled lower effort level, reported with an aligned `at` timestamp; derivative noise without a settled drop is not a breakaway.
- **TM14c — detent vs. bottoming-out.** A `detent` is a peak-then-drop while motion continues; a monotone rise to a plateau (bottoming-out) is not a `detent` and is never reported as actuation success.
- **TM15c — leading-edge breakaway.** Breakaway fires on the derivative collapse (leading edge), early enough for the consumer to arrest within the primitive's tolerance, not after the effort fully settles.
- **TM16c — temporal-alignment determinism.** A `ForceEvent`'s `(kind, at, magnitude)` is a deterministic function of the timestamped feature trace, evaluated on the slowest contributing feature's grid; identical traces yield identical events.

#### Deferred to other chapters

- **The `StopCondition` type family**, its `effort_drop` / `detent` variants, and the seating / jam force-at-state (`all_of`) rule — `01-skill-isa.md`.
- **The canonical-action encoding** of the monitored event, and the realized-trace determinism boundary for contact-dynamics primitives — `02-translation-layer.md`.
- **The force-trajectory envelope test class** (interval-sampled effort / torque bounding) and event-fixture reproducibility — `05-conformance.md`.
- **Freed-part handling** at a breakaway that frees a part — § Freed-part handling (next unit).

## Deferred to other chapters

The manifold owns the feature *definitions*. Coupled concerns are owned elsewhere and referenced, not redefined:

- **The `tactile_sensing` capability declaration** (boolean), the `tactile` frame / role, and the URDF / MJCF `<rfl:capabilities>` binding — `03-driver-interface.md`.
- **The type slots** `TactileTarget`, `Measurement`, and `StopCondition` (with its `effort_drop` / `detent` event variants) — `01-skill-isa.md`; this chapter fixes how they are *sensed*, not their type-table entries.
- **How `retarget` encodes** a monitored feature or a `TactileTarget` into the canonical action — `02-translation-layer.md`.
- **The conformance test classes, the epsilon floor**, and the verification mechanics for every TM obligation — `05-conformance.md`.

## Open issues

The remaining items of the `04` group in `02-translation-layer.md` § Open issues, each a unit still to be written on this foundation:

- **Deformation** semantics (bend / crease vs. crush)
- **Freed-part** safety handling at constraint-release
- The two **sensing-scope contracts** (measurement non-disturbance; observation-capturability)
