# Registering an extension

The extension registry is how RFL accumulates new capability without modifying
the core (`spec/06`). This is the practical on-ramp; the normative mechanism is
[`spec/06-extension-registry.md`](../spec/06-extension-registry.md) and the entry
format is [`schemas/extension-registry.schema.json`](../schemas/extension-registry.schema.json).

> The registry is **empty by design at v0.1** — per Principle 5 it is the
> *post-v1.0* growth path. This guide is the procedure for when it opens.

## What can be extended

Eight surfaces are open (`spec/06` § What is extensible): Skill ISA primitive,
capability skill-ID, auxiliary capability, frame role, limit key, `hazard_class`,
`sensor_class`, and tactile `feature`. An entry declares which one it extends.

## Before you start: is it core, registered, or unknown?

Every identifier resolves to exactly one of three states (`spec/06` § The
accept / reject rule):

- **Core** — an unprefixed identifier in a chapter's closed enumeration. You
  cannot register over a reserved core name (rejected at registration).
- **Registered** — an `ext.<ns>.<name>.v<MAJOR>` in the registry; accepted and
  passed through unchanged, negotiated exactly like a core capability.
- **Unknown** — neither; rejected, never silently ignored.

## Steps

1. **Open an Issue** using the *extension-proposal* template
   ([.github/ISSUE_TEMPLATE/extension-proposal.yml](../.github/ISSUE_TEMPLATE/extension-proposal.yml)):
   describe the extension, the surface it extends, and its motivation.
2. **Choose a namespace** (`spec/06` § Namespace rules):
   - a **vendor** namespace (`ext.<vendor>.*`) is yours on first-claim;
   - a **domain** namespace (`ext.<domain>.*`, cross-vendor) needs Maintainer
     assignment so a shared domain does not fragment.
3. **Submit a PR** adding `extensions/<namespace>/v<MAJOR>/` containing:
   - an **entry** validating against the registry-entry schema (`namespace`,
     `name`, `version`, resolved `identifier`, `surface`, `lifecycle: active`,
     and the `artifact`);
   - the **Principle-3 artifact** itself — the chapter-required fixture that makes
     the extension mechanically testable (a primitive's conformance fixture, a
     `sensor_class`'s adapter mapping, etc.). Registration without it is rejected.
4. **Review** — the Maintainer assigns reviewers; review focuses on the five
   constitutional principles and on reserved-name and namespace-assignment
   correctness (see [GOVERNANCE.md](../GOVERNANCE.md)).
5. **Registration** — on acceptance the extension resolves to the *registered*
   state and may be referenced from Skill ISA files and embodiment descriptors.

`schemas/validate.py` invariant **C8** checks every entry: schema-conformance,
`identifier == ext.<namespace>.<name>.v<MAJOR>`, and reserved-name
non-collision. CI runs it on every PR.

## Versioning, deprecation, promotion

- A contract change is a **new** `.v<MAJOR>` (no in-place mutation); the prior
  version stays valid until sunset.
- Lifecycle is `active → deprecated → retired`; a retired identifier resolves to
  *unknown* only after the notice window (`spec/06` § Deprecation lifecycle).
- An extension live ≥ 18 months with ≥ 3 implementing organizations may be
  **promoted into the core** by the TSC (`spec/06` § Promotion to core).
