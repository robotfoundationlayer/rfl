# Whitepaper

The project's argument is published in **two complementary documents**, both checked into this directory.

## The two documents

| Document | Audience | Pages | Genre |
|---|---|---|---|
| [`RFL_SPEC_v0.1_en.pdf`](RFL_SPEC_v0.1_en.pdf) | Academic peers, implementers, arXiv reviewers | **60** | **Specification proposal** (academic register) |
| [`RFL_v1.0_en.pdf`](RFL_v1.0_en.pdf) | Industry stakeholders, prospective partners, ecosystem readers | **73** | **Position paper** (specification + network-effect analysis + call for partners) |

### Which to read

- **If you want the technical specification only**: read the spec extract (`RFL_SPEC_v0.1_en.pdf`). It covers the coordination problem, why existing approaches fail, the five constitutional principles, the three-layer specification (Skill ISA + Translation Layer interface contract with 5 binding invariants + Driver Interface), a reference retargeting recipe (grasp_pinch on Allegro Hand) demonstrating I1–I5 satisfaction by construction, the split conformance regime (Class 2-strict for byte-equality / Class 2-loose for bounded-difference equivalence admitting learned native-precision retargeters), the stewardship commitments C1–C5 with operational mechanism for C1 (30%-rebalancing process, three-tier membership) and C4 (RFC and webcast mechanics), the cold-start adoption strategy, comparative analysis against ARM/CUDA/USB/ROS 2/LeRobot, and the implementation roadmap to v1.0 (2027 Q4). The conditional supermodularity property of bilateral translation is in Appendix B.
- **If you want the strategic / institutional / network-effect framing as well**: read the position paper (`RFL_v1.0_en.pdf`). It overlaps with the spec extract on the technical core (§§ 3–4) and adds: the full Master Equation derivation, the nine reinforcement loops (§ 5), the full Foundation governance design (§ 7), the Crémer-McLean structural parallel and empirical-evidence discussion for bilateral compounding, the Call to Action and Founding Consortium recruitment frame (§ 9), and Comparative Analysis at greater length (§ 8). The position paper's own preface marks it as a position paper (not a peer-reviewable academic paper) and explains the choice.

The two documents share Abstract content, Keywords, References, and Appendix A (TactileManifold formal specification) by construction; everything else is independently developed.

A legacy filename `RFL_position_paper_v1.0_en.pdf` (294 KB, 21:43) exists from an earlier naming convention. `RFL_v1.0_en.pdf` is the canonical filename for the position paper (newer, used in external citation including the Sergey Levine endorsement request).

## Status (2026-05-30)

| Item | Status |
|---|---|
| **Spec extract (English, v0.1)** | Publication-candidate state after four rounds of external review; arXiv submission prepared, awaiting endorsement |
| **Position paper (English, v1.0)** | Drafted, sync with spec v0.1 cross-references complete |
| **Position paper (Japanese, v1.0)** | Drafted (vault source only, not yet rendered in this repository) |
| **arXiv preprint ID** | Pending endorsement (Sergey Levine, sent 2026-05-30); will be filled once accepted |
| **Reference implementation v0.0.1** | Targeted 2027 Q2–Q3 (single (VLA, embodiment) pair existence proof) |
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

For the **specification extract** (academic register, arXiv submission target):

```bibtex
@misc{hara2026rflspec,
  title   = {The Embodiment Abstraction: A Specification for the Coordination Layer between Vision-Language-Action Models and Robotic Embodiments},
  author  = {Yoichiro Hara},
  year    = {2026},
  note    = {RFL Specification v0.1, arXiv preprint forthcoming; project page \url{https://github.com/robotfoundationlayer}}
}
```

For the **companion position paper**:

```bibtex
@misc{hara2026rflposition,
  title   = {The Embodiment Abstraction: A Foundation Layer for Physical AI},
  author  = {Yoichiro Hara},
  year    = {2026},
  note    = {RFL Position Paper v1.0; project page \url{https://github.com/robotfoundationlayer}}
}
```
