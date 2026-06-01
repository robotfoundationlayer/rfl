# Recursive-simulator conformance / fidelity-tier badge / trademark gate — design

Status: **BADGE GENERATOR IMPLEMENTED 2026-06-02** (`rfl-conformance::badge` +
`rfl badge <cert.json>`: per-action envelope/fidelity + the embodiment-level
rollup — regime Tier 1 for self-cert, achieved fidelity = weakest confirmed
action's tier, trademark gated to Tier 2/3; pure read of the cert JSON, no
golden churn). Trademark gate was already spec-fixed (now surfaced by the badge).
Recursive-simulator conformance remains **design-open** (needs a
simulator-provenance model). The badge derives per-**action** fidelity (the
granularity the cert carries); a true per-**capability** breakdown awaits the
cert carrying capability keys.

## The three sub-items (`spec/05` § Conformance regime)

1. **Trademark gate** — *already fixed in the spec*, no implementation needed: the
   `RFL™` mark on packaging is permitted at regime **Tier 2 / Tier 3** and **not**
   at **Tier 1** self-certification (`spec/05` line 295, tier table line 268).
   Mark *assignment/ownership* is explicitly a governance matter, not a spec or
   tooling one (`spec/00` § governance). So there is nothing to build here beyond
   surfacing the gate in the badge (below).
2. **Fidelity-tier → badge generator** — the clean, additive, golden-safe slice.
3. **Recursive-simulator conformance** — design-only for now (see § Deferred).

## Badge generator (the implementable slice)

`spec/05` § Fidelity tier and the badge: a badge records, **per capability**,
both the **regime tier** (who verified — Tier 1/2/3) and the **fidelity tier**
achieved (`manifold` > `proxy` > `proxy_reactive`), so it is honest about *that*
and *how well* a capability conforms. The certificate already carries the
fidelity tier per action (the certify-track inc.3 `ActionEntry.fidelity_tier`).

Clean v0: a derivation from an existing certificate — no new conformance logic,
no retarget/golden impact.

- **Regime tier**: an `rfl certify` certificate is **Tier 1** by construction
  (self-certification: the vendor runs their own driver). Tier 2/3 come from a
  steward/independent verifier, out of scope for the self-cert tool.
- **Per-capability fidelity**: read from the certificate's action entries
  (aggregate to the worst tier per capability — an honest badge cannot claim
  `manifold` if any action degraded to `proxy`).
- **Trademark**: `permitted = regime_tier ∈ {2,3}` ⇒ `false` for a Tier-1
  self-cert badge. Surfaced explicitly so the badge never implies mark rights.

Surface: an `rfl badge <certificate.json>` subcommand (and/or a `badge` block
the certificate can carry) printing `{ capability, regime_tier, fidelity_tier,
trademark_permitted }` rows. Reads the committed cert fixtures; golden-safe.
This is a ready-to-implement increment (new CLI arm + a small derive module +
tests over the existing cert fixtures).

## Recursive-simulator conformance — STRUCTURE IMPLEMENTED 2026-06-02

On re-reading, `spec/05` § Recursive simulator conformance is actually *resolved*,
not design-open: a simulator earns conformant status **recursively** — its
outputs must match a *physically-conformant* embodiment's within the Class
2-loose ε on a recorded reference fixture set, so it cannot bootstrap its own
fidelity. The structural form is implementable now (same pattern as the ε-table
format): `schemas/simulator-declaration.schema.json` + `simulator-declaration.yaml`
(simulator provenance + anchoring embodiment + reference fixtures + tolerance
basis + status, `pending` until ε-matched) + `validate.py` **C10** (the
no-self-bootstrap rule: a `conformant` claim must carry the anchor + non-empty
fixtures + `epsilon_match` evidence; adversarially verified). The actual ε-match
*grading* stays BLOCKED on ε-values + a real simulator run.

## Decision

Trademark gate: nothing to build (spec-fixed; surfaced via the badge).
Badge generator: clean v0 specified above, ready to implement.
Recursive-simulator: design-open, deferred. Recorded so the badge increment can
land independently without waiting on the simulator model.
