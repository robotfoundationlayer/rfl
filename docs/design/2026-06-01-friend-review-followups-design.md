# Friend-review follow-ups — paper-side handoff spec

**Status**: design + execution record (committed). Date: 2026-06-01.
**Paper source location**: the editable chapter sources live in the Obsidian vault at `Projects/RFL/whitepaper/` (per-chapter `*_en.md` / `*_ja.md`, assembled by `build_unified.sh` → unified MD → TinyTeX PDF). They are NOT in this rfl repo (`whitepaper/` here holds only the built PDFs).
**Execution done this session (EN-only, source `.md` only, no rebuild)**:
- Point 4 bridge → `spec/02-translation-layer.md` (this repo), new "Relation to the whitepaper's I1–I5 contract" section.
- B1 (VLA→symbol emitter tier) → vault `04_specification_v0.1_en.md` §4.4.
- B2 (friction honest-boundary) → vault `04_specification_v0.1_en.md` §4.3 determinism paragraph.
- A2 (UMI) → vault `10_references_en.md` (entry 27a, arXiv:2402.10329 verified) + `02_why_existing_approaches_fail_en.md` §2.2 clause.
**Remaining**: (i) author confirms the I2–I4 correspondences in the spec/02 table; (ii) `build_unified.sh` re-export + de-AI HTML check (owner: user); (iii) JA mirror of B1/B2/A2 prose (deferred); (iv) the README naming needs NO change (I1–I5 there is correct).

## Context

A friend reviewed RFL (GitHub + whitepaper, partly via an LLM pass). The triage validated each point against the current codebase. Three points map to concrete paper-side edits; one is in-repo (done); one is wontfix. This doc specifies the paper-side edits precisely so they can be executed against the paper source and re-exported.

Verification baseline this session: `cargo test --workspace` = 273 passed / 0 failed; `validate.py` C1–C7 green. Position paper `RFL_v1.0_en.pdf` (73 pp) and spec extract `RFL_SPEC_v0.1_en.pdf` (60 pp) both already carry a References section (A–F, 35+ verified entries) and treat OXE/LeRobot/GR00T/ROS 2 head-on in §2 — so the friend's "no references at all" is inaccurate for the current PDFs (likely a stale snapshot or a parser miss on the PDF reference section).

## B1 — Promote the VLA→symbol-composition defense into the paper (highest value)

**Problem**: the paper's §4.4 worked example shows a VLA emitting high-level symbolic composition (`grasp_pinch(...); reorient(...); align(...); insert(...) until seated_indicator(...)`), and states "the translation is performed by the layer, not by the model." But action-token VLAs (π0, OpenVLA, GR00T) emit low-level continuous/tokenized actions at tens of Hz and do not naturally emit this symbolic composition. The related work does not yet defend *who* produces the high-level symbolization. This is the largest unstated dependency assumption.

**The defense already exists in-repo** — it just is not woven into the paper:
- `docs/existence-proofs.md` § "The honest boundary": the Skill ISA's natural emitter is the **planner / VLM tier** (Code-as-Policies / SayCan / "System-2 VLM" archetype); action-token VLAs operate *below* the Skill ISA, near the canonical-action / driver-interface layer.
- `cobel` (`masterleopold/cobel`, public, Apache-2.0): `claude-opus-4-8` emits valid full-spec Skill ISA on 6/6 novel tasks (5/5 schema-valid at pass@k, spec-only and few-shot) — an empirical existence proof that a real frontier model in the planner tier can target the ISA.

**Edit**:
1. Add a related-work paragraph (in §2 or a dedicated subsection) positioning RFL's emitter assumption explicitly: the Skill ISA targets the **planner/VLM tier** (hierarchical-VLA / Code-as-Policies / SayCan archetype), not the action-token tier. Cite RT-2/SayCan/Code-as-Policies as the hierarchical lineage. (Verify any new citation in-session before adding.)
2. In §4.4, add one sentence making the layering explicit: the high-level symbolic composition is emitted by the planner tier; the translation layer lowers it; action-token VLAs sit below the canonical-action boundary.
3. Cite `cobel` as the empirical defense (demand-side existence proof) — the standard is *emittable* by a real model.

**Why it matters**: turns "the largest research bet, undefended" into "bet stated, and empirically supported." Material against an arXiv reviewer skim.

## B2 — Honest-boundary note: friction-based safety is declared, never runtime-measured

**Problem**: `min_holding_force` and the grasp holding capacity are derived from a *declared* friction coefficient and a schematic capacity model, `never runtime-measured` (intentional, to keep `retarget` generation byte-deterministic — RD1c). Real slip depends on contact-patch deformation, surface contamination, and load history, which a single μ does not capture. The spec/code already state this (`spec/02` § force/acceleration derivation; `crates/rfl-core/src/grasp_force.rs` header — "schematic and provider-neutral … declared bulk friction, not a vendor friction law … collapse the μ·geometry term into one documented reference number"). The friend's ask is that the paper acknowledge the consequence too.

