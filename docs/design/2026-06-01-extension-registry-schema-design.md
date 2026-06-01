# Extension-registry entry schema + `extensions/` scaffold + C8 invariant

Status: design-complete (2026-06-01)

## Problem

`spec/06-extension-registry.md` is design-complete and explicitly defers two
*implementation artifacts* to project governance (§ Scope, lines 24–26): "the
JSON Schema for a registry entry, and the populated `extensions/` directory."
Neither exists yet. Without the schema, a registry entry (when the registry
opens post-v1.0) has no machine-checkable form, and conformance Test Class 1
cannot enforce the registry's structural invariants — the same gap the C1–C7
anti-drift invariants close for the other five schemas.

## Decision

Ship three things:

1. **`schemas/extension-registry.schema.json`** — the registry-entry schema
   (Draft 2020-12, matching the house style). One entry models exactly the
   fields `spec/06` makes normative:
   - `namespace` + `namespace_kind` (`vendor` | `domain`, § Namespace rules)
   - `name`, `version` (the `.v<MAJOR>` integer, § Version semantics)
   - `identifier` — the resolved `ext.<ns>.<name>.v<MAJOR>` (§ Namespace rules);
     its consistency with the three fields above is a C8 invariant, not a schema
     constraint (JSON Schema cannot cross-reference sibling values by equality).
   - `surface` — one of the eight extension-point surfaces (§ What is
     extensible), as a named enum.
   - `lifecycle` — `active` | `deprecated` | `retired` (§ Deprecation lifecycle)
   - `successor` (optional; a deprecated entry names one *where one exists*)
   - `artifact` — the Principle-3 chapter-required fixture (registration without
     it is rejected, § What is extensible, line 53); required.
   - `description`, plus optional `implementing_organizations` (the §-Promotion
     cross-vendor-adoption evidence) and `registered` date.
   - The schema carries the spec's own illustrative example
     (`ext.softrobotics.pneumatic-grasp.v1`) in its `examples` array so C8 has a
     non-vacuous positive case while the live registry is legitimately empty.

2. **`extensions/` scaffold** — `extensions/README.md` documenting the
   `extensions/<namespace>/v<MAJOR>/` layout and stating the registry is **empty
   by design at v0.1**: the core is complete and, per Principle 5, the registry
   is the *post-v1.0* growth path, so there are no registered extensions yet.
   Inventing one would invent normative capability.

3. **`validate.py` C8** — add `extension-registry` to the well-formedness loop,
   then a C8 block that, for every real entry under `extensions/**/*.json`
   (none yet) and every entry in the schema's `examples`:
   - validates it against the schema;
   - asserts `identifier == ext.<namespace>.<name>.v<version>` (path/identifier
     consistency);
   - asserts the `name` collides with **no reserved core token** — derived at
     check time from `skill-isa`'s `PrimitiveId` enum (leaf names), the category
     gates, and the closed-core tactile features (anti-drift, exactly as C1
     derives the descriptor enum from skill-isa). A reserved-name collision is
     the § Namespace-rules "reserved names" violation.

## Why C8 is currently vacuous on the live tree (and that's correct)

There are zero registered extensions at v0.1 by design. C8's machinery is real
and fires the moment the first entry lands; until then the schema's bundled
example is the positive test. This mirrors `spec/06`'s own framing: the registry
opens post-v1.0.

## Non-goals

- No live extension is registered (would invent capability).
- The TSC membership rules and promotion governance stay in governance docs, not
  the schema (§ Scope).
