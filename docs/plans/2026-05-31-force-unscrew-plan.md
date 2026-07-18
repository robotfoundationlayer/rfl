# force.unscrew Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the `force.unscrew` primitive (reverse coupled rotation + axial retreat to extract a fastener) to the Rust engine, reusing the GF4c torque clamp and the E3 torque-trajectory verification, with no schema change.

**Architecture:** rfl-core gains a `ForceUnscrew` struct + `Primitive::ForceUnscrew` + `lower_force_unscrew` (mirrors `lower_force_screw`: GF4c reaction-torque clamp, a `rotation_sense: loosen` marker, a default disengagement `Monitor` when `completion` is omitted). rfl-conformance maps `envelope_class_for("unscrew") => ForceTrajectory` so E3 verifies it, plus a worked example (`skill-unscrew.yaml` in example 03) with per-hand goldens and adversarial torque tests.

**Tech Stack:** Rust (rfl-core / rfl-conformance), serde_yaml/serde_json, insta (goldens), boon (schema), uv + jsonschema (Class-1).

**Process discipline (this repo):**
- Prefix every cargo command with `export PATH="$HOME/.cargo/bin:$PATH"`.
- Run validation (`cargo test` / `validate.py`) and read the result in a batch **physically separate** from the `git commit`.
- `git add` explicit paths only (never `-A`); never stage `docs/plans/`. Parallel session shares the tree — `git status` immediately before each stage.
- Per commit: branch == `main`; before push `git merge-base --is-ancestor origin/main HEAD`; after push `git rev-list --left-right --count origin/main...HEAD` == `0 0`.
- Conventional Commits + `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. Real `git log` hashes.
- insta regen: `INSTA_UPDATE=always cargo test -p rfl-conformance --test <name>`, then `rm -f crates/rfl-conformance/tests/snapshots/*.snap.new`, then `git diff`.
- **Exhaustive-match trap:** the `lower` and `check_capability` matches in `translation.rs` have no catch-all. The new `Primitive::ForceUnscrew` variant + its `lower` arm + its `check_capability` arm MUST land in the SAME commit (Task 1) or rfl-core will not compile.

**Facts:** `Direction = serde_yaml::Value`; `Compliance ∈ {Passive, Active, Auto}`; `GraspHandle` exists. The screw example (`examples/03-screw-fasten`) has 8 actions (`locate`0, `pinch`1, `transport`2, `locate`3, `align`4, `screw`5, `release`6, `retract`7); the unscrew variant has the same shape with `unscrew` at index 5. The driver `estimated_mass 1.0 N` (pinch) → the GF4c clamp gives `reaction_torque_limit(2, grip_force_max, Pinch)` = allegro 0.2 / leap 0.15 / pneumatic 0.12 N·m (identical to screw E2).

---

## Task 1: rfl-core — the `force.unscrew` primitive

**Files:**
- Modify: `crates/rfl-core/src/skill_isa.rs` (add `ForceUnscrew` struct + `Primitive::ForceUnscrew`)
- Modify: `crates/rfl-core/src/translation.rs` (import, `check_capability` arm, `lower` arm, `lower_force_unscrew`, 2 tests)

- [ ] **Step 1: Write the failing translation tests**

Append to the `mod tests` block in `crates/rfl-core/src/translation.rs` (it has `load`, `retarget`, `Skill` in scope):

```rust
    #[test]
    fn unscrew_lowers_to_torque_trajectory_with_disengagement_default() {
        // Standalone (no pinch) -> ctx.held is None -> torque unclamped; completion
        // omitted -> the disengagement default monitor; rotation_sense marks the reverse.
        let yaml = "skill: t\nbody:\n  sequence:\n    - force.unscrew:\n        grasp_handle: active\n        thread_axis: -z\n        torque_budget: 2 N\u{b7}m\n        thread_pitch: 0.8 mm\n        tool_mediated: true\n        compliance: active\n        on_disengagement: retain\n";
        let skill = Skill::parse_yaml(yaml).expect("parse");
        let mut emb = load("allegro").1;
        emb.capabilities.skills.push("force.unscrew".to_string());
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["unscrew"]);
        let fp = out.actions[0].safety_envelope.force_profile.as_ref().expect("force_profile");
        assert_eq!(fp.get("torque").and_then(|v| v.as_str()), Some("2 N\u{b7}m")); // unclamped (no tool held)
        assert_eq!(fp.get("rotation_sense").and_then(|v| v.as_str()), Some("loosen"));
        assert_eq!(fp.get("on_disengagement").and_then(|v| v.as_str()), Some("retain"));
        let m = &out.actions[0].monitors[0];
        assert_eq!(m.stop_condition.get("disengagement").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn unscrew_capability_absent_when_gate_key_missing() {
        let yaml = "skill: t\nbody:\n  sequence:\n    - force.unscrew: { thread_axis: -z, torque_budget: 2 N\u{b7}m }\n";
        let skill = Skill::parse_yaml(yaml).expect("parse");
        let emb = load("allegro").1; // cable allegro lacks force.unscrew
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: force.unscrew"), "got {err}");
    }
```

- [ ] **Step 2: Run to verify they fail (compile error: no ForceUnscrew variant)**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core unscrew 2>&1 | tail -8`
Expected: FAIL — the `force.unscrew` YAML has no matching `Primitive` variant (parse error / the type does not exist).

- [ ] **Step 3a: Add the `ForceUnscrew` struct + enum variant (`skill_isa.rs`)**

After the `ForceScrew` struct (ends with the `grasp_handle` field + `}`), add:

```rust
/// `force.unscrew` parameters (v0 subset of `$defs/ForceUnscrewParams`). Mirrors
/// `ForceScrew` minus `axial_force_budget`; `completion` is optional (defaults to
/// disengagement, emitted at lowering); `on_disengagement` is the freed-fastener
/// disposition. The authored effort_drop completion variant is schema-blocked.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceUnscrew {
    /// The thread axis (in `frame`).
    pub thread_axis: Direction,
    /// Max loosening torque about `thread_axis`.
    pub torque_budget: Quantity,
    /// `ScrewStop` completion (optional; default disengagement, emitted at lowering).
    #[serde(default)]
    pub completion: Option<serde_yaml::Value>,
    /// Reverse rotation->retreat coupling pitch (carried; symbolic in v0).
    #[serde(default)]
    pub thread_pitch: Option<Quantity>,
    /// Whether a held tool transmits the loosening torque.
    #[serde(default)]
    pub tool_mediated: Option<serde_yaml::Value>,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
    /// The grasp on the fastener or the driving tool.
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
    /// Freed-fastener disposition `{retain, drop_safe}` (default retain).
    #[serde(default)]
    pub on_disengagement: Option<serde_yaml::Value>,
}
```

And in `pub enum Primitive`, after the `ForceScrew(ForceScrew)` variant (before the closing `}`):

```rust
    /// `force.unscrew`.
    #[serde(rename = "force.unscrew")]
    ForceUnscrew(ForceUnscrew),
```

- [ ] **Step 3b: Wire it into `translation.rs` (import + both match arms + lowering)**

In the `use crate::skill_isa::{...}` import (line ~20), add `ForceUnscrew` to the list (e.g. after `ForceScrew,`).

In `check_capability`, after the `Primitive::ForceScrew(_) => "force.screw",` arm:

```rust
        Primitive::ForceUnscrew(_) => "force.unscrew",
```

In `lower` (the `match prim` returning `(CanonicalAction, suffix)`), after the `Primitive::ForceScrew(p) => (lower_force_screw(p, e, ctx), "screw"),` arm:

```rust
        Primitive::ForceUnscrew(p) => (lower_force_unscrew(p, e, ctx), "unscrew"),
```

After the `lower_force_screw` function, add:

```rust
/// Lower `force.unscrew`: reverse coupled rotation + axial retreat (`spec/01` § 6.5).
/// Mirrors `lower_force_screw` — the loosening torque loads the tool grasp (GF4c
/// reuse), `completion` defaults to a disengagement monitor when omitted, and
/// `rotation_sense: loosen` marks the reverse sense (symbolic, v0).
fn lower_force_unscrew(p: &ForceUnscrew, e: &Embodiment, ctx: &GraspContext) -> CanonicalAction {
    let stop_condition = match &p.completion {
        Some(c) => yaml_to_json(c),
        None => serde_json::json!({ "disengagement": true }),
    };
    let monitors = vec![Monitor { stop_condition }];
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    // GF4c reuse: a tool-mediated loosening torque loads the held tool's grasp.
    let tool_mediated = matches!(&p.tool_mediated, Some(v) if v.as_bool() != Some(false));
    let held = if tool_mediated { ctx.held.as_ref() } else { None };
    let torque = match (
        held,
        p.torque_budget.parse(),
        e.scalar_limit("grip_force_max").and_then(|q| q.parse()),
    ) {
        (Some(h), Some((tb, tu)), Some((gm, _))) => {
            Quantity::from_si(grasp_force::reaction_torque_limit(tb, gm, h.mode), tu)
        }
        _ => p.torque_budget.clone(),
    };
    let mut fp = serde_json::json!({ "torque": torque.0.clone(), "rotation_sense": "loosen" });
    fp["on_disengagement"] = match &p.on_disengagement {
        Some(v) => yaml_to_json(v),
        None => serde_json::json!("retain"),
    };
    if let Some(tp) = &p.thread_pitch {
        fp["coupling"] = serde_json::json!({ "advance_per_turn": tp.0.clone() });
    }
    if let Some(tm) = &p.tool_mediated {
        fp["tool_mediated"] = yaml_to_json(tm);
    }
    env.force_profile = Some(fp);
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::AxisRelative {
            direction: yaml_to_json(&p.thread_axis),
            distance: Quantity("0 mm".to_string()),
        },
        force_budget: None,
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

- [ ] **Step 4: Run the tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core unscrew 2>&1 | tail -8`
Expected: PASS — both `unscrew_*` tests green.

- [ ] **Step 5: Full rfl-core suite + warning check (validation batch, separate from commit)**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | grep "test result:" | head -1 && cargo build -p rfl-core 2>&1 | grep -i warning || echo "no warnings"`
Expected: all pass; no warnings. Read and confirm.

- [ ] **Step 6: Commit**

```bash
cd ~/Documents/GitHub/rfl && git fetch origin main --quiet && git rev-parse --abbrev-ref HEAD && git status --short
git add crates/rfl-core/src/skill_isa.rs crates/rfl-core/src/translation.rs
git commit -F - <<'EOF'
feat(core): add force.unscrew primitive (reverse coupled rotation)

force.unscrew (spec/01 § 6.5) extracts a threaded fastener by reverse coupled
rotation + axial retreat. Mirrors lower_force_screw: the loosening torque loads
the held tool's grasp (GF4c reuse, reaction_torque_limit), force_profile gains a
rotation_sense: loosen marker and the on_disengagement disposition, and the
completion defaults to a disengagement monitor when omitted (the authored
effort_drop variant stays schema-blocked). Enum + lower + check_capability arms
land together (exhaustive match). No schema change (ForceUnscrewParams already
typed).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git show --stat --oneline HEAD | head -4
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```

Expected: 2 files staged; push succeeds; final `0 0`. Record the real hash.

---

## Task 2: conformance — verify force.unscrew + worked example

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (`envelope_class_for` + its unit test)
- Create: `examples/03-screw-fasten/skill-unscrew.yaml`
- Modify: `examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml` (add `force.unscrew` capability)
- Create: `crates/rfl-conformance/tests/screw_fasten_unscrew.rs` + 3 generated snapshots
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (2 tests)

- [ ] **Step 1: Map `unscrew` to the torque-trajectory class + test**

In `crates/rfl-conformance/src/lib.rs`, `envelope_class_for`, change:

```rust
        "insert_fit" | "screw" => Some(EnvelopeClass::ForceTrajectory),
```

to:

```rust
        "insert_fit" | "screw" | "unscrew" => Some(EnvelopeClass::ForceTrajectory),
```

In the `lib.rs` `mod tests` `envelope_class_for` test (next to `assert_eq!(envelope_class_for("insert_fit"), ...)`), add:

```rust
        assert_eq!(envelope_class_for("unscrew"), Some(EnvelopeClass::ForceTrajectory));
```

- [ ] **Step 2: Create the unscrew worked example**

Create `examples/03-screw-fasten/skill-unscrew.yaml`:

```yaml
# Example 03 — Screw fastening (unscrew variant)
# Extract a threaded fastener with a held driver: reverse coupled rotation + axial
# retreat (force.unscrew), the reverse of skill.yaml's force.screw. The disengagement
# completion default (fully out) is emitted at lowering; the freed fastener is retained.
skill: screw-fasten
description: >
  Grasp a screwdriver, bring it to a seated fastener, and loosen it out within a
  torque budget, retaining the freed fastener.

objects:
  driver: { ref: driver, estimated_mass: 1.0 N }   # the held screwdriver (force-transmission tool)
  screw:  { ref: screw }        # the threaded fastener to extract

body:
  sequence:

    - let: driver_t
      from:
        sense.locate:
          target_ref: driver
          modality: auto

    - grasp.pinch:
        target: driver_t
        force_budget: 6 N
        tactile_target: auto
        slip_response: retighten

    - transport.move_to_pose:
        target_pose: { frame: workpiece, offset: { along: -z, distance: 40 mm } }

    - let: screw_t
      from:
        sense.locate:
          target_ref: screw
          modality: visual
    - reach.align:
        target_frame: screw
        axes: [z]

    - force.unscrew:
        grasp_handle: active
        thread_axis: -z
        torque_budget: 2 N·m
        thread_pitch: 0.8 mm
        tool_mediated: true
        compliance: active
        on_disengagement: retain

    - grasp.release: { grasp_handle: active }
    - reach.retract:  { direction: -tool_axis, distance: 50 mm }
```

- [ ] **Step 3: Add the `force.unscrew` capability to the 3 descriptors**

In each of `examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml`, the `skills:` list contains `force.screw`. Add `force.unscrew` immediately after it. Run this exact edit and verify each list gained it once:

```bash
cd ~/Documents/GitHub/rfl && for f in allegro leap pneumatic-6f; do sed -i '' 's/force\.screw,/force.screw, force.unscrew,/' "examples/03-screw-fasten/embodiments/$f.yaml"; done
grep -c "force.unscrew" examples/03-screw-fasten/embodiments/allegro.yaml examples/03-screw-fasten/embodiments/leap.yaml examples/03-screw-fasten/embodiments/pneumatic-6f.yaml
```

Expected: each file reports `1`. (If any descriptor lists `force.screw` last with no trailing comma, edit it by hand to `force.screw, force.unscrew, sense.locate`.)

- [ ] **Step 4: Class-1 validate the new skill + descriptors**

```bash
cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python -c '
import json, yaml
from jsonschema import Draft202012Validator
sk = Draft202012Validator(json.load(open("schemas/skill-isa.schema.json")))
de = Draft202012Validator(json.load(open("schemas/embodiment-descriptor.schema.json")))
doc = yaml.safe_load(open("examples/03-screw-fasten/skill-unscrew.yaml"))
print("skill-unscrew.yaml:", "OK" if not list(sk.iter_errors(doc)) else list(sk.iter_errors(doc))[0].message)
for s in ["allegro","leap","pneumatic-6f"]:
    d = yaml.safe_load(open(f"examples/03-screw-fasten/embodiments/{s}.yaml"))
    errs = list(de.iter_errors(d))
    print(f"{s}.yaml:", "OK" if not errs else errs[0].message)
'
```

Expected: `skill-unscrew.yaml: OK` and all three descriptors `OK` (force.unscrew is in skill-isa PrimitiveId, so the capability-key enum accepts it).

- [ ] **Step 5: Write the conformance golden test (snapshots missing -> fail)**

Create `crates/rfl-conformance/tests/screw_fasten_unscrew.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for the force.unscrew variant of the screw-fasten example:
//! the retarget output is byte-deterministic, matches a committed golden, and every line
//! is a valid driver-interface execute message. The tool-mediated loosening torque is
//! GF4c-clamped (allegro 0.2 / leap 0.15 / pneumatic 0.12 N·m) inside the unscrew line.

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-unscrew.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_unscrew_allegro() {
    insta::assert_snapshot!("unscrew_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_unscrew_leap() {
    insta::assert_snapshot!("unscrew_leap", jsonl_for("leap"));
}

#[test]
fn golden_unscrew_pneumatic() {
    insta::assert_snapshot!("unscrew_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn unscrew_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn unscrew_every_line_is_a_valid_execute_message() {
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

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test screw_fasten_unscrew 2>&1 | grep -E "golden_|test result:"`
Expected: the three `golden_unscrew_*` FAIL (no snapshot); `unscrew_generation_is_byte_identical` + `unscrew_every_line_is_a_valid_execute_message` PASS.

- [ ] **Step 6: Generate + review the goldens**

```bash
export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance --test screw_fasten_unscrew
rm -f ~/Documents/GitHub/rfl/crates/rfl-conformance/tests/snapshots/*.snap.new
cd ~/Documents/GitHub/rfl && echo "--- unscrew line: clamped torque + loosen + disengagement ---" && grep -h "0006-unscrew" crates/rfl-conformance/tests/snapshots/screw_fasten_unscrew__unscrew_allegro.snap | grep -o '"force_profile":{[^}]*}[^}]*}\|"rotation_sense":"loosen"\|"disengagement":true\|"torque":"[^"]*"' | head
```

Expected: 3 new snapshots; the `0006-unscrew` line shows `"torque":"0.2 N·m"` (allegro clamp), `"rotation_sense":"loosen"`, `"on_disengagement":"retain"`, and a `"disengagement":true` monitor. Confirm pinch line (`0002`) still carries its `min_holding_force` and transport (`0003`) its propagated floor (these flow from earlier increments). If the allegro torque is not `0.2 N·m`, STOP — the GF4c clamp did not bite (check the driver `estimated_mass`/pinch sets ctx.held).

- [ ] **Step 7: Write the adversarial torque tests**

Append to `crates/rfl-conformance/tests/envelope_conformance.rs` (after `over_torque_driver_fails_screw_force_trajectory`; `screw_dir`, `suffix_of`, `drive`, `ReferenceDriver`, `FaultyDriver`, `Fault`, `check_envelope`, `EnvelopeClass`, `CheckOutcome` are all in scope):

```rust
#[test]
fn nominal_unscrew_passes_force_trajectory() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-unscrew.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // force.unscrew is action index 5.
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "unscrew");
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
}

#[test]
fn over_torque_driver_fails_unscrew_force_trajectory() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverTorque),
        &dir.join("skill-unscrew.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // force.unscrew (index 5) -> torque-trajectory must reject the over-budget wrench torque.
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "unscrew");
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}
```

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test envelope_conformance unscrew 2>&1 | grep -E "test (nominal|over).*unscrew|test result:"`
Expected: both PASS (nominal: echoed clamped torque 0.2 ≤ 0.2 budget; OverTorque: 999 > 0.2).

- [ ] **Step 8: Full validation batch (separate from commit)**

```bash
export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | grep "test result:" | head -1 && cargo test -p rfl-conformance 2>&1 | grep -E "Running|test result:" && cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1 && echo "--- screw_fasten (screw) goldens unchanged? ---" && git diff --stat crates/rfl-conformance/tests/snapshots/screw_fasten__*.snap | tail -1 || echo "(unchanged)" && ls crates/rfl-conformance/tests/snapshots/*.snap.new 2>/dev/null && echo STALE || echo "no pending"
```

Expected: rfl-core all pass; every rfl-conformance suite passes (incl. new `screw_fasten_unscrew` 5 + `envelope_conformance` now 10); validate.py C1–C7 PASS; the screw (`screw_fasten__*`) goldens UNCHANGED (only a descriptor capability was added, which is not in the retarget output); no pending snapshots. Read and confirm before staging.

- [ ] **Step 9: Commit**

```bash
cd ~/Documents/GitHub/rfl && git fetch origin main --quiet && git rev-parse --abbrev-ref HEAD && git status --short
git add crates/rfl-conformance/src/lib.rs \
  examples/03-screw-fasten/skill-unscrew.yaml \
  examples/03-screw-fasten/embodiments/allegro.yaml \
  examples/03-screw-fasten/embodiments/leap.yaml \
  examples/03-screw-fasten/embodiments/pneumatic-6f.yaml \
  crates/rfl-conformance/tests/screw_fasten_unscrew.rs \
  crates/rfl-conformance/tests/snapshots/screw_fasten_unscrew__unscrew_allegro.snap \
  crates/rfl-conformance/tests/snapshots/screw_fasten_unscrew__unscrew_leap.snap \
  crates/rfl-conformance/tests/snapshots/screw_fasten_unscrew__unscrew_pneumatic.snap \
  crates/rfl-conformance/tests/envelope_conformance.rs
git commit -F - <<'EOF'
test(conformance): force.unscrew worked example + torque-trajectory verification

Map envelope_class_for("unscrew") to the torque-trajectory class so E3 verifies
it with no new checker code. Add examples/03 skill-unscrew.yaml (grasp driver ->
transport -> align -> force.unscrew (completion omitted -> disengagement default,
on_disengagement retain) -> release -> retract) + force.unscrew capability on the
3 descriptors. screw_fasten_unscrew goldens show the GF4c-clamped loosening torque
(allegro 0.2 / leap 0.15 / pneumatic 0.12 N·m); two envelope_conformance tests
prove nominal passes and OverTorque is rejected. The screw goldens are unchanged.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git show --stat --oneline HEAD | head -12
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```

Expected: `git show --stat` lists exactly 9 files (lib.rs + skill-unscrew.yaml + 3 descriptors + screw_fasten_unscrew.rs + 3 snapshots + envelope_conformance.rs), no `docs/plans/`; push succeeds; final `0 0`. Record the real hash.

---

## Final verification gate

- [ ] Both feat/test commits on `main`, each pushed with post-push `0 0`.
- [ ] `cargo test -p rfl-core` green; `cargo test -p rfl-conformance` green (8 suites incl. screw_fasten_unscrew); `validate.py` C1–C7 EXIT 0.
- [ ] screw / cable / surface goldens unchanged; only the new `screw_fasten_unscrew` snapshots added.
- [ ] No `schemas/` change; the new `Primitive::ForceUnscrew` variant landed with both its `lower` and `check_capability` arms.
- [ ] Update README (if status-bearing) / `project_rfl.md` Implementation track / `MEMORY.md` with the two real hashes.

## Self-review (run after writing, fix inline)

**Spec coverage** (design §3-§5):
- ForceUnscrew struct + enum + lower_force_unscrew + check_capability → Task 1. ✓
- envelope_class_for("unscrew") => ForceTrajectory → Task 2 Step 1. ✓
- worked example skill-unscrew.yaml + descriptor caps → Task 2 Steps 2-4. ✓
- screw_fasten_unscrew goldens + envelope torque tests → Task 2 Steps 5-7. ✓
- Deferred (authored effort_drop, drop_safe motion, decoupling detection, other category-6) → no task, out of scope per design §7. ✓

**Placeholder scan:** every code step shows full code; every run step has an exact command + expected output; values concrete (0.2/0.15/0.12 N·m, index 5, snapshot names). No TBD. ✓

**Type consistency:** `ForceUnscrew` fields mirror `ForceScrew` (read from the tree); `Direction = serde_yaml::Value`; `lower_force_unscrew(p, e, ctx)` matches the `lower` arm signature `lower_force_screw(p, e, ctx)`; `Compliance::{Passive,Active,Auto}`, `grasp_force::reaction_torque_limit`, `Monitor { stop_condition }`, `PoseExpr::AxisRelative { direction, distance }`, `envelope_class_for` / `suffix_of` / `Fault::OverTorque` all match the read code. ✓

**Golden scope:** Task 2 adds only `screw_fasten_unscrew` snapshots; the descriptor capability edit does not change the screw retarget output (verified in Step 8). ✓
