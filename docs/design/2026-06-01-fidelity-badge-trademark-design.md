# Recursive-simulator conformance / fidelity-tier badge / trademark gate — design

Status: design-complete; **badge generator is the clean implementable slice** (the other two are spec-fixed or design-only)

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

## Deferred: recursive-simulator conformance

`spec/05` § Recursive simulator conformance governs when a simulator may stand in
for physical hardware in a conformance run. This needs a simulator-provenance
model (what the simulator attests, how its fidelity is bounded) that interacts
with the live-`--driver` path and the fidelity tier; it is a genuine design
increment, not a derivation, and is **deferred** pending that model. Not BLOCKED
on data/hardware — it is design-open.

## Decision

Trademark gate: nothing to build (spec-fixed; surfaced via the badge).
Badge generator: clean v0 specified above, ready to implement.
Recursive-simulator: design-open, deferred. Recorded so the badge increment can
land independently without waiting on the simulator model.
