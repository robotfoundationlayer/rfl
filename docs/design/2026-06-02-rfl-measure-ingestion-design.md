# `rfl measure` — trace ingestion for the ε-tolerance table

Date: 2026-06-02
Status: approved (design)

## Problem

`schemas/epsilon-tolerances.yaml` carries the per-primitive, per-quantity ε
tolerances with every value `null` — the honest "not yet measured" state. The
`rfl-conformance::measure` harness already implements the *primitive* deviation
functions (`abs_dev` / `l2_dev` / `se3_dev` / `so3_dev` / `epsilon_candidate` /
`run_to_run_abs`), but there is no path from *real driver-report traces* to a
filled table:

- no **aggregation layer** that groups realized quantities by
  `(primitive, quantity)` across N runs, and
- no **CLI subcommand** to drive it.

This is the receiving end for measured data. The values stay `null` until real
multi-run traces exist; this design builds the tool that consumes them, without
ever fabricating a value or silently overwriting the committed null table.

## Non-goals

- Filling the committed `schemas/epsilon-tolerances.yaml`. That is a normative
  promotion the user performs deliberately, from a provisional measurement.
- Time-series alignment of telemetry samples across runs (deferred; see
  "Quantity extraction").
- Producing ε from `rfl sim` output. The reference simulator is deterministic
  (run-to-run variation = 0), so it yields ε = 0 — a true value, but not a
  meaningful tolerance. Real variation requires real hardware or a
  declared-conformant stochastic simulator.

## CLI

```
rfl measure --skill S.yaml --embodiment E.yaml \
            --run trace1.jsonl --run trace2.jsonl [--run …] \
            [--percentile 0.95] [--safety 1.2] [--out provisional.yaml]
```

- `--run` is repeatable; **≥ 2 required**. A single run has no run-to-run
  variation, so fewer than two is a hard error (exit 2), not a fabricated zero.
- `--percentile` (default 0.95) and `--safety` (default 1.2) parameterise
  `epsilon_candidate` (nearest-rank percentile × safety factor).
- `--out` writes the provisional table to a file; default is stdout.

## Pipeline

1. **Retarget** `S` + `E` in-process (`rfl_core::retarget`) → `ExecuteGoal`.
   Each `CanonicalAction` carries an `action_id` and its source primitive. Build
   the `action_id → primitive` map. This is the same correlation `rfl certify`
   relies on (the driver echoes `action_id`s from the execute goals).
2. **Ingest** each `--run` trace by reusing `replay.rs`'s schema-gated line
   parser (`driver-interface.schema.json` is the authoritative
   `additionalProperties:false` gate before deserialization). Produce, per trace,
   the per-`action_id` `Telemetry` samples + terminal `Status`.
3. **Extract** one representative realized value per `(action_id, quantity)` per
   run (terminal values — see below).
4. **Aggregate**: for each `(primitive, quantity)`, collect the representative
   value from each run, compute `run_to_run` deviations vs the run-1 reference,
   then `epsilon_candidate`.
5. **Emit** the provisional table.

## Quantity extraction (terminal-only, v0)

One sample per action per run, taken from terminal/summary fields — no
time-series alignment:

| Quantity            | Source                          | Metric        | Unit  |
|---------------------|---------------------------------|---------------|-------|
| `final_position`    | `status.final_pose.position`    | `l2_norm`     | m     |
| `final_orientation` | `status.final_pose.orientation` | `geodesic_so3`| rad   |
| `wrench_force`      | last `telemetry.wrench.force`   | `l2_norm`     | N     |
| `wrench_torque`     | last `telemetry.wrench.torque`  | `l2_norm`     | N·m   |
| `securing_force`    | last `telemetry.securing_force` | `abs`         | N     |
| `station_error`     | last `telemetry.station_error`  | `abs`         | m     |

A quantity absent from a run's action contributes no sample for that action; a
`(primitive, quantity)` with < 2 samples across runs is reported with
`tolerance: null` and its sample count, never a fabricated value. Pose splits
into a position row and an orientation row because the ε-table carries one
tolerance per quantity and the two have different units (matching `se3_dev`'s
existing split).

When two actions share a primitive (e.g. two `force.screw` actions), their
samples pool into the same `(primitive, quantity)` bucket — the table is
per-primitive, not per-action.

## Output — the honesty firewall

The committed `schemas/epsilon-tolerances.yaml` (all `null`) is a truth claim:
"these have not been measured." The tool must never let a provisional
measurement masquerade as a normative tolerance.

- Output is a **separate** document, never the committed table.
- Header comment: `# PROVISIONAL — measured, NOT normative. Do not commit as schemas/epsilon-tolerances.yaml.`
- Top-level `provisional: true` and per-entry `source: measured`, `n_runs`,
  `n_samples`, `percentile`, `safety_factor`.
- Schema-shape compatible with `epsilon-tolerance.schema.json` for the
  `{metric, tolerance, unit}` core, but with the extra provenance keys — so a
  human can diff it against the committed table but cannot accidentally validate
  it *as* the committed table (`validate.py` C9 only checks the committed file).

## Components & boundaries

- `rfl-conformance::measure` gains an **aggregation** function:
  `aggregate_epsilon(goal_primitives, runs, percentile, safety) -> ProvisionalTable`,
  where `runs: Vec<Vec<ReportLine-equivalent>>`. Pure (no I/O), unit-testable
  with synthetic in-memory traces. The existing primitive deviation fns are its
  building blocks.
- `replay.rs` ingestion is reused for parsing; if its line parser is not yet
  public, expose a minimal `parse_report_lines(&str) -> Result<Vec<…>>` (the
  schema gate stays internal to it).
- `rfl-cli` gains a `Measure { … }` subcommand that does the I/O (read files,
  retarget, call `aggregate_epsilon`, serialize the provisional table) and
  nothing else.

## Testing (TDD)

- Aggregation unit tests with synthetic two-run and three-run traces: identical
  runs → ε = 0; known deviations → known percentile×safety; a quantity present
  in one run only → `null` + `n_samples: 1`; two actions sharing a primitive →
  pooled samples.
- A `< 2 runs` invocation → hard error.
- CLI integration test: `rfl retarget` a committed skill, hand-author two
  perturbed traces (clearly test fixtures, not committed ε values), run
  `rfl measure`, assert the provisional output carries `provisional: true` and a
  non-null candidate for a perturbed quantity — and assert the committed
  `schemas/epsilon-tolerances.yaml` is byte-unchanged.

## Risks

- **action_id correlation**: if a trace's `action_id`s do not match the
  retargeted goal's, the action is unmapped → reported under a `__unmapped__`
  bucket with a warning, never silently dropped.
- **Promotion discipline**: the provisional file must stay out of the committed
  schema path. Mitigated by the distinct filename default + header + the
  provenance keys that fail a strict diff against the committed shape.
