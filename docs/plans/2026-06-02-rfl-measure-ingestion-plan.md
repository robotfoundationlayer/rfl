# Plan: `rfl measure` ingestion (LOCAL ONLY — do not git add)

Design: `docs/design/2026-06-02-rfl-measure-ingestion-design.md`

## Commit 1 — `Primitive::name()` (rfl-core)
- Add `pub fn name(&self) -> &'static str` to `impl Primitive` (skill_isa.rs:264),
  explicit match returning the canonical names (`force.insert_fit`, …) — same
  strings as the serde renames. Exhaustive (compiler-enforced for future variants).
- Test: a few representative variants map to their names.
- Gates: cargo test (rfl-core) + fmt + clippy.

## Commit 2 — aggregation layer (rfl-conformance::measure)
- Types: `Metric` already exists; add `ProvisionalEntry { metric, tolerance:
  Option<f64>, unit: &'static str, n_runs, n_samples }` and `ProvisionalTable {
  percentile, safety_factor, tolerances: BTreeMap<primitive, BTreeMap<quantity,
  ProvisionalEntry>> }`.
- `pub fn aggregate_epsilon(action_primitives: &BTreeMap<String,String>, runs:
  &[BTreeMap<String, DriverReport>], percentile, safety) -> ProvisionalTable`:
  - run[0] = reference. For each action_id in reference → primitive (via map;
    unmapped → `__unmapped__`).
  - Per quantity, extract representative value (terminal): final_position
    (l2_dev), final_orientation (so3_dev via UnitQuaternion from [x,y,z,w]),
    wrench_force / wrench_torque (l2_dev, last telemetry wrench), securing_force
    / station_error (abs_dev, last telemetry Quantity scalar).
  - For each run i>0 with the same action_id, deviation(ref, run_i) → pooled into
    (primitive, quantity) sample vec. Multiple actions of one primitive pool.
  - tolerance = epsilon_candidate(samples, percentile, safety); n_samples =
    samples.len(); n_runs = runs.len(). < 1 sample → tolerance None.
- Tests: identical 2 runs → 0; known deviation → percentile×safety; quantity in
  one run only → None + n_samples 0; two actions same primitive → pooled.
- Gates: cargo test (rfl-conformance) + fmt + clippy.

## Commit 3 — CLI `Measure` + serializer + integration test + docs
- `rfl-cli` Measure subcommand: `--skill --embodiment --run(repeat) --percentile
  --safety --out`. < 2 runs → exit 2.
  - read skill+embodiment YAML; build action_primitives by walking sequence
    (retarget for suffixes + `Primitive::name`) replicating to_jsonl's id format;
    `replay_report` each --run; `aggregate_epsilon`; serialize provisional YAML
    (header comment + provisional:true + per-entry source/n_runs/n_samples/
    percentile/safety_factor) to --out or stdout.
- Integration test (`measure_cli.rs`): retarget a committed skill, write two
  perturbed fixture traces (test-local, NOT committed schema values), run
  measure, assert provisional:true + a non-null candidate, and assert
  `schemas/epsilon-tolerances.yaml` byte-unchanged.
- docs/cli-reference.md: add the measure subcommand.
- Gates: full `cargo test --workspace --all-targets` + fmt + clippy + validate.py
  + md_lint (docs changed). ff-push. `gh run watch`.
