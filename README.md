# rfl — Robot Foundation Layer

> A neutral, semantically-typed abstraction layer between Vision-Language-Action foundation models and robotic embodiments.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Status: pre-release](https://img.shields.io/badge/Status-pre--release-orange)](#status-2026-05-31)
[![Spec: v0.1 draft](https://img.shields.io/badge/Spec-v0.1_draft-green)](whitepaper/RFL_SPEC_v0.1_en.pdf)

## TL;DR

Approximately **nine** credible VLA foundation models. Approximately **thirty** robotic embodiments. Every new pair currently requires bespoke integration. RFL is a three-layer specification — Skill ISA + retargeting interface contract + ROS 2-compatible Driver Interface — that collapses the O(N×M) integration burden to additive cost, governed by five constitutional principles and stewarded under Apache 2.0 in perpetuity.

**📄 Read it now**: [`RFL_SPEC_v0.1_en.pdf`](whitepaper/RFL_SPEC_v0.1_en.pdf) (60-page specification extract, publication-candidate state).
**🧭 Read the broader framing**: [`RFL_v1.0_en.pdf`](whitepaper/RFL_v1.0_en.pdf) (73-page companion position paper).

## What is RFL?

The physical-AI ecosystem of 2026 faces a coordination problem. Approximately nine credible Vision-Language-Action (VLA) foundation models — Physical Intelligence's π0, Google DeepMind's Gemini Robotics, Figure's Helix, NVIDIA's GR00T, Stanford/UCB/TRI's OpenVLA, Hugging Face's SmolVLA and VLA-JEPA lines, Tsinghua's RDT-1B, UC Berkeley's Octo — and approximately thirty distinguishable robotic embodiments (humanoids, dexterous hands, collaborative arms) yield a combinatorial integration burden plausibly large enough to constrain industrial deployment.[^1]

**RFL** is a three-layer specification that collapses this O(N×M) integration cost to additive cost:

| Layer | Purpose |
|---|---|
| **Skill ISA** | ~50 manipulation primitives across 7 categories + a compositional algebra |
| **Translation Layer** | Canonical embodiment-agnostic action representation with deterministic retargeting interface contract (5 binding invariants I1–I5) |
| **Driver Interface** | ROS 2-compatible protocol for embodiment-side compliance |

Five constitutional principles govern every design choice: **embodiment-agnostic, compositional, verifiable, provider-neutral, forward-compatible.**

[^1]: No published per-pair benchmark exists yet. The project commits to publishing one against the v0.1 nine-cell support matrix during the 2027 Q1–Q3 review period.

## The whitepaper

The project's argument is published as a **companion pair** in [`whitepaper/`](whitepaper/):

| Document | Pages | Audience |
|---|---|---|
| [`RFL_SPEC_v0.1_en.pdf`](whitepaper/RFL_SPEC_v0.1_en.pdf) | 60 | Academic peers, implementers, arXiv reviewers — specification extract (academic register) |
| [`RFL_v1.0_en.pdf`](whitepaper/RFL_v1.0_en.pdf) | 73 | Industry stakeholders, prospective partners — position paper with network-effect, governance, and ecosystem framing |

The two documents share Abstract, Keywords, References, and Appendix A (TactileManifold) by construction; everything else is independently developed. The spec extract carries the testable commitments; the position paper carries the institutional and strategic framing. See [`whitepaper/README.md`](whitepaper/README.md) for the full split.

## The specification (in-repo)

The whitepaper is the long-form argument; the [`spec/`](spec/) directory hosts the machine-readable specification artifacts as they mature:

| File | Status |
|---|---|
| [`spec/00-overview.md`](spec/00-overview.md) | Design complete — the five constitutional principles (embodiment-agnostic / compositional / verifiable / provider-neutral / forward-compatible), specification scope and deliberate non-scope, and governance; the foundation the subsequent chapters instantiate |
| [`spec/01-skill-isa.md`](spec/01-skill-isa.md) | Design complete — 50 primitives, type system, algebra (predicates / three-valued verdict), world-state model, composition validity |
| [`spec/02-translation-layer.md`](spec/02-translation-layer.md) | Design complete — canonical action + `Envelope`, quaternion pose + single-scalar geodesic error, determinism boundary (Class 2-strict / 2-loose), grasp-force / stability derivations, trajectory generation + time-scaling; § Open issues (the cross-chapter design driver) fully resolved |
| [`spec/03-driver-interface.md`](spec/03-driver-interface.md) | Design complete — frame model, capability manifest, collision model, sensor / gravity / safety descriptors, multi-embodiment addressing, canonical driver messages (execute / telemetry / status + clearance-query service) |
| [`spec/04-tactile-manifold.md`](spec/04-tactile-manifold.md) | Design complete — feature-field model, feature taxonomy, contact-sensor descriptor, `TactileTarget`, graceful-degradation proxy, slip / force-event / deformation discrimination, freed-part handling, sensing-scope contracts, per-sensor-class adapter mapping |
| [`spec/05-conformance.md`](spec/05-conformance.md) | Design complete — envelope-class taxonomy, grasp-continuity modes, closure / stability / composition, reversibility + irreversible-operation safety, hazardous-operation benches, audit trail, conformance regime (determinism floor / fidelity tier / recursive simulator) |
| [`spec/06-extension-registry.md`](spec/06-extension-registry.md) | Skeleton |

The whitepaper PDFs above are the authoritative reference at v0.1; the in-repo `spec/*.md` files are the developing machine-readable form intended to converge with the whitepaper by v1.0 (2027 Q4).

## Repository layout

```
rfl/
├── whitepaper/           # ✅ Whitepaper PDFs + figures (60 + 73 pages)
├── spec/                 # ✅ Specification documents (Markdown, work in progress)
├── examples/             # ✅ Worked examples (01-cable-insertion, 02-surface-scan, 03-screw-fasten)
├── crates/               # ✅ Rust workspace (rfl-core / rfl-cli / rfl-conformance): rfl-cli retarget engine across three worked examples and their skill variants (structural; generative Σ raster + spiral; station-keeping reach.hover; held-interval transport.carry; mass-dependent grasp-force GF1c–GF4c incl. tool-mediated force.screw / force.unscrew + held-transport GC1); conformance test classes 1–3 green with all four envelope classes implemented (terminal / grasp-continuity / force-trajectory / interval-invariant), each verified against adversarial drivers; full 50-primitive coverage + bindings → v1.0
├── docs/                 # ✅ Docs: getting-started.md + design/ (per-increment reference-implementation design records)
├── schemas/              # ✅ Four JSON schemas + validator (skill-isa, embodiment-descriptor, driver-interface, tactile-manifold/adapter — all precisely typed; validate.py = conformance test class 1 with anti-drift invariants C1–C7)
└── bindings/             # ⏳ Python (PyO3) + C (cbindgen) bindings (planned for the v1.0 stabilization milestone, 2027 Q4)
```

Legend: ✅ populated · 🚧 scaffold present, content pending · ⏳ planned, not yet created

## Status (2026-05-31)

This repository is **pre-release**. Spec v0.1 has been published in publication-candidate state after four rounds of external review; reference-implementation engineering is now underway (the `rfl-cli retarget` engine is implemented for the worked-example paths).

### Stable architectural commitments

The three-layer division, the ~50 primitives and their categorization, the compositional algebra, the canonical action tuple, the retargeting interface contract and its five binding invariants, the TactileManifold abstraction, the split conformance regime (Class 2-strict / Class 2-loose), and the stewardship commitments C1–C5 (with operational mechanism for C1's 30%-rebalancing process and C4's transparency mechanics) are intended to be stable.

### Open through the v0.1 review period

All four machine-readable schemas (`skill-isa`, `embodiment-descriptor`, `driver-interface`, and `tactile-manifold/adapter`) are now precisely typed against their `spec/` parameter tables and backed by a committed conformance-test-class-1 validator (`schemas/validate.py`, with seven cross-schema anti-drift invariants C1–C7, run via `uv run --with jsonschema --with pyyaml python schemas/validate.py`). What remains open through the review period: the extension registry (`spec/06`) and the per-skill ε-tolerance table for Class 2-loose conformance (data-dependent, pending reference-implementation measurements). Reference-implementation engineering is well underway: the `rfl-cli retarget` engine deterministically retargets the three worked examples (cable insertion, with a `transport.carry` held-under-disturbance variant; surface scan with raster, spiral, and `reach.hover` station-keeping variants; screw fastening with `force.screw` and `force.unscrew`) onto three embodiment descriptors, including the mass-dependent grasp-force derivations (GF1c–GF4c, with the tool-mediated torque reaction for `force.screw` / `force.unscrew`) and held-transport grasp-continuity propagation, and is backed by conformance test classes 2 and 3 — byte-deterministic retarget goldens, the driver-protocol round-trip, and **all four normative envelope classes** (terminal-postcondition, grasp-continuity, force/torque-trajectory, and interval-invariant, the last now exercised by both `reach.hover` station-keeping and `transport.carry` held-under-disturbance), each verified against adversarial drivers. Full 50-primitive coverage and the language bindings remain v1.0 targets.

### Targeted milestones

| Quarter | Milestone |
|---|---|
| **2027 Q1** | v0.1 community review release; arXiv preprint |
| **2027 Q2–Q3** | Reference implementation v0.0.1 — single (VLA, embodiment) pair existence proof (π0 × LEAP or similar) demonstrating the retargeting contract is constructible; measured integration cost published |
| **2027 Q4** | v1.0 stabilization with full reference implementation (`crates/rfl-core` — Rust + Python/C bindings) and conformance test suite (`crates/rfl-conformance`) |

Watch this repository to follow development. Comments and design feedback welcome via [Issues](https://github.com/robotfoundationlayer/rfl/issues).

## License

[Apache License 2.0](LICENSE) in perpetuity. The specification is intended for stewardship transfer to a Linux Foundation-hosted subsidiary in a later phase of the project's roadmap; the Apache 2.0 commitment survives the transfer by construction (Stewardship Commitment C2 — see Spec § 6.5).

## Citation

```bibtex
@misc{hara2026rflspec,
  title   = {The Embodiment Abstraction: A Specification for the Coordination Layer between Vision-Language-Action Models and Robotic Embodiments},
  author  = {Yoichiro Hara},
  year    = {2026},
  note    = {RFL Specification v0.1, arXiv preprint forthcoming; project page \url{https://github.com/robotfoundationlayer}}
}
```

This entry covers the [specification extract](whitepaper/RFL_SPEC_v0.1_en.pdf). To cite the companion [position paper](whitepaper/RFL_v1.0_en.pdf) instead, adjust the `title` to "The Embodiment Abstraction: A Foundation Layer for Physical AI" and the `note` to "RFL Position Paper v1.0".

## Contact

- **Author**: Yoichiro Hara
- **Operating entity**: Vox Technologies, Inc.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding conventions, and PR guidelines. All contributors agree to the [Code of Conduct](CODE_OF_CONDUCT.md).
