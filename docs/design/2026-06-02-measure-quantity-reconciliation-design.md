# Reconciling `rfl measure` with the committed ε-tolerance table

Date: 2026-06-02
Status: approved (design) — supersedes the quantity model in
`2026-06-02-rfl-measure-ingestion-design.md`

## Problem

The first `rfl measure` cut emitted a *generic* terminal-quantity vocabulary
(`final_position` / `final_orientation` / `wrench_force` / `wrench_torque` /
`securing_force` / `station_error`) applied uniformly to every primitive. The
committed `schemas/epsilon-tolerances.yaml` instead keys each contact-dynamics
primitive by *domain-specific* quantities (`force.insert_fit` →
`realized_pose` / `realized_wrench` / `seating_depth`; `force.screw` →
`completion_torque` / `turns`; `in_hand.pivot` → `final_orientation` /
`securing_force`). The two vocabularies only partially overlap, so the
provisional table was **not promotable** into the committed one — defeating the
ingestion path's purpose.

## Decisions (approved)

1. **Realign to the committed vocabulary.** The committed table is the single
   source of truth for which `(primitive, quantity, metric, unit)` exist. The
   measure tool is **table-driven**: it parses the embedded committed table and,
   for each primitive that appears in the traces, emits exactly that primitive's
   committed quantities — filling the wire-derivable ones, marking the rest.

2. **Split `realized_pose`.** A single ε under `geodesic_se3` cannot bound a
   (position-m, orientation-rad) pair without a length scale. So `realized_pose`
   becomes two entries in the committed table: `realized_position`
   (`l2_norm`, m) and `realized_orientation` (`geodesic_so3`, rad). `se3_dev`
   already returns the `(translation, rotation)` split, so each grades under one
   ε in one unit. This edits the committed table on the six primitives that
   carried `realized_pose` (insert_fit, press_button, snap_engage, wipe, push,
   pull). C9 checks only the *primitive* key set, so the quantity-key change is
   C9-safe.

## Quantity → wire-source map

| Committed quantity | Metric | Wire source | Derivable? |
|---|---|---|---|
| `realized_position` | `l2_norm` (m) | `status.final_pose.position` | yes |
| `realized_orientation` | `geodesic_so3` (rad) | `status.final_pose.orientation` | yes |
| `final_orientation` (in_hand.pivot) | `geodesic_so3` (rad) | `status.final_pose.orientation` | yes |
| `realized_wrench` | `l2_norm` (N) | last `telemetry.wrench.force` | yes |
| `securing_force` | `abs` (N) | last `telemetry.securing_force` | yes |
| `seating_depth`, `completion_torque`, `turns`, `actuation_force`, `engagement_force`, `cut_depth`, `oscillation_amplitude`, `contact_force` | (per table) | — | **no** (category C) |

`in_hand.pivot` is fully measurable from today's wire. The category-C domain
quantities are not first-class wire fields; measuring them needs a richer source
(parse `events` / `tactile`, or a wire extension like `contact_geometry` was) or
a spec decision, deferred. They are emitted with `tolerance: null` and
`reason: not_wire_derivable` — honest, never a silent pass.

## Architecture

- `measure.rs` embeds the committed table via `include_str!` and parses it into
  `primitive -> quantity -> (metric, unit)` (the structural truth; tolerances are
  all `null` there and ignored).
- A `wire_extractor(quantity) -> Option<fn(&DriverReport) -> Option<Repr>>`
  registry maps a quantity *name* to its terminal-value extractor, or `None` for
  a category-C domain quantity.
- `aggregate_epsilon` walks the reference run's actions; for each primitive that
  the committed table covers, it emits each committed quantity:
  - wire-derivable + reference reports it → run-to-run deviation samples →
    `epsilon_candidate`; `reason: None`.
  - wire-derivable but the reference does not report it →
    `tolerance: null`, `reason: not_reported`.
  - category C → `tolerance: null`, `reason: not_wire_derivable`.
  Primitives not in the committed table (non-contact-dynamics) get no ε row.
- `ProvisionalEntry` gains `reason: Option<&'static str>`; the provisional YAML
  emits it on null entries.

## Testing (TDD)

- Table parse loads all 11 contact-dynamics primitives.
- `in_hand.pivot` over two perturbed runs → `final_orientation` +
  `securing_force` both filled (non-null), no other keys.
- `force.insert_fit` → `realized_position` / `realized_orientation` /
  `realized_wrench` filled when reported; `seating_depth` null +
  `not_wire_derivable`.
- `force.screw` → `completion_torque` + `turns` both null +
  `not_wire_derivable` (nothing on the wire to fill them).
- A non-contact primitive (e.g. `grasp.pinch`) in a trace → no ε row.
- CLI integration test (committed cable trace ×2): output uses committed keys
  (`realized_position`, `realized_wrench`), committed table byte-unchanged.

## Non-goals

- Filling category-C quantities (needs a measurement-source / wire decision).
- Promoting any provisional value into the committed table (deliberate, manual).
