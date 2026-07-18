# Grasp-force numerics (GF1c / GF2c / GF3c) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Derive `min_holding_force`, the dynamic `max_acceleration` clamp, and the insert-fit reaction-load limit numerically and deterministically in the Translation Layer, retiring the symbolic placeholders v0 left.

**Architecture:** Three pure derivation functions (`grasp_force.rs`) consume declared inputs (object weight, grasp mode, descriptor limits) and documented schematic per-mode factors. A small `GraspContext` is threaded through the otherwise-stateless `retarget` sequence walk so a `transport` / `force` primitive can read what grasp holds what object. Derived numbers emit as `round6`'d unit-suffixed strings (RD1c), placed in the `execute` message's `force_profile` / `motion_bounds`.

**Tech Stack:** Rust (workspace `rfl-core` / `rfl-cli` / `rfl-conformance`, edition 2024, MSRV 1.85, cargo 1.96 via rustup), `serde`/`serde_yaml`/`serde_json`, `insta` golden snapshots, `boon` JSON Schema validation, Python `validate.py` via `uv`.

**Design doc:** `docs/design/2026-05-31-grasp-force-numerics-design.md` (committed, `3069a21` + correction).

**Standing rules (every task):**
- Prefix every cargo command with `export PATH="$HOME/.cargo/bin:$PATH"` (shell state does not persist between calls).
- **Validation read and `git commit` MUST be separate batches.** Run tests, READ the result (ALL PASS / EXIT=0), and only then stage + commit. Never put the commit in the same tool call as the test run.
- `git add` explicit file paths only — never `-A` / `.` (keeps `docs/plans/` and other untracked files out).
- Before commit: `git rev-parse --abbrev-ref HEAD` == `main`. Before push: `git fetch -q origin && git merge-base --is-ancestor origin/main HEAD` (ff). After push: `git rev-list --left-right --count origin/main...HEAD` == `0 0`. A parallel session moves `main`; rebase onto `origin/main` if the ff-check fails. No `--force`, no `--no-verify`.
- Conventional Commits + the `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>` trailer.

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `schemas/skill-isa.schema.json` | add optional `estimated_mass` to closed `ObjectDecl` | 1 |
| `crates/rfl-core/src/skill_isa.rs` | `ObjectDecl.estimated_mass: Option<Quantity>` | 1 |
| `examples/01-cable-insertion/skill.yaml` | declare `connector.estimated_mass` | 1 |
| `crates/rfl-core/src/grasp_force.rs` (new) | the three pure derivations + per-mode factors | 2 |
| `crates/rfl-core/src/lib.rs` | `pub mod grasp_force;` | 2 |
| `crates/rfl-core/src/quantity.rs` | `Quantity::from_si` deterministic formatter | 3 |
| `crates/rfl-core/src/translation.rs` | `GraspContext`, weights resolution, GF1c/GF2c/GF3c lowering | 4 / 5 / 6 |
| `crates/rfl-conformance/tests/snapshots/retarget_determinism__*.snap` | regenerated goldens | 4 / 5 / 6 |

The surface-scan example/goldens are unaffected throughout (no grasp, no `transport.move_to_pose`, no `insert_fit`, no `objects` block → `build_weights` returns empty).

---

## Task 1: Object weight input wiring (schema + Rust field + fixture)

**Files:**
- Modify: `schemas/skill-isa.schema.json` (`$defs/ObjectDecl`)
- Modify: `crates/rfl-core/src/skill_isa.rs:63-67` (`ObjectDecl`)
- Modify: `examples/01-cable-insertion/skill.yaml` (objects block)
- Test: `crates/rfl-core/src/skill_isa.rs` (parse_tests) + `schemas/validate.py`

- [ ] **Step 1: Write the failing test** in `crates/rfl-core/src/skill_isa.rs`, inside `mod parse_tests`:

```rust
    #[test]
    fn connector_declares_estimated_mass() {
        let s = cable_skill();
        assert_eq!(s.objects["connector"].estimated_mass.as_ref().unwrap().0, "1.45 N");
        assert!(s.objects["receptacle"].estimated_mass.is_none());
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core connector_declares_estimated_mass`
Expected: FAIL — `no field 'estimated_mass' on type '&ObjectDecl'` (compile error).

- [ ] **Step 3: Add the field to `ObjectDecl`** in `crates/rfl-core/src/skill_isa.rs` (replace the struct at lines 62-67):

```rust
/// A task object reference (`$defs/ObjectDecl`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ObjectDecl {
    /// The object identity this local name binds to.
    pub r#ref: String,
    /// Optional declared weight (`spec/01` ObjectTarget.estimated_mass: Force), a
    /// known prior the Translation Layer reads for the grasp-force derivations.
    #[serde(default)]
    pub estimated_mass: Option<Quantity>,
}
```

- [ ] **Step 4: Declare the mass in the fixture** — in `examples/01-cable-insertion/skill.yaml`, replace the `objects:` block:

```yaml
objects:
  connector:  { ref: connector, estimated_mass: 1.45 N }   # ~148 g connector+cable stub;
                                                            # chosen so the dynamic-stability
                                                            # clamp bites on the lowest-payload hand
  receptacle: { ref: receptacle }    # the mating receptacle, fixed in the workcell (not grasped)
```

