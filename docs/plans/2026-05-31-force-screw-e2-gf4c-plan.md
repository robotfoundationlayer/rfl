# GF4c tool-mediated force — reaction-torque limit + coupling (E2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the grasp-force arc (GF1c–GF4c): derive the tool-grasp reaction-torque limit and clamp `force.screw`'s `torque_budget` to it (when tool-mediated + a tool is held), and expand the `thread_pitch` coupling into the canonical action.

**Architecture:** `rfl-core::grasp_force` gains `reaction_torque_limit` (the rotational counterpart of GF3c's `reaction_limit`) + an `R_GRIP` constant. `lower_force_screw` gains a `&GraspContext` parameter (the dispatcher's `force.screw` arm passes it, like `force.insert_fit`), reads the held tool grasp, and clamps the emitted torque; the symbolic `thread_pitch` marker becomes a structured `force_profile.coupling`. The 3 `screw_fasten` goldens regenerate with the per-hand torque clamp.

**Tech Stack:** Rust (workspace `rfl-core` / `rfl-conformance`, edition 2024, MSRV 1.85, cargo 1.96 via rustup), `serde_json`, `insta`, `boon`. Python `validate.py` via `uv`.

**Design doc:** `docs/design/2026-05-31-force-screw-e2-gf4c-design.md` (committed, `f56c15f`).

