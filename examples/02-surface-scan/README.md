# Example 02 — Surface scan and the sweep-pattern generators

> **Status**: worked example running. The base skill and its variants retarget
> onto the three embodiment descriptors and are pinned by conformance test
> class 2 (`crates/rfl-conformance`) against committed golden snapshots. Each
> variant exercises a different normative Σ sweep-pattern generator (`spec/02`
> Appendix A) or the interval-invariant envelope class.

A `reach.scan` covers a region with a generated **sweep set Σ** — the ordered
sensor poses whose union observes the region. Σ is a deterministic,
byte-reproducible function of `(region, pattern, standoff, coverage_overlap,
fov)`, so the **same skill produces a different station count per embodiment**
(each hand's sensor field of view differs) while the geometry stays normative.
This example demonstrates every parametric generator (raster / spiral / arc) and
every non-surface region kind (path / volume) plus station-keeping.

## Task

Cover a region at a fixed sensor standoff, then inspect what was observed. RFL
fixes the geometric coverage (the sweep set); the perception that interprets the
observations is out of scope (`04`, the `reach.scan` / `sense.inspect`
capturability boundary).

## Variants

| File | Pattern / primitive | Demonstrates |
|---|---|---|
| `skill.yaml` | `reach.scan` **raster** | parallel serpentine (boustrophedon) passes; the base generative-Σ case |
| `skill-spiral.yaml` | `reach.scan` **spiral** | an Archimedean spiral from the surface centroid at constant arc-length step |
| `skill-arc.yaml` | `reach.scan` **arc** | a swept arc at radius = standoff about a pivot, bore pointing inward (circumferential / cylindrical inspection) |
| `skill-path.yaml` | `reach.scan` **path** region | stations spaced s_u along a caller-supplied polyline — a one-dimensional raster (e.g. a weld seam / L-shaped feature) |
| `skill-volume.yaml` | `reach.scan` **volume** region | a boustrophedon stack of surface-raster layers at depth intervals s_v along the region's +z axis (volumetric / layered coverage) |
| `skill-hover.yaml` | `reach.hover` | sustained station-keeping over the panel — the **interval-invariant** envelope class, not a sweep |

All six target the same three embodiments under `embodiments/`
(`allegro` / `leap` / `pneumatic-6f`); only the descriptor (and its sensor FOV)
changes between command streams.

## Conformance exercised (`05`)

- **Class 2 generative Σ** — the raster / spiral / arc sweep sets and the path /
  volume region reductions are byte-deterministic and each line is a valid
  `execute` message (`surface_scan`, `surface_scan_spiral`, `surface_scan_arc`,
  `surface_scan_path`, `surface_scan_volume` golden tests).
- **Interval-invariant (ENV2 / ENV3)** — `skill-hover.yaml` holds station within
  `station_tolerance`; a sub-envelope impulse recovers within `settling_time`,
  an over-envelope impulse aborts to a safe state within `stop_time`
  (`surface_scan_hover`).
- **Perception boundary** — `sense.inspect` observes the covered region; RFL
  does not interpret the observation.

## How to run

```bash
# From the repository root — each prints one execute message per Σ station:
cargo run -p rfl-cli -- retarget examples/02-surface-scan/skill.yaml \
    --embodiment examples/02-surface-scan/embodiments/allegro.yaml
cargo run -p rfl-cli -- retarget examples/02-surface-scan/skill-spiral.yaml \
    --embodiment examples/02-surface-scan/embodiments/leap.yaml
cargo run -p rfl-cli -- retarget examples/02-surface-scan/skill-arc.yaml \
    --embodiment examples/02-surface-scan/embodiments/pneumatic-6f.yaml
cargo run -p rfl-cli -- retarget examples/02-surface-scan/skill-path.yaml \
    --embodiment examples/02-surface-scan/embodiments/allegro.yaml
cargo run -p rfl-cli -- retarget examples/02-surface-scan/skill-volume.yaml \
    --embodiment examples/02-surface-scan/embodiments/allegro.yaml
```

The station count differs per embodiment: a smaller sensor footprint (narrower
FOV) yields tighter pass spacing and more stations, from the same skill.