- [ ] **Step 5: Extend the schema** — in `schemas/skill-isa.schema.json`, replace the `ObjectDecl` definition (the closed `{ required:[ref], additionalProperties:false, properties:{ref} }` block) with:

```json
    "ObjectDecl": {
      "type": "object",
      "required": ["ref"],
      "additionalProperties": false,
      "properties": {
        "ref": {
          "type": "string",
          "description": "The object identity (ObjectRef) this local name binds to."
        },
        "estimated_mass": {
          "$ref": "#/$defs/Force",
          "description": "Optional declared weight (a known prior), mirroring ObjectTarget.estimated_mass; supplied to the grasp-force derivations at retarget time."
        }
      }
    },
```

- [ ] **Step 6: Run the Rust test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core connector_declares_estimated_mass`
Expected: PASS (`test result: ok. 1 passed`).

- [ ] **Step 7: Verify the schema still validates (validate.py) and nothing else broke**

Run: `cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py`
Expected: ends with `PASS` and EXIT=0 (the extended `skill.yaml` validates against the `estimated_mass`-aware `ObjectDecl`; C1–C7 unaffected).

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test`
Expected: all crates green — the cable goldens are UNCHANGED (objects are not serialized to `execute` messages, so no output changed).

- [ ] **Step 8: Commit** (separate batch from Step 7 — only after reading PASS/ok above)

```bash
cd ~/Documents/GitHub/rfl && git add schemas/skill-isa.schema.json crates/rfl-core/src/skill_isa.rs examples/01-cable-insertion/skill.yaml && git commit -m "feat(core): declare object estimated_mass (input for grasp-force derivations)

Adds an optional estimated_mass to the closed ObjectDecl (schema + Rust),
reusing \$defs/Force and mirroring ObjectTarget.estimated_mass, and declares
the cable connector's weight. No execute output changes yet.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: `grasp_force.rs` — the three pure derivations

**Files:**
- Create: `crates/rfl-core/src/grasp_force.rs`
- Modify: `crates/rfl-core/src/lib.rs:23-31` (add `pub mod grasp_force;` in alphabetical position, before `pose`)
- Test: in-module `#[cfg(test)] mod tests`

- [ ] **Step 1: Create `crates/rfl-core/src/grasp_force.rs` with the failing tests and the functions**

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Grasp-force and stability derivations (`spec/02` § Grasp-force and stability
//! derivations, GF1c–GF3c).
//!
//! The capacity model is schematic and provider-neutral (Principle 4): RFL fixes
//! the *dependency* — capacity is a function of closure, grip, geometry, and
//! friction — not a vendor friction law. The per-mode factors below collapse the
//! `μ · geometry` term into one documented reference number per closure mode; they
//! are a v0 reference-implementation choice, pinned by golden, non-normative.

/// Standard gravity (m/s^2). `estimated_mass` is typed as weight under standard
/// gravity (`spec/01`), so only the dynamic clamp needs this as a scale.
pub const G0: f64 = 9.80665;

/// A held-grasp closure mode. v0 lowers only `grasp.pinch`; the enum gives the
/// derivations a forward-compatible key for the per-mode factors and payload limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraspMode {
    /// Antipodal force-closure pinch.
    Pinch,
}

impl GraspMode {
    /// The schematic holding factor: required grip per unit held weight
    /// (`min_holding_force = weight · k_holding`). `≈ 1/(2μ)` at μ=0.25 for a
    /// 2-contact force-closure pinch.
    #[must_use]
    pub fn k_holding(self) -> f64 {
        match self {
            GraspMode::Pinch => 2.0,
        }
    }

    /// The schematic reaction factor: a unit of grip resists `grip / k_reaction` of
    /// axial pull-out. Same μ basis as `k_holding`.
    #[must_use]
    pub fn k_reaction(self) -> f64 {
        match self {
            GraspMode::Pinch => 2.0,
        }
    }

    /// The descriptor limit key for this mode's rated payload (max holdable weight).
    #[must_use]
    pub fn payload_key(self) -> &'static str {
        match self {
            GraspMode::Pinch => "payload_grasp_pinch",
        }
    }
}

/// GF1c — the static grip floor below which the held object falls under its own
/// weight. `weight_n` is the object weight in newtons (`target.estimated_mass`).
#[must_use]
pub fn min_holding_force(weight_n: f64, mode: GraspMode) -> f64 {
    weight_n * mode.k_holding()
}

/// GF2c — the dynamic-stability acceleration limit: the largest acceleration at
/// which inertial + gravity load (worst case collinear, `weight·(1 + a/g₀)`) stays
/// within the mode's rated holding capacity (`payload_n`, a weight). Returns the raw
/// dynamic limit in m/s^2; the caller takes the min with the kinematic ceiling.
/// Clamped at 0 — a weight above the rated payload (negative pre-clamp) is the
/// deferred payload-violation case, surfaced here as "cannot accelerate".
#[must_use]
pub fn dynamic_a_max(weight_n: f64, payload_n: f64) -> f64 {
    (G0 * (payload_n / weight_n - 1.0)).max(0.0)
}

