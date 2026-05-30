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
- Tactile-target representation under TactileManifold (see `04-tactile-manifold.md`)
- Time-scaling semantics under embodiment kinematic limits
