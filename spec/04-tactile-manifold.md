# TactileManifold — Specification (skeleton)

> **Status**: pre-release skeleton. Full text targeted for v0.1 (2027 Q1). Formal mathematical specification is in the white paper Appendix B; this chapter is the implementation-facing version.

## Scope

TactileManifold is the formal abstraction RFL uses to represent tactile feedback across heterogeneous tactile sensors:

- Discrete force sensors at fingertips (FT-class)
- Distributed force arrays (XELA-class)
- Visuotactile sensors (GelSight / DIGIT-class)
- Pneumatic pressure sensors (in McKibben / bellows-style hands)

The abstraction lets a Skill ISA primitive express a tactile target (e.g., "until contact at fingertip with force ≥ 0.5 N") without committing to a specific sensor class.

## Mapping schema (provisional)

A TactileManifold instance is a triple:

```
TactileManifold := (
    sites:    Set<SiteId>,                  // discrete or dense
    fields:   SiteId -> Set<FeatureType>,   // e.g. normal_force, shear_x/y, slip
    resolution: (spatial, temporal_hz, bits_per_feature)
)
```

Per-sensor-class mappings live in `schemas/tactile-manifold/` and are referenced from the embodiment descriptor.

## Open issues

- Whether the field set is closed (fixed enumeration in v1.0) or extensible (registry-based)
- How temporal alignment is specified when multiple sensors run at different rates
- Calibration uncertainty propagation
