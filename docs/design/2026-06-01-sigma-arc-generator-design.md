# Σ arc sweep-pattern generator + arc region model — design

Status: design-complete; implementation **deferred to a focused session** (normative byte-reproducibility)

## Context

`reach.scan` covers a region with a generated sweep set `Σ` — ordered sensor
poses whose union observes the region (`spec/02` § Trajectory generation, TG1c).
Four patterns are normative: `raster`, `spiral`, `arc`, `waypoints`. The
reference implementation (`rfl-core::sigma`) ships `raster`, `spiral`, and
`waypoints`; **`arc` falls back to raster** today
(`translation.rs:2141`: "arc needs a pivot + variable orientation"). This is the
one missing normative generator.

## Why arc is not a trivial add

Raster and spiral lay poses on a surface with a **fixed** station orientation
(`sigma::station_orientation`). An arc sweeps about a **pivot**, so each station
sits on a circular path and its sensor orientation **varies** — it must keep the
bore pointed at the surface as the arc progresses. That variable per-station
orientation is the new element, and it must match `spec/02` § Appendix A —
Normative sweep-pattern generators **exactly**, because `Σ` is byte-reproducible
across implementations (TG1c, `reach.scan` C2). A close-but-not-exact
construction is worse than the honest raster fallback: it would mint goldens that
disagree with a conformant peer implementation.

## The clean v0 (decided, ready to implement)

1. **Arc region wire model** — a new `ScanRegion::Arc` variant carrying
   `{ pivot: Pose6D, radius: Quantity, arc_extent: [start_angle, end_angle],
   surface_normal }`. Additive (a new enum variant + its serde), no change to
   the existing surface/waypoints regions, so no existing golden moves.
2. **`sigma::arc(region, standoff, coverage_overlap, fov) -> Vec<Pose6D>`** — the
   normative construction: angular station spacing derived from the same
   footprint `2·standoff·tan(half_fov)` reduced by `coverage_overlap` (TG1c),
   converted to an angular step `Δθ = footprint / radius`; one station per step
   across `arc_extent`; each station's position on the circle and its
   orientation pointed inward along the local surface normal — transcribed
   verbatim from `spec/02` Appendix A.
3. Dispatch `ScanPattern::Arc` to `sigma::arc` in `lower_reach_scan` (replacing
   the raster fallback), plus an example `skill-arc.yaml` variant under
   `examples/02-surface-scan/` with per-embodiment goldens, and `sigma` unit
   tests pinning the station count + the inward orientation.

## Why deferred, not done inline

The construction is **normative and byte-reproducible**: it must be transcribed
exactly from `spec/02` Appendix A's arc subsection (the precise angular-spacing
rounding and the per-station quaternion), which warrants a focused read of that
appendix and a golden review, not a rushed inline pass during a docs-heavy
batch. The wire model and generator signature above are settled; this is a
ready-to-pick-up increment, not an open design question. **Not BLOCKED** (no data
/ hardware dependency) — scheduled-for-focus.
