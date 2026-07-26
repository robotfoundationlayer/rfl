# Code map — how the reference implementation fits together

Where each piece of the RFL reference implementation lives and how a skill
becomes a certificate. For the specification itself see [`spec/`](../spec/); for
the CLI surface (flags, exit codes, worked I/O) see
[cli-reference.md](cli-reference.md).

## The pipeline

```
skill.yaml ─ validate ─→ retarget(skill, embodiment) ─→ canonical actions
   ─ to_jsonl ─→ ExecuteGoal stream ─→ driver (sim / vendor, replay or live stdio)
   ─→ DriverReport (telemetry + status) ─→ conformance battery ─→ certificate
   ─→ verify / sign / badge;   N traces ─→ measure ─→ provisional ε table
```

## `rfl-core`

| File | Role |
|---|---|
| `skill_isa.rs` | The 50 primitives, parsing, and the compositional algebra. Its enums are matched exhaustively downstream, so a new variant breaks lowering at compile time (by design). |
| `translation.rs` | The heart (~3.4k lines): `retarget(&Skill, &Embodiment) -> RetargetOutput`, one lowering per primitive. |
| `grasp_force.rs` | Mass-dependent grasp-force derivations GF1c–GF4c, including the tool-mediated torque reaction. |
| `stability.rs` | Closure classification and the STB2 / STB3 stability-class obligations. |
| `sigma.rs` | The Σ sweep generators: raster, spiral, arc, path, volume. |
| `pose.rs`, `region.rs`, `geometry.rs`, `quantity.rs` | Supporting math for the lowerings above. |
| `canonical.rs` | The canonical wire, both directions (`to_jsonl` / `from_jsonl`). |
| `driver.rs` | `ExecuteGoal` / telemetry / status types (ROS 2-compatible). |
| `embodiment.rs` | Embodiment descriptor parsing (capabilities, DOF, frames). |

## `rfl-conformance`

| File | Role |
|---|---|
| `lib.rs` | ~3.6k lines: every `check_*` obligation **and** the adversarial drivers that must fail them (`FaultyDriver`, `DisturbanceDriver`, `HoverSettlingDriver`, `PressButtonDriver`, `SnapEngageDriver`, `CutDriver`, `FreeingDriver`, `FlipDriver`). A new obligation lands together with the adversary that violates it. |
| `battery.rs` | `verify_action` runs every per-action obligation — the envelope class is chosen by `envelope_class_for(suffix)`, where the suffix is the action-id tail. `verify_sequence` runs the cross-action ones (AUD2 `momentary_release` propagation). |
| `certify.rs`, `certificate.rs`, `badge.rs` | Deterministic content-hashed certificate, ed25519 signing, badge derivation (regime + fidelity tier + trademark gate). |
| `replay.rs` | JSONL replay **and** the live `--driver` stdio protocol. |
| `measure.rs` | ε ingestion from N driver-report traces into a provisional table. |
| `stochastic.rs` | The declared noise model behind `rfl sim --seed --variation`. |
| `ros2.rs` | The ROS 2 transport binding surface. |

## `rfl-cli`

Subcommands: `validate`, `retarget`, `certify`, `verify`, `keygen`, `sign`,
`badge`, `sim`, `measure`, `version`.

`rfl sim` is both the report generator (`--skill`/`--embodiment`) and the live
driver spoken to over stdio by `rfl certify --driver`, which is what lets
[`scripts/demo.sh`](../scripts/demo.sh) run the whole loop with no hardware.

## Tests and goldens

- One integration test per primitive or family in
  `crates/rfl-conformance/tests/`, with insta snapshots under
  `crates/rfl-conformance/tests/snapshots/`.
- End-to-end CLI tests in `crates/rfl-cli/tests/`.
- Certificate fixtures in `crates/rfl-conformance/tests/certificate_fixtures.rs`
  (blessed with `RFL_BLESS=1`).

## Workspace notes

`bindings/python` is **excluded** from the Cargo workspace: maturin builds it as
a PyO3 cdylib, and excluding it keeps workspace-wide `cargo test` / `cargo build`
byte-identical while letting maturin invoke cargo inside it.
