# Governance

> **Status: draft scaffold.** This document is the home that `spec/00` §
> Specification governance and `spec/05` § The trademark gate point to for the
> rules they deliberately keep out of the specification. The **authoritative**
> statement of the stewardship model is the whitepaper § 7 (Stewardship
> Governance) and its commitments **C1–C5**; this file operationalizes those
> commitments and will be ratified by the Technical Steering Committee at the
> v0.1 → v1.0 transition. Specifics marked *(draft)* await ratification.

## Roles

- **Maintainer** — day-to-day stewardship: reviews, merges, namespace
  assignment, and routine extension registration (`spec/06` § Registration
  process). Assigns reviewers; does not decide major-version or principle-level
  questions alone.
- **Technical Steering Committee (TSC)** — owns major-version transitions, any
  deviation from the five constitutional principles, and promotion of an
  extension into the core (`spec/06` § Promotion to core). TSC sign-off is
  required where `spec/00` § governance says so.

## C1 — neutrality via membership rebalancing *(operational mechanism)*

The stewardship body is structured so no single party can capture it:

- **Three-tier membership** *(draft)* — founding stewards, implementing members,
  and community members, with distinct rights, so governance weight is not
  proportional to commercial scale alone.
- **30%-rebalancing process** *(draft)* — no single organization (or aligned
  bloc) holds more than **30%** of TSC voting weight; if adoption shifts the
  balance past the cap, seats rebalance at the next cycle. This is the
  operational form of the C1 neutrality commitment.

The precise tier rights, seat counts, and rebalancing cadence are ratified by the
TSC; the **30% cap** and the **three-tier structure** are the fixed commitments
from the whitepaper.

## C4 — transparency mechanics

Specification changes and steward decisions are made in the open:

- **RFC process** — a substantive change (a new primitive, a contract change, a
  major-version proposal) lands as a public RFC with a comment window before the
  TSC decides; the extension registry's Issue + PR flow (`spec/06`) is the RFC
  path for extensions.
- **Public decision record + webcast** *(draft)* — TSC decisions are minuted
  publicly and substantive sessions are webcast / recorded, so the rationale for
  a decision is auditable after the fact. This is the C4 transparency commitment.

## The trademark gate

Use of the `RFL™` mark on packaging is gated by the **regime tier** (`spec/05` §
The trademark gate): **Tier 2** (steward-verified) and **Tier 3** (independently
verified) conformance permit the mark; **Tier 1** self-certification does not.
The *gate* is fixed in the spec; the *assignment and ownership* of the mark after
the Foundation donation is a governance matter resolved here, not in the spec.

## Spec-change and extension-registration discipline

- Spec changes follow [CONTRIBUTING.md](CONTRIBUTING.md) § Spec-change discipline.
- Registering an extension follows `spec/06` § Registration process; the
  practical on-ramp is [docs/registering-an-extension.md](docs/registering-an-extension.md).
- Major-version transitions and principle-level deviations require Maintainer +
  TSC sign-off (`spec/00` § governance).

## Amending this document

Until the TSC is constituted, this scaffold is maintained by the Maintainer and
tracks the whitepaper § 7 model. After constitution, changes to governance follow
the C4 RFC process above.
