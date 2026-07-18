# `force.screw` worked example + structural lowering (E1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `force.screw` as a working primitive end-to-end — a new `Primitive` (parse + structural lowering + capability gate), a 3rd worked example (drive a screw with a held driver), and Class 2 conformance — with the tool-mediated reaction and `thread_pitch` coupling kept symbolic (GF4c = E2).

**Architecture:** `rfl-core::skill_isa` gains a `ForceScrew` variant + param struct (typed from the schema's `ForceScrewParams`); `rfl-core::translation` gains `lower_force_screw` (torque budget → `force_profile.torque`, `ScrewStop` completion → a `Monitor`, compliance; tool-mediated/coupling symbolic) + a `force.screw` capability-gate arm. A new `examples/03-screw-fasten/` (skill + 3 descriptors copied from the cable example + the capability). A new `rfl-conformance/tests/screw_fasten.rs` (3 goldens + determinism + boon), plus the `screw → force-trajectory` envelope-class mapping arm.

**Tech Stack:** Rust (workspace `rfl-core` / `rfl-cli` / `rfl-conformance`, edition 2024, MSRV 1.85, cargo 1.96 via rustup), `serde`/`serde_yaml`/`serde_json`, `insta`, `boon`. Python `validate.py` / ad-hoc jsonschema via `uv`.

**Design doc:** `docs/design/2026-05-31-force-screw-e1-design.md` (committed, `8d34de0`).

**Standing rules (every task):**
- Prefix every cargo command with `export PATH="$HOME/.cargo/bin:$PATH"`.
- **Validation read and `git commit` MUST be separate batches.** Run tests, READ `ok`/`PASS`, then stage + commit later.
- `git add` explicit paths only — never `-A` (keeps `docs/plans/` out).
- Before commit: branch == `main`. Before push: `git fetch -q origin && git merge-base --is-ancestor origin/main HEAD`. After push: `git rev-list --left-right --count origin/main...HEAD` == `0 0`. Rebase onto `origin/main` if ff fails. No `--force`, no `--no-verify`.
- Conventional Commits + `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. Keep `cargo test` warning-clean.
- **Exhaustive-match rule:** `lower` / `check_capability` have no catch-all. The new `Primitive::ForceScrew` variant and its two match arms land in the SAME commit (Task 1) or the workspace will not compile.

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `crates/rfl-core/src/skill_isa.rs` | `ForceScrew` variant + param struct + parse | 1 |
| `crates/rfl-core/src/translation.rs` | `force.screw` gate arm + `lower` arm + `lower_force_screw` | 1 |
| `examples/03-screw-fasten/skill.yaml` + `embodiments/*.yaml` + `run.py` | the worked example | 2 |
| `crates/rfl-conformance/tests/screw_fasten.rs` | 3 goldens + determinism + boon | 3 |
| `crates/rfl-conformance/src/lib.rs` | `envelope_class_for` `screw` arm | 3 |

No `spec/` or `schemas/` change (`force.screw` is already typed in `skill-isa.schema.json`). `validate.py` is unchanged (it validates example 01 only; the new skill is Class-1-checked via a one-off jsonschema run in Task 2).

---

## Task 1: the `force.screw` primitive (parse + lower + gate)

**Files:**
- Modify: `crates/rfl-core/src/skill_isa.rs` (add `ForceScrew` variant + struct; add a parse test)
- Modify: `crates/rfl-core/src/translation.rs` (import `ForceScrew`; add gate arm + `lower` arm + `lower_force_screw`; add lower/gate tests)

- [ ] **Step 1: Write the failing parse test** — in `crates/rfl-core/src/skill_isa.rs` `mod parse_tests`, add:

```rust
    #[test]
    fn force_screw_parses() {
        let yaml = "skill: t\nbody:\n  sequence:\n    - force.screw:\n        grasp_handle: active\n        thread_axis: -z\n        torque_budget: 2 N\u{b7}m\n        thread_pitch: 0.8 mm\n        tool_mediated: true\n        compliance: active\n        completion:\n          all_of:\n            - effort_rise: 1.5 N\u{b7}m\n            - reached: { advance: 5 mm }\n";
        let s = Skill::parse_yaml(yaml).expect("parse");
        let Statement::Primitive(Primitive::ForceScrew(p)) = &s.body.sequence[0] else {
            panic!("expected force.screw");
        };
        assert_eq!(p.torque_budget.0, "2 N\u{b7}m");
        assert_eq!(p.thread_pitch.as_ref().unwrap().0, "0.8 mm");
        assert!(matches!(p.compliance, Some(Compliance::Active)));
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core force_screw_parses 2>&1 | grep -E "no variant|cannot find|error\[|test result" | head`
Expected: FAIL — `ForceScrew` is not a `Primitive` variant.

- [ ] **Step 3: Add the `ForceScrew` variant + param struct** — in `crates/rfl-core/src/skill_isa.rs`:

In the `Primitive` enum, add (after the `SenseInspect` variant, before the closing `}`):

```rust
    /// `force.screw`.
    #[serde(rename = "force.screw")]
    ForceScrew(ForceScrew),
```

And add the struct (after the `SenseInspect` struct, near the end of the param structs):

```rust
/// `force.screw` parameters (v0 subset of `$defs/ForceScrewParams`). `completion`
/// (a `ScrewStop`) and `tool_mediated` / `thread_pitch` are carried structurally;
/// the tool-mediated reaction + rotation↔advance coupling are a later increment.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceScrew {
    /// The screw / thread axis (in `frame`).
    pub thread_axis: Direction,
    /// Max torque about `thread_axis`.
    pub torque_budget: Quantity,
    /// `ScrewStop` completion, lowered into `monitors` by the Translation Layer.
    pub completion: serde_yaml::Value,
    /// Rotation→advance coupling pitch (carried; symbolic in v0).
    #[serde(default)]
    pub thread_pitch: Option<Quantity>,
    /// Whether a held tool transmits the torque (carried; reaction is a later increment).
    #[serde(default)]
    pub tool_mediated: Option<serde_yaml::Value>,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
    /// The grasp on the fastener or the driving tool.
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
}
```

- [ ] **Step 4: Run the parse test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core force_screw_parses 2>&1 | grep -E "test result|error\[" | head`
Expected: the parse test passes. (`cargo` may also report `non-exhaustive patterns` *errors* in `translation.rs::lower`/`check_capability` — that is expected and fixed in Step 5-7; if the parse test cannot run because `translation.rs` fails to compile, proceed to Step 5 and re-run after.)

- [ ] **Step 5: Write the failing lower + gate tests** — in `crates/rfl-core/src/translation.rs` `mod tests`, add:

```rust
    const SCREW_SKILL: &str = "skill: t\nbody:\n  sequence:\n    - force.screw:\n        grasp_handle: active\n        thread_axis: -z\n        torque_budget: 2 N\u{b7}m\n        thread_pitch: 0.8 mm\n        tool_mediated: true\n        compliance: active\n        completion:\n          all_of:\n            - effort_rise: 1.5 N\u{b7}m\n            - reached: { advance: 5 mm }\n";

    #[test]
    fn screw_lowers_torque_and_completion() {
        let skill = Skill::parse_yaml(SCREW_SKILL).unwrap();
        let mut emb = load("allegro").1; // cable allegro descriptor
        emb.capabilities.skills.push("force.screw".to_string());
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["screw"]);
        let a = &out.actions[0];
        let fp = serde_json::to_string(&a.safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"torque\":\"2 N\u{b7}m\""), "got {fp}");
        assert!(fp.contains("\"thread_pitch\":\"0.8 mm\""), "got {fp}");
        assert_eq!(a.safety_envelope.compliance.as_deref(), Some("active"));
        let mon = serde_json::to_string(&a.monitors).unwrap();
        assert!(mon.contains("effort_rise"));
        assert!(mon.contains("advance"));
        assert!(a.force_budget.is_none()); // the budget is a torque, in force_profile
    }

    #[test]
    fn screw_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(SCREW_SKILL).unwrap();
        let emb = load("allegro").1; // cable allegro lacks force.screw
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: force.screw"), "got {err}");
    }
```

- [ ] **Step 6: Add the import + gate arm + lower arm** — in `crates/rfl-core/src/translation.rs`:

Add `ForceScrew` to the `use crate::skill_isa::{...}` list.

In `check_capability`, add the arm (alongside `ForceInsertFit`):

```rust
        Primitive::ForceScrew(_) => "force.screw",
```

In `lower`, add the arm (alongside `ForceInsertFit`):

```rust
        Primitive::ForceScrew(p) => (lower_force_screw(p, e), "screw"),
```

- [ ] **Step 7: Add `lower_force_screw`** — in `crates/rfl-core/src/translation.rs`, after `lower_force_insert_fit`:

```rust
/// Lower `force.screw`: carry the torque budget into the force_profile (the
/// force-trajectory envelope's torque case), lower the `ScrewStop` completion into a
/// monitor, set compliance, and carry `thread_pitch` / `tool_mediated` as symbolic
/// markers. The tool-mediated reaction-torque limit and the rotation↔advance
/// coupling are a later increment (GF4c, E2). The target_pose drives along the
/// thread axis; the advance is governed by the completion (symbolic distance in v0).
fn lower_force_screw(p: &ForceScrew, e: &Embodiment) -> CanonicalAction {
    let monitors = vec![Monitor { stop_condition: yaml_to_json(&p.completion) }];
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    let mut fp = serde_json::json!({ "torque": p.torque_budget.0.clone() });
    if let Some(tp) = &p.thread_pitch {
        fp["thread_pitch"] = serde_json::Value::String(tp.0.clone());
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

- [ ] **Step 8: Run all rfl-core tests + warning check**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | grep -E "warning:|test result|FAILED|error\[" | head`
Expected: all `ok` (the 3 new tests + the existing 48); no warnings.

- [ ] **Step 9: Commit** (separate batch from Step 8 — the variant + both arms together)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-core/src/skill_isa.rs crates/rfl-core/src/translation.rs && git commit -m "feat(core): add force.screw primitive (parse + structural lowering + gate)

New Primitive::ForceScrew (typed from \$defs/ForceScrewParams) + lower_force_screw:
torque budget -> force_profile.torque, ScrewStop completion -> monitor, compliance;
thread_pitch / tool_mediated carried symbolically (GF4c = E2). force.screw gated.
Variant + lower + check_capability arms in one commit (exhaustive match).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: the `examples/03-screw-fasten/` worked example

**Files:**
- Create: `examples/03-screw-fasten/skill.yaml`
- Create: `examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml` (copies of the cable descriptors + `force.screw`)
- Create: `examples/03-screw-fasten/run.py`

- [ ] **Step 1: Create the skill** — `examples/03-screw-fasten/skill.yaml`:

```yaml
# Example 03 — Screw fastening
# Drive a threaded fastener with a held driver. Embodiment-agnostic Skill ISA
# composition: the driver is held in a precision pinch and force.screw transmits
# torque through it (tool_mediated). Per-embodiment bounds are resolved by retarget.

skill: screw-fasten
description: >
  Grasp a screwdriver, bring it to a threaded fastener, and drive the fastener to a
  seated state within a torque budget, discriminating a tight seat from a cross-thread.

objects:
  driver: { ref: driver }       # the held screwdriver (the force-transmission tool)
  screw:  { ref: screw }        # the threaded fastener to drive

body:
  sequence:

    # 1 — Locate the driver and bind its measured pose.
    - let: driver_t
      from:
        sense.locate:
          target_ref: driver
          modality: auto

    # 2 — Grasp the driver (a precision hold of the tool shank).
    - grasp.pinch:
        target: driver_t
        force_budget: 6 N
        tactile_target: auto
        slip_response: retighten

    # 3 — Bring the driver to a standoff above the fastener.
    - transport.move_to_pose:
        target_pose: { frame: workpiece, offset: { along: -z, distance: 40 mm } }

    # 4 — Locate the fastener and align the driver's tool axis to the thread axis.
    - let: screw_t
      from:
        sense.locate:
          target_ref: screw
          modality: visual
    - reach.align:
        target_frame: screw
        axes: [z]

    # 5 — Drive the fastener: torque rise AT the expected advance = seated (the all_of
    #     rule, 01 § Force-at-state). tool_mediated: the torque is transmitted through
    #     the held driver; the tool-grasp reaction limit + thread coupling are E2 (GF4c).
    - force.screw:
        grasp_handle: active
        thread_axis: -z
        torque_budget: 2 N·m
        thread_pitch: 0.8 mm
        tool_mediated: true
        compliance: active
        completion:
          all_of:
            - effort_rise: 1.5 N·m
            - reached: { advance: 5 mm }

    # 6 — Release the driver and retract clear.
    - grasp.release: { grasp_handle: active }
    - reach.retract:  { direction: -tool_axis, distance: 50 mm }
```

- [ ] **Step 2: Create the descriptors** — copy the three cable descriptors and add the capability:

```bash
cd ~/Documents/GitHub/rfl && mkdir -p examples/03-screw-fasten/embodiments && \
for h in allegro leap pneumatic-6f; do cp "examples/01-cable-insertion/embodiments/$h.yaml" "examples/03-screw-fasten/embodiments/$h.yaml"; done
```

Then, in each `examples/03-screw-fasten/embodiments/*.yaml`, add `force.screw` to the `capabilities.skills` list. For example, allegro's line:

`    skills: [grasp.pinch, grasp.lateral, grasp.envelope, transport, force.insert_fit, sense.locate]`

becomes:

`    skills: [grasp.pinch, grasp.lateral, grasp.envelope, transport, force.insert_fit, force.screw, sense.locate]`

(Make the analogous edit to `leap.yaml` and `pneumatic-6f.yaml` — append `force.screw` to whatever their `skills:` list already contains; do not otherwise modify the descriptors.)

- [ ] **Step 3: Create the runner** — copy example 01's runner (it resolves the skill + embodiment relative to its own directory):

```bash
cd ~/Documents/GitHub/rfl && cp examples/01-cable-insertion/run.py examples/03-screw-fasten/run.py
```

- [ ] **Step 4: Class-1 validate the new skill + descriptors against the schemas** (one-off; `validate.py` itself is example-01-only)

Run:
```bash
cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python - <<'PY'
import json, sys, yaml, pathlib
from jsonschema import Draft202012Validator
root = pathlib.Path(".")
sk = Draft202012Validator(json.loads((root/"schemas/skill-isa.schema.json").read_text()))
de = Draft202012Validator(json.loads((root/"schemas/embodiment-descriptor.schema.json").read_text()))
ex = root/"examples/03-screw-fasten"
bad = 0
errs = list(sk.iter_errors(yaml.safe_load((ex/"skill.yaml").read_text())))
print("skill.yaml:", "ok" if not errs else errs[0].message); bad += len(errs)
for f in sorted((ex/"embodiments").glob("*.yaml")):
    e = list(de.iter_errors(yaml.safe_load(f.read_text())))
    print(f.name, ":", "ok" if not e else e[0].message); bad += len(e)
sys.exit(1 if bad else 0)
PY
echo "EXIT=$?"
```
Expected: `skill.yaml: ok`, each descriptor `ok`, `EXIT=0`. **If the skill fails**, the `force.screw` block does not match `ForceScrewParams` — inspect the error (likely the `completion` ScrewStop shape or the `N·m` torque unit) and fix `skill.yaml` before proceeding.

- [ ] **Step 5: Smoke-test retarget via the CLI**

Run: `cd ~/Documents/GitHub/rfl && export PATH="$HOME/.cargo/bin:$PATH" && cargo run -q -p rfl-cli -- retarget examples/03-screw-fasten/skill.yaml --embodiment examples/03-screw-fasten/embodiments/allegro.yaml | grep -c '"message":"execute"'`
Expected: `8` (locate, pinch, transport, locate, align, screw, release, retract). Spot-check a `screw` line carries `"force_profile":{"torque":"2 N·m"` and a `monitors` with `effort_rise`.

- [ ] **Step 6: Commit** (separate batch from Steps 4-5)

```bash
cd ~/Documents/GitHub/rfl && git add examples/03-screw-fasten/skill.yaml examples/03-screw-fasten/embodiments/allegro.yaml examples/03-screw-fasten/embodiments/leap.yaml examples/03-screw-fasten/embodiments/pneumatic-6f.yaml examples/03-screw-fasten/run.py && git commit -m "docs(example): add the screw-fasten worked example (03)

Drive a screw with a held driver: locate/pinch driver -> transport -> align ->
force.screw (torque budget, all_of seating/cross-thread) -> release -> retract.
Three self-contained descriptors (cable copies + force.screw capability).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Class 2 conformance + envelope mapping

**Files:**
- Create: `crates/rfl-conformance/tests/screw_fasten.rs`
- Modify: `crates/rfl-conformance/src/lib.rs` (`envelope_class_for` `screw` arm)
- Test/golden: `crates/rfl-conformance/tests/snapshots/screw_fasten__screw_{allegro,leap,pneumatic}.snap`

- [ ] **Step 1: Add the envelope-mapping arm** — in `crates/rfl-conformance/src/lib.rs` `envelope_class_for`, add `scan` already maps to Terminal; add `screw` to the ForceTrajectory arm:

```rust
        "insert_fit" | "screw" => Some(EnvelopeClass::ForceTrajectory),
```

(Replace the existing `"insert_fit" => Some(EnvelopeClass::ForceTrajectory),` arm with the line above. Forward-looking — `force.screw`'s torque-trajectory check + driver torque echo are E2; no current test exercises it.)

- [ ] **Step 2: Create the conformance test** — `crates/rfl-conformance/tests/screw_fasten.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for the screw-fasten example: the force.screw retarget
//! output is byte-deterministic, matches a committed golden, and each emitted line is
//! a valid driver-interface execute message.

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_screw_allegro() {
    insta::assert_snapshot!("screw_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_screw_leap() {
    insta::assert_snapshot!("screw_leap", jsonl_for("leap"));
}

#[test]
fn golden_screw_pneumatic() {
    insta::assert_snapshot!("screw_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn screw_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn screw_every_line_is_a_valid_execute_message() {
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

- [ ] **Step 3: Run the non-golden tests first** (determinism + boon — confirms the screw execute message validates)

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test screw_fasten -- screw_generation_is_byte_identical screw_every_line_is_a_valid_execute_message 2>&1 | grep -E "test .* (ok|FAILED)|test result|panic" | head`
Expected: both PASS. **If `screw_every_line_is_a_valid_execute_message` fails**, the screw canonical action violates the driver-interface schema — inspect (likely the `force_profile` content); the open `canonical_action`/`Envelope` floor should accept it.

- [ ] **Step 4: Generate + eyeball the goldens**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test screw_fasten` (expect 3 `golden_*` FAIL — no snapshot)
Then: `export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance --test screw_fasten >/dev/null 2>&1; rm -f crates/rfl-conformance/tests/snapshots/screw_fasten__*.snap.new; ls crates/rfl-conformance/tests/snapshots/screw_fasten__*; grep screw crates/rfl-conformance/tests/snapshots/screw_fasten__screw_allegro.snap`
Expected: 3 `.snap` files (no `.new`); the screw line shows `"force_profile":{"torque":"2 N·m","thread_pitch":"0.8 mm","tool_mediated":true}` + a `monitors` with `effort_rise`/`advance` + `"compliance":"active"`.

- [ ] **Step 5: Re-run to confirm green**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test screw_fasten 2>&1 | grep "test result"`
Expected: `test result: ok. 5 passed`.

- [ ] **Step 6: Gating validation batch** (separate from the commit)

Run: `cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1 && export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "test result|FAILED|error|warning:" | tail -12`
Expected: validate.py `PASS` (C1–C7 unchanged); every crate green (rfl-core 51 + conformance lib + driver_protocol 7 + envelope_conformance 4 + retarget_determinism 5 + surface_scan 5 + screw_fasten 5); no warnings.

- [ ] **Step 7: Commit** (separate batch — after reading PASS)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-conformance/src/lib.rs crates/rfl-conformance/tests/screw_fasten.rs crates/rfl-conformance/tests/snapshots/screw_fasten__screw_allegro.snap crates/rfl-conformance/tests/snapshots/screw_fasten__screw_leap.snap crates/rfl-conformance/tests/snapshots/screw_fasten__screw_pneumatic.snap && git commit -m "test(conformance): Class 2 golden for the screw-fasten example + screw->force-trajectory map

Three insta goldens (one per hand) + generate-twice determinism + per-line boon
validation of the force.screw execute stream. Adds the screw->force-trajectory
envelope-class mapping arm (forward-looking; the torque check is E2).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Full verification + push + memory

**Files:** none.

- [ ] **Step 1: Final full suite + validate.py**

Run: `cd ~/Documents/GitHub/rfl && export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "test result|FAILED|error" && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: all green; validate.py `PASS`.

- [ ] **Step 2: Confirm pushes** (Tasks 1-3 each pushed per the standing rules)

Run: `cd ~/Documents/GitHub/rfl && git rev-parse --abbrev-ref HEAD && git fetch -q origin && git merge-base --is-ancestor origin/main HEAD && git push -q origin main; git rev-list --left-right --count origin/main...HEAD && git log --oneline -5`
Expected: branch `main`; final `0 0`; the 3 E1 commits + the design-doc commit visible.

- [ ] **Step 3: Update the memory** (`~/.claude/.../memory/project_rfl.md` "## Implementation track") with the real commit hashes from `git log --oneline -6`, recording this as the 6th increment (force.screw E1 — worked example + structural lowering), and note E2 (GF4c numeric: tool-grasp reaction-torque limit + thread_pitch coupling + torque-trajectory checker + driver torque echo) as the planned follow-up. Update the `MEMORY.md` RFL index line (6 increments, new HEAD). Not a repo commit — memory only.

---

## Self-review (completed during planning)

- **Spec coverage:** design §3 example → Task 2; §4 new primitive (parse) → Task 1 (Steps 1-4); §5 lowering → Task 1 (Steps 5-7); §6 envelope mapping → Task 3 (Step 1); §7 conformance → Task 3; §2 deferrals (GF4c reaction/coupling, torque checker, unscrew, rotational limits) → out of scope, no task. No gaps.
- **Placeholder scan:** every code step shows complete code; commands show expected output. The descriptor edit (Step 2.2) is a copy + an explicit one-line skills edit shown verbatim for allegro.
- **Type consistency:** `ForceScrew` (variant + struct), `lower_force_screw`, the `"force.screw"` gate key, the `"screw"` suffix, and `envelope_class_for("screw")` are used identically across Tasks 1-3. `lower_force_screw` reads `p.thread_axis`/`torque_budget`/`completion`/`thread_pitch`/`tool_mediated`/`compliance` — the exact `ForceScrew` fields defined in Task 1. The example's `force.screw` block (Task 2) uses only schema-valid `ForceScrewParams` fields, verified by Step 2.4.
- **Determinism:** `torque_budget` / `thread_pitch` carried as authored strings (`"2 N·m"`, `"0.8 mm"`); `target_pose` distance fixed `"0 mm"`; no float emitted — golden + generate-twice covered.
- **Exhaustive match:** Task 1 lands the variant + `lower` arm + `check_capability` arm in one commit (Step 9), so the crate compiles.
