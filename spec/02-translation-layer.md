# Translation Layer — Specification (skeleton)

> **Status**: pre-release skeleton. Full text targeted for v0.1 (2027 Q1).

## Scope

This chapter defines:

1. The canonical embodiment-agnostic action representation
2. The embodiment descriptor schema (`schemas/embodiment-descriptor.schema.json`)
3. The deterministic retargeting algorithm `retarget(skill, embodiment) -> canonical_actions`
4. The safety envelope semantics and how envelope constraints flow from Skill ISA into per-embodiment action parameters

## Canonical action representation (provisional)

A canonical action is a tuple:

```
CanonicalAction := (
    target_frame:   FrameRef,
    target_pose:    Pose6D,
    force_budget:   Option<Force[N]>,
    timing:         TimingHints,
    tactile_target: Option<TactileTarget>,
    safety_envelope: Envelope,
)
```

The full type definitions, including the `Envelope` algebra, will be specified before v0.1.

## Retargeting algorithm — determinism requirement

`retarget(skill, embodiment)` must be **deterministic**: identical inputs produce identical canonical-action sequences, byte-for-byte. The determinism requirement is binding because conformance testing (`05-conformance.md`) compares observed retargeting output against frozen fixtures.

## Resolved in the 2026-05-30 design pass

These issues surfaced during `reach` / `grasp` primitive design and are now addressed in the Skill ISA type system and the grasp state model (`01-skill-isa.md`). Detail lives in the spec body; retained here as a design-history trail.

- **`SurfaceTarget` / `ObjectTarget` / `FeatureRef` / `ScanRegion` types** → `01` § Skill ISA type system. Perception-derived target types are defined (fields + `UncertaintyBound`); the perception that produces them stays out of scope.
- **`ObjectTarget` center-of-mass** → `01` type system (`center_of_mass` field).
- **Grasp stability metadata in the result schema** (`closure`, `stable_directions`) → `01` § Grasp state model. Applied retroactively to `grasp.pinch` / `grasp.power`.
- **Stability-metadata generalization** (form-held vs friction-held vs balance-held DOF) → `01` `StabilityMetadata.secured_dof`.
- **Third closure type `support`** → `01` `StabilityMetadata.closure ∈ {force, form, support}`.
- **Extrinsic / surface-bound flags**, **`compliant`**, **`rotation_constrained`**, **`residual_mobility`** → `01` `StabilityMetadata.flags` / fields.
- **Active-grasp state model and `GraspRef`** → `01` § Grasp state model (`GraspState`, lifecycle FSM, transition table).
- **"Grasp core" abstraction structuring** → `01` Category 2 intro (core + contact-pattern delta documented; the full DRY rewrite of each primitive remains a pre-freeze formatting pass, not a semantic gap).

## Open issues

### Owned by `03-driver-interface.md` (embodiment descriptor + collision model)

- **Control-frame declaration**: `control_frames` set and `default_control_frame` field; retargeting maps `controlled_frame → embodiment native frame`. Non-anthropomorphic embodiments may expose several control frames.
- **`default_tool_axis`** (per control frame): used by `reach.approach` / `reach.align` and the `force` family.
- **`default_sensor_frame` + per-sensor `fov` / `bore_axis` / `max_sweep_rate`**: generalize the tactile-only descriptor to visual/range sensors (`reach.scan`).
- **`tracking_bandwidth`** in `embodiment.limits`: gates `reach.hover(track_target)` and its `track_lost` mode.
- **Collision-model target exclusion**: `reach.approach` / `reach.scan` apply `clearance` against the static model **excluding** the approached/scanned target; the model must designate a per-primitive exempt subset.
- **Swept-volume clearance query**: `reach.align` rotates in place; the model must answer swept-volume queries, not only point/corridor.
- **Per-grasp-mode descriptor fields**: `default_grasp_frame`, `grasp_envelope`, capability flags (`pinch_grasp`, `power_grasp`, `hook_grasp`, `tripod_grasp`, `lateral_grasp`, `platform_support`, `pin_grasp`, `envelope_grasp`), `default_support_frame`, support-contact-polygon geometry, and limits (`grip_force_max`, `v_grasp`, `payload_grasp_*`, `enclosure_span`, `hook_load_capacity`, `precision_object_size_max`, `lateral_grasp_max_thickness`, `payload_support`, enclosure range).
- **In-hand descriptor fields**: capability `in_hand_manipulation` (with sub-capabilities rotate/translate/…), `embodiment.limits.w_inhand_max` / `v_inhand_max`, and per-grasp-mode `inhand_rotation_range` / `inhand_translation_range` (`in_hand.rotate`, `in_hand.translate`).

