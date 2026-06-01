# Design — `grasp.pin` + STB3 stability-class composition (v0)

**Date:** 2026-06-01
**Track:** grasp stability — the first real STB check (consumes the increment-4a foundation)
**Status:** approved (autonomous run — user directive "推奨順に全てのタスクを自動実行")
**Scope:** `rfl-core` (`grasp_force::GraspMode::Pin`; `stability::for_mode(Pin)`; `skill_isa` `GraspPin` + `Primitive::GraspPin` + a grasp-classification helper + STB3 in `Skill::validate`; `translation` `lower_grasp_pin` + dispatch arms). `rfl-cli` validate handler already surfaces composition errors. No schema change (`GraspPinParams` already typed); no driver/wire change.

## 0. Why

STB3 (`spec/05` § Composition validity by stability class) is the first stability obligation implementable without a driver-protocol change or the deferred ε-table: it is a **static class-1 composition check** decidable from each grasp's `StabilityMetadata` (built in 4a). The table's most self-contained row is **`surface_bound` (pin) → forbids free `transport.*`** — the pinned object is invalid once its supporting surface is lost (`transport_inadmissible`, `spec/01` § 2.7 + § Grasp state model). It needs exactly one new grasp mode, `grasp.pin`, whose successor it forbids (`transport.move_to_pose` / `transport.carry`) already exists. STB1 (tripod) is deferred: its "minimum-area threshold" is in the deferred data-dependent ε-table and it needs driver-reported contact geometry the report does not carry.

