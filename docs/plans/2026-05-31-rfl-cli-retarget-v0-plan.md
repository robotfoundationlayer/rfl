# rfl-cli retarget v0 (cable-insertion path) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `retarget(skill, embodiment) -> canonical_actions` for the seven cable-insertion primitives, emit driver-interface `execute` messages as JSONL, and make conformance test class 2 (generation determinism, RD1c) runnable in CI.

**Architecture:** Ahead-of-time compilation. `rfl-core` parses the Skill ISA composition and the embodiment descriptor into typed ASTs, lowers each skill statement into a `CanonicalAction` (resolving embodiment-dependent values now, leaving runtime-measured poses symbolic), and serializes each action as an `execute` message. `rfl-cli` wires the `retarget` subcommand to stdout JSONL. `rfl-conformance` pins determinism with insta golden snapshots, a generate-twice property test, and per-line schema validation.

**Tech Stack:** Rust (edition 2024, workspace), serde + serde_yaml + serde_json, nalgebra (pose geometry), insta (golden snapshots), proptest (determinism property), boon (JSON Schema Draft 2020-12 validation).

---

## Design references (authoritative, read before coding the relevant task)

- Design doc: `docs/design/2026-05-31-rfl-cli-retarget-v0-design.md`.
- `spec/02-translation-layer.md`: `CanonicalAction` tuple, `Envelope`, `Pose6D`, `theta_orient`, the under-constrained residual rule, RD1c/CA4c.
- `schemas/skill-isa.schema.json` `$defs`: the **precise** parameter source for each primitive (already read; cited per task).
- `schemas/embodiment-descriptor.schema.json` and the three committed descriptors under `examples/01-cable-insertion/embodiments/`: the descriptor shape and limit names.
- `examples/01-cable-insertion/driver-messages/execute.yaml`: the **message structure** of a canonical action (field names only; its concrete pose numbers are a runtime-resolved illustration, not retarget output).
- `spec/03-driver-interface.md` § Capability checking / § Capability manifest: the `capability_absent` gate rule (transcribe in Task 7).
- `spec/04-tactile-manifold.md` § The `TactileTarget` type / § Graceful degradation and the force/position proxy: the `auto` expansion and proxy degradation (transcribe in Task 8).

## Conventions for every task

- The reference instances (`skill.yaml`, the three descriptors, `driver-messages/*.yaml`) are committed ground truth: tests that deserialize them are the anti-fabrication oracle. Never hand-transcribe a parameter set from memory; copy it from the cited `schemas/skill-isa.schema.json` `$def`.
- v0 models only the parameters the reference `skill.yaml` actually uses (YAGNI). The full per-primitive parameter set lives in the schema and is added when a future skill exercises it.
- Physical quantities are carried as their original unit-suffixed strings (`"8 N"`), never parsed to float for output. This removes float-formatting nondeterminism by construction (RD1c).
- Run `cargo test` and read PASS in a step **separate** from any `git commit` step. Never stage while a check is red.
- `git add` names explicit files (never `-A`). Before each commit: `git rev-parse --abbrev-ref HEAD` is `main`. Before push: `git merge-base --is-ancestor origin/main HEAD`. After push: `git rev-list --left-right --count origin/main...HEAD` is `0 0`. A parallel session moves `main`; re-verify each batch.
- Commit messages: Conventional Commits with the trailer `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

## Task 1: Baseline smoke check

**Files:** none (verification only)

- [ ] **Step 1: Build and test the untouched scaffold**

Run: `cd ~/Documents/GitHub/rfl && cargo build 2>&1 | tail -5 && cargo test 2>&1 | tail -15`
Expected: build succeeds; `rfl-core` / `rfl-cli` / `rfl-conformance` compile; existing tests (if any) pass. The `rfl spec-version` arm works; `retarget` currently bails "not yet implemented".

- [ ] **Step 2: Confirm the input-schema suite is green (unchanged baseline)**

Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py; echo "EXIT=$?"`
Expected: `EXIT=0`. This suite stays green through the whole plan (it validates inputs, not retarget output).

No commit (verification only).

---

## Task 2: Shared input scalar types (`rfl-core`)

**Files:**
- Create: `crates/rfl-core/src/quantity.rs`
- Modify: `crates/rfl-core/src/lib.rs` (add `pub mod quantity;`)

The Skill ISA dimensioned scalars (`schemas/skill-isa.schema.json` `$defs` Force/Length/Duration/Angle/Velocity/Acceleration/AngularVelocity) are unit-suffixed strings. v0 carries them verbatim and parses a magnitude+unit only when clamping.

- [ ] **Step 1: Write the failing test**

In `crates/rfl-core/src/quantity.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Dimensioned scalar quantities (`spec/01` § Dimensioned scalars).
//!
//! Quantities are carried as their original unit-suffixed strings so that
//! retargeting never reformats a float (the determinism lever, RD1c). A
//! magnitude and unit are parsed only when a value must be clamped.

/// A dimensioned scalar as authored: a magnitude and a unit suffix (e.g. `8 N`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Quantity(pub String);

impl Quantity {
    /// Parse the leading magnitude and the trailing unit, e.g. `"8 N"` -> `(8.0, "N")`.
    /// Returns `None` if the string is not a `<number> <unit>` quantity.
    #[must_use]
    pub fn parse(&self) -> Option<(f64, &str)> {
        let s = self.0.trim();
        let (num, unit) = s.split_once(char::is_whitespace)?;
        let value: f64 = num.trim().parse().ok()?;
        Some((value, unit.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_magnitude_and_unit() {
        assert_eq!(Quantity("8 N".into()).parse(), Some((8.0, "N")));
        assert_eq!(Quantity("50 mm".into()).parse(), Some((50.0, "mm")));
        assert_eq!(Quantity("0.05 m/s".into()).parse(), Some((0.05, "m/s")));
    }

    #[test]
    fn rejects_non_quantity() {
        assert_eq!(Quantity("auto".into()).parse(), None);
    }
}
```

- [ ] **Step 2: Add the module to `lib.rs`**

In `crates/rfl-core/src/lib.rs`, alongside `pub mod skill_isa;`:

```rust
pub mod quantity;
```

- [ ] **Step 3: Run the test**

Run: `cargo test -p rfl-core quantity:: 2>&1 | tail -15`
Expected: 2 tests PASS.

- [ ] **Step 4: Commit**

```bash
git rev-parse --abbrev-ref HEAD   # expect: main
git add crates/rfl-core/src/quantity.rs crates/rfl-core/src/lib.rs
git commit -m "$(printf 'feat(core): add Quantity scalar type\n\nCarry dimensioned scalars as unit-suffixed strings and parse a\nmagnitude/unit only for clamping, keeping retarget output free of\nfloat-formatting nondeterminism (RD1c).\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 3: Skill ISA input AST and parser (`rfl-core`)

**Files:**
- Modify: `crates/rfl-core/src/skill_isa.rs` (extend the existing `Category` / `PrimitiveId` with the AST + parser)
- Test: same file, `#[cfg(test)]`

Model the concrete YAML algebra (`schemas/skill-isa.schema.json` `$defs` Composition/Sequence/LetBind/PrimitiveCall) and the seven primitives, modeling only the parameters `skill.yaml` uses. The committed `skill.yaml` is the completeness oracle.

The seven primitives' v0 parameter sets, copied from the cited `$defs`:
- `sense.locate` (`$defs/SenseLocateParams`): `target_ref` (Ref), `modality` (enum visual|tactile|fused|auto).
- `grasp.pinch` (`$defs/GraspPinchParams`): `target` (ObjectTarget|Ref), `force_budget` (Force), `tactile_target` (TactileTarget|auto|Ref), `slip_response` (enum abort|retighten|hold).
- `transport.move_to_pose` (`$defs/TransportMoveToPoseParams`): `target_pose` (object|Ref).
- `reach.align` (`$defs/ReachAlignParams`): `target_frame` (FrameRef), `axes` (`all` | array of x|y|z).
- `force.insert_fit` (`$defs/ForceInsertFitParams`): `target_fit` (Compound|Ref), `force_budget` (Force), `compliance` (enum passive|active|auto), `stop_condition` (SeatingSpec).
- `grasp.release` (`$defs/GraspReleaseParams`): `grasp_handle` (`active` | Ref).
- `reach.retract` (`$defs/ReachRetractParams`): `direction` (Direction), `distance` (Length).

- [ ] **Step 1: Write the failing test (deserialize the committed skill)**

Append to `crates/rfl-core/src/skill_isa.rs`:

