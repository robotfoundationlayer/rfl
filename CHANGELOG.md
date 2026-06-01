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

- Six JSON schemas: `skill-isa`, `embodiment-descriptor`, `driver-interface`,
  `tactile-manifold/adapter`, `certificate`, `extension-registry`.
- `schemas/validate.py` enforces well-formedness, reference-instance validation,
  and the eight cross-schema anti-drift invariants C1–C8 — now run in CI.

### Reference implementation

- `rfl-core` lowers the **full 50-primitive Skill ISA** across all seven
  categories onto three embodiment descriptors, deterministically.
- `rfl-conformance` implements all four normative envelope classes plus the
  grasp-continuity (GC1–6), stability (STB2/STB3), audit (AUD1–3), reversibility
  (REV1–3), force-event, contact-band, capability-gate, abort-timing, and
  freed-part-disposition checks, each against adversarial drivers.
- `rfl` CLI: `validate`, `retarget`, `certify` (replay or live `--driver`),
  `verify`, `sign`, `keygen`, `spec-version` — see
  [docs/cli-reference.md](docs/cli-reference.md).
- Third-party self-certification: deterministic, content-hashed, optionally
  ed25519-signed conformance certificates.

### Documentation

- Getting-started + hardware-free `scripts/demo.sh`, CLI reference, Skill and
  embodiment authoring guides, the certifying-a-driver on-ramp, and the
  spec↔whitepaper convergence tracker.

### Tooling

- CI gates: rustfmt, clippy (`-D warnings`), workspace tests (Linux + macOS),
  schema validation, and a markdown doc linter (links, dividers, de-AI).

[Unreleased]: https://github.com/robotfoundationlayer/rfl/commits/main
