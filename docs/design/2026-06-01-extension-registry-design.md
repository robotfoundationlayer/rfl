# spec/06 Extension Registry — flesh out the skeleton

**Status**: design-complete, 2026-06-01. Spec-design task (user-approved). Takes `spec/06` from a
45-line pre-release skeleton to a design-complete chapter that serves as the single authority for
the `ext.*` references chapters 01–05 already make.

## Why now

Chapters 01–05 already point at `06` for eight distinct extension surfaces and assume a precise
accept/reject contract (the I5 "extension-registry pass-through + unknown-tag rejection" of `02`).
The skeleton states namespace/process/promotion but does not (a) enumerate what is extensible, nor
(b) fix the normative accept/reject rule the other chapters depend on. This chapter supplies both,
so `06` stops being a forward reference and becomes the authority.

## Design decisions

1. **Extension-point taxonomy** — enumerate the eight extensible surfaces the other chapters cite,
   each as a typed registry entry: Skill ISA primitives (`01`), capability skill-IDs + auxiliary
   names (`03`), frame role tags (`03`), limit keys (`03`), `hazard_class` values (`03`),
   `sensor_class` tokens (`04`), tactile `feature` types (`04`), irreversible-operation classes
   (`05`). Each carries the conformance fixture Principle 3 demands.

2. **The accept/reject rule (I5, load-bearing).** Every identifier in a Skill ISA file or
   embodiment descriptor resolves to exactly one of three states: **core** (unprefixed, in the
   closed enumeration → accepted); **registered extension** (`ext.<ns>.<name>` present in the
   registry → accepted and carried through unchanged, its contract honored); **unknown**
   (unregistered, or an extension-shaped name with no namespace → **rejected, never silently
   ignored** — fail-closed, because dropping a tag could mask a capability the planner relied on).
   This is the exact contract `02` I5, `03` (unknown role tag / skill ID rejected), and `04`
   (TM27c feature closure) cite.

3. **Namespace + version semantics.** `ext.<ns>.<name>.v<MAJOR>`. Vendor namespaces
   (`ext.<vendor>.*`) are self-reserved; domain namespaces (`ext.<domain>.*`) are Maintainer-
   assigned. The `rfl-x:` short prefix `03` uses for role/aux tags is the registry's compact form
   for those surfaces. Core unprefixed names (`01`'s reserved list) may never be re-used. A
   `.v<MAJOR>` bump mints a **new** identifier; the prior version stays valid (forward-compat,
   Principle 5) until sunset.

4. **Resolve the three Open issues.** Sunsetting = a deprecation lifecycle (active → deprecated →
   retired) with a minimum notice window, never an in-place removal. Conflict resolution =
   namespace isolation makes two extensions coexist mechanically; semantic overlap is a review-time
   Principle-2 concern, Maintainer-arbitrated as a last resort. Renaming = forbidden in place; a new
   name is a new extension with the old one deprecated (so no live reference breaks).

5. **Style.** Match the sibling chapters: a `design-complete` Status line that closes the chapter's
   own Open issues, em-dash prose density as in 01–05 (this is the in-repo technical spec, not a
   vault PUBLISH doc), no horizontal-rule `---`. The spec-lint CI job only checks file existence;
   correctness is editorial.

## Out of scope (unchanged)

The concrete `extensions/<namespace>/` directory contents, the JSON Schema for a registry entry, and
TSC membership rules (governance docs, not the spec) stay future work — the chapter defines the
mechanism, not a populated registry.