```rust
#[cfg(test)]
mod parse_tests {
    use super::*;
    use std::path::Path;

    fn cable_skill() -> Skill {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/01-cable-insertion/skill.yaml");
        let text = std::fs::read_to_string(p).expect("read skill.yaml");
        Skill::parse_yaml(&text).expect("parse skill.yaml")
    }

    #[test]
    fn parses_header_and_objects() {
        let s = cable_skill();
        assert_eq!(s.skill, "cable-insertion");
        assert!(s.objects.contains_key("connector"));
        assert_eq!(s.objects["receptacle"].r#ref, "receptacle");
    }

    #[test]
    fn body_has_eight_statements_in_order() {
        let s = cable_skill();
        let stmts = &s.body.sequence;
        assert_eq!(stmts.len(), 8);
        assert!(matches!(stmts[0], Statement::LetBind(_)));        // let connector_t
        assert!(matches!(&stmts[1], Statement::Primitive(Primitive::GraspPinch(_))));
        assert!(matches!(&stmts[2], Statement::Primitive(Primitive::TransportMoveToPose(_))));
        assert!(matches!(stmts[3], Statement::LetBind(_)));        // let receptacle_t
        assert!(matches!(&stmts[4], Statement::Primitive(Primitive::ReachAlign(_))));
        assert!(matches!(&stmts[5], Statement::Primitive(Primitive::ForceInsertFit(_))));
        assert!(matches!(&stmts[6], Statement::Primitive(Primitive::GraspRelease(_))));
        assert!(matches!(&stmts[7], Statement::Primitive(Primitive::ReachRetract(_))));
    }

    #[test]
    fn pinch_params_parse() {
        let s = cable_skill();
        let Statement::Primitive(Primitive::GraspPinch(p)) = &s.body.sequence[1] else {
            panic!("expected grasp.pinch");
        };
        assert_eq!(p.force_budget.0, "8 N");
        assert!(matches!(p.tactile_target, TactileTargetArg::Auto));
        assert_eq!(p.slip_response, Some(SlipResponse::Retighten));
    }

    #[test]
    fn insert_fit_stop_condition_parses() {
        let s = cable_skill();
        let Statement::Primitive(Primitive::ForceInsertFit(p)) = &s.body.sequence[5] else {
            panic!("expected force.insert_fit");
        };
        assert_eq!(p.force_budget.0, "15 N");
        assert_eq!(p.compliance, Some(Compliance::Active));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p rfl-core parse_tests 2>&1 | tail -20`
Expected: FAIL to compile (`Skill`, `Statement`, `Primitive`, etc. not defined).

- [ ] **Step 3: Write the AST and parser**

Replace the trailing `// TODO` in `crates/rfl-core/src/skill_isa.rs` with the AST. Keep the existing `Category` / `PrimitiveId`. Add:

