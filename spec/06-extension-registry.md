# Extension Registry — Specification (skeleton)

> **Status**: pre-release skeleton. Full text targeted for v0.1 (2027 Q1).

## Scope

The extension registry is the controlled mechanism by which RFL accumulates new capability without modifying the core specification. It is the spec's pressure-release valve: capabilities that are too specific to belong in the core, or too novel to commit to before field validation, enter the registry and may be promoted into the core in subsequent major versions if they accumulate cross-vendor adoption.

## Namespace rules

Each extension lives under a namespaced identifier:

```
ext.<vendor-or-domain>.<extension-name>.v<MAJOR>
```

Examples:

- `ext.softrobotics.pneumatic-grasp.v1` — a pneumatic-hand-specific grasp primitive
- `ext.bimanual.coordinated-handoff.v1` — a bimanual coordination primitive

Vendor-claimed namespaces (`ext.<vendor-name>.*`) are reserved for that vendor's extensions; cross-vendor namespaces (`ext.<domain>.*`) require Maintainer assignment via Issue + PR.

## Registry process (provisional)

1. Author opens an Issue describing the extension and its motivation
2. Author submits a PR adding the extension under `extensions/<namespace>/v1/`
3. Maintainer assigns reviewers; review focuses on the five constitutional principles
4. After acceptance, the extension is registered and may be referenced from Skill ISA files

## Promotion to core

An extension may be promoted to the core in a major-version bump if all of:

1. It has been live in the registry for ≥ 18 months
2. It has at least three implementing organizations (cross-vendor adoption)
3. The promotion does not violate any constitutional principle

Promotion is a TSC decision, not a Maintainer decision.

## Open issues

- Sunsetting rules for extensions that fail to attract cross-vendor adoption
- Conflict resolution when two extensions overlap
- Renaming policy across major-version bumps
