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

## Open issues

- Pose representation choice (SE(3) matrix vs. quaternion + translation vs. axis-angle)
  - *Constraint surfaced during `reach.to_pose` design (2026-05-30)*: the Skill ISA expresses `orientation_tolerance` as a single geodesic angle on SO(3). Whichever pose representation is chosen must admit a single-scalar geodesic orientation error, so that `pose_not_reached` is mechanically decidable. Fix this before the representation is frozen.
- `controlled_frame` declaration responsibility (surfaced during `reach.to_pose` design, 2026-05-30): `reach` primitives parameterize the moved frame as `controlled_frame`, defaulting to `embodiment.default_control_frame`. Retargeting must map `controlled_frame → embodiment native frame`. Non-anthropomorphic embodiments may expose several control frames, so the embodiment descriptor (`schemas/embodiment-descriptor.schema.json`) needs a `control_frames` set and a `default_control_frame` field. Resolve before the descriptor schema is frozen.
- Rest-at-goal vs. trajectory blending (surfaced during `reach.to_pose` design, 2026-05-30): `reach.to_pose` v0.1 guarantees termination at rest, which keeps composition with `grasp`/`force` unambiguous. Efficient non-stop blending through a waypoint chain is a foreseeable extension; reserve design room (e.g. a future `stop_at_goal: bool`) so it can be added without breaking the rest-at-goal guarantee (Principle 5).
- `embodiment.default_tool_axis` descriptor field (surfaced during `reach.approach` design, 2026-05-30): `reach.approach`, `reach.align`, and the `force` family parameterize a working/approach axis defaulting to `embodiment.default_tool_axis`. The embodiment descriptor needs a per-control-frame tool-axis declaration. Resolve alongside the `control_frames` set above.
- Collision-model partitioning — clearance must exclude the approach target (surfaced during `reach.approach` design, 2026-05-30): `reach.approach` applies `clearance` against the static model **excluding** the object being approached, otherwise the approach is self-forbidding. The retargeting algorithm and the Driver Interface collision-model representation must both be able to designate a target subset that is exempt from clearance for the duration of a primitive. Non-trivial; resolve before the collision-model schema is frozen.
- `SurfaceTarget` type and the perception-scope boundary (surfaced during `reach.approach` design, 2026-05-30): `reach.approach` consumes a `point` + outward `normal`, both perception-derived. RFL deliberately does not define perception, so a `SurfaceTarget` must be supplied as an already-resolved frame — typically the result of `sense.locate` hoisted through `LetBind`. The `SurfaceTarget` type definition (point, unit normal, frame, uncertainty bound) is owned by the Skill ISA; the perception that produces it is out of scope.
- Swept-volume clearance queries (surfaced during `reach.align` design, 2026-05-30): `reach.to_pose` and `reach.approach` check clearance over a straight-line corridor, but `reach.align` rotates in place and must check clearance over the rotation's **swept volume**. The Driver Interface collision-model representation must support a swept-volume clearance query, not only point/corridor queries. Resolve before the collision-model schema is frozen.
- Under-constrained-orientation residual rule — consistency (decided during `reach.align` design, 2026-05-30): when an orientation constraint leaves a residual rotational DOF (single-axis `reach.align`; the free normal-axis rotation of `reach.approach`), the residual is resolved as the **minimum geodesic rotation from the current orientation**. This rule is now decided and binding for `retarget` determinism; it must be applied uniformly wherever orientation is under-constrained across the spec, not re-litigated per primitive.
- Maintained-invariant conformance methodology (surfaced during `reach.hover` design, 2026-05-30; owned by `05-conformance.md`): terminal-postcondition primitives (`reach.to_pose`, `reach.approach`, `reach.align`) are verifiable by endpoint check, but sustained primitives (`reach.hover`, and later `force.wipe`/`force.scrub`) hold an invariant over an interval and require **interval sampling**. The conformance suite needs an "interval-invariant" test class distinct from the endpoint classes.
- `reactive`-body envelope precedence (surfaced during `reach.hover` design, 2026-05-30; owned by `01-skill-isa.md` BNF note 1): when `hover` is the body of `reactive(hover, until(pred))` and an envelope violation coincides with the `until` predicate firing, the safety-abort must take precedence over clean predicate termination. Fix this ordering when the `reactive` `until` semantics are formalized.
- `embodiment.limits.tracking_bandwidth` descriptor field (surfaced during `reach.hover` design, 2026-05-30): `reach.hover(track_target = true)` requires the embodiment to declare its target-tracking bandwidth so the precondition and `track_lost` failure mode are decidable. Add to `embodiment.limits`.
- `ScanRegion` type definition (surfaced during `reach.scan` design, 2026-05-30; owned by `01-skill-isa.md`): `reach.scan` consumes a region geometry (volume / surface / path). This is a task-specified, not perception-derived, type and belongs to the Skill ISA type system. Define before scan reaches freeze.
- Visual/range sensor descriptor fields (surfaced during `reach.scan` design, 2026-05-30; owned by `03-driver-interface.md`): scan needs `embodiment.default_sensor_frame` and per-sensor `fov`, `bore_axis`, and `max_sweep_rate`. The existing descriptor covers tactile sensor placement; generalize it to visual/range sensors so the `auto` overlap and motion-blur velocity cap are decidable.
- Normative sweep-pattern generators (surfaced during `reach.scan` design, 2026-05-30): the `Σ` sweep-pose set for `pattern ∈ {raster, spiral, arc, waypoints}` must be a deterministic, algorithmically specified function of `(region, pattern, standoff, overlap, FOV)` so that `retarget` determinism and cross-implementation conformance (`reach.scan` C2) hold. Specify the generators in a normative appendix.
- Tactile-target representation under TactileManifold (see `04-tactile-manifold.md`)
- Time-scaling semantics under embodiment kinematic limits