**Edit**: if the paper's limitations/assumptions section does not already carry it, add 1–2 sentences: the safety-critical holding-force lower bound rests on a declared friction coefficient and a schematic capacity model, never runtime-measured; this is a deliberate abstraction boundary chosen to preserve generation determinism (RD1c), and embodiments needing tighter guarantees layer runtime force sensing above the contract. Frame as an accepted, disclosed trade-off, not a defect.

## A2 — Add UMI to related work (only real literature gap)

**Problem**: Open X-Embodiment and LeRobot are already cited and treated head-on (§2.2, §2.3; refs 18, 22). **UMI (Universal Manipulation Interface) is absent** from spec, README, and both PDFs — the one genuinely missing adjacent work, and the closest prior art to RFL's "interface" framing.

**Verified citation (WebFetch arxiv.org/abs/2402.10329, 2026-06-01)**:
> Chi, C., Xu, Z., Pan, C., Cousineau, E., Burchfiel, B., Feng, S., Tedrake, R., & Song, S. (2024). Universal Manipulation Interface: In-The-Wild Robot Teaching Without In-The-Wild Robots. arXiv:2402.10329. (Venue not stated on arXiv abstract page; confirm RSS 2024 at editorial polish before asserting a venue.)

**Edit**: add UMI to References category E or F, and one clause in §2 distinguishing it: UMI is a *data-collection / demonstration-transfer interface* (a hand-held gripper + policy-learning pipeline), not a cross-vendor semantic action contract with conformance — i.e., it standardizes *how demonstrations are captured*, not *how a model's intent is retargeted and verified across embodiments*. This sharpens RFL's "neutral, conformance-checkable specification" wedge rather than weakening it.

## 4 — Naming reconciliation (the real shape of the friend's point 4)

**Corrected finding** (an earlier pass mis-read this): I1–I5 is **not** a phantom. The spec extract defines the five binding invariants explicitly (`RFL_SPEC_v0.1_unified_en.md` → from the spec-extract chapter source):

- **I1** Determinism · **I2** Embodiment-respect · **I3** Composability (rest-stable boundaries) · **I4** Failure-mode preservation · **I5** Extension-namespace isolation

So the whitepaper + spec extract use **I1–I5** (a 5-invariant *conceptual contract*), and the in-repo machine-readable `spec/02-translation-layer.md` uses **CA1c–CA4c + RD1c–RD4c** (a finer *mechanization*). Neither artifact references the other's labels — that absent cross-reference is the actual substance of the friend's point 4, not a fake label.

Authoritative direction is fixed by `README.md:57`: "the whitepaper PDFs are the authoritative reference at v0.1; the in-repo `spec/*.md` files are the developing machine-readable form intended to converge with the whitepaper." So **the bridge belongs in `spec/02`, referencing the authoritative I1–I5** — not in the README (whose "I1–I5" is already correct and must stay).

**The mapping is not clean 1:1** and partly spans chapters (derive carefully, author to confirm):
- I1 Determinism ↔ RD1c (generation determinism) + RD2c (realized-execution loose carve-out)
- I2 Embodiment-respect ↔ CA1c (reachability decidable) + CA4c (envelope clamping) + the `03` capability gate
- I3 Composability ↔ primarily the `spec/01` algebra's rest-stable composition validity (not a CA/RD item) — note the cross-chapter home
- I4 Failure-mode preservation ↔ CA4c (malformed on limit breach) + the binary `capability_absent` gate / RD4c routing
- I5 Extension-namespace isolation ↔ the `spec/06` extension registry + unknown-tag rejection (`03`)

**Recommended fix (in-repo, executable here):** add a short "Relation to the whitepaper's I1–I5 contract" note in `spec/02` (and/or `spec/00`) giving the mapping above, so a reader who lands on the machine-readable spec can trace each CA/RD obligation back to the named contract. The README needs no change. Author should confirm the I2/I3/I4 correspondences before this is committed.

## Out of scope / wontfix

- **Per-skill ε-tolerance table (Class 2-loose)** — `spec/05` § Open issues: open, data-dependent, pending reference-implementation measurements (Milchick bring-up / cobel). The strict/loose split and its assignment rule are fixed; only the numbers wait. Do not fabricate values.

## Verification checklist for the paper edits

- Every new citation WebFetch-verified in-session before writing (URL + author/year/arXiv ID).
- After re-export, confirm HTML/PDF has zero `——` and no heavy em-dash clusters in prose (de-AI gate).
- Re-run `validate.py` and `cargo test` if any in-repo artifact is touched (none expected for the paper edits).
