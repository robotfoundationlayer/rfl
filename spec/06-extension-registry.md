# Extension Registry — Specification

> **Status**: design-complete (2026-06-01) — the extension-point taxonomy (the eight extensible
> surfaces chapters 01–05 cite), the three-valued accept / reject rule (core / registered / unknown,
> the `02` I5 contract), the namespace and version semantics, the registration and promotion flow,
> and the deprecation lifecycle are specified. The chapter's own Open issues (sunsetting, conflict
> resolution, renaming) are closed. Remaining pre-freeze work is implementation artifacts — the JSON
> Schema for a registry entry and the populated `extensions/` directory — not open design questions.

## Scope

The extension registry is the controlled mechanism by which RFL accumulates new capability without
modifying the core specification. It is the spec's pressure-release valve: capabilities that are too
specific to belong in the core, or too novel to commit to before field validation, enter the
registry and may be promoted into the core in a subsequent major version if they accumulate
cross-vendor adoption.

The registry is the operational form of **Principle 5 (forward-compatible)**: once v1.0 ships, no
breaking change lands in any v1.x release, so every capability introduced after v1.0 enters here.
It is constrained by **Principle 3 (verifiable)** — an extension is registered only with the
conformance fixture that makes it mechanically testable — and **Principle 4 (provider-neutral)** —
a vendor namespace isolates a vendor's extensions without privileging them in the core.

This chapter defines the *mechanism*. The concrete contents of the `extensions/` directory, the
JSON Schema for a registry entry, and the Technical Steering Committee's membership rules live
outside the spec (project governance documents).

## What is extensible

Eight surfaces across the spec are deliberately left open, each a closed core enumeration plus a
registry escape hatch. This section is the single authority the owning chapters point back to; an
extension entry declares which surface it extends.

| # | Surface | Owning chapter | Core form | Extension form |
|---|---|---|---|---|
| 1 | Skill ISA primitive | `01` § Provisional primitive enumeration | `category.primitive` (reserved) | `ext.<ns>.<primitive>` |
| 2 | Capability skill-ID | `03` § Capability manifest | a core primitive / category key | a registered `ext.<ns>.<primitive>` |
| 3 | Auxiliary capability name | `03` § Capability manifest | core aux key (`tactile_sensing`, `tool_safety`, …) | namespaced (`rfl-x:<name>` / `ext.<ns>.<name>`) |
| 4 | Frame role tag | `03` § Embodiment frame model | core role (`grasp` / `sensor` / `tactile` / `support` / …) | namespaced (`rfl-x:<role>`) |
| 5 | Limit key | `03` § Limits | core limit (`grip_force_max`, …) | namespaced (`ext.<ns>.<limit>`) |
| 6 | `hazard_class` value | `03` § Safety capabilities | `cut` / shear | `weld`, `thermal`, … via `ext.<ns>.<hazard>` |
| 7 | `sensor_class` token | `04` § The adapter mapping | `ft` / `array` / `visuotactile` / `pneumatic` | `ext.<ns>.<class>` (+ its adapter) |
| 8 | Tactile `feature` type | `04` § Feature taxonomy | the closed-core feature set | `ext.<ns>.<feature>` |

A ninth, **irreversible-operation classes** (`05` § Reversibility and irreversible-operation safety — `weld`, `adhesive`, and the
reverse `snap_disengage`), is registered as a Skill ISA primitive (surface 1) carrying the
irreversible safety class; it is not a separate surface but a primitive whose registry entry sets
the irreversible flag the `05` safety bench reads.

Each registered surface keeps the property its owning chapter requires: a registry primitive (1)
ships its seven-field rubric and conformance-test sketch; a registry `sensor_class` (7) ships its
raw → feature adapter; a registry `feature` (8) is producible by some adapter (the `04` TM27c
closure). Registration without the chapter-required artifact is rejected at review.

## The accept / reject rule