/// GF3c — the reaction-load limit: a force primitive's reaction may not exceed the
/// grasp's axial capacity, bounded by the grip a `slip_response: retighten` grasp
/// can muster (`grip_force_max_n`) divided by the mode's reaction factor. Returns the
/// smaller of the requested budget and that capacity (both newtons).
#[must_use]
pub fn reaction_limit(force_budget_n: f64, grip_force_max_n: f64, mode: GraspMode) -> f64 {
    force_budget_n.min(grip_force_max_n / mode.k_reaction())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_holding_force_is_weight_times_factor() {
        // 1.45 N connector, pinch k=2.0 -> 2.9 N.
        assert!((min_holding_force(1.45, GraspMode::Pinch) - 2.9).abs() < 1e-9);
    }

    #[test]
    fn dynamic_a_max_bites_below_pneumatic_ceiling() {
        // payload 1.5 N, held 1.45 N -> 9.80665 / 29 ≈ 0.33816 (< 0.8 ceiling).
        let a = dynamic_a_max(1.45, 1.5);
        assert!((a - 0.338160).abs() < 1e-5, "got {a}");
        assert!(a < 0.8);
    }

    #[test]
    fn dynamic_a_max_exceeds_strong_hand_ceiling() {
        // payload 3 N, held 1.45 N -> ≈10.5 (kinematic ceiling 1.5 wins downstream).
        assert!(dynamic_a_max(1.45, 3.0) > 1.5);
    }

    #[test]
    fn dynamic_a_max_clamps_at_zero_over_payload() {
        // held > payload -> pre-clamp negative -> 0.
        assert_eq!(dynamic_a_max(2.0, 1.5), 0.0);
    }

    #[test]
    fn reaction_limit_takes_the_smaller() {
        // 15 N budget vs 12/2 = 6 capacity -> 6.
        assert!((reaction_limit(15.0, 12.0, GraspMode::Pinch) - 6.0).abs() < 1e-9);
        // 5 N budget vs 20/2 = 10 capacity -> 5 (budget lower).
        assert!((reaction_limit(5.0, 20.0, GraspMode::Pinch) - 5.0).abs() < 1e-9);
    }
}
```

- [ ] **Step 2: Register the module** — in `crates/rfl-core/src/lib.rs`, add between `pub mod embodiment;` and `pub mod pose;`:

```rust
pub mod grasp_force;
```

- [ ] **Step 3: Run the tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core grasp_force`
Expected: PASS (`test result: ok. 5 passed`).

