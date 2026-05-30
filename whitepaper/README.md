# Whitepaper

The project's argument is published in **two complementary documents**, both checked into this directory.

## The two documents

| Document | Audience | Pages | Genre |
|---|---|---|---|
| [`RFL_SPEC_v0.1_en.pdf`](RFL_SPEC_v0.1_en.pdf) | Academic peers, implementers, arXiv reviewers | 47 | **Specification proposal** (academic register) |
| [`RFL_position_paper_v1.0_en.pdf`](RFL_position_paper_v1.0_en.pdf) | Industry stakeholders, prospective partners, ecosystem readers | 72 | **Position paper** (specification + network-effect analysis + call for partners) |

### Which to read

- **If you want the technical specification only**: read the spec extract (`RFL_SPEC_v0.1_en.pdf`). It covers the coordination problem, why existing approaches fail, the five constitutional principles, the three-layer specification (Skill ISA + Translation Layer interface contract + Driver Interface), comparative analysis against ARM/CUDA/USB/ROS 2/LeRobot, the implementation roadmap, and the supermodularity property of bilateral translation.
- **If you want the strategic / institutional / network-effect framing as well**: read the position paper (`RFL_position_paper_v1.0_en.pdf`). It contains everything the spec extract contains, plus a chapter on network effects and ecosystem design (§ 5), a chapter on foundation governance (§ 7), and a chapter inviting founding partners (§ 9). The position paper's own preface marks it as a position paper (not a peer-reviewable academic paper) and explains the choice.

A legacy filename `RFL_v1.0_en.pdf` is maintained as a symbolic alias to the position paper to preserve external links.

## Status (2026-05-30)

| Item | Status |
|---|---|
| **Spec extract (English, v0.1)** | Drafted, intended for arXiv submission |
| **Position paper (English, v1.0)** | Drafted |
| **Position paper (Japanese, v1.0)** | Drafted (not yet in this repository) |
| **arXiv preprint ID** | Pending endorsement; will be filled once accepted |
| **17 figures** | Rendered to SVG and indexed below |

## Figures

The 17 figures referenced from the white paper are rendered as standalone SVG files under [`figures/`](figures/):

| Figure | Title | Section |
|---|---|---|
| F1.1 | The N × M Combinatorial Crisis | § 1.1 |
| F1.2 | Per-Pair Integration Cost Decomposition | § 1.2 |
| F1.3 | Fragmentation Tax Decomposition (4 components) | § 1.3 |
| F1.4 | Triple Inflection Timeline (2020–2027) | § 1.4 |
| F2.1 | Adjacent-Effort Capability Matrix | § 2 |
| F3.1 | Five First Principles (Pentagon) | § 3.2 |
| F3.2 | RFL ↔ ARM ISA Isomorphism | § 3.3 |
| F4.1 | Three-Layer RFL Architecture | § 4 intro |
| F4.4 | Worked Example — One Instruction, Three Embodiments | § 4.4 |
| F5.1 | Two-Sided Market Structure of Physical AI | § 5.1 |
| F5.2 | Bilateral Intelligence Spectrum (L0–L4) for VLA × Embodiment | § 5.2 |
| F5.4 | Two-Sided Cold Start — Hard Side First | § 5.3 |
| F5.5 | Nine Reinforcement Loops (Causal Diagram) | § 5.4 |
| F5.8 | Helmer 7 Powers Comparison | § 5.6 |
| F6.1 | Three-Track Roadmap Timeline | § 6 |
| F7.1 | Foundation Governance Structure | § 7 |
| F8.6 | Coordination-Layer Positioning | § 8.6 |

## Structure

1. The Embodiment Crisis — quantification of the O(N×M) integration burden
2. Why Existing Approaches Fail to Occupy the Coordination Layer — ROS 2, Open X-Embodiment, LeRobot, VLAs themselves, NVIDIA Isaac/GR00T
3. RFL: First Principles — the five constitutional commitments
4. The RFL Specification v0.1 — Skill ISA + Translation Layer + Driver Interface
5. Network Effects and Ecosystem Design — two-sided market + nine reinforcement loops + Master Equation
6. Implementation Roadmap
7. Stewardship Governance
8. Comparative Analysis — ARM, CUDA, USB, ROS 2, LeRobot, contemporary cohort
9. Call to Action — Lead Customer recruitment

Appendices: Data Sources and Methodology, TactileManifold formal specification, Supermodularity and Master Equation derivations.

## Citation

```bibtex
@misc{hara2026rfl,
  title   = {The Embodiment Abstraction: A Foundation Layer for Physical AI},
  author  = {Yoichiro Hara},
  year    = {2026},
  note    = {arXiv preprint forthcoming; project page \url{https://github.com/robotfoundationlayer}}
}
```
