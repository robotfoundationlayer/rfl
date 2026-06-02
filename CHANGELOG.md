# Changelog

All notable changes to this repository are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project versions in
lockstep across the workspace (see
[docs/packaging-and-versioning.md](docs/packaging-and-versioning.md)) and tracks
the specification version (`rfl spec-version`).

## [Unreleased]

The repository is **pre-release**; nothing has been tagged yet. The first tagged
release is the **v0.1 community-review** milestone (targeted 2026 Q3). This
section accumulates what that release will contain.

### Specification

- Chapters `00`–`06` design-complete (overview, Skill ISA, Translation Layer,
  Driver Interface, TactileManifold, Conformance, Extension Registry).
- The cross-chapter open-issues TODO is closed; the per-skill ε-tolerance table
  (data-dependent) is the one item open through the review period.

### Schemas (conformance test class 1)

- **Eight** JSON schemas: `skill-isa`, `embodiment-descriptor`,
  `driver-interface`, `tactile-manifold/adapter`, `certificate`,
  `extension-registry`, `epsilon-tolerance`, `simulator-declaration`.
- `schemas/validate.py` enforces well-formedness, reference-instance validation,
  and the **eleven** cross-schema anti-drift invariants C1–C11 — run in CI. C8
  (extension-registry consistency), C9 (ε-table key completeness), C10 (the
  recursive-simulator no-self-bootstrap rule), and C11 (the simulator's
  `variation_model` names only ε-table quantities) were added this cycle.
- The per-skill ε-tolerance table now has a **format** (`epsilon-tolerance`
  schema + `epsilon-tolerances.yaml`) and complete **ingestion tooling** (see
  below); its values remain `null` pending hardware-anchored measurement.

### Reference implementation

- `rfl-core` lowers the **full 50-primitive Skill ISA** across all seven
  categories onto three embodiment descriptors, deterministically — including
  the Σ **arc** sweep generator. The canonical execute wire is now
  **bidirectional** (`to_jsonl` / `from_jsonl`): a driver can parse the goals it
  receives.
- `rfl-conformance` implements all four normative envelope classes plus the
  grasp-continuity (GC1–6), stability (**STB1**/STB2/STB3, STB1 via the new
  `contact_geometry` telemetry field), audit (AUD1–3), reversibility (REV1–3),
  force-event, contact-band, capability-gate, abort-timing, and
  freed-part-disposition checks, each against adversarial drivers. New: the
  complete ε **measurement pipeline** — the `measure` aggregation (table-driven,
  keyed by the committed ε-table; kinematic/wrench quantities from their wire
  channels, domain scalars from the opt-in `measured_quantities` telemetry map),
  the realized-pose contents now carried through replay so pose deviation grades,
  and the **stochastic reference sim** (a declared `variation_model` perturbs the
  nominal driver, so a seed sweep yields a provisional, sim-derived ε); the
  conformance **badge** derivation; and the canonical-message → **ROS 2**
  interface classification.
- `rfl` CLI: `validate`, `retarget`, `certify` (replay or live `--driver`),
  `verify`, `sign`, `keygen`, `badge`, `sim`, `measure`, `spec-version` — see
  [docs/cli-reference.md](docs/cli-reference.md). `rfl sim` is the reference
  simulator driver (regenerate, a live stdin `--driver`, and a stochastic
  `--seed --variation` mode); `rfl measure` builds a provisional ε table from N
  driver-report traces (`scripts/provisional-epsilon-from-sim.sh` runs the
  hardware-free sweep).
- Third-party self-certification: deterministic, content-hashed, optionally
  ed25519-signed conformance certificates.

### Bindings

- **C** ABI binding (`bindings/c`, via cbindgen): `rfl_retarget` /
  `rfl_string_free` / `rfl_spec_version`.
- **ROS 2** interface package (`bindings/ros2/rfl_msgs`): the `Execute` action,
  `Telemetry` topic, and `ClearanceQuery` service realizing the spec/03 driver
  messages (the live `rclrs` node awaits a ROS 2 environment).
- Python (PyO3): the minimal `rfl.retarget` binding.

### Documentation

- Getting-started + hardware-free `scripts/demo.sh` (driven by the reference
  simulator), CLI reference, Skill and embodiment authoring guides, per-example
  READMEs, the certifying-a-driver and registering-an-extension on-ramps, the
  integration-cost methodology, the packaging/versioning guide, and the
  spec↔whitepaper convergence tracker.

### Governance and security

- `GOVERNANCE.md` (stewardship model), `SECURITY.md` (disclosure policy),
  a PR template, and Dependabot.

### Tooling

- CI gates (8 jobs): rustfmt, clippy (`-D warnings`), workspace tests
  (Linux + macOS), schema validation, a markdown doc linter (links, dividers,
  de-AI), **cargo-deny** (licenses + advisories), and an **MSRV** check
  (Rust 1.86) — under least-privilege workflow permissions.

[Unreleased]: https://github.com/robotfoundationlayer/rfl/commits/main
