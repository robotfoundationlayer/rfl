# Design — `rfl validate` CLI (Skill ISA composition validation, v0)

**Date:** 2026-06-01
**Track:** reference CLI — fill the `validate` stub
**Status:** approved (autonomous run — user directive "execute all tasks")
**Scope:** `rfl-core` (`Skill::validate`), `rfl-cli` (`Validate` handler). No schema / spec change.

## 0. Why

`rfl validate <skill.yaml>` is the last stubbed subcommand (`"validate not yet implemented"`). It is conformance **Test Class 1** (Skill ISA parser conformance) at the Rust layer — a vendor authoring a skill checks it is well-formed *before* retargeting onto any embodiment. Completes the CLI surface (validate / retarget / certify / verify / sign / keygen).

## 1. What it checks (v0)

1. **Parse** — `Skill::parse_yaml` already validates the YAML against the v0-supported subset (unknown primitives / shapes error). A parse error is a malformed skill.
2. **Composition validity (embodiment-independent):** `Skill::validate(&self) -> crate::Result<()>` — v0 enforces **unique let-binding names** (a duplicate `let` shadows / re-binds, an authoring error). This is the one composition rule decidable now without `StabilityMetadata`.
3. **Structural report** to stdout (skill name, statement count, primitive-call vs let-bind counts).

**Deferred (needs breadth):** the STB3 stability-class → permitted-successor composition algebra (`spec/05` § Composition validity) requires `StabilityMetadata` (`closure` / `secured_dof` / `flags`) on grasp primitives, which the Rust model does not yet have (18/50 primitives). `Skill::validate` is the seam where STB3 lands once those exist.

## 2. CLI

`rfl validate <skill.yaml>`:
- read + parse → on error, `eprintln!` + exit **2** (malformed).
- `skill.validate()` → on `Err`, print the composition error + exit **2**.
- on success, print `VALID: skill '<name>' — N statement(s) (P primitive calls, L let-binds)` + exit **0**.

## 3. Testing

- `rfl-core` unit: `Skill::validate` accepts a unique-let skill, rejects a duplicate-let skill.
- CLI smoke (`rfl-cli` test): `rfl validate` on the committed `examples/01-cable-insertion/skill.yaml` → exit 0 + `VALID`; a hand-written duplicate-let skill → exit 2.

## 4. Out of scope (YAGNI)

STB1–3 (need `StabilityMetadata` / grasp-mode primitives — breadth); let-reference *resolution* across pose args (the retarget path already errors on an unresolved ref; v0 validate is parse + structural); any embodiment-specific check (validate is embodiment-independent).
