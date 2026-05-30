# rfl — Robot Foundation Layer

> A neutral, semantically-typed abstraction layer between Vision-Language-Action foundation models and robotic embodiments.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Status: pre-release](https://img.shields.io/badge/Status-pre--release-orange)](#status)

## What is RFL?

The physical-AI ecosystem of 2026 faces a coordination crisis. Nine credible Vision-Language-Action (VLA) foundation models and roughly thirty distinguishable robotic embodiments yield a combinatorial integration burden — a median of approximately **$300,000 per (VLA, embodiment) pair**, with annual aggregate spend plausibly on the order of low single-digit billions of dollars and rising.

**RFL** is a three-layer specification that collapses this O(N×M) integration cost to additive cost:

| Layer | Purpose |
|---|---|
| **Skill ISA** | 50 manipulation primitives across 7 categories + a compositional algebra |
| **Translation Layer** | Canonical embodiment-agnostic action representation with deterministic retargeting |
| **Driver Interface** | ROS 2-compatible protocol for embodiment-side compliance |

Five constitutional principles govern every design choice: **embodiment-agnostic, compositional, verifiable, provider-neutral, forward-compatible.**

The full motivation, derivation, comparative analysis, and network-effect economics are presented in the [whitepaper](whitepaper/) (arXiv link forthcoming).

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

This repository is **pre-release**. The specification draft v0.1 is targeted for **2027 Q1**, accompanied by:

- Reference implementation (`crates/rfl-core` — Rust + Python/C bindings)
- Conformance test suite (`crates/rfl-conformance`)

Watch this repository to follow development. Comments and design feedback are welcome via [Issues](https://github.com/robotfoundationlayer/rfl/issues).

## License

[Apache License 2.0](LICENSE) in perpetuity. The specification is intended for stewardship transfer to a Linux Foundation-hosted subsidiary in Stage 2 of the project's staged-donation roadmap; the Apache 2.0 commitment survives the transfer by construction.

## Citation

```bibtex
@misc{hara2026rfl,
  title   = {The Embodiment Abstraction: A Foundation Layer for Physical AI},
  author  = {Yoichiro Hara},
  year    = {2026},
  note    = {arXiv preprint forthcoming; project page \url{https://github.com/robotfoundationlayer}}
}
```

## Contact

- **Author**: Yoichiro Hara
- **Email**: <yo@vox.delivery>
- **Operating entity**: Vox Technologies, Inc. (Delaware C-corp)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding conventions, and PR guidelines. All contributors agree to the [Code of Conduct](CODE_OF_CONDUCT.md).
