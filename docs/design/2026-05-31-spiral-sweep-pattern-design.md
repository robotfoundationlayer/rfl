# Design: spiral sweep-pattern generator (Σ Appendix A)

Status: approved design, pre-implementation (2026-05-31)

This is the ninth reference-implementation increment. It implements the normative
`spiral` sweep-set generator from `spec/02` Appendix A, so `reach.scan` with
`pattern: spiral` over a surface region produces an Archimedean-spiral `Σ` instead
of a serpentine raster. It is the first follow-up to the second increment
(surface-scan / raster), completing one of the four normative patterns
(`raster` done, `waypoints` done, `spiral` here, `arc` deferred). The
specification under `spec/` is authoritative; this document describes how the
reference implementation realizes it.

## 1. Why this increment

`spec/02` Appendix A fixes the exact construction of `Σ` for each `pattern`, making
`Σ` a byte-reproducible function of `(region, pattern, standoff, coverage_overlap,
fov)` (TG1c; `reach.scan` C2). The reference engine currently realizes only the
`raster` and `waypoints` patterns; `ScanPattern::Spiral` is accepted but silently
falls back to raster geometry (increment 2 left it deferred). This increment makes
`spiral` real: a surface region scanned with `pattern: spiral` emits a genuine
Archimedean spiral, and — because the pass spacing is a function of the sensor FOV —
the same skill yields a different spiral per embodiment, the Principle-1 point
increment 2 demonstrated for raster, now for a second pattern.

## 2. Scope

In scope (increment 9):

- A pure `sigma::spiral(...)` generator transcribing Appendix A § spiral.
- Dispatch in `translation::lower_reach_scan`'s Surface arm on `pattern` so
  `Spiral` calls `spiral()` (Raster keeps calling `raster()`; `Arc` /
  `Waypoints`-on-surface keep the documented raster fallback).
- A worked-example demonstration: `examples/02-surface-scan/skill-spiral.yaml`
  (the same surface region as `skill.yaml`, `pattern: spiral`) + a
  `surface_scan_spiral.rs` conformance test (per-hand goldens, generate-twice,
  boon execute-schema validation).

Deferred (documented, each a future bite):

- **`arc`** — needs a declared pivot + `arc_extent`/`arc_start` inputs *and* the
  first non-fixed scan orientation (bore pointing at the pivot). The
  general-orientation work; out of scope here.
- **`path` region** — a 1-D raster (stations spaced `s_u` along an authored
  polyline). New region kind.
- **`volume` region** — a deterministic stack of surface layers at depth intervals
  `s_v` along the principal axis. New region kind; reduces to repeated surface
  coverage.
- **`coverage_overlap: auto`** — *blocked*. The schema documents the default as
  "auto, derived from the sensor FOV", but Appendix A fixes no normative derivation
  formula. Implementing it would mean inventing normative geometry; it stays
  deferred pending a `spec/02` decision.
- General non-axis-aligned surface orientation (the existing fixed
  `station_orientation` is reused).

No `spec/` change. No `schemas/` change — the region is the open
`{ "type": "object" } | Ref` floor in `ReachScanParams`, and `ScanPattern` already
enumerates `spiral`. No new `Primitive`/`ScanPattern` enum variant → no
exhaustive-match obligation; only the geometry dispatch is filled.

## 3. The generator (`sigma::spiral`)

Signature parallel to `sigma::raster`, in the region frame, SI scalars in:

```
spiral(size_u, size_v, standoff, coverage_overlap, h_angle_rad, v_angle_rad) -> Vec<Pose6D>
```

Shared setup (identical to raster, Appendix A § Common construction):

```
f_u = 2 · standoff · tan(h_angle / 2)      f_v = 2 · standoff · tan(v_angle / 2)
s_u = f_u · (1 − coverage_overlap)          s_v = f_v · (1 − coverage_overlap)
```

Spiral construction (Appendix A § spiral):

```
s   = min(s_u, s_v)                          # isotropic pass spacing
a   = s / (2π)                               # Archimedean coefficient, r(θ) = a·θ
centroid = (U/2, V/2)                        # surface-rectangle centre, region frame
R   = √((U/2)² + (V/2)²)                     # bounding radius = half-diagonal (v0 choice)
```

Stations are placed at constant arc-length step `Δℓ = s` from `θ = 0` outward:

- station `i = 0`: `θ = 0`, `r = 0` → the centroid itself;
- station `i ≥ 1`: solve `L(θ_i) = i · s` for `θ_i`, where `L(θ)` is the
  Archimedean arc length from 0:

  ```
  L(θ) = (a/2) · [ θ·√(1 + θ²) + asinh(θ) ]
  ```

  (from the polar arc-length element `ds = a·√(θ² + 1) dθ`).

