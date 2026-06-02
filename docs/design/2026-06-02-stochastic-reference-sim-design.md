# A stochastic reference simulator for provisional ε

Date: 2026-06-02
Status: approved (design)

## Problem

`rfl measure` produces ε from run-to-run variation, but the only generator,
`rfl sim`, is deterministic (variation = 0 → ε = 0). Real ε needs real hardware
traces. The honesty rule forbids fabricating values into the committed
`epsilon-tolerances.yaml` (it stays `null`), but the project explicitly permits a
second source: *the output of a simulator with a documented variation model,
used with a provisional label, anchored to `simulator-declaration.yaml` at
`status: pending`*. This builds that source so the measurement pipeline produces
a meaningful (non-zero) **provisional** ε end to end, with fully transparent
provenance.

## Key property

For ε only the run-to-run *deviation* matters, so the nominal value cancels:
`ε ≈ percentile(|noise_i − noise_0|) × safety`. The candidate ε is therefore a
pure function of the **declared σ**. The simulator cannot manufacture a
physically meaningful tolerance — only one that transparently reflects its own
declared noise. That is exactly why the output is provisional and sim-anchored,
never promoted to the normative table.

## Components

### 1. The variation model (declared, in the pending declaration)

`simulator-declaration.yaml` gains an optional `variation_model`: per ε-table
quantity name, `{ nominal, sigma, unit? }`. These are **illustrative declared**
parameters of `rfl-reference-sim`, documented as such — never physical truth.
They live in the `status: pending` declaration, never in the normative ε table.
`simulator-declaration.schema.json` gains the optional field; C10
(no-self-bootstrap) is unaffected (it gates only a `conformant` claim).

### 2. `StochasticDriver` (rfl-conformance)

A single entry point, reusing the existing pieces:

```rust
pub fn stochastic_run(
    skill: &Skill, embodiment: &Embodiment,
    model: &NoiseModel, seed: u64,
) -> Result<Vec<DriverReport>>
```

It retargets, runs the nominal `run_reference_driver`, then perturbs each report
with a seeded `StdRng::seed_from_u64(seed)`:

- **Kinematic / wrench** quantities perturb the existing fields: `realized_pose`
  / `final_pose` position (`+ N(0, σ)` per axis) and orientation (small-angle
  rotation), `wrench.force`, `securing_force`.
- **Domain scalars** are emitted into `measured_quantities`, keyed per the
  action's primitive: for each committed quantity of that primitive that routes
  to the `measured_quantities` channel (i.e. not one of the four kinematic
  names), emit `nominal + N(0, σ)` in the declared unit. The primitive is
  recovered from the retarget `action_id -> primitive` map (the same
  `action_primitives` `rfl measure` uses) against the embedded committed table.

Gaussian sampling is Box-Muller over two `rng.gen::<f64>()` uniforms — no new
dependency. `StochasticDriver` is a *new* path: the nominal `ReferenceDriver` /
`rfl sim` output and every golden are unchanged.

### 3. `rfl sim --seed N --variation <decl.yaml>`

With `--skill` + `--embodiment` + `--seed` + `--variation`, `rfl sim` emits a
*stochastic* report for that seed (deterministic given the seed). Without
`--seed`, it is the existing deterministic nominal driver. N varied runs are N
invocations with seeds `0 .. N`; `rfl measure` over them yields the provisional
table.

## Data flow

```
rfl sim --skill S --embodiment E --variation decl.yaml --seed 0  > run0.jsonl
rfl sim … --seed 1 > run1.jsonl   …   --seed K-1 > run{K-1}.jsonl
rfl measure --skill S --embodiment E --run run0.jsonl … --run run{K-1}.jsonl
  -> provisional table, non-zero ε, # PROVISIONAL banner
```

## What is and is not committed

- **Committed (tools):** the `variation_model` in the pending declaration, the
  schema field, `StochasticDriver`, the `rfl sim` flags, a generation script
  (`examples/.../generate-provisional-epsilon.sh`), and a docs section.
- **Not committed (output):** the provisional ε table itself. It is generated
  on demand; leaving declared-σ-derived numbers out of the repo keeps "the tool
  exists, the values are unmeasured" honest. `epsilon-tolerances.yaml` stays
  all-`null`.

## Testing (TDD)

- `gaussian`: seeded, reproducible; mean ≈ 0, stdev ≈ σ over many samples
  (loose bounds).
- `stochastic_run`: same seed → byte-identical reports (reproducible); different
  seeds → different realized values; emits `measured_quantities` for a
  contact-dynamics primitive's domain scalars.
- End-to-end (the proof): seeds `0..K` → `aggregate_epsilon` → a contact-dynamics
  quantity has non-zero `tolerance` and `reason: None` (the pipeline yields
  meaningful provisional ε); the committed `epsilon-tolerances.yaml` is
  byte-unchanged.
- `rfl sim --seed` CLI smoke: deterministic per seed; nominal (no `--seed`)
  output unchanged.

## Non-goals

- Promoting any value into the committed table (deliberate, hardware-gated).
- Physical realism of σ (declared, illustrative).
- A `conformant` simulator claim (C10 still requires hardware-anchored
  ε-match evidence; this stays `pending`).
