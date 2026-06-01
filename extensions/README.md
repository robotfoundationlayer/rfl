# Extensions

The extension registry: the controlled mechanism by which RFL accumulates new
capability without modifying the core specification. See
[`spec/06-extension-registry.md`](../spec/06-extension-registry.md) for the full
mechanism (the eight extensible surfaces, the core / registered / unknown
accept-reject rule, namespace and version semantics, the registration and
promotion flow, and the deprecation lifecycle).

## This directory is empty by design at v0.1

There are **no registered extensions yet**. The core is complete, and per
**Principle 5 (forward-compatible)** the registry is the *post-v1.0* growth
path: once v1.0 ships, every capability introduced afterward enters here rather
than mutating the core. Inventing an extension now would invent normative
capability.

## Layout

Each registered extension lives under a namespaced, versioned directory:

```
extensions/<namespace>/v<MAJOR>/
```

and ships:

1. An **entry** validating against
   [`schemas/extension-registry.schema.json`](../schemas/extension-registry.schema.json)
   (the registry-entry schema), declaring its `namespace`, `name`, `version`,
   resolved `identifier` (`ext.<namespace>.<name>.v<MAJOR>`), the `surface` it
   extends, its `lifecycle` state, and its Principle-3 `artifact`.
2. The **chapter-required artifact** itself (a primitive's conformance fixture,
   a `sensor_class`'s adapter mapping, etc.), so the extension is mechanically
   testable at registration (Principle 3). Registration without it is rejected.

`schemas/validate.py` invariant **C8** validates every entry present here
against the schema, asserts its `identifier` is consistent with its
`namespace`/`name`/`version`, and asserts its `name` collides with no reserved
core token (derived at check time from `skill-isa`'s primitive set — the same
anti-drift discipline as C1). The registry being empty, C8's live coverage is
currently exercised by the schema's bundled worked example.

## Registering an extension

Open an Issue describing the extension and the surface it extends, then a PR
adding the directory above. The Maintainer assigns reviewers; review focuses on
the five constitutional principles and on reserved-name and
namespace-assignment correctness. See `spec/06` § Registration process.