This increment lands in two green commits: **(T1)** `grasp.pin` as a complete primitive (parse + lower + mode + metadata), **(T2)** the STB3 surface_bound composition check in `Skill::validate`, reading `StabilityMetadata::for_mode` — closing the loop that 4a opened (the foundation's first consumer).

## 1. T1 — `grasp.pin` primitive

### 1.1 `grasp_force::GraspMode::Pin`

Add the variant; the exhaustive `k_holding` / `k_reaction` / `payload_key` matches gain a `Pin` arm. v0 schematic (documented, non-normative, golden-pinned, same posture as `Pinch`): a pin holds an object against a surface through two friction interfaces (effector + surface), so the same `1/(2μ)` schematic gives `k_holding = k_reaction = 2.0`; `payload_key = "payload_grasp_pin"`. **`lower_grasp_pin` does not invoke the weight floor** — a pinned object's retention is surface-normal-directional, not free-hang, so v0 emits no `min_holding_force` for pin (deferred, honest; STB3 needs only the flags). The factors exist for forward-compatible completeness.

### 1.2 `stability::StabilityMetadata::for_mode(Pin)`

Per the `spec/01` grasp-mode table row `pin | force | clamp-normal friction_held | extrinsic, surface_bound`:
- `closure: Force`
- `secured_dof: {"clamp_normal": FrictionHeld}` (the clamp-normal axis; the v0 conventional key for pin's single secured direction)
- `stable_directions: Set(["against_surface.normal"])` (directional — NOT omnidirectional; `spec/01` § 2.7 postcondition `stable_directions = {against_surface.normal}`)
- `flags: { extrinsic: true, surface_bound: true }`
- `residual_mobility: None`, `min_holding_force: None`

This is the first non-empty `flags` and the first non-`omnidirectional` `stable_directions` on the wire — exercising the 4a serialization paths (the `empty_flags_are_omitted` test already covers the inverse).

### 1.3 `skill_isa::GraspPin`

```rust
/// `grasp.pin` parameters (v0 subset of `$defs/GraspPinParams`, § 2.7). target +
/// against_surface + force_budget required; against_surface is a frame ref in v0
/// (the SurfaceTarget point+normal is floored to a name, as force.wipe floors `surface`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraspPin {
    pub target: Ref,
    pub against_surface: FrameRef,
    pub force_budget: Quantity,
    #[serde(default = "tactile_auto")]
    pub tactile_target: TactileTargetArg,
}
```

`Primitive::GraspPin(GraspPin)` (externally-tagged `grasp.pin`).

### 1.4 `lower_grasp_pin`

Mirrors `lower_grasp_pinch`'s structure, minus the weight floor:
- `force_budget` = `clamp_force(&p.force_budget, "grip_force_max", e)` (the § 2.7 clamp to grip_force_max; `target.max_contact_force` is not modeled in v0).
- `tactile_target` = the same manifold/proxy resolution as pinch (auto = effector contact + surface reaction; proxy when no tactile).
- `target_pose` = `PoseExpr::Ref { ref: p.target }` (symbolic, v0 — `pin_pose: auto`).
- `safety_envelope` = `base_envelope(e)`; `force_profile` carries the pin's `against_surface` as a symbolic marker `{ "against_surface": <name>, "extrinsic": true }` (so the directional pin is legible on the wire) — rides the open Envelope floor, no schema change.
- `grasp_stability: Some(StabilityMetadata::for_mode(GraspMode::Pin))`.
- Does **not** set `ctx.held` (a pinned object is surface-bound, not a freely-carried held object — keeping it out of `ctx.held` is what makes a following `transport` see no held load; STB3 catches the illegal compose separately at validation).

Dispatch: `check_capability` arm `Primitive::GraspPin(_) => "grasp.pin"`; `lower` arm `Primitive::GraspPin(p) => (lower_grasp_pin(p, e, ctx, weights), "pin")`. (Both land with the enum variant in T1 — the exhaustive matches force it.)

### 1.5 T1 tests

- `grasp_force`: `min_holding_force` / `reaction_limit` unaffected (Pinch tests unchanged); a Pin `payload_key` unit assertion.
- `stability`: `for_mode(Pin)` → force closure, `flags.surface_bound && flags.extrinsic`, `stable_directions == Set(["against_surface.normal"])`, `min_holding_force == None`.
- `translation`: lower a `GraspPin` against the allegro embodiment directly (bypassing capability, as the release test does) → `grasp_stability` carries `surface_bound`; the wire JSON contains `"surface_bound":true` and `"against_surface"`.

## 2. T2 — STB3 surface_bound composition check

### 2.1 The classification helper (`skill_isa`)

```rust
impl Primitive {
    /// The grasp mode this primitive establishes (None for non-grasp primitives).
    pub fn establishes_grasp(&self) -> Option<GraspMode> { Pinch->Pinch, Pin->Pin, _ -> None }
    /// True if this primitive releases the active grasp.
    pub fn releases_grasp(&self) -> bool { matches!(self, GraspRelease) }
    /// True if this primitive freely transports a held object (the surface_bound-forbidden successor).
    pub fn is_free_transport(&self) -> bool { matches!(self, TransportMoveToPose | TransportCarry) }
}
```

### 2.2 `Skill::validate` extension

After the existing duplicate-let check, walk the sequence tracking the active grasp's mode:

```rust
let mut active: Option<GraspMode> = None;
for stmt in &self.body.sequence {
    if let Statement::Primitive(p) = stmt {
        if p.is_free_transport() {
            if let Some(mode) = active {
                if StabilityMetadata::for_mode(mode).flags.surface_bound {
                    return Err(composition error: "transport_inadmissible: a surface_bound
                        grasp (<mode>) cannot be freely transported (spec/05 STB3)");
                }
            }
        }
        if let Some(mode) = p.establishes_grasp() { active = Some(mode); }
        else if p.releases_grasp() { active = None; }
    }
}
```

Order matters: a `grasp.pin` immediately followed by `transport` is illegal (the transport sees the active surface_bound grasp); a `grasp.pin` → `grasp.release` → `transport` is legal (release clears `active` before the transport). The check is `surface_bound`-only in this increment; the other three STB3 rows (`form_held`/`rotation_constrained` → in_hand; `support` → transport+open-release) are deferred with their grasp modes (`hook`/`tripod`/`platform`) and in_hand breadth.

This reads `StabilityMetadata::for_mode` — the 4a authority — so the rule and the wire metadata cannot drift.

### 2.3 T2 tests

- `skill_isa` unit (`validate`): a `grasp.pin → transport.move_to_pose` skill → `Err` (transport_inadmissible); a `grasp.pin → grasp.release → transport.move_to_pose` skill → `Ok`; a `grasp.pinch → transport` skill → `Ok` (pinch is freely transportable). Inline YAML (validate is descriptor-independent, pre-retarget).
- `rfl-cli` validate smoke: the illegal pin→transport skill → exit 2 with the composition message (extends `validate_cli.rs`).

## 3. Out of scope (YAGNI / deferred)

STB1 (tripod min-area — ε-table + driver contact geometry); STB2 (`grasp.platform` support safe-state — needs the platform mode + release semantics); the other three STB3 rows (need `hook`/`tripod`/`platform` + `in_hand.rotate`/`slide`/`regrasp`); `grasp.pin` full lowering richness (`pin_pose` geometry, `target.max_contact_force`, surface-reaction monitor, the in-surface `slide`/`regrasp` permitted successors); pin's `min_holding_force` model; a dedicated pin example dir (inline test skills suffice — STB3 is class-1, pre-retarget). No driver/schema change.