**Standing rules (every task):**
- Prefix every cargo command with `export PATH="$HOME/.cargo/bin:$PATH"`.
- **Validation read and `git commit` MUST be separate batches.** Run tests, READ `ok`/`PASS`, then stage + commit later.
- `git add` explicit paths only — never `-A` (keeps `docs/plans/` out).
- Before commit: branch == `main`. Before push: `git fetch -q origin && git merge-base --is-ancestor origin/main HEAD`. After push: `git rev-list --left-right --count origin/main...HEAD` == `0 0`. Rebase onto `origin/main` if ff fails. No `--force`, no `--no-verify`.
- Conventional Commits + `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. Keep `cargo test` warning-clean.

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `crates/rfl-core/src/grasp_force.rs` | `R_GRIP` + `reaction_torque_limit` + unit tests | 1 |
| `crates/rfl-core/src/translation.rs` | `lower_force_screw` ctx + clamp + coupling; dispatcher arm; tests | 2 |
| `crates/rfl-conformance/tests/snapshots/screw_fasten__screw_*.snap` | regenerated goldens | 2 |

No `spec/` or `schemas/` change (the clamped torque + `coupling` object ride the open `Envelope` floor). `validate.py` unchanged.

---

## Task 1: the `reaction_torque_limit` derivation (`rfl-core::grasp_force`)

**Files:**
- Modify: `crates/rfl-core/src/grasp_force.rs` (add `R_GRIP` + `reaction_torque_limit` + tests)

- [ ] **Step 1: Write the failing tests** — in `crates/rfl-core/src/grasp_force.rs` `mod tests`, add:

```rust
    #[test]
    fn reaction_torque_limit_clamps_to_rotational_capacity() {
        // allegro grip 20 N: 20 * 0.02 / 2.0 = 0.2 N·m; the 2 N·m budget clamps to it.
        let a = reaction_torque_limit(2.0, 20.0, GraspMode::Pinch);
        assert!((a - 0.2).abs() < 1e-9, "got {a}");
        // pneumatic grip 12 N: 12 * 0.02 / 2.0 = 0.12.
        assert!((reaction_torque_limit(2.0, 12.0, GraspMode::Pinch) - 0.12).abs() < 1e-9);
    }

    #[test]
    fn reaction_torque_limit_keeps_budget_when_capacity_is_higher() {
        // a tiny 0.05 N·m budget under a 20 N grip (0.2 capacity) -> budget kept.
        assert!((reaction_torque_limit(0.05, 20.0, GraspMode::Pinch) - 0.05).abs() < 1e-9);
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core reaction_torque_limit 2>&1 | grep -E "cannot find|error\[|test result" | head`
Expected: FAIL — `cannot find function 'reaction_torque_limit'`.

- [ ] **Step 3: Add `R_GRIP` + `reaction_torque_limit`** — in `crates/rfl-core/src/grasp_force.rs`, after the `reaction_limit` function (before `#[cfg(test)]`):

```rust
/// Schematic effective grip radius (m) for the tool-grasp rotational capacity — a v0
/// reference-implementation constant (pinned by golden, non-normative).
pub const R_GRIP: f64 = 0.02;

/// GF4c — the reaction-torque limit: a tool-mediated `force` primitive's reaction is a
/// torque about the tool axis; the held tool's grasp must resist it with rotational
/// holding capacity (≈ grip force × lever ÷ the reaction factor), or the tool spins
/// in-grasp. Returns the smaller of the requested torque budget and that capacity
/// (both N·m). The rotational counterpart of `reaction_limit` (GF3c).
#[must_use]
pub fn reaction_torque_limit(torque_budget_nm: f64, grip_force_max_n: f64, mode: GraspMode) -> f64 {
    torque_budget_nm.min(grip_force_max_n * R_GRIP / mode.k_reaction())
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core grasp_force 2>&1 | grep -E "test result|FAILED|error\[" | head`
Expected: PASS (the 5 existing grasp_force tests + the 2 new ones = 7).

- [ ] **Step 5: Confirm warning-clean**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | grep -E "warning:|test result" | tail`
Expected: all `ok`, no warnings.

- [ ] **Step 6: Commit** (separate batch from Steps 4-5)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-core/src/grasp_force.rs && git commit -m "feat(core): add reaction_torque_limit (GF4c rotational reaction)

The tool-grasp rotational holding capacity (grip_force_max * R_GRIP / k_reaction)
bounds a tool-mediated force primitive's reaction torque -- the rotational
counterpart of GF3c's reaction_limit. R_GRIP is a documented v0 reference constant.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: clamp the torque + expand the coupling (`lower_force_screw`)

**Files:**
- Modify: `crates/rfl-core/src/translation.rs` (`lower` screw arm; `lower_force_screw`; the existing + a new test)
- Test/golden: `crates/rfl-conformance/tests/snapshots/screw_fasten__screw_{allegro,leap,pneumatic}.snap`

- [ ] **Step 1: Update the existing screw test + add the clamp test** — in `crates/rfl-core/src/translation.rs` `mod tests`:

Replace the `thread_pitch` assertion in `screw_lowers_torque_and_completion` (the bare inline skill has no preceding grasp, so the torque stays unclamped and `thread_pitch` becomes `coupling`):

```rust
        assert!(fp.contains("\"torque\":\"2 N\u{b7}m\""), "got {fp}"); // no held tool -> no clamp
        assert!(fp.contains("\"advance_per_turn\":\"0.8 mm\""), "got {fp}");
```

(That is: keep the existing `torque` assertion, and replace the line
`assert!(fp.contains("\"thread_pitch\":\"0.8 mm\""), "got {fp}");` with the
`advance_per_turn` line above. Leave the `compliance` / `monitors` / `force_budget`
assertions in that test unchanged.)

Then add a new test (after `screw_capability_absent_when_not_declared`):

```rust
    #[test]
    fn screw_torque_clamped_to_tool_grasp_capacity() {
        // examples/03: grasp.pinch holds the driver, so the tool-mediated force.screw
        // clamps the 2 N·m budget to grip_force_max * R_GRIP / k_reaction = 20*0.02/2 = 0.2.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill.yaml")).unwrap()).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .unwrap();
        let out = retarget(&skill, &emb).expect("retarget");
        // force.screw is action index 5 (locate, pinch, transport, locate, align, screw, ...).
        let fp = serde_json::to_string(&out.actions[5].safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"torque\":\"0.2 N\u{b7}m\""), "got {fp}");
        assert!(fp.contains("\"advance_per_turn\":\"0.8 mm\""), "got {fp}");
    }
