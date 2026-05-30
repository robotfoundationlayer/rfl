# rfl — Robot Foundation Layer

> A neutral, semantically-typed abstraction layer between Vision-Language-Action foundation models and robotic embodiments.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Status: pre-release](https://img.shields.io/badge/Status-pre--release-orange)](#status)

## What is RFL?

The physical-AI ecosystem of 2026 faces a coordination problem. Approximately nine credible Vision-Language-Action (VLA) foundation models and approximately thirty distinguishable robotic embodiments yield a combinatorial integration burden plausibly large enough to constrain industrial deployment (no published per-pair benchmark exists yet; the project commits to publishing one against the v0.1 nine-cell support matrix during the 2027 Q1–Q3 review period).

**RFL** is a three-layer specification that collapses this O(N×M) integration cost to additive cost:

| Layer | Purpose |
|---|---|
| **Skill ISA** | ~50 manipulation primitives across 7 categories + a compositional algebra |
| **Translation Layer** | Canonical embodiment-agnostic action representation with deterministic retargeting interface contract (5 binding invariants I1–I5) |
| **Driver Interface** | ROS 2-compatible protocol for embodiment-side compliance |

Five constitutional principles govern every design choice: **embodiment-agnostic, compositional, verifiable, provider-neutral, forward-compatible.**

The full motivation, derivation, comparative analysis, and network-effect economics are presented in the [whitepaper](whitepaper/), which is structured as a **companion pair**:
- **[`RFL_SPEC_v0.1_en.pdf`](whitepaper/RFL_SPEC_v0.1_en.pdf)** (60 pages) — academic specification extract, intended for arXiv submission
- **[`RFL_v1.0_en.pdf`](whitepaper/RFL_v1.0_en.pdf)** (73 pages) — position paper with full network-effect, governance, and ecosystem framing

## Repository layout

```
rfl/
├── spec/                 # Specification documents (Apache 2.0)
├── schemas/              # JSON schemas for the spec
├── crates/               # Rust workspace
│   ├── rfl-core/         # Skill ISA parser + Translation Layer
│   ├── rfl-cli/          # CLI tool (validate, retarget, ...)
│   └── rfl-conformance/  # Conformance test runner
├── bindings/             # Python (PyO3) + C (cbindgen) bindings
├── examples/             # Worked examples
├── conformance/          # Conformance test suite
├── docs/                 # Documentation site
└── whitepaper/           # Whitepaper + figures
```

## Status (2026-05-30)

This repository is **pre-release**.

**Specification draft v0.1**: Published as `whitepaper/RFL_SPEC_v0.1_en.pdf` in publication-candidate state after four rounds of external review. The architectural commitments — the three-layer division, the ~50 primitives and their categorization, the compositional algebra, the canonical action tuple, the retargeting interface contract and its five binding invariants, the TactileManifold abstraction, the split conformance regime (Class 2-strict / Class 2-loose), the stewardship commitments C1–C5 — are intended to be stable. Schema details, error codes, the extension registry, and the per-skill ε-tolerance table for Class 2-loose remain open through the v0.1 review period.

**Targeted milestones**:

- **2027 Q1**: v0.1 community review release; arXiv preprint
- **2027 Q2–Q3**: Reference implementation v0.0.1 (single (VLA, embodiment) pair existence proof — π0 × LEAP or similar — to demonstrate the retargeting contract is constructible; measured integration cost published)
- **2027 Q4**: v1.0 stabilization release with full reference implementation (`crates/rfl-core` — Rust + Python/C bindings) and conformance test suite (`crates/rfl-conformance`)

Watch this repository to follow development. Comments and design feedback are welcome via [Issues](https://github.com/robotfoundationlayer/rfl/issues).

## License

[Apache License 2.0](LICENSE) in perpetuity. The specification is intended for stewardship transfer to a Linux Foundation-hosted subsidiary in Stage 2 of the project's staged-donation roadmap; the Apache 2.0 commitment survives the transfer by construction.

## Citation

For the **specification extract** (academic register, arXiv submission target):

```bibtex
@misc{hara2026rflspec,
  title   = {The Embodiment Abstraction: A Specification for the Coordination Layer between Vision-Language-Action Models and Robotic Embodiments},
  author  = {Yoichiro Hara},
  year    = {2026},
  note    = {RFL Specification v0.1, arXiv preprint forthcoming; project page \url{https://github.com/robotfoundationlayer}}
}
```

For the **companion position paper** (network-effect and ecosystem framing):

```bibtex
@misc{hara2026rflposition,
  title   = {The Embodiment Abstraction: A Foundation Layer for Physical AI},
  author  = {Yoichiro Hara},
  year    = {2026},
  note    = {RFL Position Paper v1.0; project page \url{https://github.com/robotfoundationlayer}}
}
```

## Contact

- **Author**: Yoichiro Hara
- **Email**: <yo@vox.delivery>
- **Operating entity**: Vox Technologies, Inc. (Delaware C-corp)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding conventions, and PR guidelines. All contributors agree to the [Code of Conduct](CODE_OF_CONDUCT.md).
