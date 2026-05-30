# TactileManifold — Specification

> **Status**: foundation specified (2026-05-31) — the feature-field model, the field-set discipline, the feature taxonomy, the contact-sensor descriptor, the site model, and the `TactileTarget` type. Remaining before freeze: the per-feature semantic units (proxy degradation; slip; force-events; deformation; freed-part safety; sensing-scope contracts), tracked in `02-translation-layer.md` § Open issues (Owned by `04`). The formal mathematical specification is in the white paper Appendix B; this chapter is the implementation-facing version.

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

## Deferred to other chapters

The manifold owns the feature *definitions*. Coupled concerns are owned elsewhere and referenced, not redefined:

- **The `tactile_sensing` capability declaration** (boolean), the `tactile` frame / role, and the URDF / MJCF `<rfl:capabilities>` binding — `03-driver-interface.md`.
- **The type slots** `TactileTarget`, `Measurement`, and `StopCondition` (with its `effort_drop` / `detent` event variants) — `01-skill-isa.md`; this chapter fixes how they are *sensed*, not their type-table entries.
- **How `retarget` encodes** a monitored feature or a `TactileTarget` into the canonical action — `02-translation-layer.md`.
- **The conformance test classes, the epsilon floor**, and the verification mechanics for every TM obligation — `05-conformance.md`.

## Open issues

The remaining items of the `04` group in `02-translation-layer.md` § Open issues, each a unit still to be written on this foundation:

- Graceful-degradation **proxy** for tactile-absent force-closure confirmation
- **Slip** feature semantics (intended-migration vs. loss-of-control; closed-loop slip sensing)
- **Force-event** semantics (breakaway / detent; detent vs. bottoming-out) and the multi-rate **temporal-alignment** model (the chapter's second skeleton open issue)
- **Deformation** semantics (bend / crease vs. crush)
- **Freed-part** safety handling at constraint-release
- The two **sensing-scope contracts** (measurement non-disturbance; observation-capturability)