### Owned by `05-conformance.md` (test classes)

- **Interval-invariant test class**: sustained primitives (`reach.hover`; later `force.wipe`/`force.scrub`) require interval sampling, distinct from endpoint checks.
- **Grasp-continuity envelope class**: held → held invariant "holding force never drops below `min_holding_force`", verified from the force trace (`grasp.adjust`).
- **Hold-test canonicalization + closure branching**: the hold test (calibrated sub-budget perturbation → retention) is the operational definition of closure; it branches on `closure ∈ {force, form, support}` (omnidirectional / `load_direction`-only / level-gentle).
- **Non-degenerate-triangle confirmation**: non-collinearity / minimum-triangle-area threshold for `grasp.precision_tripod`.
- **Support-grasp safe state**: "lower to the nearest surface, minimize fall height" for balanced (support) objects, which cannot be released by opening.
- **Composition validity by grasp stability class**: the lifecycle transition table (`01`) is the foundation; `05` needs the full "stability class → permitted successor primitive" constraint table (e.g. `surface_bound` forbids free transport).
- **Gaiting + make-before-break continuity verification**: finger gaiting (`in_hand.rotate`/`translate`) and grasp handover (`in_hand.regrasp`) both pass through intermediate contact sets; continuity holds iff at every instant the union of engaged contacts (old, new, or both) satisfies `min_holding_force`, and for regrasp the new grasp is confirmed *before* the old is released (make-before-break ordering). Define how the suite verifies "a securing contact set exists at all instants" and the make-before-break ordering from the force trace. The most general form of the grasp-continuity envelope class.
- **Controlled-under-actuation continuity mode**: `in_hand.pivot` intentionally releases exactly one DOF while the remaining DOF keep the object secured at `≥ min_holding_force`. A third continuity mode alongside gaiting and make-before-break; the suite must verify "exactly the released DOF is under-constrained, all others securing" and that the released DOF is re-secured at completion.
- **Bounded continuity-exception subclass**: `in_hand.flip` is the *only* primitive that suspends grasp continuity (momentary release). The suite must verify the exception is bounded — unsecured window `≤ max_release_time`, re-catch within `catch_envelope`, and on failure the object lands within `safe_drop_zone`. Define this as an explicit, bounded subclass of the grasp-continuity envelope class, distinct from the continuity-preserving modes.
- **`momentary_release` audit propagation**: a continuity-suspending operation (`in_hand.flip`) sets `momentary_release = true` in its result; persist and propagate it so downstream primitives, certification, and insurance (L4 / L8) can trace operations that broke continuity. Safety-critical transparency requirement.

### Owned by `04-tactile-manifold.md` (field set)

- **Tactile-absent confirmation proxy**: a force/position proxy for "force closure achieved" when `tactile_sensing` is not declared (graceful degradation, Principle 5).
- **Bend / crease indicator**: thin-object bending as a field distinct from crush/compression (`grasp.lateral`).
- **Intended-rolling vs gross-slip discrimination**: `in_hand.roll` migrates the contact point by design, so a raw slip feature cannot be an abort trigger. Define how the manifold (plus the kinematic rolling model) distinguishes controlled rolling from loss-of-control gross slip.
- **Intended-slip vs drop-slip discrimination + closed-loop slip sensing**: `in_hand.slide` permits intended slip along `slide_direction` but must abort on drop-slip of a securing (orthogonal) DOF — discriminated by direction. Closed-loop slip sensing is a capability requirement for `slide`; define the manifold feature and the directional discrimination.
- Tactile-target representation under TactileManifold (general).

