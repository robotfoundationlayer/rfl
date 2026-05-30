# Conformance — Specification (skeleton)

> **Status**: pre-release skeleton. Full text targeted for v0.1 (2027 Q1).

## Scope

This chapter defines:

1. The four conformance test classes
2. The three-tier conformance regime (self-certification → steward-verified → notified-body-certified)
3. The trademark gate (which tier allows use of the `RFL™` trademark on packaging)

## Four test classes

| # | Class | What it tests |
|---|---|---|
| 1 | **Skill ISA parser conformance** | A YAML file conforms to the Skill ISA JSON schema and the compositional algebra parses without error |
| 2 | **Translation Layer determinism** | Given a fixed skill + fixed embodiment descriptor, `retarget()` produces identical output byte-for-byte across runs and platforms |
| 3 | **Driver Interface protocol compliance** | A driver implementation accepts the canonical actions, executes them within stated tolerances, and reports back via the protocol |
| 4 | **End-to-end execution conformance** | A skill expressed at the Skill ISA layer executes correctly on the target embodiment via the full Translation → Driver Interface chain |

Test classes 1 and 2 are pure-compute and can run in CI. Class 3 requires the driver binary. Class 4 requires the physical embodiment (or a high-fidelity simulator declared as conformant).

## Three-tier conformance regime

| Tier | Verifier | Cost to implementer | Permits use of RFL™ trademark? |
|---|---|---|---|
| 1 — Self-certification | Implementer publishes own results | Free | No |
| 2 — Steward-verified | RFL Inc. or RFL Foundation engineering staff runs the suite | Free for Lead Customers / Founding Members; fee otherwise | Yes |
| 3 — Notified-body | Independent body (TÜV or comparable, post-MoU) | Independent fee | Yes (and enables ISO 10218 / 13482 flow-through) |

## Open issues

- Determinism requirement floor (bit-identical vs. semantic-equivalence with epsilon)
- How a simulator earns "high-fidelity / conformant" status (recursive conformance)
- Trademark assignment after Stage 2 Foundation donation