- [ ] **Step 4: Confirm the workspace still builds and is green**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test`
Expected: all green (no engine wiring yet, so no output changed).

- [ ] **Step 5: Commit** (separate batch from Steps 3-4)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-core/src/grasp_force.rs crates/rfl-core/src/lib.rs && git commit -m "feat(core): add grasp-force derivations (GF1c/GF2c/GF3c pure functions)

Schematic per-mode capacity model (Principle 4): min_holding_force = W*k,
dynamic a_max = g0*(payload/W - 1), reaction limit = grip_max/k_reaction.
Pure, no engine wiring yet.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: `Quantity::from_si` — deterministic formatter for derived numbers

**Files:**
- Modify: `crates/rfl-core/src/quantity.rs` (impl block + tests)

- [ ] **Step 1: Write the failing test** in `crates/rfl-core/src/quantity.rs`, inside `mod tests`:

```rust
    #[test]
    fn from_si_formats_shortest_roundtrip() {
        assert_eq!(Quantity::from_si(10.0, "N").0, "10 N");
        assert_eq!(Quantity::from_si(7.5, "N").0, "7.5 N");
        assert_eq!(Quantity::from_si(2.9, "N").0, "2.9 N");
        // 9.80665 / 29 ≈ 0.33816034 -> round6 -> 0.33816.
        assert_eq!(Quantity::from_si(9.80665 / 29.0, "m/s^2").0, "0.33816 m/s^2");
    }

    #[test]
    fn from_si_is_repeatable() {
        let v = 9.80665 / 29.0;
        assert_eq!(Quantity::from_si(v, "m/s^2"), Quantity::from_si(v, "m/s^2"));
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core from_si`
Expected: FAIL — `no function or associated item named 'from_si'`.

- [ ] **Step 3: Add `from_si`** to the `impl Quantity` block in `crates/rfl-core/src/quantity.rs` (after `parse`):

```rust
    /// Build a quantity from an SI magnitude and unit, rounding the magnitude to 6
    /// decimals for byte-deterministic emission (RD1c) and formatting it with the
    /// shortest round-trip representation (`10.0 -> "10"`, `7.5 -> "7.5"`). Used for
    /// retarget-*derived* quantities (grasp-force / acceleration), which, unlike
    /// authored quantities, are computed floats.
    #[must_use]
    pub fn from_si(value: f64, unit: &str) -> Quantity {
        Quantity(format!("{} {}", crate::canonical::round6(value), unit))
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core quantity`
Expected: PASS (the new two plus the existing parse tests).

- [ ] **Step 5: Commit** (separate batch)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-core/src/quantity.rs && git commit -m "feat(core): add Quantity::from_si deterministic formatter for derived values

round6 + shortest-round-trip Display so derived forces/accelerations emit as
byte-stable unit-suffixed strings (10.0 -> \"10 N\", 9.80665/29 -> \"0.33816 m/s^2\").

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: GF1c — `min_holding_force` + the grasp-context plumbing

This task introduces `GraspContext`, the `weights` resolution, and threads `&mut GraspContext` + `&weights` through `lower`. Only `grasp.pinch` (sets context, emits the floor) and `grasp.release` (clears context) change behaviour; the `transport` / `insert_fit` arms keep their 2-arg calls until Tasks 5/6.

**Files:**
- Modify: `crates/rfl-core/src/translation.rs` (imports, `retarget`, `lower`, `lower_grasp_pinch`, `lower_grasp_release`, new `GraspContext`/`HeldObject`/`build_weights`, tests)
- Test/golden: `crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_{allegro,leap,pneumatic}.snap`

- [ ] **Step 1: Write the failing test** in `crates/rfl-core/src/translation.rs`, inside `mod tests`:

```rust
    #[test]
    fn pinch_emits_min_holding_force_floor() {
        let (skill, emb) = load("allegro");
        let out = retarget_pinch_only(&skill, &emb);
        let fp = serde_json::to_string(&out.safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"min_holding_force\":\"2.9 N\""), "got {fp}");
        // 8 N task budget exceeds the 2.9 N floor and is under the 20 N ceiling -> unchanged.
        assert_eq!(out.force_budget.as_ref().unwrap().0, "8 N");
    }

    #[test]
    fn release_clears_held_context() {
        let (skill, emb) = load("allegro");
        let weights = super::build_weights(&skill);
        let mut ctx = super::GraspContext::default();
        let Statement::Primitive(Primitive::GraspPinch(p)) = &skill.body.sequence[1] else { panic!() };
        super::lower_grasp_pinch(p, &emb, &mut ctx, &weights);
        assert!(ctx.held.is_some());
        let Statement::Primitive(Primitive::GraspRelease(r)) = &skill.body.sequence[6] else { panic!() };
        super::lower_grasp_release(r, &emb, &mut ctx);
        assert!(ctx.held.is_none());
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core -- pinch_emits_min_holding_force_floor release_clears_held_context`
Expected: FAIL — `build_weights` / `GraspContext` not found, `lower_grasp_pinch` arity mismatch.

- [ ] **Step 3: Extend the imports** at the top of `crates/rfl-core/src/translation.rs` — add after the existing `use` lines:

```rust
use crate::grasp_force::{self, GraspMode};
use std::collections::BTreeMap;
```

- [ ] **Step 4: Add the context types + weight resolver** in `crates/rfl-core/src/translation.rs`, immediately after the `RetargetOutput` struct (before `retarget`):

```rust
/// The object currently held by the active grasp, carried across the sequence walk
/// so a later `transport` / `force` primitive can derive mass-dependent bounds
/// (`spec/02` GF2c / GF3c). Set when a grasp closes, cleared when it releases.
#[derive(Debug, Clone)]
struct HeldObject {
    /// Held-object weight in newtons (`target.estimated_mass`).
    weight_n: f64,
    /// The closure mode of the active grasp.
    mode: GraspMode,
}

/// Mutable grasp state threaded through the retarget sequence walk.
#[derive(Debug, Clone, Default)]
struct GraspContext {
    /// The object currently held, if any.
    held: Option<HeldObject>,
}

/// Resolve each let-variable bound from a `sense.locate` to the declared
/// `estimated_mass` of its target object, giving `let-var -> weight`. A declared
/// prior, never runtime-measured (RD1c), so a grasp targeting such a variable can
/// look up the held-object weight at retarget time.
fn build_weights(skill: &Skill) -> BTreeMap<String, Quantity> {
    let mut weights = BTreeMap::new();
    for stmt in &skill.body.sequence {
        if let Statement::LetBind(b) = stmt {
            if let Primitive::SenseLocate(sl) = b.from.as_ref() {
                if let Some(decl) = skill.objects.get(&sl.target_ref) {
                    if let Some(mass) = &decl.estimated_mass {
                        weights.insert(b.r#let.clone(), mass.clone());
                    }
                }
            }
        }
    }
    weights
}
```

- [ ] **Step 5: Thread the context through `retarget`** — replace the body of `retarget` (the loop) in `crates/rfl-core/src/translation.rs`:

```rust
pub fn retarget(skill: &Skill, embodiment: &Embodiment) -> crate::Result<RetargetOutput> {
    let mut actions = Vec::new();
    let mut suffixes = Vec::new();
    let weights = build_weights(skill);
    let mut ctx = GraspContext::default();
    for stmt in &skill.body.sequence {
        let prim = match stmt {
            Statement::Primitive(p) => p,
            Statement::LetBind(b) => &b.from,
        };
        check_capability(prim, embodiment)?;
        let (action, suffix) = lower(prim, embodiment, &mut ctx, &weights);
        actions.push(action);
        suffixes.push(suffix);
    }
    Ok(RetargetOutput { actions, suffixes })
}
```

- [ ] **Step 6: Update the `lower` dispatcher signature** — replace `fn lower(...)` in `crates/rfl-core/src/translation.rs` (the `transport` and `insert_fit` arms still call their 2-arg fns in this task):

```rust
fn lower(
    prim: &Primitive,
    e: &Embodiment,
    ctx: &mut GraspContext,
    weights: &BTreeMap<String, Quantity>,
) -> (CanonicalAction, &'static str) {
    match prim {
        Primitive::SenseLocate(p) => (lower_sense_locate(p, e), "locate"),
        Primitive::GraspPinch(p) => (lower_grasp_pinch(p, e, ctx, weights), "pinch"),
        Primitive::TransportMoveToPose(p) => (lower_transport_move_to_pose(p, e), "transport"),
        Primitive::ReachAlign(p) => (lower_reach_align(p, e), "align"),
        Primitive::ForceInsertFit(p) => (lower_force_insert_fit(p, e), "insert_fit"),
        Primitive::GraspRelease(p) => (lower_grasp_release(p, e, ctx), "release"),
        Primitive::ReachRetract(p) => (lower_reach_retract(p, e), "retract"),
        Primitive::ReachScan(p) => (lower_reach_scan(p, e), "scan"),
        Primitive::SenseInspect(p) => (lower_sense_inspect(p, e), "inspect"),
    }
}
```

- [ ] **Step 7: Rewrite `lower_grasp_pinch`** in `crates/rfl-core/src/translation.rs`:

```rust
/// Lower `grasp.pinch`. force_budget is clamped to grip_force_max (CA4c) and floored
/// at the GF1c static minimum derived from the declared held weight; the floor is also
/// emitted in force_profile (the grasp-continuity invariant 05 GC1 samples it). The
/// held object is recorded for the downstream transport / force derivations.
fn lower_grasp_pinch(
    p: &GraspPinch,
    e: &Embodiment,
    ctx: &mut GraspContext,
    weights: &BTreeMap<String, Quantity>,
) -> CanonicalAction {
    let mut force_budget = clamp_force(&p.force_budget, "grip_force_max", e);
    let tactile_target = Some(match (&p.tactile_target, e.tactile_sensing()) {
        (TactileTargetArg::Auto(_), true) => TactileTargetOut::Auto,
        (TactileTargetArg::Auto(_), false) => TactileTargetOut::Proxy {
            proxy: ProxySpec { tier: "proxy", criterion: "position_convergence_and_force_hold" },
        },
        (TactileTargetArg::Other(v), _) => {
            TactileTargetOut::Explicit(serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
        }
    });
    let mut env = base_envelope(e);
    if let Some((weight_n, _)) = weights.get(&p.target).and_then(|q| q.parse()) {
        let mhf = grasp_force::min_holding_force(weight_n, GraspMode::Pinch);
        env.force_profile =
            Some(serde_json::json!({ "min_holding_force": Quantity::from_si(mhf, "N").0 }));
        if let Some((fb, unit)) = force_budget.parse() {
            if mhf > fb {
                let unit = unit.to_string();
                force_budget = Quantity::from_si(mhf, &unit);
            }
        }
        ctx.held = Some(HeldObject { weight_n, mode: GraspMode::Pinch });
    }
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target.clone() },
        force_budget: Some(force_budget),
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target,
        monitors: vec![],
        safety_envelope: env,
    }
}
```

- [ ] **Step 8: Update `lower_grasp_release`** in `crates/rfl-core/src/translation.rs` — add the `ctx` parameter and clear the held state (keep the existing body otherwise):

```rust
/// Lower `grasp.release`: clears the active grasp from the context and emits a
/// zero-distance withdraw along the default retract direction (break-contact
/// postcondition symbolic in v0).
fn lower_grasp_release(_p: &GraspRelease, e: &Embodiment, ctx: &mut GraspContext) -> CanonicalAction {
    ctx.held = None;
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::AxisRelative {
            direction: serde_json::Value::String("-tool_axis".into()),
            distance: Quantity("0 mm".into()),
        },
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}
```

- [ ] **Step 9: Fix the existing test helper** — in `crates/rfl-core/src/translation.rs` `mod tests`, replace `retarget_pinch_only`:

```rust
    fn retarget_pinch_only(skill: &Skill, emb: &Embodiment) -> CanonicalAction {
        let Statement::Primitive(Primitive::GraspPinch(p)) = &skill.body.sequence[1] else {
            panic!("expected grasp.pinch at index 1");
        };
        let weights = super::build_weights(skill);
        let mut ctx = super::GraspContext::default();
        super::lower_grasp_pinch(p, emb, &mut ctx, &weights)
    }
```

- [ ] **Step 10: Run the rfl-core tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core`
Expected: PASS — the two new tests plus all existing translation tests (`pinch_clamps_force...`, `pinch_degrades...` still hold: force_budget "8 N", tactile unchanged).

- [ ] **Step 11: Regenerate and eyeball the cable goldens**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance`
Expected: FAIL on `golden_allegro` / `golden_leap` / `golden_pneumatic` — each pinch action now carries `"force_profile":{"min_holding_force":"2.9 N"}`.

Run: `export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance` then `git diff -- crates/rfl-conformance/tests/snapshots/`
Expected diff: ONLY the pinch line of each `.snap` gains `force_profile.min_holding_force = "2.9 N"`; nothing else changes. Eyeball that the `generation_is_byte_identical` and `every_line_is_a_valid_execute_message` tests still pass on re-run:

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance`
Expected: PASS (all 5 conformance tests green). If `every_line_is_a_valid_execute_message` fails, the `force_profile` floor in `driver-interface.schema.json` is tighter than expected — inspect `$defs` for the Envelope/canonical_action floor before proceeding.

- [ ] **Step 12: Confirm validate.py + full suite green** (separate batch from commit)

Run: `cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py && export PATH="$HOME/.cargo/bin:$PATH" && cargo test`
Expected: validate.py `PASS` (EXIT=0); all cargo tests green.

- [ ] **Step 13: Commit** (separate batch — only after reading PASS above)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-core/src/translation.rs crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_allegro.snap crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_leap.snap crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_pneumatic.snap && git commit -m "feat(core): derive min_holding_force on grasp.pinch (GF1c) + grasp context

Threads a GraspContext through the retarget walk (set on pinch, cleared on
release) and resolves the held object's declared weight via the let-binding
chain. grasp.pinch emits force_profile.min_holding_force (W*k_pinch) and floors
the commanded grip at it. Goldens regenerated.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: GF2c — dynamic `max_acceleration` clamp on `transport`

**Files:**
- Modify: `crates/rfl-core/src/translation.rs` (`lower` transport arm, `lower_transport_move_to_pose`, tests)
- Test/golden: `crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_pneumatic.snap`

- [ ] **Step 1: Write the failing tests** in `crates/rfl-core/src/translation.rs` `mod tests`:

```rust
    #[test]
    fn transport_a_max_clamps_dynamically_on_weakest_hand() {
        // pneumatic: payload 1.5 N, held 1.45 N -> 9.80665/29 ≈ 0.33816 < 0.8 ceiling.
        let (skill, emb) = load("pneumatic-6f");
        let out = retarget(&skill, &emb).expect("retarget");
        // transport is action index 2 (locate, pinch, transport).
        assert_eq!(
            out.actions[2].safety_envelope.motion_bounds.a_max.as_ref().unwrap().0,
            "0.33816 m/s^2"
        );
    }

    #[test]
    fn transport_a_max_keeps_kinematic_ceiling_on_strong_hand() {
        // allegro: payload 3 N, held 1.45 N -> dynamic ≈ 10.5 > 1.5 ceiling (kept verbatim).
        let (skill, emb) = load("allegro");
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(
            out.actions[2].safety_envelope.motion_bounds.a_max.as_ref().unwrap().0,
            "1.5 m/s^2"
        );
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core -- transport_a_max_clamps_dynamically_on_weakest_hand transport_a_max_keeps_kinematic_ceiling_on_strong_hand`
Expected: FAIL — pneumatic transport still emits the kinematic `"0.8 m/s^2"`.

- [ ] **Step 3: Pass the context to the transport arm** — in `crates/rfl-core/src/translation.rs` `lower`, change the transport arm:

```rust
        Primitive::TransportMoveToPose(p) => (lower_transport_move_to_pose(p, e, ctx), "transport"),
```

- [ ] **Step 4: Rewrite `lower_transport_move_to_pose`** in `crates/rfl-core/src/translation.rs`:

```rust
/// Lower `transport.move_to_pose`. The frame-relative target is carried through;
/// a_max is clamped to the GF2c dynamic-stability limit when a held object makes it
/// tighter than the kinematic ceiling `base_envelope` set (otherwise the ceiling's
/// authored string is kept verbatim).
fn lower_transport_move_to_pose(
    p: &TransportMoveToPose,
    e: &Embodiment,
    ctx: &GraspContext,
) -> CanonicalAction {
    let target_pose = match yaml_to_json(&p.target_pose) {
        serde_json::Value::Object(map) => {
            let frame = map
                .get("frame")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("task")
                .to_string();
            let offset = map.get("offset").cloned().unwrap_or(serde_json::Value::Null);
            PoseExpr::FrameRelative { frame, offset }
        }
        other => PoseExpr::FrameRelative { frame: "task".into(), offset: other },
    };
    let mut env = base_envelope(e);
    if let Some(held) = &ctx.held {
        let payload = e.scalar_limit(held.mode.payload_key()).and_then(|q| q.parse());
        let ceiling = e.scalar_limit("a_cartesian_max").and_then(|q| q.parse());
        if let (Some((payload_n, _)), Some((ceiling_v, unit))) = (payload, ceiling) {
            let unit = unit.to_string();
            let dyn_a = grasp_force::dynamic_a_max(held.weight_n, payload_n);
            if dyn_a < ceiling_v {
                env.motion_bounds.a_max = Some(Quantity::from_si(dyn_a, &unit));
            }
        }
    }
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose,
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::TimeScalable,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: env,
    }
}
```

- [ ] **Step 5: Fix the existing transport unit test** — in `crates/rfl-core/src/translation.rs` `mod tests`, update `transport_carries_frame_relative_pose` to pass an empty context (no clamp, ceiling kept):

```rust
    #[test]
    fn transport_carries_frame_relative_pose() {
        let (skill, emb) = load("allegro");
        let Statement::Primitive(Primitive::TransportMoveToPose(p)) = &skill.body.sequence[2] else {
            panic!("expected transport.move_to_pose at index 2");
        };
        let ctx = super::GraspContext::default();
        let a = super::lower_transport_move_to_pose(p, &emb, &ctx);
        let json = serde_json::to_string(&a.target_pose).unwrap();
        assert!(json.contains("\"frame\":\"receptacle\""));
        // no held object in this isolated call -> kinematic ceiling kept.
        assert_eq!(a.safety_envelope.motion_bounds.a_max.as_ref().unwrap().0, "1.5 m/s^2");
    }
```

- [ ] **Step 6: Run the rfl-core tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core`
Expected: PASS (the two new GF2c tests + the updated transport test + all others).

- [ ] **Step 7: Regenerate and eyeball the cable goldens**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance`
Expected: FAIL on `golden_pneumatic` only (allegro/leap transport keeps the ceiling, unchanged).

Run: `export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance` then `git diff -- crates/rfl-conformance/tests/snapshots/`
Expected diff: ONLY the pneumatic transport action's `a_max` changes `"0.8 m/s^2"` → `"0.33816 m/s^2"`.

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance`
Expected: PASS (all 5 green; boon still validates the clamped a_max string).

- [ ] **Step 8: Confirm validate.py + full suite green** (separate batch from commit)

Run: `cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py && export PATH="$HOME/.cargo/bin:$PATH" && cargo test`
Expected: validate.py `PASS`; all cargo green.

- [ ] **Step 9: Commit** (separate batch)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-core/src/translation.rs crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_pneumatic.snap && git commit -m "feat(core): clamp transport max_acceleration to dynamic stability (GF2c)

When a held object's dynamic-stability limit g0*(payload/W - 1) is tighter than
the kinematic ceiling, transport a_max is clamped to it (pneumatic 0.8 ->
0.33816 m/s^2 for the 1.45 N connector); strong hands keep their ceiling.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: GF3c — reaction-load clamp on `force.insert_fit`

**Files:**
- Modify: `crates/rfl-core/src/translation.rs` (`lower` insert_fit arm, `lower_force_insert_fit`, tests)
- Test/golden: all three `retarget_determinism__retarget_*.snap`

- [ ] **Step 1: Write the failing test** in `crates/rfl-core/src/translation.rs` `mod tests`:

```rust
    #[test]
    fn insert_fit_reaction_clamps_budget_per_hand() {
        // 15 N budget clamped to grip_force_max / 2: allegro 10, leap 7.5, pneumatic 6.
        for (stem, expected) in [("allegro", "10 N"), ("leap", "7.5 N"), ("pneumatic-6f", "6 N")] {
            let (skill, emb) = load(stem);
            let out = retarget(&skill, &emb).expect("retarget");
            // insert_fit is action index 5.
            assert_eq!(
                out.actions[5].force_budget.as_ref().unwrap().0,
                expected,
                "stem {stem}"
            );
        }
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core insert_fit_reaction_clamps_budget_per_hand`
Expected: FAIL — insert_fit still emits the unclamped `"15 N"`.

- [ ] **Step 3: Pass the context to the insert_fit arm** — in `crates/rfl-core/src/translation.rs` `lower`, change the insert_fit arm:

```rust
        Primitive::ForceInsertFit(p) => (lower_force_insert_fit(p, e, ctx), "insert_fit"),
```

- [ ] **Step 4: Rewrite `lower_force_insert_fit`** in `crates/rfl-core/src/translation.rs`:

```rust
/// Lower `force.insert_fit`: carry the axial force_budget (clamped by GF3c to the
/// held grasp's reaction capacity so the part does not slip in-grasp), lower the
/// SeatingSpec stop_condition into a monitor, set compliance and the axial profile.
fn lower_force_insert_fit(p: &ForceInsertFit, e: &Embodiment, ctx: &GraspContext) -> CanonicalAction {
    let mut force_budget = p.force_budget.clone();
    if let Some(held) = &ctx.held {
        if let Some((budget_n, unit)) = force_budget.parse() {
            let unit = unit.to_string();
            if let Some((grip_max_n, _)) = e.scalar_limit("grip_force_max").and_then(|q| q.parse()) {
                let limit = grasp_force::reaction_limit(budget_n, grip_max_n, held.mode);
                force_budget = Quantity::from_si(limit, &unit);
            }
        }
    }
    let monitors = vec![Monitor { stop_condition: yaml_to_json(&p.stop_condition) }];
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    env.force_profile = Some(serde_json::json!({ "axial": force_budget.0.clone() }));
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target_fit.clone() },
        force_budget: Some(force_budget),
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::TimeScalable,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors,
        safety_envelope: env,
    }
}
```

- [ ] **Step 5: Fix the existing insert_fit unit test** — in `crates/rfl-core/src/translation.rs` `mod tests`, update `insert_fit_lowers_stop_condition_into_monitors` to pass an empty context (no clamp, budget kept):

```rust
    #[test]
    fn insert_fit_lowers_stop_condition_into_monitors() {
        let (skill, emb) = load("allegro");
        let Statement::Primitive(Primitive::ForceInsertFit(p)) = &skill.body.sequence[5] else {
            panic!("expected force.insert_fit at index 5");
        };
        let ctx = super::GraspContext::default();
        let a = super::lower_force_insert_fit(p, &emb, &ctx);
        assert_eq!(a.force_budget.as_ref().unwrap().0, "15 N"); // no held object -> unclamped
        assert_eq!(a.safety_envelope.compliance.as_deref(), Some("active"));
        let monitors = serde_json::to_string(&a.monitors).unwrap();
        assert!(monitors.contains("all_of"));
        assert!(monitors.contains("effort_rise"));
        assert!(monitors.contains("depth"));
    }
```

- [ ] **Step 6: Run the rfl-core tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core`
Expected: PASS (the new per-hand reaction test + the updated insert_fit test + all others).

- [ ] **Step 7: Regenerate and eyeball the cable goldens**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance`
Expected: FAIL on all three goldens — insert_fit `force_budget` (and the `force_profile.axial`) change 15 → 10 / 7.5 / 6.

Run: `export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance` then `git diff -- crates/rfl-conformance/tests/snapshots/`
Expected diff: ONLY the insert_fit action's `force_budget` and `force_profile.axial` change (allegro 10, leap 7.5, pneumatic 6).

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance`
Expected: PASS (all 5 green).

- [ ] **Step 8: Confirm validate.py + full suite green** (separate batch from commit)

Run: `cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py && export PATH="$HOME/.cargo/bin:$PATH" && cargo test`
Expected: validate.py `PASS`; all cargo green.

- [ ] **Step 9: Commit** (separate batch)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-core/src/translation.rs crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_allegro.snap crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_leap.snap crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_pneumatic.snap && git commit -m "feat(core): clamp insert_fit force budget to grasp reaction capacity (GF3c)

The held grasp's reaction capacity (grip_force_max / k_reaction) bounds the
insertion force so the part does not slip in-grasp before seating: the 15 N
budget clamps to 10 / 7.5 / 6 N for allegro / leap / pneumatic.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: Full verification + push

**Files:** none (verification + integration).

- [ ] **Step 1: Run the complete test suite + schema validation**

Run: `cd ~/Documents/GitHub/rfl && export PATH="$HOME/.cargo/bin:$PATH" && cargo test && uv run --with jsonschema --with pyyaml python schemas/validate.py`
Expected: every crate green (rfl-core unit tests incl. grasp_force/quantity/translation; rfl-conformance 5 + surface_scan 5); validate.py `PASS` EXIT=0.

- [ ] **Step 2: Eyeball the retargeted output for each hand** (sanity — confirm the derived values appear)

Run: `cd ~/Documents/GitHub/rfl && for h in allegro leap pneumatic-6f; do echo "== $h =="; python examples/01-cable-insertion/run.py --embodiment $h | python -c "import sys,json; [print(json.loads(l)['action_id'], json.loads(l)['canonical_action']['safety_envelope'].get('force_profile'), json.loads(l)['canonical_action']['safety_envelope']['motion_bounds'].get('a_max')) for l in sys.stdin]"; done`
Expected: pinch lines show `{'min_holding_force': '2.9 N'}`; pneumatic transport shows `a_max 0.33816 m/s^2` while allegro/leap show `1.5`/`2.0 m/s^2`; insert_fit shows `{'axial': '10 N'/'7.5 N'/'6 N'}`. (If `run.py` takes a different flag, fall back to `cargo run -p rfl-cli -- retarget examples/01-cable-insertion/skill.yaml --embodiment examples/01-cable-insertion/embodiments/<h>.yaml`.)

- [ ] **Step 3: Push the increment** (ff-checked; the four commits from Tasks 1-6)

Run: `cd ~/Documents/GitHub/rfl && git rev-parse --abbrev-ref HEAD && git fetch -q origin && git merge-base --is-ancestor origin/main HEAD && git push -q origin main && git rev-list --left-right --count origin/main...HEAD`
Expected: branch `main`; push succeeds; final count `0 0`. If the ff-check fails (parallel session advanced `main`), `git rebase origin/main` (conflict-free for these files), re-run the full suite (Step 1), then push.

- [ ] **Step 4: Update the memory** (`~/.claude/.../memory/project_rfl.md` "## Implementation track") with the real commit hashes from `git log --oneline -6`, recording this as the 3rd increment (GF1c/GF2c/GF3c). Not a repo commit — memory only.

---

## Self-review (completed during planning)

- **Spec coverage:** design §3 GF1c → Task 4; GF2c → Task 5; GF3c → Task 6; capacity model (§3) → Task 2; plumbing (§4 Decision A) → Task 4; weight input (§4 Decision B + §6 fixture) → Task 1; output placement (§5) → Tasks 4/5/6; determinism/from_si (§4 Decision C) → Task 3; conformance (§7) → Tasks 4/5/6 goldens + Task 7. GF4c + payload-violation explicitly out of scope (design §2). No gaps.
- **Type consistency:** `GraspMode::Pinch` / `k_holding` / `k_reaction` / `payload_key` / `min_holding_force` / `dynamic_a_max` / `reaction_limit` / `Quantity::from_si` / `GraspContext` / `HeldObject{weight_n, mode}` / `build_weights` used identically across Tasks 2-6. `lower` arity (4 args) consistent from Task 4 onward; transport/insert_fit arms migrated to pass `ctx` in Tasks 5/6.
- **No placeholders:** every code step shows complete code; every run step shows the exact command + expected output.
- **Determinism:** derived values via `round6` + `from_si`; kinematic-ceiling case keeps the authored string verbatim (no reformat that would turn `"2.0 m/s^2"` into `"2 m/s^2"`).