```rust
use std::collections::BTreeMap;
use crate::quantity::Quantity;

/// A reference to a let-bound value (`$defs/Ref`): a bare identifier naming a value
/// bound earlier by a let-bind.
pub type Ref = String;
/// A calibrated frame name (`$defs/FrameRef`).
pub type FrameRef = String;

/// A parsed Skill ISA composition file (`schemas/skill-isa.schema.json` top level).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Skill {
    pub skill: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub objects: BTreeMap<String, ObjectDecl>,
    pub body: Sequence,
}

/// A task object reference (`$defs/ObjectDecl`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ObjectDecl {
    pub r#ref: String,
}

/// An ordered composition (`$defs/Sequence`). v0 supports the sequence body only.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Sequence {
    pub sequence: Vec<Statement>,
}

/// One statement of a sequence: a primitive call or a let-bind.
/// `serde(untagged)` discriminates by shape: a `{let, from}` object is a LetBind,
/// any other single-key object is a primitive call.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum Statement {
    LetBind(LetBind),
    Primitive(Primitive),
}

/// A let-bind statement (`$defs/LetBind`): bind `let` to the result of `from`
/// (a sense.* call), scoped over the rest of the enclosing sequence.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LetBind {
    pub r#let: String,
    pub from: Box<Primitive>,
}

/// A core primitive invocation (`$defs/PrimitiveCall`): a single key naming the
/// primitive, mapping to its parameters. v0 models the seven cable-insertion
/// primitives; any other key deserializes to `Unsupported` and is rejected at
/// lowering time.
#[derive(Debug, Clone, serde::Deserialize)]
pub enum Primitive {
    #[serde(rename = "sense.locate")]
    SenseLocate(SenseLocate),
    #[serde(rename = "grasp.pinch")]
    GraspPinch(GraspPinch),
    #[serde(rename = "transport.move_to_pose")]
    TransportMoveToPose(TransportMoveToPose),
    #[serde(rename = "reach.align")]
    ReachAlign(ReachAlign),
    #[serde(rename = "force.insert_fit")]
    ForceInsertFit(ForceInsertFit),
    #[serde(rename = "grasp.release")]
    GraspRelease(GraspRelease),
    #[serde(rename = "reach.retract")]
    ReachRetract(ReachRetract),
}

/// `sense.locate` modality (`$defs/SenseLocateParams.modality`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modality { Visual, Tactile, Fused, Auto }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SenseLocate {
    pub target_ref: Ref,
    #[serde(default)]
    pub modality: Option<Modality>,
}

/// `grasp.pinch` slip response (`$defs/SlipResponse`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlipResponse { Abort, Retighten, Hold }

/// `tactile_target` argument (`$defs/TactileTargetOrAuto`). v0 needs only the
/// `auto` form the reference uses; an inline target or a let-reference round-trips
/// as `Other` (carried, not interpreted, in v0).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum TactileTargetArg {
    Auto(AutoLiteral),
    Other(serde_yaml::Value),
}

/// The literal string `auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum AutoLiteral { #[serde(rename = "auto")] Auto }

// Convenience matcher so tests can write `TactileTargetArg::Auto`-like checks.
impl TactileTargetArg {
    #[must_use]
    pub fn is_auto(&self) -> bool { matches!(self, TactileTargetArg::Auto(_)) }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraspPinch {
    pub target: Ref,
    pub force_budget: Quantity,
    #[serde(default = "tactile_auto")]
    pub tactile_target: TactileTargetArg,
    #[serde(default)]
    pub slip_response: Option<SlipResponse>,
}

fn tactile_auto() -> TactileTargetArg { TactileTargetArg::Auto(AutoLiteral::Auto) }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TransportMoveToPose {
    /// `Pose6D` floored as an inline object (here a frame-relative offset) or a Ref.
    pub target_pose: serde_yaml::Value,
}

/// `reach.align` axes (`$defs/Axes`): the literal `all` or a set of principal axes.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum Axes {
    All(AllLiteral),
    Set(Vec<Axis>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum AllLiteral { #[serde(rename = "all")] All }
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis { X, Y, Z }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReachAlign {
    pub target_frame: FrameRef,
    #[serde(default = "axes_all")]
    pub axes: Axes,
}
fn axes_all() -> Axes { Axes::All(AllLiteral::All) }

/// `force.insert_fit` compliance (`$defs/Compliance`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Compliance { Passive, Active, Auto }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceInsertFit {
    pub target_fit: Ref,
    pub force_budget: Quantity,
    #[serde(default)]
    pub compliance: Option<Compliance>,
    /// `SeatingSpec` (`$defs/SeatingSpec`): carried structurally in v0 and lowered
    /// into `monitors` in Task 10.
    pub stop_condition: serde_yaml::Value,
}

/// A grasp handle (`$defs/GraspHandle`): the literal `active` or a Ref.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum GraspHandle { Active(ActiveLiteral), Ref(Ref) }
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum ActiveLiteral { #[serde(rename = "active")] Active }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraspRelease {
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
}

/// A direction (`$defs/Direction`): a signed named axis (`-tool_axis`), an inline
/// unit vector, or a Ref. v0 carries the string form the reference uses.
pub type Direction = serde_yaml::Value;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReachRetract {
    pub direction: Direction,
    pub distance: Quantity,
}

impl Skill {
    /// Parse a Skill ISA composition from YAML text.
    ///
    /// # Errors
    /// Returns `Error::SkillIsa` if the YAML does not match the v0-supported
    /// subset (header + sequence body + the seven primitives + let-bind).
    pub fn parse_yaml(text: &str) -> crate::Result<Self> {
        serde_yaml::from_str(text).map_err(|e| crate::Error::SkillIsa(e.to_string()))
    }
}
```

Note: `serde_yaml::Error` is not one of `Error`'s `#[from]` variants, so the parser maps it explicitly. If `crate::Error` needs a `serde_yaml` arm elsewhere, add it in `lib.rs`; v0 does not require it.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p rfl-core parse_tests 2>&1 | tail -25`
Expected: 4 tests PASS. If `Statement(untagged)` mis-discriminates a let-bind as a primitive, confirm `LetBind` is the first `untagged` variant (serde tries variants in order).

- [ ] **Step 5: Commit**

```bash
cargo test -p rfl-core 2>&1 | tail -5   # read PASS in this step
```
```bash
git rev-parse --abbrev-ref HEAD   # expect: main
git add crates/rfl-core/src/skill_isa.rs
git commit -m "$(printf 'feat(core): parse the Skill ISA cable-insertion subset\n\nModel the sequence/let-bind algebra and the seven cable-insertion\nprimitives (parameters per schemas/skill-isa.schema.json defs), parsed\nfrom YAML. The committed skill.yaml is the completeness oracle.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 4: Embodiment descriptor input (`rfl-core`)

**Files:**
- Create: `crates/rfl-core/src/embodiment.rs`
- Modify: `crates/rfl-core/src/lib.rs` (add `pub mod embodiment;`)
- Test: same file

Model the descriptor fields retarget reads (`schemas/embodiment-descriptor.schema.json`; the three committed descriptors are the oracle): `id`, the capability `skills` list, `aux.tactile_sensing`, `aux.compliance`, and the `limits` retarget clamps to.

- [ ] **Step 1: Write the failing test**

In `crates/rfl-core/src/embodiment.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Embodiment descriptor input (`spec/03`, `schemas/embodiment-descriptor.schema.json`).
//!
//! Models the fields the Translation Layer reads: the capability manifest (for the
//! `capability_absent` gate), `aux.tactile_sensing` (for tactile/proxy routing), and
//! the `limits` retargeting clamps envelope bounds to.

use std::collections::BTreeMap;
use crate::quantity::Quantity;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn descriptor(stem: &str) -> Embodiment {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../examples/01-cable-insertion/embodiments/{stem}.yaml"));
        let text = std::fs::read_to_string(p).expect("read descriptor");
        Embodiment::parse_yaml(&text).expect("parse descriptor")
    }

    #[test]
    fn allegro_declares_tactile_and_grip_limit() {
        let e = descriptor("allegro");
        assert_eq!(e.id, "wonik-allegro-v4");
        assert!(e.has_skill("grasp.pinch"));
        assert!(e.has_skill("transport"));
        assert_eq!(e.tactile_sensing(), true);
        assert_eq!(e.limits.get("grip_force_max").unwrap().0, "20 N");
        assert_eq!(e.grasp_frame(), "tcp_thumb"); // role_defaults.grasp
        assert_eq!(e.control_frame(), "tcp_index"); // first control_frames entry
    }

    #[test]
    fn pneumatic_has_no_tactile_sensing() {
        let e = descriptor("pneumatic-6f");
        assert_eq!(e.id, "generic-pneumatic-6f");
        assert!(e.has_skill("grasp.pinch"));
        assert_eq!(e.tactile_sensing(), false);
        assert_eq!(e.limits.get("grip_force_max").unwrap().0, "12 N");
        assert_eq!(e.grasp_frame(), "palm"); // role_defaults.grasp differs per embodiment
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p rfl-core embodiment 2>&1 | tail -15`
Expected: FAIL to compile (`Embodiment` not defined).

- [ ] **Step 3: Implement the descriptor**

Add to `crates/rfl-core/src/embodiment.rs`:

```rust
/// A parsed embodiment descriptor. The file wraps everything under `embodiment:`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EmbodimentFile {
    pub embodiment: Embodiment,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Embodiment {
    pub id: String,
    #[serde(default)]
    pub class: Option<String>,
    pub capabilities: Capabilities,
    #[serde(default)]
    pub limits: BTreeMap<String, Quantity>,
    /// Contact-sensor descriptor (`spec/04`); absent on a no-tactile embodiment.
    #[serde(default)]
    pub tactile: Option<serde_yaml::Value>,
    /// Frame model (`spec/03` § Embodiment frame model): control frames + role defaults.
    #[serde(default)]
    pub frames: Frames,
    #[serde(default)]
    pub sensors: Option<serde_yaml::Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Capabilities {
    pub skills: Vec<String>,
    #[serde(default)]
    pub aux: Aux,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Aux {
    #[serde(default)]
    pub tactile_sensing: Option<bool>,
    #[serde(default)]
    pub compliance: Option<serde_yaml::Value>,
}

/// The embodiment frame model (`spec/03` § Embodiment frame model). Roles, never
/// body parts: `role_defaults` names which control frame plays each role.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Frames {
    #[serde(default)]
    pub control_frames: Vec<String>,
    #[serde(default)]
    pub role_defaults: RoleDefaults,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct RoleDefaults {
    #[serde(default)]
    pub grasp: Option<String>,
    #[serde(default)]
    pub sensor: Option<String>,
    #[serde(default)]
    pub tactile: Option<String>,
    #[serde(default)]
    pub support: Option<String>,
}

impl Embodiment {
    /// Parse a descriptor from YAML text (unwrapping the `embodiment:` envelope).
    ///
    /// # Errors
    /// Returns `Error::Driver` if the YAML does not match the descriptor shape.
    pub fn parse_yaml(text: &str) -> crate::Result<Self> {
        let f: EmbodimentFile =
            serde_yaml::from_str(text).map_err(|e| crate::Error::Driver(e.to_string()))?;
        Ok(f.embodiment)
    }

    /// Whether a capability key (a primitive id, a grasp mode, or a category gate
    /// such as `transport`) is declared.
    #[must_use]
    pub fn has_skill(&self, key: &str) -> bool {
        self.capabilities.skills.iter().any(|s| s == key)
    }

    /// Whether the embodiment declares `aux.tactile_sensing: true`.
    #[must_use]
    pub fn tactile_sensing(&self) -> bool {
        self.capabilities.aux.tactile_sensing.unwrap_or(false)
    }

    /// The default grasp control frame (`frames.role_defaults.grasp`, e.g. `tcp_thumb`).
    /// Differs per embodiment, realizing the Principle-1 point that only the
    /// descriptor changes between hands.
    #[must_use]
    pub fn grasp_frame(&self) -> &str {
        self.frames.role_defaults.grasp.as_deref().unwrap_or("grasp")
    }

    /// The default sensor frame (`frames.role_defaults.sensor`).
    #[must_use]
    pub fn sensor_frame(&self) -> &str {
        self.frames.role_defaults.sensor.as_deref().unwrap_or("sensor")
    }

    /// The default control frame. v0 uses the first declared control frame; TRANSCRIBE
    /// the exact `default_control_frame` rule from `spec/03` § Embodiment frame model
    /// when refining.
    #[must_use]
    pub fn control_frame(&self) -> &str {
        self.frames.control_frames.first().map(String::as_str).unwrap_or("control")
    }
}
```

Add to `crates/rfl-core/src/lib.rs`:

```rust
pub mod embodiment;
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p rfl-core embodiment 2>&1 | tail -15`
Expected: 2 tests PASS.

- [ ] **Step 5: Commit**

```bash
git rev-parse --abbrev-ref HEAD   # expect: main
git add crates/rfl-core/src/embodiment.rs crates/rfl-core/src/lib.rs
git commit -m "$(printf 'feat(core): parse the embodiment descriptor\n\nModel the capability manifest, aux.tactile_sensing, and the limits\nnamespace the Translation Layer reads. The three committed cable-\ninsertion descriptors are the oracle.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 5: Pose geometry (`rfl-core`)

**Files:**
- Create: `crates/rfl-core/src/pose.rs`
- Modify: `crates/rfl-core/src/lib.rs` (add `pub mod pose;`)
- Test: same file

`spec/02` § Pose representation: `Pose6D` = position (R³, m) + unit quaternion; `theta_orient = 2 * arccos(|<q_t, q_c>|)`. v0 provides the type and the decision function; numeric pose resolution is deferred, so these are lightly used now but verified.

- [ ] **Step 1: Write the failing test**

In `crates/rfl-core/src/pose.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Pose representation and orientation error (`spec/02` § Pose representation).

use nalgebra::UnitQuaternion;

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn q(w: f64, x: f64, y: f64, z: f64) -> UnitQuaternion<f64> {
        UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(w, x, y, z))
    }

    #[test]
    fn identical_orientations_have_zero_error() {
        let a = q(1.0, 0.0, 0.0, 0.0);
        assert!(theta_orient(&a, &a).abs() < 1e-9);
    }

    #[test]
    fn opposite_sign_is_same_orientation() {
        // q and -q denote one orientation: the double-cover resolves to ~0.
        let a = q(1.0, 0.0, 0.0, 0.0);
        let b = q(-1.0, 0.0, 0.0, 0.0);
        assert!(theta_orient(&a, &b).abs() < 1e-9);
    }

    #[test]
    fn quarter_turn_about_z_is_half_pi() {
        let a = q(1.0, 0.0, 0.0, 0.0);
        let half = (PI / 4.0).cos();
        let b = q(half, 0.0, 0.0, half); // 90 deg about z
        assert!((theta_orient(&a, &b) - PI / 2.0).abs() < 1e-6);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p rfl-core pose 2>&1 | tail -15`
Expected: FAIL to compile (`theta_orient` not defined).

- [ ] **Step 3: Implement**

Add to `crates/rfl-core/src/pose.rs`:

```rust
/// A rigid-body pose: position in R^3 (metres) and a unit-quaternion orientation
/// (`spec/02` § Pose representation). The unit quaternion is the canonical
/// serialization; SE(3) is the group beneath it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose6D {
    pub position: [f64; 3],
    pub orientation: UnitQuaternion<f64>,
}

/// The single-scalar geodesic orientation error (`spec/02`, CA1c):
/// `theta = 2 * arccos(|<q_t, q_c>|)`, in `[0, pi]`. The absolute value resolves
/// the quaternion double-cover so `q` and `-q` denote one orientation.
#[must_use]
pub fn theta_orient(q_t: &UnitQuaternion<f64>, q_c: &UnitQuaternion<f64>) -> f64 {
    let dot = q_t.quaternion().dot(q_c.quaternion()).abs().min(1.0);
    2.0 * dot.acos()
}
```

Add to `crates/rfl-core/src/lib.rs`:

```rust
pub mod pose;
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p rfl-core pose 2>&1 | tail -15`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git rev-parse --abbrev-ref HEAD   # expect: main
git add crates/rfl-core/src/pose.rs crates/rfl-core/src/lib.rs
git commit -m "$(printf 'feat(core): add Pose6D and the geodesic orientation error\n\nProvide the quaternion pose type and theta_orient (spec/02 CA1c), with\nthe double-cover resolved so q and -q denote one orientation.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 6: Canonical action output model and serialization (`rfl-core`)

**Files:**
- Create: `crates/rfl-core/src/canonical.rs`
- Modify: `crates/rfl-core/src/lib.rs` (add `pub mod canonical;`)
- Test: same file

The output model mirrors `spec/02`'s `CanonicalAction` tuple and `Envelope`, and the `execute` message structure of `examples/01-cable-insertion/driver-messages/execute.yaml`. **`PoseExpr`'s symbolic variants are a v0 reference-implementation choice** (the eventual concrete `Pose6D` representation is owned by `spec/02`); they carry the runtime-deferred poses and are pinned by the golden snapshots, not by the spec. Serialization is deterministic: struct field order is fixed, dynamic maps use `BTreeMap`, quantities are strings, and no float is emitted by v0 lowering.

- [ ] **Step 1: Write the failing test**

In `crates/rfl-core/src/canonical.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Canonical action output model and `execute`-message serialization.
//!
//! Mirrors the `spec/02` CanonicalAction tuple / Envelope and the `execute`
//! message of `schemas/driver-interface.schema.json`. PoseExpr's symbolic
//! variants are a v0 reference-implementation choice carrying runtime-deferred
//! poses; the concrete Pose6D representation is owned by spec/02.

use crate::quantity::Quantity;

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CanonicalAction {
        CanonicalAction {
            target_frame: "tcp_thumb".into(),
            target_pose: PoseExpr::Ref { r#ref: "receptacle_t".into() },
            force_budget: Some(Quantity("15 N".into())),
            timing: TimingHints { nominal_duration: None, timing_mode: TimingMode::Strict, stop_at_goal: true },
            tactile_target: Some(TactileTargetOut::Auto),
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: None,
                clearance: None,
                compliance: Some("active".into()),
                stop_time: Some(Quantity("0.2 s".into())),
            },
        }
    }

    #[test]
    fn execute_message_has_expected_shape() {
        let msg = ExecuteGoal::wrap("cable-insertion/wonik-allegro-v4/0001-pinch", sample());
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"message\":\"execute\""));
        assert!(json.contains("\"action_id\":\"cable-insertion/wonik-allegro-v4/0001-pinch\""));
        assert!(json.contains("\"target_frame\":\"tcp_thumb\""));
        assert!(json.contains("\"force_budget\":\"15 N\""));
    }

    #[test]
    fn serialization_is_deterministic() {
        let msg = ExecuteGoal::wrap("a/b/0001-x", sample());
        let a = serde_json::to_string(&msg).unwrap();
        let b = serde_json::to_string(&msg).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn jsonl_joins_actions_one_per_line() {
        let actions = vec![sample(), sample()];
        let jsonl = to_jsonl("cable-insertion", "wonik-allegro-v4", &actions, &["pinch", "insert_fit"]);
        assert_eq!(jsonl.lines().count(), 2);
        assert!(jsonl.lines().next().unwrap().contains("0001-pinch"));
        assert!(jsonl.lines().nth(1).unwrap().contains("0002-insert_fit"));
        assert!(jsonl.ends_with('\n'));
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p rfl-core canonical 2>&1 | tail -15`
Expected: FAIL to compile.

- [ ] **Step 3: Implement the output model**

Add to `crates/rfl-core/src/canonical.rs`:

```rust
/// A canonical action (`spec/02` § Canonical action representation). Field order is
/// fixed so JSON serialization is byte-deterministic (RD1c).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CanonicalAction {
    pub target_frame: String,
    pub target_pose: PoseExpr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_budget: Option<Quantity>,
    pub timing: TimingHints,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tactile_target: Option<TactileTargetOut>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub monitors: Vec<Monitor>,
    pub safety_envelope: Envelope,
}

/// A target-pose expression. v0 carries runtime-deferred poses symbolically; the
/// concrete `{position, orientation}` form is reserved for resolved poses (spec/02).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum PoseExpr {
    /// A let-bound sensed pose (grasp.pinch target, force.insert_fit target_fit).
    Ref { r#ref: String },
    /// A frame-relative offset pose (transport.move_to_pose).
    FrameRelative { frame: String, offset: serde_json::Value },
    /// An orientation-only alignment with the under-constrained residual rule
    /// (reach.align): bring `axes` parallel to `target_frame`, residual = minimum
    /// geodesic rotation (spec/02 CA2c).
    OrientationAlign { align: AlignSpec },
    /// An embodiment-axis-relative withdraw (reach.retract, grasp.release).
    AxisRelative { direction: serde_json::Value, distance: Quantity },
    /// A fully resolved pose (not produced by v0 lowering; reserved).
    Concrete { position: [f64; 3], orientation: [f64; 4] },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AlignSpec {
    pub target_frame: String,
    pub axes: Vec<String>,
    pub residual: &'static str, // always "min_geodesic_rotation" in v0
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TimingHints {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nominal_duration: Option<Quantity>,
    pub timing_mode: TimingMode,
    pub stop_at_goal: bool,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimingMode { Strict, TimeScalable }

/// A StopCondition / ForceEvent the action watches (`spec/02` monitors). v0 carries
/// the stop condition structurally (built from the skill's `stop_condition`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Monitor {
    pub stop_condition: serde_json::Value,
}

/// The safety envelope (`spec/02` § The Envelope), bounds clamped to the
/// embodiment's limits.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Envelope {
    pub motion_bounds: MotionBounds,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_profile: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clearance: Option<Quantity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_time: Option<Quantity>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct MotionBounds {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v_max: Option<Quantity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a_max: Option<Quantity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w_max: Option<Quantity>,
}

/// The realized tactile-confirmation criterion after retargeting.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum TactileTargetOut {
    /// Manifold-tier: the `auto` criterion was kept (tactile embodiment).
    Auto,
    /// Proxy-tier degradation on a no-tactile embodiment (`spec/04`).
    Proxy { proxy: ProxySpec },
    /// An explicit inline target carried through.
    Explicit(serde_json::Value),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProxySpec {
    pub tier: &'static str, // "proxy"
    pub criterion: &'static str, // "position_convergence_and_force_hold"
}

/// An `execute` action Goal (`schemas/driver-interface.schema.json` ExecuteGoal).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecuteGoal {
    pub message: &'static str,
    pub action_id: String,
    pub canonical_action: CanonicalAction,
}

impl ExecuteGoal {
    /// Wrap a canonical action with its correlation id.
    #[must_use]
    pub fn wrap(action_id: impl Into<String>, canonical_action: CanonicalAction) -> Self {
        Self { message: "execute", action_id: action_id.into(), canonical_action }
    }
}

/// Serialize a retargeted action sequence as JSON Lines. The `action_id` is the
/// deterministic `{skill}/{embodiment_id}/{NNNN}-{suffix}` (1-based, zero-padded 4).
#[must_use]
pub fn to_jsonl(skill: &str, embodiment_id: &str, actions: &[CanonicalAction], suffixes: &[&str]) -> String {
    let mut out = String::new();
    for (i, (action, suffix)) in actions.iter().zip(suffixes).enumerate() {
        let id = format!("{skill}/{embodiment_id}/{:04}-{suffix}", i + 1);
        let msg = ExecuteGoal::wrap(id, action.clone());
        out.push_str(&serde_json::to_string(&msg).expect("serialize execute message"));
        out.push('\n');
    }
    out
}
```

Add to `crates/rfl-core/src/lib.rs`:

```rust
pub mod canonical;
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p rfl-core canonical 2>&1 | tail -15`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git rev-parse --abbrev-ref HEAD   # expect: main
git add crates/rfl-core/src/canonical.rs crates/rfl-core/src/lib.rs
git commit -m "$(printf 'feat(core): add canonical action model and execute serialization\n\nMirror the spec/02 CanonicalAction tuple and Envelope and the driver-\ninterface execute message; serialize deterministically (fixed field\norder, quantities as strings) and join as JSONL with a deterministic\naction_id. PoseExpr symbolic variants are a v0 reference-impl choice.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 7: Retarget engine skeleton, capability gate, and `sense.locate`

**Files:**
- Modify: `crates/rfl-core/src/translation.rs` (fill the existing TODO)
- Test: same file

**Read first:** `spec/03-driver-interface.md` § Capability checking and § Capability manifest. Transcribe the exact `capability_absent` rule (which keys gate which primitives: `reach.*` is the unkeyed baseline; a category such as `transport` gates its sub-primitives; `grasp.<mode>` gates that mode; how `grasp.release` is gated). Encode it; do not guess. The committed descriptors' `capabilities.skills` lists are: allegro `[grasp.pinch, grasp.lateral, grasp.envelope, transport, force.insert_fit, sense.locate]`, pneumatic `[grasp.pinch, grasp.envelope, transport, force.insert_fit, sense.locate]`.

- [ ] **Step 1: Write the failing test**

Replace the `// TODO` in `crates/rfl-core/src/translation.rs` with a test module that exercises the gate and the first primitive. Load the committed skill + a descriptor, retarget, and assert:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_isa::Skill;
    use crate::embodiment::Embodiment;
    use std::path::Path;

    fn load(stem: &str) -> (Skill, Embodiment) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion");
        let skill = Skill::parse_yaml(&std::fs::read_to_string(root.join("skill.yaml")).unwrap()).unwrap();
        let emb = Embodiment::parse_yaml(&std::fs::read_to_string(root.join(format!("embodiments/{stem}.yaml"))).unwrap()).unwrap();
        (skill, emb)
    }

    #[test]
    fn retarget_emits_one_action_per_motion_statement() {
        let (skill, emb) = load("allegro");
        let out = retarget(&skill, &emb).expect("retarget");
        // The two let-bound sense.locate calls plus the six motion primitives:
        // v0 emits an action for sense.locate too (a perception action). Eight total.
        assert_eq!(out.actions.len(), 8);
    }

    #[test]
    fn capability_absent_when_gate_key_missing() {
        let (skill, mut emb) = load("allegro");
        emb.capabilities.skills.retain(|s| s != "grasp.pinch");
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(matches!(err, crate::Error::Translation(_)));
        assert!(err.to_string().contains("capability_absent"));
        assert!(err.to_string().contains("grasp.pinch"));
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p rfl-core translation 2>&1 | tail -15`
Expected: FAIL to compile (`retarget`, `RetargetOutput` not defined).

- [ ] **Step 3: Implement the engine skeleton + gate + sense.locate**

Replace the `// TODO` body of `crates/rfl-core/src/translation.rs`:

```rust
use crate::canonical::{CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode};
use crate::embodiment::Embodiment;
use crate::skill_isa::{Primitive, Skill, Statement};

/// The retargeting result: the canonical action stream plus the per-action
/// primitive suffix used to build deterministic action ids.
pub struct RetargetOutput {
    pub actions: Vec<CanonicalAction>,
    pub suffixes: Vec<&'static str>,
}

/// Retarget a skill onto an embodiment (`spec/02`). Deterministic for identical
/// inputs (RD1c): embodiment-dependent values are resolved now; runtime-measured
/// poses stay symbolic.
///
/// # Errors
/// Returns `Error::Translation("capability_absent: <key>")` when a primitive's gate
/// key is absent from the descriptor's capability manifest, and for v0-unsupported
/// primitives.
pub fn retarget(skill: &Skill, embodiment: &Embodiment) -> crate::Result<RetargetOutput> {
    let mut actions = Vec::new();
    let mut suffixes = Vec::new();
    for stmt in &skill.body.sequence {
        let prim = match stmt {
            Statement::Primitive(p) => p,
            Statement::LetBind(b) => &b.from,
        };
        check_capability(prim, embodiment)?;
        let (action, suffix) = lower(prim, embodiment)?;
        actions.push(action);
        suffixes.push(suffix);
    }
    Ok(RetargetOutput { actions, suffixes })
}

/// The `capability_absent` gate (`spec/03` § Capability checking). TRANSCRIBE the
/// exact rule from the spec before finalizing; the structure below gates by the
/// key each primitive maps to.
fn check_capability(prim: &Primitive, e: &Embodiment) -> crate::Result<()> {
    let key: Option<&str> = match prim {
        // reach.* is the unkeyed baseline (always present): no gate.
        Primitive::ReachAlign(_) | Primitive::ReachRetract(_) => None,
        Primitive::SenseLocate(_) => Some("sense.locate"),
        Primitive::GraspPinch(_) => Some("grasp.pinch"),
        // grasp.release is gated by the presence of grasp capability — confirm the
        // exact rule in spec/03 § Capability checking.
        Primitive::GraspRelease(_) => None,
        Primitive::TransportMoveToPose(_) => Some("transport"),
        Primitive::ForceInsertFit(_) => Some("force.insert_fit"),
    };
    if let Some(k) = key {
        if !e.has_skill(k) {
            return Err(crate::Error::Translation(format!("capability_absent: {k}")));
        }
    }
    Ok(())
}

/// Lower one primitive to a canonical action. Each primitive arm is implemented in
/// its own task; this skeleton implements sense.locate and routes the rest to a
/// not-yet-implemented error so the engine compiles incrementally.
fn lower(prim: &Primitive, e: &Embodiment) -> crate::Result<(CanonicalAction, &'static str)> {
    match prim {
        Primitive::SenseLocate(p) => Ok((lower_sense_locate(p, e), "locate")),
        other => Err(crate::Error::Translation(format!(
            "lowering not yet implemented for {}",
            primitive_name(other)
        ))),
    }
}

fn primitive_name(p: &Primitive) -> &'static str {
    match p {
        Primitive::SenseLocate(_) => "sense.locate",
        Primitive::GraspPinch(_) => "grasp.pinch",
        Primitive::TransportMoveToPose(_) => "transport.move_to_pose",
        Primitive::ReachAlign(_) => "reach.align",
        Primitive::ForceInsertFit(_) => "force.insert_fit",
        Primitive::GraspRelease(_) => "grasp.release",
        Primitive::ReachRetract(_) => "reach.retract",
    }
}

/// `sense.locate` lowers to a perception action: the sensor acquires the named
/// object's pose; the result binds the enclosing let-variable at runtime. The
/// emitted target_pose references the object identity (symbolic, runtime-resolved).
fn lower_sense_locate(p: &crate::skill_isa::SenseLocate, e: &Embodiment) -> CanonicalAction {
    CanonicalAction {
        target_frame: e.sensor_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target_ref.clone() },
        force_budget: None,
        timing: TimingHints { nominal_duration: None, timing_mode: TimingMode::Strict, stop_at_goal: true },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: Envelope {
            motion_bounds: MotionBounds::default(),
            force_profile: None,
            clearance: None,
            compliance: None,
            stop_time: None,
        },
    }
}
```

Note: the first test (`retarget_emits...len()==8`) will fail until the remaining primitives are lowered (Tasks 8–10). Keep it but expect it red until Task 10; verify only the capability-gate test here. Adjust the test to assert `retarget` returns an error mentioning "not yet implemented" for now, OR mark `retarget_emits_one_action_per_motion_statement` `#[ignore]` with a note to un-ignore in Task 10. Choose the ignore approach.

- [ ] **Step 4: Run the gate test**

Run: `cargo test -p rfl-core translation 2>&1 | tail -20`
Expected: `capability_absent_when_gate_key_missing` PASS; `retarget_emits...` ignored.

- [ ] **Step 5: Commit**

```bash
git rev-parse --abbrev-ref HEAD   # expect: main
git add crates/rfl-core/src/translation.rs
git commit -m "$(printf 'feat(core): retarget engine skeleton, capability gate, sense.locate\n\nWalk the skill sequence, apply the spec/03 capability_absent gate, and\nlower sense.locate to a perception action binding its let-variable. The\nremaining primitives error as not-yet-implemented pending their tasks.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 8: Lower `grasp.pinch` (force clamp + tactile auto / proxy)

**Files:**
- Modify: `crates/rfl-core/src/translation.rs`
- Test: same file

**Read first:** `spec/04-tactile-manifold.md` § The `TactileTarget` type (the `auto` expansion: `normal_force >= epsilon at_least(2, antipodal)`) and § Graceful degradation and the force/position proxy (the degradation when `tactile_sensing` is undeclared). Transcribe the proxy criterion.

- [ ] **Step 1: Write the failing test**

Add to the `translation` test module:

```rust
#[test]
fn pinch_clamps_force_and_keeps_manifold_tier_on_allegro() {
    let (skill, emb) = load("allegro");
    let out = retarget_pinch_only(&skill, &emb);
    // force_budget 8 N <= grip_force_max 20 N -> stays 8 N.
    assert_eq!(out.force_budget.as_ref().unwrap().0, "8 N");
    assert!(matches!(out.tactile_target, Some(crate::canonical::TactileTargetOut::Auto)));
}

#[test]
fn pinch_degrades_to_proxy_on_pneumatic() {
    let (skill, emb) = load("pneumatic-6f");
    let out = retarget_pinch_only(&skill, &emb);
    assert_eq!(out.force_budget.as_ref().unwrap().0, "8 N"); // 8 <= 12
    assert!(matches!(out.tactile_target, Some(crate::canonical::TactileTargetOut::Proxy { .. })));
}

// Helper: lower just the grasp.pinch statement (index 1 of the sequence).
fn retarget_pinch_only(skill: &Skill, emb: &Embodiment) -> CanonicalAction {
    let Statement::Primitive(Primitive::GraspPinch(p)) = &skill.body.sequence[1] else { panic!() };
    super::lower_grasp_pinch(p, emb)
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p rfl-core translation::tests::pinch 2>&1 | tail -15`
Expected: FAIL to compile (`lower_grasp_pinch` not defined).

- [ ] **Step 3: Implement**

Add to `crates/rfl-core/src/translation.rs` a clamp helper and the lowering, and route `Primitive::GraspPinch` in `lower`:

```rust
use crate::canonical::{ProxySpec, TactileTargetOut};
use crate::quantity::Quantity;
use crate::skill_isa::{GraspPinch, TactileTargetArg};

/// Clamp a force quantity to a descriptor limit by magnitude, emitting the smaller
/// as a string (CA4c). Both are assumed to share a unit (N for grip force); if a
/// limit is absent the value passes through.
fn clamp_force(value: &Quantity, limit_key: &str, e: &Embodiment) -> Quantity {
    let Some(limit) = e.limits.get(limit_key) else { return value.clone() };
    match (value.parse(), limit.parse()) {
        (Some((v, vu)), Some((l, lu))) if vu == lu && l < v => limit.clone(),
        _ => value.clone(),
    }
}

/// Lower `grasp.pinch`. force_budget is clamped to grip_force_max (CA4c). The
/// tactile_target `auto` is kept (manifold tier) on a tactile embodiment and
/// degraded to the force/position proxy when tactile_sensing is undeclared
/// (spec/04 § Graceful degradation).
fn lower_grasp_pinch(p: &GraspPinch, e: &Embodiment) -> CanonicalAction {
    let force_budget = Some(clamp_force(&p.force_budget, "grip_force_max", e));
    let tactile_target = Some(match (&p.tactile_target, e.tactile_sensing()) {
        (TactileTargetArg::Auto(_), true) => TactileTargetOut::Auto,
        (TactileTargetArg::Auto(_), false) => TactileTargetOut::Proxy {
            proxy: ProxySpec { tier: "proxy", criterion: "position_convergence_and_force_hold" },
        },
        (TactileTargetArg::Other(v), _) => {
            TactileTargetOut::Explicit(serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
        }
    });
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target.clone() },
        force_budget,
        timing: TimingHints { nominal_duration: None, timing_mode: TimingMode::Strict, stop_at_goal: true },
        tactile_target,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}

/// A baseline envelope with motion bounds clamped to the embodiment's reach limits
/// and stop_time taken from the descriptor (CA4c). Force-specific fields are added
/// per-primitive.
fn base_envelope(e: &Embodiment) -> Envelope {
    Envelope {
        motion_bounds: MotionBounds {
            v_max: e.limits.get("v_cartesian_max").cloned(),
            a_max: e.limits.get("a_cartesian_max").cloned(),
            w_max: e.limits.get("w_cartesian_max").cloned(),
        },
        force_profile: None,
        clearance: None,
        compliance: None,
        stop_time: e.limits.get("stop_time").cloned(),
    }
}
```

Route it in `lower`:

```rust
        Primitive::GraspPinch(p) => Ok((lower_grasp_pinch(p, e), "pinch")),
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p rfl-core translation 2>&1 | tail -20`
Expected: the two pinch tests PASS plus the gate test.

- [ ] **Step 5: Commit**

```bash
git rev-parse --abbrev-ref HEAD   # expect: main
git add crates/rfl-core/src/translation.rs
git commit -m "$(printf 'feat(core): lower grasp.pinch with force clamp and tactile proxy\n\nClamp force_budget to grip_force_max (CA4c) and keep the auto tactile\ntarget on a tactile embodiment, degrading it to the force/position\nproxy when tactile_sensing is undeclared (spec/04 graceful degradation).\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 9: Lower `transport.move_to_pose` and `reach.align`

**Files:**
- Modify: `crates/rfl-core/src/translation.rs`
- Test: same file

`transport.move_to_pose`: target_pose is the skill's frame-relative offset object, carried through; max_acceleration is clamped to the embodiment kinematic ceiling `a_cartesian_max` (the mass-dependent dynamic-stability tightening is deferred, design § 3). `reach.align`: orientation-only; emit the under-constrained residual directive (spec/02 CA2c).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn transport_carries_frame_relative_pose() {
    let (skill, emb) = load("allegro");
    let Statement::Primitive(Primitive::TransportMoveToPose(p)) = &skill.body.sequence[2] else { panic!() };
    let a = super::lower_transport_move_to_pose(p, &emb);
    let json = serde_json::to_string(&a.target_pose).unwrap();
    assert!(json.contains("\"frame\":\"receptacle\""));
    // a_max clamped to the embodiment ceiling.
    assert_eq!(a.safety_envelope.motion_bounds.a_max.as_ref().unwrap().0, "1.5 m/s^2");
}

#[test]
fn align_emits_residual_directive() {
    let (skill, emb) = load("allegro");
    let Statement::Primitive(Primitive::ReachAlign(p)) = &skill.body.sequence[4] else { panic!() };
    let a = super::lower_reach_align(p, &emb);
    let json = serde_json::to_string(&a.target_pose).unwrap();
    assert!(json.contains("min_geodesic_rotation"));
    assert!(json.contains("\"target_frame\":\"receptacle\""));
    assert!(json.contains("\"z\""));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p rfl-core translation::tests::transport translation::tests::align 2>&1 | tail -15`
Expected: FAIL to compile.

- [ ] **Step 3: Implement**

```rust
use crate::canonical::AlignSpec;
use crate::skill_isa::{Axes, Axis, ReachAlign, TransportMoveToPose};

/// Lower `transport.move_to_pose`. The target_pose (a frame-relative offset) is
/// carried through; a_max is clamped to the embodiment kinematic ceiling (the
/// mass-dependent dynamic-stability clamp is a later increment, design § 3).
fn lower_transport_move_to_pose(p: &TransportMoveToPose, e: &Embodiment) -> CanonicalAction {
    let target_pose = match yaml_to_json(&p.target_pose) {
        serde_json::Value::Object(map) => {
            let frame = map.get("frame").and_then(|v| v.as_str()).unwrap_or("task").to_string();
            let offset = map.get("offset").cloned().unwrap_or(serde_json::Value::Null);
            PoseExpr::FrameRelative { frame, offset }
        }
        other => PoseExpr::FrameRelative { frame: "task".into(), offset: other },
    };
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose,
        force_budget: None,
        timing: TimingHints { nominal_duration: None, timing_mode: TimingMode::TimeScalable, stop_at_goal: true },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}

/// Lower `reach.align`: orientation-only; the residual is the minimum geodesic
/// rotation from the current orientation (spec/02 CA2c), emitted as a directive.
fn lower_reach_align(p: &ReachAlign, e: &Embodiment) -> CanonicalAction {
    let axes = match &p.axes {
        Axes::All(_) => vec!["x".into(), "y".into(), "z".into()],
        Axes::Set(v) => v.iter().map(|a| match a { Axis::X => "x", Axis::Y => "y", Axis::Z => "z" }.to_string()).collect(),
    };
    CanonicalAction {
        target_frame: e.control_frame().to_string(),
        target_pose: PoseExpr::OrientationAlign {
            align: AlignSpec { target_frame: p.target_frame.clone(), axes, residual: "min_geodesic_rotation" },
        },
        force_budget: None,
        timing: TimingHints { nominal_duration: None, timing_mode: TimingMode::Strict, stop_at_goal: true },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}

/// Convert a serde_yaml::Value to serde_json::Value deterministically.
fn yaml_to_json(v: &serde_yaml::Value) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}
```

Route both in `lower`:

```rust
        Primitive::TransportMoveToPose(p) => Ok((lower_transport_move_to_pose(p, e), "transport")),
        Primitive::ReachAlign(p) => Ok((lower_reach_align(p, e), "align")),
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p rfl-core translation 2>&1 | tail -20`
Expected: transport + align tests PASS (plus prior tests).

- [ ] **Step 5: Commit**

```bash
git rev-parse --abbrev-ref HEAD   # expect: main
git add crates/rfl-core/src/translation.rs
git commit -m "$(printf 'feat(core): lower transport.move_to_pose and reach.align\n\nCarry the frame-relative transport pose and clamp a_max to the\nkinematic ceiling; emit reach.align as an orientation-only minimum-\ngeodesic-rotation residual directive (spec/02 CA2c).\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 10: Lower `force.insert_fit`, `grasp.release`, `reach.retract` (full skill retargets)

**Files:**
- Modify: `crates/rfl-core/src/translation.rs`
- Test: same file

`force.insert_fit`: build the `monitors` from the skill's `stop_condition` (the all_of seating/jam condition), clamp the axial force_budget, set `compliance` and a `force_profile` in the envelope; the reaction-load bound is symbolic in v0 (design § 3). `grasp.release`: a state-change with a small withdraw. `reach.retract`: an axis-relative withdraw. After this task the full skill retargets to eight actions; un-ignore the Task 7 length test.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn insert_fit_lowers_stop_condition_into_monitors() {
    let (skill, emb) = load("allegro");
    let Statement::Primitive(Primitive::ForceInsertFit(p)) = &skill.body.sequence[5] else { panic!() };
    let a = super::lower_force_insert_fit(p, &emb);
    assert_eq!(a.force_budget.as_ref().unwrap().0, "15 N");
    assert_eq!(a.safety_envelope.compliance.as_deref(), Some("active"));
    let monitors = serde_json::to_string(&a.monitors).unwrap();
    assert!(monitors.contains("all_of"));
    assert!(monitors.contains("effort_rise"));
    assert!(monitors.contains("depth"));
}

#[test]
fn retract_is_axis_relative() {
    let (skill, emb) = load("allegro");
    let Statement::Primitive(Primitive::ReachRetract(p)) = &skill.body.sequence[7] else { panic!() };
    let a = super::lower_reach_retract(p, &emb);
    let json = serde_json::to_string(&a.target_pose).unwrap();
    assert!(json.contains("-tool_axis"));
    assert!(json.contains("50 mm"));
}

#[test]
fn full_skill_retargets_to_eight_actions() {  // un-ignored from Task 7
    let (skill, emb) = load("allegro");
    let out = retarget(&skill, &emb).expect("retarget");
    assert_eq!(out.actions.len(), 8);
    assert_eq!(out.suffixes, vec!["locate","pinch","transport","locate","align","insert_fit","release","retract"]);
}
```

Remove the `#[ignore]` from the Task 7 `retarget_emits_one_action_per_motion_statement` test (or delete it in favor of `full_skill_retargets_to_eight_actions`).

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p rfl-core translation 2>&1 | tail -20`
Expected: FAIL to compile (`lower_force_insert_fit`, `lower_reach_retract` not defined) and the full-skill test errors.

- [ ] **Step 3: Implement**

```rust
use crate::skill_isa::{ForceInsertFit, GraspRelease, ReachRetract};

/// Lower `force.insert_fit`: clamp the axial force_budget, lower the SeatingSpec
/// stop_condition into a monitor, set compliance and the axial force_profile. The
/// reaction-load bound is symbolic in v0 (design § 3).
fn lower_force_insert_fit(p: &ForceInsertFit, e: &Embodiment) -> CanonicalAction {
    let force_budget = Some(p.force_budget.clone()); // axial fit force; no grip-mode limit applies
    let monitors = vec![crate::canonical::Monitor { stop_condition: yaml_to_json(&p.stop_condition) }];
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| match c {
        crate::skill_isa::Compliance::Passive => "passive",
        crate::skill_isa::Compliance::Active => "active",
        crate::skill_isa::Compliance::Auto => "auto",
    }.to_string());
    env.force_profile = Some(serde_json::json!({ "axial": p.force_budget.0 }));
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target_fit.clone() },
        force_budget,
        timing: TimingHints { nominal_duration: None, timing_mode: TimingMode::TimeScalable, stop_at_goal: true },
        tactile_target: None,
        monitors,
        safety_envelope: env,
    }
}

/// Lower `grasp.release`: a state change releasing the active grasp; v0 emits a
/// short axis-relative withdraw along the default retract direction. The break-
/// contact postcondition (spec/01) is symbolic in v0.
fn lower_grasp_release(_p: &GraspRelease, e: &Embodiment) -> CanonicalAction {
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::AxisRelative {
            direction: serde_json::Value::String("-tool_axis".into()),
            distance: crate::quantity::Quantity("0 mm".into()),
        },
        force_budget: None,
        timing: TimingHints { nominal_duration: None, timing_mode: TimingMode::Strict, stop_at_goal: true },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}

/// Lower `reach.retract`: an axis-relative withdraw (direction + distance), force-
/// monotonicity abort is symbolic in v0.
fn lower_reach_retract(p: &ReachRetract, e: &Embodiment) -> CanonicalAction {
    CanonicalAction {
        target_frame: e.control_frame().to_string(),
        target_pose: PoseExpr::AxisRelative {
            direction: yaml_to_json(&p.direction),
            distance: p.distance.clone(),
        },
        force_budget: None,
        timing: TimingHints { nominal_duration: None, timing_mode: TimingMode::Strict, stop_at_goal: true },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}
```

Route all three in `lower`:

```rust
        Primitive::ForceInsertFit(p) => Ok((lower_force_insert_fit(p, e), "insert_fit")),
        Primitive::GraspRelease(p) => Ok((lower_grasp_release(p, e), "release")),
        Primitive::ReachRetract(p) => Ok((lower_reach_retract(p, e), "retract")),
```

Then remove the catch-all `other => Err(...)` arm in `lower` (all seven are now handled).

- [ ] **Step 4: Run the tests**

Run: `cargo test -p rfl-core 2>&1 | tail -20`
Expected: all `rfl-core` tests PASS, including `full_skill_retargets_to_eight_actions`.

- [ ] **Step 5: Commit**

```bash
git rev-parse --abbrev-ref HEAD   # expect: main
git add crates/rfl-core/src/translation.rs
git commit -m "$(printf 'feat(core): lower insert_fit, release, retract; full skill retargets\n\nLower the SeatingSpec stop_condition into monitors with axial force\nprofile and compliance; emit release/retract as axis-relative withdraws.\nThe full cable-insertion skill now retargets to eight canonical actions.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 11: Wire the `rfl retarget` CLI arm

**Files:**
- Modify: `crates/rfl-cli/src/main.rs` (fill the existing `Command::Retarget` arm)

- [ ] **Step 1: Implement the arm**

Replace the `Command::Retarget { skill, embodiment }` bail in `crates/rfl-cli/src/main.rs`:

```rust
        Command::Retarget { skill, embodiment } => {
            let skill_text = std::fs::read_to_string(&skill)
                .map_err(|e| anyhow::anyhow!("read skill {skill:?}: {e}"))?;
            let emb_text = std::fs::read_to_string(&embodiment)
                .map_err(|e| anyhow::anyhow!("read embodiment {embodiment:?}: {e}"))?;
            let parsed_skill = rfl_core::skill_isa::Skill::parse_yaml(&skill_text)?;
            let emb = rfl_core::embodiment::Embodiment::parse_yaml(&emb_text)?;
            let out = rfl_core::translation::retarget(&parsed_skill, &emb)?;
            let jsonl = rfl_core::canonical::to_jsonl(
                &parsed_skill.skill, &emb.id, &out.actions, &out.suffixes,
            );
            print!("{jsonl}");
            Ok(())
        }
```

Note: `rfl_core::Error` must convert into `anyhow::Error`. It derives `thiserror::Error` and implements `std::error::Error`, so `?` works against `anyhow::Result`. If a trait-bound error appears, add `.map_err(anyhow::Error::from)`.

- [ ] **Step 2: Run the CLI on all three embodiments**

Run:
```bash
cargo run -q -p rfl-cli -- retarget examples/01-cable-insertion/skill.yaml \
  --embodiment examples/01-cable-insertion/embodiments/allegro.yaml | head -2
```
Expected: two JSON lines, each a complete `execute` message; first action_id ends `0001-locate`.

- [ ] **Step 3: Run the example entry script**

Run: `python3 examples/01-cable-insertion/run.py --embodiment pneumatic-6f | head -1`
Expected: a JSON line; the pinch action (line 2) shows the proxy tactile target. (`run.py` prints the invocation then the stream.)

- [ ] **Step 4: Commit**

```bash
cargo build -p rfl-cli 2>&1 | tail -3   # read success
```
```bash
git rev-parse --abbrev-ref HEAD   # expect: main
git add crates/rfl-cli/src/main.rs
git commit -m "$(printf 'feat(cli): implement the retarget subcommand\n\nLoad the skill and embodiment, retarget, and stream the canonical\nactions as execute-message JSONL on stdout. examples/01-cable-\ninsertion/run.py now runs on all three embodiments.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 12: Conformance test class 2 (insta golden + determinism + schema validation)

**Files:**
- Modify: `crates/rfl-conformance/Cargo.toml` (add dev-deps: `insta`, `proptest`, `boon`, `serde_json`)
- Create: `crates/rfl-conformance/tests/retarget_determinism.rs`
- Modify: `crates/rfl-conformance/src/lib.rs` (expose a small helper used by the tests)
- Snapshots: `crates/rfl-conformance/tests/snapshots/` (created by `cargo insta`)

`insta` and `proptest` are workspace deps; add them and `boon` to `rfl-conformance`. **Verify `boon` supports Draft 2020-12 and the `oneOf` discriminated union before pinning the version** (check docs.rs/crates.io in this step); if `boon`'s API differs from the sketch below, adapt it, or fall back to the `jsonschema` crate.

- [ ] **Step 1: Add dev-dependencies**

In `crates/rfl-conformance/Cargo.toml`:

```toml
[dev-dependencies]
insta = { workspace = true }
proptest = { workspace = true }
serde_json = { workspace = true }
boon = "0.6"
```

(Confirm the latest `boon` version and its Draft 2020-12 support in this step; adjust the literal accordingly.)

- [ ] **Step 2: Expose the retarget-to-JSONL helper**

Replace the `// TODO` in `crates/rfl-conformance/src/lib.rs` with:

```rust
use std::path::Path;

/// Retarget the named example skill onto the named embodiment stem and return the
/// JSONL stream. The cable-insertion reference lives under `examples/`.
///
/// # Errors
/// Propagates parse / retarget errors.
pub fn retarget_example_to_jsonl(skill_path: &Path, embodiment_path: &Path) -> anyhow::Result<String> {
    let skill = rfl_core::skill_isa::Skill::parse_yaml(&std::fs::read_to_string(skill_path)?)?;
    let emb = rfl_core::embodiment::Embodiment::parse_yaml(&std::fs::read_to_string(embodiment_path)?)?;
    let out = rfl_core::translation::retarget(&skill, &emb)?;
    Ok(rfl_core::canonical::to_jsonl(&skill.skill, &emb.id, &out.actions, &out.suffixes))
}
```

- [ ] **Step 3: Write the golden + determinism + schema tests**

Create `crates/rfl-conformance/tests/retarget_determinism.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 (spec/05 § The determinism floor; spec/02 RD1c):
//! retarget generation is byte-deterministic, matches a committed golden, and
//! every emitted line is a valid driver-interface execute message.

use std::path::{Path, PathBuf};
use rfl_conformance::retarget_example_to_jsonl;

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(&dir.join("skill.yaml"), &dir.join(format!("embodiments/{stem}.yaml")))
        .expect("retarget")
}

#[test]
fn golden_allegro() {
    insta::assert_snapshot!("retarget_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_leap() {
    insta::assert_snapshot!("retarget_leap", jsonl_for("leap"));
}

#[test]
fn golden_pneumatic() {
    insta::assert_snapshot!("retarget_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn every_line_is_a_valid_execute_message() {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/driver-interface.schema.json");
    let schema: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&schema_path).unwrap()).unwrap();

    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource("driver-interface.schema.json", schema).unwrap();
    let idx = compiler.compile("driver-interface.schema.json", &mut schemas).unwrap();

    for stem in ["allegro", "leap", "pneumatic-6f"] {
        for line in jsonl_for(stem).lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            schemas.validate(&v, idx).unwrap_or_else(|e| panic!("{stem}: {e}"));
        }
    }
}
```

- [ ] **Step 4: Generate and accept the golden snapshots**

Run (review the generated JSONL is the intended retarget output, then accept):
```bash
cargo test -p rfl-conformance 2>&1 | tail -25
cargo insta accept   # if cargo-insta is installed; else `cargo insta review`
```
If `cargo-insta` is absent, install with `cargo install cargo-insta` (or set `INSTA_UPDATE=always` for the first run). Inspect `crates/rfl-conformance/tests/snapshots/*.snap` and confirm the snapshots are genuinely per-embodiment: allegro/leap show the `auto` tactile target on the pinch action while pneumatic shows the `proxy` tier; the grasp/insert actions' `target_frame` differs per hand (allegro `tcp_thumb` vs pneumatic `palm`); and the motion bounds differ (allegro `a_max` `1.5 m/s^2` vs pneumatic `0.8 m/s^2`).

- [ ] **Step 5: Run the full suite green**

Run: `cargo test -p rfl-conformance 2>&1 | tail -15`
Expected: all 5 tests PASS (3 golden + determinism + schema validation).

- [ ] **Step 6: Commit (snapshots + tests together)**

```bash
git rev-parse --abbrev-ref HEAD   # expect: main
git add crates/rfl-conformance/Cargo.toml crates/rfl-conformance/src/lib.rs \
        crates/rfl-conformance/tests/retarget_determinism.rs \
        crates/rfl-conformance/tests/snapshots
git commit -m "$(printf 'test(conformance): add retarget determinism class 2\n\nPin the per-embodiment retarget JSONL with insta golden snapshots, a\ngenerate-twice byte-equality property, and per-line validation against\nthe driver-interface execute schema (RD1c, spec/05 determinism floor).\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 13: Closeout — example status, READMEs, memory

**Files:**
- Modify: `examples/01-cable-insertion/README.md` (status line: rfl-cli now runs)
- Modify: `crates/*/src/*.rs` doc comments if any `not yet implemented` text is now stale
- Update: the project memory `project_rfl.md` with the real commit hashes

- [ ] **Step 1: Run the whole workspace suite + the input-schema suite**

Run:
```bash
cargo test 2>&1 | tail -20
uv run --with jsonschema --with pyyaml python schemas/validate.py; echo "EXIT=$?"
```
Expected: all crates' tests PASS; `validate.py` `EXIT=0` (unchanged).

- [ ] **Step 2: Update the example README status**

In `examples/01-cable-insertion/README.md`, change the status note from "`run.py` invokes the reference CLI (`crates/rfl-cli`, in progress)" to reflect that retarget v0 runs on all three embodiments and emits execute JSONL; keep the walkthrough.

- [ ] **Step 3: Commit the docs**

```bash
git rev-parse --abbrev-ref HEAD   # expect: main
git add examples/01-cable-insertion/README.md
git commit -m "$(printf 'docs(example): mark cable-insertion retarget as running\n\nrfl-cli retarget v0 now produces the per-embodiment execute JSONL for\nall three hands; update the example status.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

- [ ] **Step 4: Push and verify**

```bash
git fetch origin
git merge-base --is-ancestor origin/main HEAD && echo FF_OK || echo NOT_FF
git push origin main
git rev-list --left-right --count origin/main...HEAD   # expect: 0 0
```

- [ ] **Step 5: Update memory `project_rfl.md`** with the retarget-v0 milestone and the real pushed commit hashes (use `git log --oneline` values, never predicted).

---

## Self-review notes

- **Spec coverage:** Task 3 (parse) + Task 4 (descriptor) cover design § 4 inputs; Task 5 (pose) § 2; Task 6 (canonical/serialization) § 4/§ 6/§ 7; Tasks 7–10 (engine + gate + tactile + clamps + monitors) § 2/§ 3 in-scope transformations; Task 11 (CLI) the `rfl retarget` contract; Task 12 the § 6/§ 7 determinism + golden + schema validation. Deferred items (Σ, mass-dependent grasp-force, routing, time-scaling, class 3/4) carry no task by design (§ 3).
- **Spec-rule transcription tasks:** the capability gate (Task 7) and the tactile proxy/auto rule (Task 8) reference the authoritative § to transcribe rather than fabricating the rule; their tests anchor behavior against the committed descriptors. The grasp/sensor frames resolve from `frames.role_defaults` (Task 4, faithful and per-embodiment distinct); only the control-frame default (Task 4 `control_frame`, v0 = first declared control frame) carries a transcribe note for `spec/03`'s exact `default_control_frame` rule.
- **Type consistency:** `RetargetOutput { actions, suffixes }` is produced in Task 7 and consumed in Tasks 11–12; `to_jsonl(skill, embodiment_id, actions, suffixes)` signature is fixed in Task 6 and used unchanged later; `PoseExpr` variants defined in Task 6 are the only ones the lowering tasks construct; the frame resolvers `grasp_frame`/`sensor_frame`/`control_frame` are `Embodiment` methods (Task 4) called directly by the lowering tasks (no local stub functions).
- **Known v0 simplifications pinned by tests, not hidden:** the control-frame default approximates `spec/03`'s rule by the first declared control frame; `grasp.release` emits a zero-distance withdraw; the reaction-load and break-contact bounds are symbolic; the mass-dependent dynamic-stability clamp is deferred (a_max uses the kinematic ceiling). Each is called out in its task and captured in the golden snapshot, so a later increment changes the snapshot deliberately.
