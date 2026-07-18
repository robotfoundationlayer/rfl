# transport.carry — held interval-invariant — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline,
> per the RFL main-branch direct-commit discipline) to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking. LOCAL-ONLY: never `git add` this file.

**Goal:** Add `transport.carry` (spec/01 § 4.4) — transport a held object while maintaining
grasp stability under perturbation — verified as the held leg of the interval-invariant
envelope class (spec/05 ENV1).

**Architecture:** A new `grasp_force::carry_a_max` clamp (acceleration below the plain
dynamic limit, reserving margin for `disturbance_budget`); a new `Primitive::TransportCarry`
lowered like `transport.move_to_pose` but emitting the GC1 floor + `disturbance_budget` +
`stability_margin` and the carry-clamped `a_max`; `envelope_class_for("carry") →
IntervalInvariant` (one arm, no signature change — spec/05 ENV1 assigns exactly one class);
and an *enriched* IntervalInvariant check that also samples the held-secured floor (vacuous
for `reach.hover`). Worked example = a skill variant in `01-cable-insertion`.

**Tech Stack:** Rust (rustup stable 1.96 at `~/.cargo/bin`; prefix every cargo cmd with
`export PATH="$HOME/.cargo/bin:$PATH"`), serde / serde_yaml / serde_json, insta goldens,
boon (schema validation), `uv run --with jsonschema --with pyyaml python schemas/validate.py`.

