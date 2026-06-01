# Design — `StabilityMetadata` foundation (grasp stability class on the wire, v0)

**Date:** 2026-06-01
**Track:** primitive breadth → grasp stability (the STB1-3 / GC2-6 unblock)
**Status:** approved (autonomous run — user directive "execute all tasks")
**Scope:** `rfl-core` (`stability.rs` new module; `CanonicalAction.grasp_stability`; `lower_grasp_pinch` emits it). No `rfl-conformance` check yet, no schema change beyond an additive optional field, no spec change.

## 0. Why

`StabilityMetadata` (`closure` / `secured_dof` / `flags`, spec/01 § Grasp state model) is the single missing type that blocks the entire grasp-stability obligation group: **STB1-3** (non-degenerate tripod, support safe state, stability-class composition) and **GC2-6** (hold-test closure, make-before-break, controlled under-actuation, bounded exception, two-party continuity) all read it (spec/05 §§ Closure/stability/composition, Grasp-continuity). The conformance suite reads `ExecuteGoal.canonical_action`, so the metadata must ride the **wire** — not just live in `GraspContext`. This increment introduces the type and emits it from the one grasp primitive that exists today (`grasp.pinch`), so every later STB/GC increment has a populated field to verify against instead of also having to invent the type.

This is the foundation slice: **no conformance check is added here.** Its value is the type + the mode→metadata authority + the wire plumbing, verified by unit and serialization tests. The first check (STB1) lands in the next increment with `grasp.precision_tripod`.

## 1. The type (spec/01 § Grasp state model, verbatim shape)

```
StabilityMetadata := {
  closure:           {force, form, support},
  secured_dof:       Map<DOF, {form_held, friction_held, balance_held}>,
  stable_directions: set<Direction> | omnidirectional,
  flags:             subset of {extrinsic, surface_bound, compliant, rotation_constrained},
  residual_mobility: Length | None,
  min_holding_force: Force,
}
```

Rust modeling (`crates/rfl-core/src/stability.rs`, new module — focused, one responsibility):

- `Closure` — `enum {Force, Form, Support}`, `#[serde(rename_all = "lowercase")]`.
- `DofSecuring` — `enum {FormHeld, FrictionHeld, BalanceHeld}`, `#[serde(rename_all = "snake_case")]`.
- `StabilityFlags` — a `Default` struct of four `bool`s (`extrinsic`, `surface_bound`, `compliant`, `rotation_constrained`), each `#[serde(skip_serializing_if = "is_false")]`; an `is_empty()` helper.
- `StableDirections` — `enum {Omnidirectional, Set(Vec<String>)}` (`#[serde(untagged)]` so omnidirectional serializes as the string `"omnidirectional"` and a set as a JSON array). v0 only emits `Omnidirectional`.
- `StabilityMetadata` — struct:
  - `closure: Closure`
  - `secured_dof: BTreeMap<String, DofSecuring>` (deterministic key order — Class 2)
  - `stable_directions: StableDirections`
  - `flags: StabilityFlags` (`skip_serializing_if = "StabilityFlags::is_empty"`)
  - `residual_mobility: Option<Quantity>` (`skip_serializing_if = "Option::is_none"`)
  - `min_holding_force: Option<Quantity>` (`skip_serializing_if = "Option::is_none"`)

Serialize-only (the wire types are output-only; conformance never deserializes them). Derives: `Debug, Clone, PartialEq, Serialize`.

### 1.1 The mode → metadata authority

```rust
impl StabilityMetadata {
    /// The static stability class a grasp mode establishes (spec/01 § Grasp state model,
    /// the grasp-mode table). The single source of truth read by lowering, downstream
    /// transport/in_hand accel-clamp + admissibility, and the STB/GC conformance checks.
    #[must_use]
    pub fn for_mode(mode: GraspMode) -> Self { /* match mode { Pinch => force/friction/omni } */ }
}
```

v0 `GraspMode` has only `Pinch`. Per the spec table, **pinch / power → `closure: force`, friction_held (all DOF), no flags, omnidirectional**. v0 models "all DOF friction_held" as the single conventional key `"all_axes" → FrictionHeld`; the per-axis DOF vocabulary is deferred to the increment that introduces the first DOF-reading check (STB3 composition / GC4 under-actuation), which is the consumer that forces specific axis names. `min_holding_force` is filled in by the caller (weight-dependent), not by `for_mode` (mode-static).

## 2. Wire attachment

`CanonicalAction` gains:

```rust
/// The grasp stability class this action establishes (grasp primitives only).
/// None for non-grasp actions — skipped on the wire so their output is unchanged.
#[serde(skip_serializing_if = "Option::is_none")]
pub grasp_stability: Option<StabilityMetadata>,
```

All existing `CanonicalAction { … }` literals add `grasp_stability: None`. Only `lower_grasp_pinch` sets `Some`. Because `skip_serializing_if = None`, every non-grasp action's JSONL is byte-identical to today, and the certificate body does not embed `canonical_action` at all — so **no certificate re-bless and no golden-output churn**.

## 3. `lower_grasp_pinch` change

After the existing min-holding-force block, build the metadata from the authority and graft in the weight-dependent floor:

```rust
let mut stability = StabilityMetadata::for_mode(GraspMode::Pinch);
if let Some((weight_n, _)) = weights.get(&p.target).and_then(|q| q.parse()) {
    stability.min_holding_force =
        Some(Quantity::from_si(grasp_force::min_holding_force(weight_n, GraspMode::Pinch), "N"));
}
// … CanonicalAction { …, grasp_stability: Some(stability) }
```

(The `min_holding_force` recompute reuses the value already derived for `env.force_profile`; factor it into one `let mhf = …` to avoid computing twice.) Schemas: `schemas/canonical-action.schema.json` (if it constrains additional properties) gets an additive optional `grasp_stability` object; verified against the emitted JSON.

## 4. Testing (TDD, red→green)

1. **`stability.rs` unit** — `for_mode(Pinch)` returns `closure == Force`, `secured_dof == {"all_axes": FrictionHeld}`, `flags.is_empty()`, `stable_directions == Omnidirectional`, `min_holding_force == None`.
2. **`StabilityFlags` serialization** — an all-false flags struct serializes to `{}` (skipped) inside a metadata; a `rotation_constrained = true` flags serializes `{"rotation_constrained": true}` only (other three skipped). (Forward check for the tripod increment.)
3. **`lower_grasp_pinch` unit / retarget integration** — retargeting `examples/01-cable-insertion` produces a `grasp.pinch` action whose `grasp_stability` is `Some` with `closure: force` and a populated `min_holding_force` (cable has a mass); a non-grasp action (`reach.align`) has `grasp_stability == None`.
4. **Wire serialization** — the retarget JSONL for the pinch action contains `"grasp_stability":{"closure":"force",…}`; the reach action's JSON does **not** contain the key `grasp_stability` (skip-None confirmed).
5. **Full gate** — `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, then `gh run` CI-green.

## 5. Out of scope (YAGNI)

No STB/GC conformance check (next increments). No new grasp modes (`power`/`tripod`/`platform`/`pin`/`hook`/`lateral`/`envelope_*` arrive with the checks that read them). No per-axis DOF taxonomy (forced later by STB3/GC4). No downstream re-wiring of transport accel-clamp to read `grasp_stability` (it already derives conservatively from `mode`; rewire when a check demands the wire value). No deserialization (wire types stay Serialize-only).
