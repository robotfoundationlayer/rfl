# Σ `path` / `volume` sweep-pattern generators (spec/02 Appendix A)

Status: design-complete (2026-06-02). Closes the last two deferred `ScanRegion`
kinds. `arc` (`f79aa6d`) is the precedent; this mirrors it end to end.

## Motivation

`spec/02` Appendix A § "Region kinds beyond surface" fixes how non-surface
regions reduce to the existing parametric construction:

> A `surface` `ScanRegion` uses the generators directly. A `path` region sweeps
> stations spaced `s_u` along the given path (a one-dimensional `raster`). A
> `volume` region is covered as a deterministic stack of surface layers at depth
> intervals `s_v` along the volume's principal axis, each layer a surface sweep.

`surface`, `arc`, `waypoints`, `spiral` are implemented; `path` and `volume`
still fell back to nothing (the `match &p.region` had no arm). This adds both as
deterministic, byte-reproducible generators.

## Design decisions

The spec leaves three points under-specified that must be pinned to keep `Σ`
byte-reproducible. Resolved (user-approved 2026-06-02):

1. **Volume stacking axis = frame `+z`** (the `size_w` dimension), fixed. Matches
   the existing `surface` = frame xy-plane convention; no data-dependent
   "longest dimension" tie-break to specify. "Principal axis" is read as the
   region-frame depth axis.
2. **Orientation = the surface v0 flat-normal convention**: bore (`+z`)
   anti-parallel to the frame `+z` normal, reusing `sigma::station_orientation`.
   `path` points are assumed to lie on the frame xy-plane (a planar path); each
   station is `point + standoff·ẑ`. Consistent with `raster` / `spiral` v0.
   Per-point normal/tangent frames are deferred (they would invent
   contact-geometry the v0 `surface` does not carry).
3. **`Σ` stays a continuous path across volume layers** via boustrophedon
   stacking: the pose order of odd-indexed layers is reversed, so the last pose
   of layer `k` is adjacent to the first pose of layer `k+1`. Honours the
   Appendix A Common-construction statement that `Σ`'s order is a continuous
   sweep, extending `raster`'s serpentine ethos to 3D.

`pattern` and `region.kind` stay orthogonal (the `arc` precedent): the region
kind drives the generator; the `pattern` field is echoed to the wire. Both
example skills set `pattern: raster` — `path` *is* a 1-D raster, `volume` *is*
stacked raster — so **no new `ScanPattern` enum variant**.

No schema change: `schemas/skill-isa.schema.json` (line ~845) already lists
"volume / surface / path" and defers `ScanRegion` internals to an inline object.

## `path` construction

Input: `ScanRegion::Path { frame, points: Vec<[f64; 3]> }` — an ordered polyline
of surface points in the region frame.

```
s_u = 2·standoff·tan(h_angle/2) · (1 − coverage_overlap)   # u footprint spacing
L   = Σ ‖points[i+1] − points[i]‖                          # total polyline length
n   = ceil(L / s_u).max(1)                                 # station count
ℓ_j = (j + 0.5)·L / n,   j = 0 … n−1                       # centered, as raster u_j
P_j = arc-length interpolation of the polyline at ℓ_j
pose_j = { position: P_j + standoff·[0,0,1], orientation: station_orientation() }
```

Arc-length interpolation: walk the segments accumulating length; locate the
segment containing `ℓ_j`; linearly interpolate within it. Only `h_angle` is used
(single pass; `v_angle` is irrelevant, as in `arc`).

Degenerate guards (mirror `arc`/`spiral`):
- `s_u ≤ 0` (flat FOV or zero standoff) → a single station at the path midpoint
  (`ℓ = L/2`).
- zero-length path (all points coincident, `L = 0`) → `n = 1`, one station at the
  point.
- empty `points` → empty `Σ` (nothing to cover).

## `volume` construction

Input: `ScanRegion::Volume { frame, size_u, size_v, size_w }` — an axis-aligned
box `[0,U] × [0,V] × [0,W]` in the region frame.

```
s_v = 2·standoff·tan(v_angle/2) · (1 − coverage_overlap)   # depth/layer spacing
n_w = ceil(W / s_v).max(1)                                 # layer count
w_k = (k + 0.5)·W / n_w,   k = 0 … n_w−1                   # centered layer depths
layer_k = raster(U, V, standoff, overlap, h, v)            # reuse surface generator
          with each pose's z shifted by +w_k               # surface at depth w_k
Σ = concat over k of layer_k, pose order reversed when k is odd   # boustrophedon
```

The in-plane sweep is the existing `raster` (poses at `z = standoff`); adding
`w_k` puts the sensor `standoff` above the layer surface at depth `w_k`. `s_v`
serves both as the inter-layer depth spacing (here) and as `raster`'s in-plane v
spacing (inside `raster`), exactly per the spec wording "depth intervals `s_v`".

Degenerate guard: `s_v ≤ 0` → `n_w = 1` (single mid-depth layer). The in-plane
`raster` carries its own behaviour unchanged (it is the same code surface scans
already exercise).

## Integration

`crates/rfl-core/src/translation.rs` `lower_reach_scan`: the `match &p.region` is
exhaustive, so the two new arms are compiler-enforced. Each arm reads the sensor
`fov` exactly as the `surface`/`arc` arms do and calls the new generator.

Per the repo discipline (new enum variant + dispatch arm in one commit for the
exhaustive match), each generator ships as **one commit**: the `ScanRegion`
variant + `sigma::` fn + dispatch arm + example skill + per-embodiment goldens +
`sigma`/`region` unit tests — the `arc` commit `f79aa6d` shape.

## Testing

`sigma.rs` unit tests pin, per generator:
- analytic station / layer count for a worked FOV (a mismatch means recheck the
  math, not a blind golden update);
- the geometric property — `path`: every station lies on the polyline + at
  `standoff` in `+z`; `volume`: every pose's `(x,y)` is inside `[0,U]×[0,V]` and
  `z = w_k + standoff` for some layer;
- centered placement (first `path` station at `ℓ = 0.5·L/n`);
- boustrophedon continuity for `volume` (layer-boundary poses are adjacent);
- degenerate guards (flat FOV → single station/layer, never divergence).

`region.rs` adds parse tests for the two new YAML forms.

Conformance goldens `surface_scan_path` / `surface_scan_volume` (new
`examples/02-surface-scan/skill-path.yaml` / `skill-volume.yaml` + per-embodiment
`.snap`) prove the count varies with FOV across allegro / leap / pneumatic, like
the existing `surface_scan_arc` / `surface_scan_spiral` tests.

## Out of scope (deferred, unchanged)

General bore/normal orientation (non-axis-aligned surfaces), per-point path
normals, non-`+z` volume principal axes, curved (spline) path input — all
deferred exactly as the v0 `surface` orientation is. This change adds breadth
(two region kinds), no new dimension.