Include stations while `r(θ_i) = a · θ_i ≤ R`; stop at the first `i` whose radius
exceeds `R`. The centroid (`i = 0`) is always included.

Each station maps to a sensor pose:

```
position    = (U/2 + r·cosθ, V/2 + r·sinθ, standoff)   # CCW, region frame, z = standoff
orientation = station_orientation()                    # the existing fixed +z-bore pose
```

`R` as the half-diagonal (circumscribing radius) is the v0 interpretation of
"bounding radius": it guarantees the spiral reaches the rectangle's corners. Like
the raster `station_orientation` and the grasp-force constants, it is a documented,
non-normative v0 reference choice, pinned by the golden — not a spec value.

### 3.1 Solving `L(θ) = i·s` deterministically

`L` is strictly increasing, so each `θ_i` is unique. The solver is **bisection**,
chosen over Newton for byte-determinism: bisection is pure comparison + midpoint on
a guaranteed bracket, so its control flow is platform-invariant (Newton mixes
`asinh`/`sqrt`/division per step, whose libm-dependent results can change the
iteration trajectory). The bracket is `[θ_{i-1}, θ_{i-1} + 2π]`: since
`dL/dθ = a·√(θ² + 1) ≥ a`, advancing the arc length by `s = 2πa` needs at most
`Δθ = s/a = 2π`, and the spiral only tightens as `θ` grows, so the upper bound
always brackets the root. Bisect to a tight tolerance on `θ` (≈ 1e-12); the final
position coordinates are `round6`-rounded at the canonical layer
(`SweepPose::from_pose`), absorbing transcendental ULP exactly as the raster
increment established for its `tan`/division. Generate-twice byte-equality proves
intra-run determinism.

## 4. The lowering dispatch (`translation::lower_reach_scan`)

Today the Surface arm always calls `raster()`. Restructure it to dispatch on the
resolved `pattern`:

- `Raster` → `raster(...)` (unchanged);
- `Spiral` → `spiral(...)` (new);
- `Arc` | `Waypoints` (with a Surface region) → `raster(...)` fallback (unchanged,
  documented — arc is a future bite, waypoints-pattern is driven by a Waypoints
  region not a Surface one).

The Waypoints *region* arm is untouched (it already calls `sigma::waypoints`). The
emitted `pattern_name` string already carries the requested pattern, so a spiral
skill emits `target_pose.pattern = "spiral"` with the real spiral poses — this also
makes the previously-cosmetic name truthful for the spiral case. A translation unit
test asserts that `pattern: spiral` over the surface region yields a
`SweepPath { pattern: "spiral", poses }` with the spiral station count (distinct
from the raster count for the same region).

## 5. Worked-example demonstration

`examples/02-surface-scan/skill-spiral.yaml`: the same `surface` region, `standoff`,
and `sense.inspect` as `skill.yaml`, but `pattern: spiral`. It reuses the three
existing embodiment descriptors (`allegro`, `leap`, `pneumatic-6f`) unchanged.

`crates/rfl-conformance/tests/surface_scan_spiral.rs` mirrors `surface_scan.rs`:

- three per-hand insta goldens (`spiral_allegro`, `spiral_leap`,
  `spiral_pneumatic`), loaded via the existing `retarget_example_to_jsonl(
  skill-spiral.yaml, embodiments/<stem>.yaml)` helper;
- generate-twice byte-equality;
- per-line boon validation against `driver-interface.schema.json` (the execute
  variant) — the spiral `Σ` rides the same open envelope floor as raster.

Because `validate.py` is example-01-only, the new `skill-spiral.yaml` is Class-1
validated out-of-band against `skill-isa.schema.json` via a one-off
`uv run --with jsonschema` script (the same approach used for the screw example).

## 6. Conformance

`validate.py` (C1–C7) is unchanged (no schema edit). The rfl-core suite gains the
`spiral` unit tests; the existing `surface_scan` raster goldens are **untouched**
(spiral is a separate skill file + separate test — re-run to confirm). All other
conformance suites stay green. No `rfl-core` public-type change beyond the new
`sigma::spiral` fn and the lowering dispatch.

## 7. Sections to transcribe during implementation

- `spec/02-translation-layer.md` Appendix A § Common construction + § spiral
  (the footprint/spacing setup and the `r(θ)`/`Δℓ`/bounding-radius construction) —
  already read for this design.
- The existing `sigma::raster` (the shared `f_u/f_v`, `s_u/s_v` setup and the
  `station_orientation` reuse) and `sigma::waypoints` (the module conventions).
- `translation::lower_reach_scan` (the Surface/Waypoints region match + the
  `pattern_name` emission) and `canonical::{round6, SweepPose::from_pose}`.
- `crates/rfl-conformance/tests/surface_scan.rs` (the golden + generate-twice +
  boon test shape) and `rfl_conformance::retarget_example_to_jsonl`.