Every identifier appearing in a Skill ISA composition or an embodiment descriptor resolves to
**exactly one** of three states. This three-valued rule is the normative contract the rest of the
spec cites as **I5** (`02` § Relation to the whitepaper's I1–I5 contract), the unknown-role/skill-ID rejection
(`03`), and the feature-vocabulary closure (`04` TM27c).

- **Core.** An unprefixed identifier in the owning chapter's closed enumeration. **Accepted** and
  interpreted by its core semantics.
- **Registered extension.** An `ext.<ns>.<name>` (or the `rfl-x:<name>` compact form for role /
  auxiliary tags) that is present in the registry. **Accepted** and **passed through** unchanged:
  the Translation Layer carries it without reinterpreting it, honoring only the contract its
  registry entry declares. A capability asserted under a registered extension participates in
  capability negotiation exactly as a core capability does — a skill that uses it retargets only
  onto embodiments that declare it; an embodiment that does not declare it yields `capability_absent`
  (not a parse error).
- **Unknown.** An identifier that is neither core nor registered — an unregistered `ext.*`, or an
  extension-shaped name carrying no namespace where one is required. **Rejected**, never silently
  ignored. Fail-closed is mandatory: silently dropping an unrecognized tag could mask a capability
  the planner relied on (`03` § Embodiment frame model). Rejection is a validation error
  (conformance Test Class 1), not a runtime surprise.

The asymmetry between *pass-through* and *reject* is the whole forward-compatibility guarantee: a
toolchain built against v1.0 accepts a v1.1-era registered extension (carrying it through without
understanding it) yet refuses an unknown tag — so new capability never breaks an old toolchain, and
an unrecognized capability never silently degrades a skill.

## Namespace rules

Each extension lives under a namespaced, versioned identifier:

```
ext.<vendor-or-domain>.<extension-name>.v<MAJOR>
```

Examples:

- `ext.softrobotics.pneumatic-grasp.v1` — a pneumatic-hand-specific grasp primitive
- `ext.bimanual.coordinated-handoff.v1` — a bimanual coordination primitive

Two namespace kinds:

- **Vendor namespaces** (`ext.<vendor-name>.*`) are reserved for that vendor's extensions on
  first-claim. A vendor governs its own namespace and need not coordinate beyond registration.
- **Domain namespaces** (`ext.<domain>.*`, e.g. `ext.bimanual.*`) are cross-vendor and require
  Maintainer assignment via Issue + PR, so a shared domain does not fragment into incompatible
  vendor variants.

For the role-tag and auxiliary-name surfaces (`03`), the compact `rfl-x:<name>` prefix is the
registry's short form: `rfl-x:` denotes "a registered extension to a `03` tag space," resolving to a
registry entry under the registrant's namespace. It is the same registry, surfaced with the prefix
`03` already uses in its examples (`rfl-x:vacuum`).

**Reserved names.** The unprefixed core identifiers — every `category.primitive` in `01`, every core
capability / aux / limit / role / `hazard_class` / `sensor_class` / `feature` token — are reserved
and may never be claimed by an extension. An extension that collides with a reserved name is
rejected at registration. This keeps the core namespace stable across the registry's growth.

## Version semantics

The `.v<MAJOR>` suffix versions each extension independently of the core spec and of other
extensions. A major bump (`…​.v1` → `…​.v2`) mints a **new identifier**; the prior version remains
valid and registered until it is sunset. Consequences:

- A v1.x core release may add registered extensions freely without a core version change (the
  registry is the v1.x growth path).
- A consumer pinned to `ext.<ns>.<name>.v1` is never broken by the publication of `.v2`.
- There is **no in-place mutation** of a registered extension's contract; a changed contract is a
  new `.v<MAJOR>`.

## Registration process

1. The author opens an **Issue** describing the extension, the surface it extends (§ What is
   extensible), and its motivation.
2. The author submits a **PR** adding the extension under `extensions/<namespace>/v<MAJOR>/`,
   including the chapter-required artifact (the rubric + conformance fixture for a primitive, the
   adapter for a `sensor_class`, etc.) so Principle 3 is satisfiable.
3. The **Maintainer** assigns reviewers; review focuses on the five constitutional principles and on
   reserved-name and namespace-assignment correctness.
4. On acceptance the extension is **registered** and may be referenced from Skill ISA files and
   embodiment descriptors (it then resolves to the *registered* state of § The accept / reject
   rule). Until registration it is *unknown* and rejected.

## Promotion to core

An extension may be promoted into the core in a major-version bump if all of:

1. It has been live in the registry for ≥ 18 months.
2. It has at least three implementing organizations (cross-vendor adoption, Principle 4).
3. The promotion violates no constitutional principle.

Promotion is a Technical Steering Committee decision, not a Maintainer decision. On promotion the
core gains the unprefixed name and the registered `ext.<ns>.<name>` is sunset (deprecated, then
retired) per the lifecycle below, with the core name as its successor — so live references migrate
without a break.

## Deprecation lifecycle

Every registered extension is in exactly one lifecycle state:

- **active** — registered and referenceable.
- **deprecated** — superseded (by a `.v<MAJOR>` successor or a core promotion) or withdrawn;
  still resolvable (so existing skills do not break) but flagged, with a named successor where one
  exists. An extension enters `deprecated` only by an explicit registry change, never silently.
- **retired** — removed from the registry after a minimum notice window (no shorter than the
  promotion dwell, ≥ 18 months) in `deprecated`. A reference to a retired identifier resolves to
  *unknown* (rejected) — the only point at which a once-valid identifier stops validating, and only
  after the full notice window.

This resolves the three former Open issues:

- **Sunsetting** of an extension that fails to attract cross-vendor adoption is the
  `deprecated → retired` path: a Maintainer may deprecate a dormant extension, and it retires after
  the notice window. Removal is never in-place.
- **Conflict** between two extensions is handled first by namespace isolation — two registered
  extensions under distinct namespaces coexist mechanically (the accept / reject rule treats each
  independently). A genuine *semantic* overlap (two extensions claiming the same capability) is a
  Principle-2 review concern caught at registration; an overlap discovered post-registration is
  Maintainer-arbitrated, with one extension deprecated in favor of the other.
- **Renaming** is forbidden in place (it would break live references). A new name is a new
  extension; the old identifier is deprecated with the new one as its successor.

## Cross-references

- Primitive registration rules and the reserved core-name list: `01-skill-isa.md`.
- Capability / aux / role / limit / `hazard_class` extension points: `03-driver-interface.md`.
- `sensor_class` and `feature` extension points and their closure rules: `04-tactile-manifold.md`.
- The I5 pass-through + unknown-rejection invariant and its conformance: `02-translation-layer.md`,
  `05-conformance.md`.
- Spec-change discipline and TSC sign-off: `CONTRIBUTING.md`, project governance documents.
