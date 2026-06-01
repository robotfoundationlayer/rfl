# Spec ↔ whitepaper convergence tracker

The project publishes its argument as two whitepaper PDFs and, in parallel,
develops the machine-readable specification in [`spec/`](../spec/). These are
**two forms of one contract** at different granularities. This page tracks how
far the in-repo `spec/*.md` chapters have converged with the authoritative
whitepaper, and what deltas remain.

## Authority and direction

- **The whitepaper PDFs are authoritative at v0.1.** The technical specification
  is [`whitepaper/RFL_SPEC_v0.1_en.pdf`](../whitepaper/RFL_SPEC_v0.1_en.pdf) (60
  pages, academic register); the broader framing is
  [`whitepaper/RFL_v1.0_en.pdf`](../whitepaper/RFL_v1.0_en.pdf) (73 pages,
  position paper). See [`whitepaper/README.md`](../whitepaper/README.md).
- **The in-repo `spec/*.md` is the developing machine-readable form**, intended
  to converge with the whitepaper by **v1.0 (2027 Q2–Q3)** (README § The
  specification). Where the two differ in *granularity*, the spec is the finer
  form: the whitepaper states a contract as prose invariants, the spec
  mechanizes it as the obligations a conformance test reads. They are not
  competing claims.
- **Whitepaper source lives in the Obsidian vault** (`Projects/RFL/whitepaper/`),
  not this repo; this repo carries built **PDF mirrors** only. A spec change that
  should also change the published argument is mirrored into the vault chapter
  set separately.

## Chapter convergence

| In-repo chapter | Whitepaper counterpart | Bridge | State |
|---|---|---|---|
| [`spec/00-overview`](../spec/00-overview.md) | § 3 First Principles (five constitutional commitments) + § 7 governance | implicit | Converged — the five principles + scope + governance match the whitepaper's § 3 / § 7 |
| [`spec/01-skill-isa`](../spec/01-skill-isa.md) | § 4 (Skill ISA layer of the three-layer spec) | implicit | Spec finer — 50 typed primitives + algebra + composition validity mechanize § 4's Skill ISA |
| [`spec/02-translation-layer`](../spec/02-translation-layer.md) | § 4 (Translation Layer interface contract, **I1–I5**) + § 4.4 worked recipe + Appendix B (supermodularity) | **explicit** | Spec finer, **traced** — see § The I1–I5 bridge below |
| [`spec/03-driver-interface`](../spec/03-driver-interface.md) | § 4 (Driver Interface layer) | implicit | Spec finer — frame model + capability manifest + canonical messages mechanize § 4's driver layer |
| [`spec/04-tactile-manifold`](../spec/04-tactile-manifold.md) | **Appendix A** (TactileManifold formal specification) | implicit | Converged by construction — the two documents share Appendix A |
| [`spec/05-conformance`](../spec/05-conformance.md) | § 4 (split conformance regime: Class 2-strict / 2-loose) + § 6 roadmap | implicit | Spec finer — the four test classes + envelope taxonomy + audit regime mechanize the whitepaper's conformance prose |
| [`spec/06-extension-registry`](../spec/06-extension-registry.md) | Principle 5 (forward-compatible) — the registry mechanism is elaborated in the spec beyond the whitepaper's prose | implicit | Spec ahead — the registry mechanism (surfaces, accept/reject, lifecycle) is fuller in-repo; to fold back into the whitepaper at v1.0 |

"Bridge = explicit" means the chapter carries a named **Relation to the
whitepaper** section tracing each whitepaper claim to its mechanized form;
"implicit" means the chapter is faithful but carries no such explicit
cross-reference yet. Adding an explicit bridge section to each chapter is the
remaining convergence work (§ Open deltas).

## The I1–I5 bridge (the one explicit trace today)

`spec/02` § *Relation to the whitepaper's I1–I5 contract* is the worked model for
how a chapter should trace to the whitepaper. The whitepaper states the
retargeting contract as five binding invariants **I1–I5**; `spec/02` mechanizes
them as the finer obligations **CA1c–CA4c** (canonical-action) and **RD1c–RD4c**
(retarget-determinism). The mapping is a **traceability map, not a 1:1 rename**
(I2–I4 each span more than one obligation, and I3 lives in the `spec/01`
algebra, not `spec/02`):

| Whitepaper invariant | Mechanized by |
|---|---|
| I1 — Determinism | RD1c (generation byte-determinism) + RD2c (Class 2-loose realized-execution boundary) |
| I2 — Embodiment-respect | CA1c (pose reachability) + CA4c (envelope clamped to declared limits) + the `03` `capability_absent` gate |
| I3 — Composability (rest-stable) | the `01` compositional-algebra composition validity (not a `02` obligation; recorded for traceability) |
| I4 — Failure-mode preservation | CA4c (over-limit action is malformed) + the binary `capability_absent` refusal |
| I5 — Extension-namespace isolation | the `06` registry pass-through + the `03` unknown-tag rejection |

`I1–I5` is the authoritative public statement; `CA*c` / `RD*c` are the same
contract at conformance-test granularity.

## Open deltas

| Delta | Where | Status |
|---|---|---|
| Per-skill **ε-tolerance table** (Class 2-loose bounds) | `spec/02` RD2c / `spec/05` | Open in **both** forms — data-dependent, pending reference-implementation measurements. Not a divergence, a shared gap. |
| Explicit **Relation to the whitepaper** sections on chapters 00, 01, 03, 04, 05, 06 | `spec/*` | To author — `spec/02`'s I1–I5 section is the template; each chapter should trace its whitepaper claims the same way before the v1.0 freeze. |
| Registry mechanism fold-back | `spec/06` ↔ whitepaper Principle 5 | The in-repo registry is fuller than the whitepaper's prose; reconcile at v1.0. |

## Keeping them in sync

When a change touches the published contract:

1. Edit the in-repo `spec/*.md` (the machine-readable form).
2. Mirror the corresponding change into the vault whitepaper chapter set(s)
   (`Projects/RFL/whitepaper/`), rebuild the PDF, and refresh the repo's PDF
   mirror.
3. If the change adds or alters an invariant, update the I1–I5 ↔ CA/RD trace in
   `spec/02` (and this tracker).

This tracker is updated as chapters gain explicit bridge sections and as the
open deltas close.