**Process discipline (every task):** TDD red → green → (if golden output changed)
`INSTA_UPDATE=always` regenerate + `rm -f crates/rfl-conformance/tests/snapshots/*.snap.new`
+ `git diff` eyeball ONLY the intended lines → run the full suite + validate.py and **READ
the result in a batch PHYSICALLY SEPARATE from the git commit** → only if green, stage
explicit files (no `-A`, never `docs/plans/`) and commit (Conventional Commits +
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`) → before push
`git merge-base --is-ancestor origin/main HEAD`, after push `git rev-list --left-right
--count origin/main...HEAD` == `0 0`. `cargo test` warning-clean (clippy pedantic is
pre-existing, not gated). The design doc commit `f12da2c` already landed.

---

## File Structure

- **Modify** `crates/rfl-core/src/grasp_force.rs` — add `carry_a_max` (pure fn + tests).
- **Modify** `crates/rfl-core/src/embodiment.rs` — add `ratio_limit` (bare-number ratio accessor).
- **Modify** `crates/rfl-core/src/skill_isa.rs` — `TransportCarry` struct + `DisturbanceArg` /
  `StabilityMarginArg` enums + `Primitive::TransportCarry` variant.
- **Modify** `crates/rfl-core/src/translation.rs` — `check_capability` arm, `lower` arm,
  `lower_transport_carry` (+ tests).
- **Create** `examples/01-cable-insertion/skill-carry.yaml` — locate → pinch → transport.carry.
- **Modify** `examples/01-cable-insertion/embodiments/{allegro,leap,pneumatic-6f}.yaml` — add
  `transport.carry` capability + `stability_margin` limit.
- **Create** `crates/rfl-conformance/tests/transport_carry.rs` — per-hand retarget goldens +
  determinism + boon.
- **Modify** `crates/rfl-conformance/src/lib.rs` — `envelope_class_for("carry")` arm; enrich
  the `IntervalInvariant` check; factor `securing_floor_violation`.
- **Modify** `crates/rfl-conformance/tests/envelope_conformance.rs` — nominal carry interval
  pass + `UnderSecure` fail + `MidIntervalDrop` fail.
- **Modify** `README.md` (Task 5) — note the carry variant + refresh counts.

---

## Task 1: `grasp_force::carry_a_max`

**Files:** Modify `crates/rfl-core/src/grasp_force.rs`

- [ ] **Step 1: Write the failing tests** — append to the `tests` module in `grasp_force.rs`:

```rust
    #[test]
    fn carry_a_max_reduces_to_dynamic_at_zero_disturbance_and_margin() {
        // m=0, D=0 -> identical to the plain dynamic-stability clamp.
        let c = carry_a_max(1.45, 3.0, 0.0, 0.0);
        let d = dynamic_a_max(1.45, 3.0);
        assert!((c - d).abs() < 1e-12, "carry {c} != dynamic {d}");
    }

    #[test]
    fn carry_a_max_is_below_dynamic_under_disturbance_and_margin() {
        // D, m > 0 strictly reserves capacity -> below the plain dynamic limit.
        assert!(carry_a_max(1.45, 3.0, 0.4, 0.5) < dynamic_a_max(1.45, 3.0));
    }

    #[test]
    fn carry_a_max_matches_worked_example_on_allegro() {
        // connector 1.45 N, pinch payload 3 N, D=0.4 N, m=0.5:
        // (3/1.5 - 0.4)/1.45 - 1 = 0.1034483 ; * g0 = 1.014481 m/s^2.
        let a = carry_a_max(1.45, 3.0, 0.4, 0.5);
        assert!((a - 1.014481).abs() < 1e-5, "got {a}");
    }

    #[test]
    fn carry_a_max_clamps_to_zero_when_headroom_is_insufficient() {
        // leap (payload 2) and pneumatic (payload 1.5) have no headroom for a 1.45 N
        // object + 0.4 N disturbance + 50% margin -> quasi-static (a_max = 0).
        assert_eq!(carry_a_max(1.45, 2.0, 0.4, 0.5), 0.0);
        assert_eq!(carry_a_max(1.45, 1.5, 0.4, 0.5), 0.0);
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core grasp_force::tests::carry_a_max 2>&1 | tail -20`
Expected: FAIL — `cannot find function carry_a_max in this scope`.

- [ ] **Step 3: Implement `carry_a_max`** — insert after `dynamic_a_max` (before `reaction_limit`) in `grasp_force.rs`:

```rust
/// The transport.carry acceleration clamp (`spec/01` § 4.4, `spec/02` § Dynamic stability,
/// line 137): the largest acceleration at which the GF2c worst-case collinear
/// inertial+gravity load `weight·(1 + a/g₀)` **plus** the `disturbance_budget` stays within
/// the mode's rated holding capacity (`payload_n`, a weight) with `stability_margin`
/// headroom, from § 4.4's precondition `payload ≥ (1+m)·(inertial_load + D)`:
///
/// ```text
/// a_max = g₀·( ( payload/(1+m) − D ) / weight − 1 ),  clamped ≥ 0.
/// ```
///
/// Reduces to `dynamic_a_max` at `m = 0, D = 0`, and is ≤ it for `m, D ≥ 0` — the "clamped
/// below the plain-transport limit" of `spec/02`:137. The pre-clamp going negative is the
/// § 4.4 `insufficient_stability_margin` case, surfaced as `a_max = 0` (carry quasi-statically
/// only), exactly as `dynamic_a_max` surfaces an over-payload weight. A v0 reference choice,
/// pinned by golden, non-normative (the same posture as `dynamic_a_max` / `k_holding`).
#[must_use]
pub fn carry_a_max(weight_n: f64, payload_n: f64, disturbance_n: f64, stability_margin: f64) -> f64 {
    let effective_payload = payload_n / (1.0 + stability_margin) - disturbance_n;
    (G0 * (effective_payload / weight_n - 1.0)).max(0.0)
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core grasp_force 2>&1 | tail -20`
Expected: PASS (all `grasp_force` tests, incl. the 4 new ones).

- [ ] **Step 5: Run the full rfl-core suite (separate from commit) and READ it**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | tail -15`
Expected: `test result: ok.` — read it before staging.

- [ ] **Step 6: Commit**

```bash
cd ~/Documents/GitHub/rfl
git add crates/rfl-core/src/grasp_force.rs
git commit -F - <<'EOF'
feat(core): carry_a_max — disturbance-and-margin acceleration clamp

spec/01 § 4.4 + spec/02:137. The transport.carry acceleration limit: the GF2c
inertial model plus the disturbance_budget within holding capacity at
stability_margin headroom. Reduces to dynamic_a_max at m=0,D=0 and is provably
<= it (the "below the plain-transport limit"). A v0 reference choice, golden-pinned.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```
Expected: push ok, `0 0`.

---

## Task 2: `transport.carry` AST + lowering + capability gate

One commit: `Primitive::TransportCarry` makes the exhaustive `lower` / `check_capability`
matches (no catch-all) require their new arms in the same compile.

**Files:** Modify `crates/rfl-core/src/skill_isa.rs`, `embodiment.rs`, `translation.rs`

- [ ] **Step 1: Write the failing tests**

In `skill_isa.rs` `parse_tests` module:

```rust
    #[test]
    fn transport_carry_parses() {
        let yaml = "skill: t\nbody:\n  sequence:\n    - transport.carry:\n        motion: { to_pose: { frame: staging, offset: { along: +z, distance: 100 mm } } }\n        disturbance_budget: 0.4 N\n        stability_margin: auto\n";
        let s = Skill::parse_yaml(yaml).expect("parse");
        let Statement::Primitive(Primitive::TransportCarry(p)) = &s.body.sequence[0] else {
            panic!("expected transport.carry");
        };
        assert!(p.motion.get("to_pose").is_some());
        assert!(matches!(p.stability_margin, Some(StabilityMarginArg::Auto(_))));
        let Some(DisturbanceArg::Force(q)) = &p.disturbance_budget else { panic!("explicit force") };
        assert_eq!(q.0, "0.4 N");
    }
```

In `translation.rs` `tests` module:

```rust
    const CARRY_SKILL: &str = "skill: cable-carry\nobjects:\n  connector: { ref: connector, estimated_mass: 1.45 N }\nbody:\n  sequence:\n    - let: connector_t\n      from:\n        sense.locate: { target_ref: connector, modality: auto }\n    - grasp.pinch: { target: connector_t, force_budget: 8 N, tactile_target: auto }\n    - transport.carry:\n        motion: { to_pose: { frame: staging, offset: { along: +z, distance: 100 mm } } }\n        disturbance_budget: 0.4 N\n        stability_margin: auto\n";

    fn carry_emb(stem: &str) -> Embodiment {
        let (_, mut emb) = load(stem);
        emb.capabilities.skills.push("transport.carry".to_string());
        emb.limits.insert(
            "stability_margin".to_string(),
            serde_yaml::from_str("0.5").unwrap(),
        );
        emb
    }

    #[test]
    fn carry_emits_floor_disturbance_margin_and_clamps_a_max_on_allegro() {
        let skill = Skill::parse_yaml(CARRY_SKILL).expect("parse");
        let emb = carry_emb("allegro");
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["locate", "pinch", "carry"]);
        let fp = out.actions[2].safety_envelope.force_profile.as_ref().expect("force_profile");
        assert_eq!(fp.get("min_holding_force").and_then(|v| v.as_str()), Some("2.9 N"));
        assert_eq!(fp.get("disturbance_budget").and_then(|v| v.as_str()), Some("0.4 N"));
        assert_eq!(fp.get("stability_margin").and_then(serde_json::Value::as_f64), Some(0.5));
        // allegro carry a_max clamps from the 1.5 kinematic ceiling to 1.014481.
        assert_eq!(
            out.actions[2].safety_envelope.motion_bounds.a_max.as_ref().unwrap().0,
            "1.014481 m/s^2"
        );
    }

    #[test]
    fn carry_a_max_clamps_to_zero_on_weaker_hands() {
        let skill = Skill::parse_yaml(CARRY_SKILL).expect("parse");
        for stem in ["leap", "pneumatic-6f"] {
            let out = retarget(&skill, &carry_emb(stem)).expect("retarget");
            assert_eq!(
                out.actions[2].safety_envelope.motion_bounds.a_max.as_ref().unwrap().0,
                "0 m/s^2", "stem {stem}"
            );
        }
    }

    #[test]
    fn carry_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(CARRY_SKILL).expect("parse");
        let (_, emb) = load("allegro"); // cable allegro lacks transport.carry
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: transport.carry"), "got {err}");
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | tail -20`
Expected: FAIL — `no variant ... TransportCarry`, `cannot find ... StabilityMarginArg`, `ratio_limit`.

- [ ] **Step 3a: Add the AST** — in `skill_isa.rs`, add the variant to `Primitive` (after `ForceUnscrew`):

```rust
    /// `transport.carry`.
    #[serde(rename = "transport.carry")]
    TransportCarry(TransportCarry),
```

and the struct + arg enums (place near `TransportMoveToPose`):

```rust
/// `disturbance_budget` argument (`$defs`, § 4.4): `Force | auto`. v0 lowers the explicit
/// `Force`; `auto` (derive from the min_holding_force margin) is deferred — no normative
/// formula exists (same posture as `coverage_overlap: auto`).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum DisturbanceArg {
    /// The literal `auto` (deferred derivation).
    Auto(AutoLiteral),
    /// An explicit force budget.
    Force(Quantity),
}

/// `stability_margin` argument (§ 4.4): `Ratio | auto`. `auto` resolves to the descriptor's
/// `limits.stability_margin` (the M2-required embodiment default); an explicit ratio overrides.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum StabilityMarginArg {
    /// The literal `auto` (= the embodiment default limit).
    Auto(AutoLiteral),
    /// An explicit headroom ratio.
    Ratio(f64),
}

/// `transport.carry` parameters (v0 subset of `$defs/TransportCarryParams`, § 4.4). The
/// `MoveSpec` `motion` is carried opaquely — v0 lowers the `{to_pose: Pose6D}` form,
/// `{trajectory: T}` is deferred. The remaining § 4.4 params (grasp_handle / frame /
/// position_tolerance / max_velocity / max_acceleration / contact_response / timeout) carry
/// their spec defaults and are not emitted in v0 (serde ignores them).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TransportCarry {
    /// `MoveSpec`: `{to_pose: Pose6D}` (lowered) or `{trajectory: T}` (deferred).
    pub motion: serde_yaml::Value,
    /// External perturbation the carry must reject (`Force | auto`).
    #[serde(default)]
    pub disturbance_budget: Option<DisturbanceArg>,
    /// Required holding-capacity headroom (`Ratio | auto`).
    #[serde(default)]
    pub stability_margin: Option<StabilityMarginArg>,
}
```

- [ ] **Step 3b: Add `ratio_limit`** — in `embodiment.rs`, add to the `impl Embodiment` block (after `scalar_limit`):

```rust
    /// A bare-number ratio limit (e.g. `stability_margin`), if present and numeric. Unlike
    /// `scalar_limit` (unit-suffixed quantities), a dimensionless ratio deserializes to
    /// `LimitValue::Other(Number)`.
    #[must_use]
    pub fn ratio_limit(&self, key: &str) -> Option<f64> {
        match self.limits.get(key) {
            Some(LimitValue::Other(serde_yaml::Value::Number(n))) => n.as_f64(),
            _ => None,
        }
    }
```

- [ ] **Step 3c: Add the capability gate** — in `translation.rs::check_capability`, add an arm (after `TransportMoveToPose`):

```rust
        // transport.carry is a DISTINCT capability beyond the base transport gate
        // (§ 4.4 precondition: "declares transport with carry support").
        Primitive::TransportCarry(_) => "transport.carry",
```

- [ ] **Step 3d: Add the `lower` arm + the lowering** — in `translation.rs`, update the imports from `crate::skill_isa` to include `DisturbanceArg, StabilityMarginArg, TransportCarry`; add the `lower` match arm (after the `TransportMoveToPose` arm):

```rust
        Primitive::TransportCarry(p) => (lower_transport_carry(p, e, ctx), "carry"),
```

and the function (after `lower_transport_move_to_pose`):

```rust
/// Lower `transport.carry` (`spec/01` § 4.4): transport a held object while rejecting
/// disturbance. The held-secured floor (GC1) and the carry-clamped a_max are emitted from
/// `ctx.held` (like `lower_transport_move_to_pose`), but the acceleration is clamped *below*
/// the plain dynamic limit via `grasp_force::carry_a_max`, reserving margin for the
/// `disturbance_budget` (`spec/02`:137). `disturbance_budget` + `stability_margin` are emitted
/// for the ENV3 bench (a later increment). v0 lowers the `{to_pose: P}` MoveSpec; `{trajectory}`
/// is deferred. `stability_margin: auto` reads the descriptor default.
fn lower_transport_carry(p: &TransportCarry, e: &Embodiment, ctx: &GraspContext) -> CanonicalAction {
    let target_pose = match p.motion.get("to_pose").map(yaml_to_json) {
        Some(serde_json::Value::Object(map)) => {
            let frame = map
                .get("frame")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("task")
                .to_string();
            let offset = map.get("offset").cloned().unwrap_or(serde_json::Value::Null);
            PoseExpr::FrameRelative { frame, offset }
        }
        // {trajectory: ...} or any other MoveSpec form is carried opaquely in v0.
        _ => PoseExpr::FrameRelative { frame: "task".into(), offset: yaml_to_json(&p.motion) },
    };
    let mut env = base_envelope(e);
    if let Some(held) = &ctx.held {
        let disturbance_n = match &p.disturbance_budget {
            Some(DisturbanceArg::Force(q)) => q.parse().map_or(0.0, |(v, _)| v),
            _ => 0.0, // auto deferred -> no extra reserve in v0
        };
        let margin = match &p.stability_margin {
            Some(StabilityMarginArg::Ratio(m)) => *m,
            _ => e.ratio_limit("stability_margin").unwrap_or(0.0), // auto -> descriptor default
        };
        let mhf = grasp_force::min_holding_force(held.weight_n, held.mode);
        env.force_profile = Some(serde_json::json!({
            "min_holding_force": Quantity::from_si(mhf, "N").0,
            "disturbance_budget": Quantity::from_si(disturbance_n, "N").0,
            "stability_margin": margin,
        }));
        let payload = e.scalar_limit(held.mode.payload_key()).and_then(|q| q.parse());
        let ceiling = e.scalar_limit("a_cartesian_max").and_then(|q| q.parse());
        if let (Some((payload_n, _)), Some((ceiling_v, unit))) = (payload, ceiling) {
            let unit = unit.to_string();
            let carry_a = grasp_force::carry_a_max(held.weight_n, payload_n, disturbance_n, margin);
            if carry_a < ceiling_v {
                env.motion_bounds.a_max = Some(Quantity::from_si(carry_a, &unit));
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

- [ ] **Step 4: Run to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | tail -15`
Expected: PASS — all rfl-core tests incl. the new carry parse + lowering + capability tests.

- [ ] **Step 5: Run the full workspace suite + validate.py (separate from commit) and READ**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | tail -25`
Then: `uv run --with jsonschema --with pyyaml python schemas/validate.py; echo EXIT=$?`
Expected: all `test result: ok.`; validate.py prints PASS, `EXIT=0`. (No goldens changed yet —
no example uses carry; existing conformance goldens unaffected.) Read both before staging.

- [ ] **Step 6: Commit**

```bash
cd ~/Documents/GitHub/rfl
git add crates/rfl-core/src/skill_isa.rs crates/rfl-core/src/embodiment.rs crates/rfl-core/src/translation.rs
git commit -F - <<'EOF'
feat(core): transport.carry — held disturbance-rejecting transport

spec/01 § 4.4. New Primitive::TransportCarry lowered like move_to_pose but the
acceleration is clamped via carry_a_max (below the plain dynamic limit, reserving
margin for disturbance_budget); emits the GC1 floor + disturbance_budget +
stability_margin (the ENV3 foothold). Distinct capability gate transport.carry
(§ 4.4 precondition). stability_margin: auto reads the descriptor (new ratio_limit).
v0 lowers the {to_pose} MoveSpec; {trajectory} deferred. No schema change.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```
Expected: push ok, `0 0`.

---

## Task 3: worked example + descriptor caps + retarget goldens

**Files:** Create `examples/01-cable-insertion/skill-carry.yaml`; modify the 3 descriptors;
create `crates/rfl-conformance/tests/transport_carry.rs`.

- [ ] **Step 1: Create the skill variant** — `examples/01-cable-insertion/skill-carry.yaml`:

```yaml
# Example 01 (variant) — Cable carry under disturbance
# Reuses the cable connector + the three descriptors; swaps the plain transport for a
# transport.carry, exercising the held interval-invariant envelope class (spec/01 § 4.4,
# spec/05 ENV1). The pinch establishes the held object (connector.estimated_mass) so the
# carry has a grasp to keep secured; the carry a_max is clamped below the plain dynamic
# limit to reserve margin for the disturbance_budget (spec/02:137).

skill: cable-carry
description: >
  Grasp a connector and carry it under disturbance to a staging pose, maintaining
  grasp stability throughout (transport.carry — the held interval-invariant).

objects:
  connector: { ref: connector, estimated_mass: 1.45 N }   # ~148 g; the held object the carry secures

body:
  sequence:

    # 1 — Locate the connector; bind its measured pose.
    - let: connector_t
      from:
        sense.locate: { target_ref: connector, modality: auto }

    # 2 — Grasp the connector (antipodal pinch); establishes the held object.
    - grasp.pinch:
        target: connector_t
        force_budget: 8 N
        tactile_target: auto
        slip_response: retighten

    # 3 — Carry the held connector to a staging pose under disturbance. The acceleration
    #     is clamped below the plain-transport limit to reserve stability_margin for the
    #     0.4 N disturbance budget; stability_margin: auto = the embodiment default.
    - transport.carry:
        motion: { to_pose: { frame: staging, offset: { along: +z, distance: 100 mm } } }
        disturbance_budget: 0.4 N
        stability_margin: auto
```

- [ ] **Step 2: Add carry capability + limit to the 3 descriptors**

`allegro.yaml` — `capabilities.skills` append `transport.carry`; under `limits` add (near the
per-grasp-mode block):

```yaml
    stability_margin:            0.5   # transport.carry holding-capacity headroom (03 § Transport)
```

So allegro's skills line becomes:
```yaml
    skills: [grasp.pinch, grasp.lateral, grasp.envelope, transport, transport.carry, force.insert_fit, sense.locate]
```

Repeat identically for `leap.yaml` (skills: add `transport.carry`; add `stability_margin: 0.5`)
and `pneumatic-6f.yaml` (skills: add `transport.carry`; add `stability_margin: 0.5`).

- [ ] **Step 3: Verify the new skill is schema-valid (Class 1, one-off)**

Run (validate.py is ex01-`skill.yaml`-only; check the variant directly):
```bash
cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python - <<'PY'
import json, yaml, jsonschema, pathlib
root = pathlib.Path("schemas")
skill = jsonschema.Draft202012Validator(json.load(open(root/"skill-isa.schema.json")))
desc  = jsonschema.Draft202012Validator(json.load(open(root/"embodiment-descriptor.schema.json")))
skill.validate(yaml.safe_load(open("examples/01-cable-insertion/skill-carry.yaml")))
for s in ["allegro","leap","pneumatic-6f"]:
    desc.validate(yaml.safe_load(open(f"examples/01-cable-insertion/embodiments/{s}.yaml")))
print("CLASS1_OK")
PY
```
Expected: `CLASS1_OK` (skill matches the transport.carry typing; each descriptor satisfies M2
`stability_margin` required-when-`transport.carry`-declared).

- [ ] **Step 4: Write the retarget golden test** — create `crates/rfl-conformance/tests/transport_carry.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for the transport.carry variant: the retarget output is
//! byte-deterministic, matches a committed golden, and every line is a valid
//! driver-interface execute message. (The interval-invariant + held-floor verification of
//! the carry is in envelope_conformance.rs.)

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-carry.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_carry_allegro() {
    insta::assert_snapshot!("carry_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_carry_leap() {
    insta::assert_snapshot!("carry_leap", jsonl_for("leap"));
}

#[test]
fn golden_carry_pneumatic() {
    insta::assert_snapshot!("carry_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn carry_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn carry_every_line_is_a_valid_execute_message() {
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

- [ ] **Step 5: Generate the goldens, then eyeball them**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance --test transport_carry 2>&1 | tail -15`
Then read the three new snapshots:
`cat crates/rfl-conformance/tests/snapshots/transport_carry__carry_*.snap`
Expected (verify analytically, do NOT reverse-engineer): each = 3 execute lines
(`0001-locate`, `0002-pinch`, `0003-carry`). The `0003-carry` line carries
`force_profile` `{"disturbance_budget":"0.4 N","min_holding_force":"2.9 N","stability_margin":0.5}`
(keys sorted) and `motion_bounds.a_max` = **allegro `"1.014481 m/s^2"`**, **leap `"0 m/s^2"`**,
**pneumatic `"0 m/s^2"`**; `target_frame` = allegro/leap `tcp_thumb`, pneumatic `palm`;
`timing.timing_mode` = `time_scalable`. Then remove any stray new files:
`rm -f crates/rfl-conformance/tests/snapshots/*.snap.new`.

- [ ] **Step 6: Confirm existing goldens are byte-identical (the guard)**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance 2>&1 | tail -20 && git status --porcelain crates/rfl-conformance/tests/snapshots/`
Expected: all conformance tests `ok.`; `git status` shows ONLY the 3 new `transport_carry__*.snap`
files (no modifications to `cable_*` / `screw_*` / `surface_scan*` / `driver_protocol` snapshots —
the descriptor additions did not change any non-carry retarget).

- [ ] **Step 7: Run validate.py (separate from commit) and READ**

Run: `cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py; echo EXIT=$?`
Expected: PASS, `EXIT=0` (the 3 descriptors still validate; C1–C7 green — `transport.carry` is in
the C1 capability enum, M2 `stability_margin` satisfied).

- [ ] **Step 8: Commit**

```bash
cd ~/Documents/GitHub/rfl
git add examples/01-cable-insertion/skill-carry.yaml \
        examples/01-cable-insertion/embodiments/allegro.yaml \
        examples/01-cable-insertion/embodiments/leap.yaml \
        examples/01-cable-insertion/embodiments/pneumatic-6f.yaml \
        crates/rfl-conformance/tests/transport_carry.rs \
        crates/rfl-conformance/tests/snapshots/transport_carry__carry_allegro.snap \
        crates/rfl-conformance/tests/snapshots/transport_carry__carry_leap.snap \
        crates/rfl-conformance/tests/snapshots/transport_carry__carry_pneumatic.snap
git commit -F - <<'EOF'
feat(examples): cable-carry worked example + carry retarget goldens

skill-carry.yaml = locate -> grasp.pinch -> transport.carry, reusing the
connector mass + the three descriptors (each gains transport.carry + the
M2-required stability_margin limit). Per-hand carry a_max (allegro 1.014481,
leap/pneumatic 0 — insufficient headroom for a 1.45 N object under 0.4 N
disturbance) is Principle-1 for the carry bound. Existing cable/screw/scan
goldens unchanged.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```
Expected: push ok, `0 0`.

---

## Task 4: conformance — interval-invariant for carry + the held-floor sub-check

**Files:** Modify `crates/rfl-conformance/src/lib.rs`, `tests/envelope_conformance.rs`

- [ ] **Step 1: Write the failing tests**

In `lib.rs` `tests` module, extend the ENV1 mapping test and add a carry interval test stub —
add to `envelope_class_mapping_follows_env1`:

```rust
        assert_eq!(envelope_class_for("carry"), Some(EnvelopeClass::IntervalInvariant));
```

In `tests/envelope_conformance.rs` add (carry uses skill-carry.yaml, action index 2):

```rust
    #[test]
    fn nominal_carry_passes_interval_with_held_floor() {
        let dir = example_dir();
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill-carry.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        let (goal, report) = &pairs[2]; // locate, pinch, carry
        // interval-invariant: multi-sampled (N=3) and every sample carries a pose.
        assert_eq!(report.telemetry.len(), 3, "carry is interval-sampled");
        assert_eq!(check_envelope(EnvelopeClass::IntervalInvariant, goal, report), CheckOutcome::Pass);
        // non-vacuous: the held floor IS present and IS being checked.
        assert!(report.telemetry[0].securing_force.is_some());
    }

    #[test]
    fn under_secure_fails_carry_interval_held_floor() {
        let dir = example_dir();
        let pairs = drive(
            FaultyDriver::new(Fault::UnderSecure),
            &dir.join("skill-carry.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        let (goal, report) = &pairs[2];
        // the held leg of the interval invariant bites: securing_force 0.1 < 2.9.
        assert!(matches!(
            check_envelope(EnvelopeClass::IntervalInvariant, goal, report),
            CheckOutcome::Fail(_)
        ));
    }

    #[test]
    fn mid_interval_drop_fails_carry_interval_but_passes_terminal() {
        let dir = example_dir();
        let pairs = drive(
            FaultyDriver::new(Fault::MidIntervalDrop),
            &dir.join("skill-carry.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        let (goal, report) = &pairs[2];
        // the station leg: a mid-interval pose gap fails interval, endpoint still conforms.
        assert!(matches!(
            check_envelope(EnvelopeClass::IntervalInvariant, goal, report),
            CheckOutcome::Fail(_)
        ));
        assert_eq!(check_envelope(EnvelopeClass::TerminalPostcondition, goal, report), CheckOutcome::Pass);
    }
```

Note the test file needs the example dir helper to be at the 01 example. The existing
`example_dir()` in `envelope_conformance.rs` already points at `01-cable-insertion` (it loads
`skill.yaml` there) — reuse it (carry uses `skill-carry.yaml` in the same dir).

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance 2>&1 | tail -25`
Expected: FAIL — `envelope_class_for("carry")` returns `None` (so the mapping assert fails), and
the carry interval tests fail (carry currently maps to `None` → driver gives 1 sample, and the
`IntervalInvariant` arm does not yet check the held floor).

- [ ] **Step 3a: Map carry to interval-invariant** — in `lib.rs::envelope_class_for`, add to the
`hover` arm:

```rust
        "hover" | "carry" => Some(EnvelopeClass::IntervalInvariant),
```
(replace the existing `"hover" => ...` line).

- [ ] **Step 3b: Factor the shared floor helper** — in `lib.rs`, add (near `quantity_mag`):

```rust
/// If the action carries a `min_holding_force` floor and any telemetry sample's
/// `securing_force` is below it, the failure reason; else `None`. Shared by the
/// grasp-continuity check and the held leg of the interval-invariant check (`05` GC1).
fn securing_floor_violation(goal: &ExecuteGoal, report: &DriverReport) -> Option<String> {
    let floor = goal
        .canonical_action
        .safety_envelope
        .force_profile
        .as_ref()
        .and_then(|fp| fp.get("min_holding_force"))
        .and_then(serde_json::Value::as_str)
        .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v))?;
    for t in &report.telemetry {
        if let Some(sf) = t.securing_force.as_ref().and_then(quantity_mag) {
            if sf < floor {
                return Some(format!("securing_force {sf} < min_holding_force {floor}"));
            }
        }
    }
    None
}
```

- [ ] **Step 3c: Rewrite the `GraspContinuity` arm** to use the helper (preserves behavior —
floor absent → Pass; present + breach → Fail):

```rust
        EnvelopeClass::GraspContinuity => match securing_floor_violation(goal, report) {
            Some(reason) => CheckOutcome::Fail(reason),
            None => CheckOutcome::Pass,
        },
```

- [ ] **Step 3d: Enrich the `IntervalInvariant` arm** — after the existing pose-present loop and
before `CheckOutcome::Pass`, add the held-floor sub-check:

```rust
            // Held interval (transport.carry, § 4.4): if the action carries a
            // min_holding_force floor, the grasp must stay secured at >= floor at EVERY
            // interval sample — the held leg of the interval invariant. Vacuous for
            // reach.hover (no floor), so hover is unaffected.
            if let Some(reason) = securing_floor_violation(goal, report) {
                return CheckOutcome::Fail(reason);
            }
```

- [ ] **Step 4: Run to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance 2>&1 | tail -25`
Expected: PASS — the new carry tests + the unchanged hover/grasp-continuity tests (the
existing `interval_invariant_checks_every_sample` still passes: its `sample_action` has no
`force_profile`, so the held-floor sub-check is vacuous).

- [ ] **Step 5: Full suite + validate.py (separate from commit) and READ**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | tail -25`
Then: `cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py; echo EXIT=$?`
Then confirm no golden churn: `git status --porcelain crates/rfl-conformance/tests/snapshots/`
Expected: all `ok.`; validate.py PASS `EXIT=0`; `git status` clean (this task is pass/fail
asserts + the driver mapping — driver_protocol goldens unchanged because carry is not in the
cable `skill.yaml` that those goldens use; re-run them explicitly to confirm byte-identical).

- [ ] **Step 6: Commit**

```bash
cd ~/Documents/GitHub/rfl
git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/envelope_conformance.rs
git commit -F - <<'EOF'
feat(conformance): carry interval-invariant + held-floor over the interval

spec/05 ENV1: transport.carry is interval-invariant (exactly one class). One
envelope_class_for arm (no signature change); the multi-sample driver is
inherited. The IntervalInvariant check is enriched with a securing-floor
sub-check (factored, shared with grasp-continuity) so a held carry's interval
invariant also requires securing_force >= min_holding_force at every sample
(§ 4.4); vacuous for reach.hover. UnderSecure fails the held leg, MidIntervalDrop
the station leg; both reuse existing adversarial drivers.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```
Expected: push ok, `0 0`.

---

## Task 5: README refresh + memory update (wrap-up)

**Files:** Modify `README.md` (commit); update `~/.claude/.../memory/project_rfl.md` + `MEMORY.md`
(NOT git — memory files, written with real hashes after Tasks 1–4 push).

- [ ] **Step 1: Read the README's example / status section**

Run: `cd ~/Documents/GitHub/rfl && grep -n "skill-hover\|skill-spiral\|surface-scan\|examples/\|envelope" README.md | head -30`
Then read that region.

- [ ] **Step 2: Edit the README** — add the carry variant beside the hover/spiral variants
(e.g. "`examples/01-cable-insertion/skill-carry.yaml` — `transport.carry`, held
interval-invariant"), and refresh any test-count line to the new totals (read the actual
`cargo test` summary; do not guess).

- [ ] **Step 3: Verify + commit the README**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | tail -5` (confirm green; counts).
```bash
cd ~/Documents/GitHub/rfl
git add README.md
git commit -F - <<'EOF'
docs(readme): note the transport.carry variant + refresh counts

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```
Expected: push ok, `0 0`.

- [ ] **Step 4: Update the auto-memory with REAL hashes** (not git): in
`~/.claude/projects/.../memory/`, append the 13th-increment milestone to the
`## Implementation track` of `project_rfl.md` (design `f12da2c` + the real Task 1–5 hashes
from `git log --oneline`), and refresh the `MEMORY.md` NEXT-options line (transport.carry DONE;
remaining = ENV3 = increment 2, then in-process incrementing ends). Record per-hand carry
a_max (allegro 1.014481 / leap 0 / pneumatic 0), the ENV1 single-class finding, the enriched
IntervalInvariant check, and that nothing else changed (no schema change).

---

## Self-Review

**Spec coverage** (design doc § by §): § 4 rfl-core → Tasks 1–2; § 5 conformance → Task 4;
§ 6 descriptors + example → Task 3; § 7 tests → Tasks 1–4; § 8 deferred → none implemented
(correct). The ENV1 single-class finding (§ 1) → Task 4 Step 3a (one arm, no signature change).
The carry_a_max formula (§ 2) → Task 1. The two auto params (§ 3) → Task 2 lowering. ✓

**Placeholder scan:** every code step has complete code; every run step has an exact command
+ expected output; goldens are generated + analytically verified (the established RFL pattern),
not hand-written. No TBD/TODO. ✓

**Type consistency:** `carry_a_max(weight_n, payload_n, disturbance_n, stability_margin)` is
identical in Task 1 (def), Task 2 (call site), and the design doc. `DisturbanceArg` /
`StabilityMarginArg` defined in Task 2 Step 3a, used in the same step's lowering.
`ratio_limit` defined (embodiment.rs) + called (translation.rs) in Task 2.
`securing_floor_violation(goal, report)` defined Task 4 Step 3b, used in 3c + 3d.
`envelope_class_for` return type unchanged (`Option<EnvelopeClass>`) — `"carry"` joins the
`"hover"` arm. ✓