### Owned by `01-skill-isa.md` (algebra + world state)

- **`reactive`-body envelope precedence**: a safety-abort takes precedence over clean `until` termination when both coincide (`reach.hover` as a `reactive` body). Fix when `reactive` semantics are formalized.
- **Start-in-contact + break-contact envelope clause**: reusable clause (`break_contact` postcondition + directional force-monotonicity) shared by `reach.retract`, `grasp.release`, `place.put_down`.
- **Supported-state predicate**: "object stably supported" (resting, CoM inside its own support polygon), reusing the `grasp.platform` CoM-over-polygon logic; gates `grasp.release` and `place`.
- **Last-resort `flip` admissibility**: `in_hand.flip` must be rejected (`continuity_alternative_exists`) when a continuity-preserving primitive (`rotate` / `roll` / `regrasp`) achieves the same reorientation. Build the "is there a continuity-preserving alternative?" check into composition validity / planning so the continuity-breaking primitive is genuinely a last resort.
- **Stability-metadata-driven admissibility**: an `in_hand` operation is admissible iff the DOF it moves is `friction_held`, not `form_held` (`in_hand.rotate` rejects rotation about a `rotation_constrained` axis). Build this DOF-admissibility check into composition validity; it recurs across `rotate`/`translate`/`slide`/`roll`/`pivot`.
- **`ContactConfig` type**: a contact configuration (which effector regions touch which object surfaces), supplied to `in_hand.regrasp` / `grasp.*` as `auto` (planner-derived) or explicit. Add to the Skill ISA type system.
- **Rollable-geometry in `GeometryRef`**: `in_hand.roll` needs `target.geometry` to expose a rolling surface and its rolling axis (cylinder / sphere / cone section). Add rolling-surface information to `GeometryRef` in the type system.
- **`SlideStop` type**: a sum type for `in_hand.slide`'s stop condition — `distance(d)` / `tactile_landmark(ref)` / `external_reference(ref)`. Landmark / reference stops are first-class because friction-dependent slip makes pure distance fragile. Add to the type system.
- **`Region` type**: a spatial region (`in_hand.flip` `catch_envelope` / `safe_drop_zone`). Evaluate unifying with `ScanRegion`, since both denote spatial regions. Add to the type system.
- **New-`GraspRef` supersession**: `in_hand.regrasp` produces a new `GraspState` and supersedes the originating `GraspRef`. Define the invalidation semantics for a `LetBind`-bound stale handle (dangling-reference prevention) in the grasp state model.

### Owned by `02-translation-layer.md` (this chapter)

- **Pose representation choice** (SE(3) / quat+t / axis-angle): must admit a single-scalar geodesic orientation error so `pose_not_reached` is decidable.
- **Under-constrained-orientation residual rule** (*decided*): minimum geodesic rotation from the current orientation; applied uniformly wherever orientation is under-constrained, binding for `retarget` determinism.
- **Rest-at-goal vs. trajectory blending**: v0.1 guarantees rest-at-goal; reserve room (e.g. `stop_at_goal: bool`) for non-stop blending without breaking the guarantee (Principle 5).
- **Normative sweep-pattern generators**: `Σ` for `pattern ∈ {raster, spiral, arc, waypoints}` as a deterministic function of `(region, pattern, standoff, overlap, FOV)`, in a normative appendix.
- **`min_holding_force` derivation**: deterministic from `target` mass, grasp mode, friction, load direction; required for `grasp.adjust` / `transport` safety.
- **Uncertainty-robustness in capability negotiation**: route uncertain-geometry tasks to envelope-capable embodiments using `grasp.envelope`'s declared robustness.
- **Passive-drive determinism boundary**: `in_hand.pivot(drive = gravity | external)` produces an outcome dependent on embodiment dynamics, not just the canonical action. `retarget` remains byte-for-byte deterministic in *generating* the canonical action, but the passive execution result is not — its tolerance is loosened accordingly. Make this determinism boundary explicit (parallel to `reach.scan`'s purely-kinematic contract), so passive primitives are not held to active-execution determinism.
- Time-scaling semantics under embodiment kinematic limits.