```

- [ ] **Step 2: Run to verify failures**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core -- screw_lowers_torque_and_completion screw_torque_clamped_to_tool_grasp_capacity 2>&1 | grep -E "test .* (ok|FAILED)|test result" | head`
Expected: both FAIL — `screw_lowers_torque_and_completion` fails on the `advance_per_turn` assertion (still emits `thread_pitch`), and `screw_torque_clamped_to_tool_grasp_capacity` fails on the `0.2 N·m` (still emits `2 N·m`, no clamp).

- [ ] **Step 3: Pass the context to the screw arm** — in `crates/rfl-core/src/translation.rs` `lower`, change the screw arm:

```rust
        Primitive::ForceScrew(p) => (lower_force_screw(p, e, ctx), "screw"),
```

- [ ] **Step 4: Rewrite `lower_force_screw`** in `crates/rfl-core/src/translation.rs`:

```rust
/// Lower `force.screw`: a tool-mediated screw's reaction torque loads the held tool's
/// grasp (GF4c), so the torque budget is clamped to the grasp's rotational holding
/// capacity (`grasp_force::reaction_torque_limit`) when a tool is held; otherwise it
/// passes through. The `ScrewStop` completion lowers into a monitor; `thread_pitch`
/// is expanded into a structured `force_profile.coupling` (the linked DOF). The
/// runtime decoupling-as-failure detection is a driver concern (deferred).
fn lower_force_screw(p: &ForceScrew, e: &Embodiment, ctx: &GraspContext) -> CanonicalAction {
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
    // GF4c reaction-torque clamp: tool-mediated + a tool held -> bound the torque to
    // the tool grasp's rotational capacity (or the driver spins in-grasp).
    let tool_mediated = matches!(&p.tool_mediated, Some(v) if v.as_bool() != Some(false));
    let held = if tool_mediated { ctx.held.as_ref() } else { None };
    let torque = match (held, p.torque_budget.parse(), e.scalar_limit("grip_force_max").and_then(|q| q.parse())) {
        (Some(h), Some((tb, tu)), Some((gm, _))) => {
            Quantity::from_si(grasp_force::reaction_torque_limit(tb, gm, h.mode), tu)
        }
        _ => p.torque_budget.clone(),
    };
    let mut fp = serde_json::json!({ "torque": torque.0.clone() });
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

- [ ] **Step 5: Run the rfl-core tests + warning check**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | grep -E "warning:|test result|FAILED|error\[" | head`
Expected: all `ok` (the updated + new screw tests pass; the bare-screw test keeps `2 N·m`); no warnings.

- [ ] **Step 6: Regenerate + eyeball the screw goldens**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test screw_fasten 2>&1 | grep -E "golden.* (ok|FAILED)|test result" | head`
Expected: the 3 `golden_screw_*` FAIL (torque clamped + `thread_pitch`→`coupling`).

Run: `export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance --test screw_fasten >/dev/null 2>&1; rm -f crates/rfl-conformance/tests/snapshots/screw_fasten__*.snap.new; git diff -- crates/rfl-conformance/tests/snapshots/ | grep -E "^[+-].*(torque|coupling|advance_per_turn|thread_pitch)" | head`
Expected diff: each hand's screw line changes `"thread_pitch":"0.8 mm"` → `"coupling":{"advance_per_turn":"0.8 mm"}` and `"torque":"2 N·m"` → `"0.2 N·m"` (allegro) / `"0.15 N·m"` (leap) / `"0.12 N·m"` (pneumatic). Nothing else changes.

- [ ] **Step 7: Re-run to confirm green**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test screw_fasten 2>&1 | grep "test result"`
Expected: `test result: ok. 5 passed` (the boon + determinism tests confirm the clamped torque + coupling still validate against the schema).

- [ ] **Step 8: Gating validation batch** (separate from the commit)

Run: `cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1 && export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "test result|FAILED|error|warning:" | tail -12`
Expected: validate.py `PASS`; every crate green (rfl-core 53 + conformance lib + driver_protocol 7 + envelope_conformance 4 + retarget_determinism 5 + surface_scan 5 + screw_fasten 5); no warnings.

- [ ] **Step 9: Commit** (separate batch — after reading PASS)

```bash
cd ~/Documents/GitHub/rfl && git add crates/rfl-core/src/translation.rs crates/rfl-conformance/tests/snapshots/screw_fasten__screw_allegro.snap crates/rfl-conformance/tests/snapshots/screw_fasten__screw_leap.snap crates/rfl-conformance/tests/snapshots/screw_fasten__screw_pneumatic.snap && git commit -m "feat(core): clamp force.screw torque to tool-grasp capacity + expand coupling (GF4c)

A tool-mediated force.screw's reaction torque loads the held driver's grasp; the
2 N·m budget clamps to grip_force_max*R_GRIP/k_reaction (0.2/0.15/0.12 N·m per
hand). thread_pitch expanded into force_profile.coupling.advance_per_turn.
Completes the grasp-force arc GF1c-GF4c. Goldens regenerated.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Full verification + push + memory

**Files:** none.

- [ ] **Step 1: Final full suite + validate.py**

Run: `cd ~/Documents/GitHub/rfl && export PATH="$HOME/.cargo/bin:$PATH" && cargo test 2>&1 | grep -E "test result|FAILED|error" && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: all green; validate.py `PASS`.

- [ ] **Step 2: Confirm pushes** (Tasks 1-2 each pushed per the standing rules)

Run: `cd ~/Documents/GitHub/rfl && git rev-parse --abbrev-ref HEAD && git fetch -q origin && git merge-base --is-ancestor origin/main HEAD && git push -q origin main; git rev-list --left-right --count origin/main...HEAD && git log --oneline -4`
Expected: branch `main`; final `0 0`; the 2 E2 commits + the design-doc commit visible.

- [ ] **Step 3: Update the memory** (`~/.claude/.../memory/project_rfl.md` "## Implementation track") with the real commit hashes from `git log --oneline -5`, recording this as the 7th increment (force.screw E2 — GF4c tool-mediated reaction-torque + coupling), and note that the grasp-force arc GF1c–GF4c is now complete; remaining for the screw line = the torque-trajectory checker (Class-3) + decoupling detection + force.unscrew. Update the `MEMORY.md` RFL index line (7 increments, new HEAD). Not a repo commit — memory only.

---

## Self-review (completed during planning)

- **Spec coverage:** design §3 `reaction_torque_limit` → Task 1; §4 lowering clamp + coupling → Task 2; §5 per-hand demo → Task 2 (the new test + goldens); §6 conformance → Task 2 (Steps 6-8); §2 deferrals (torque checker, decoupling detection, unscrew) → out of scope, no task. No gaps.
- **Placeholder scan:** every code step shows complete code; commands show expected output. Step 1 of Task 2 gives the exact assertion replacement.
- **Type consistency:** `R_GRIP` + `reaction_torque_limit(torque_budget_nm, grip_force_max_n, mode)` defined in Task 1 are called identically in Task 2's `lower_force_screw`; `GraspContext`/`ctx.held`/`held.mode` match the existing `translation.rs` definitions; `Quantity::from_si` + `Quantity::parse` are the existing helpers. `lower_force_screw`'s new signature `(p, e, ctx)` matches the dispatcher arm.
- **Determinism:** the clamped torque emits via `round6` + `from_si` (byte-stable); `thread_pitch`/`coupling` carried as the authored string; no wall-clock/RNG — golden + generate-twice covered.
- **Backward-compat:** the bare-`force.screw` unit test (no preceding grasp → `ctx.held` None → no clamp) keeps `2 N·m`; only its `thread_pitch`→`coupling` assertion changes.
