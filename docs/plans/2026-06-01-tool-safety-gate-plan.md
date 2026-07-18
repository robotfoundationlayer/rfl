# tool_safety gate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement
> this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> **LOCAL-ONLY** — never `git add` this file.

**Goal:** Enforce force.cut's hazardous-tool capability (spec/01 §6.7): a cut requires the
embodiment declare `capabilities.aux.tool_safety` (hazard_class), today unenforced.

**Architecture:** Pure rfl-core gate. `Aux` gains an optional `tool_safety` (parsed); the
`ForceCut` capability gate becomes conjunctive (require `force.cut` AND `tool_safety`); the 3
example descriptors declare `tool_safety` so the existing cut suite stays green. No schema /
golden / conformance-check change. Design: `docs/design/2026-06-01-tool-safety-gate-design.md`.

**Tech Stack:** Rust (rfl-core), `schemas/validate.py`.

**Discipline (every task):** TDD red→green. After green, run `cargo test` (workspace, NOT `-p`)
+ `uv run --with jsonschema --with pyyaml python schemas/validate.py` in a batch SEPARATE from
the commit. Before each push: `git fetch -q && git merge-base --is-ancestor origin/main HEAD`,
`git add <explicit paths>`, commit, `git push origin main`, verify sync `0 0` + `git show --stat`.

---

### Task 1: the conjunctive `ForceCut` gate + `tool_safety` capability

**Files:**
- Modify: `crates/rfl-core/src/embodiment.rs` (Aux struct + ToolSafety + has_tool_safety)
- Modify: `crates/rfl-core/src/translation.rs` (ForceCut gate arm; new test)
- Modify: `examples/03-screw-fasten/embodiments/{allegro,leap,pneumatic-6f}.yaml` (aux.tool_safety)

- [ ] **Step 1: Write the failing test** — in `translation.rs` tests module, near
`cut_capability_absent_when_not_declared`:

```rust
    const EMB_CUT_NO_TOOL_SAFETY: &str =
        "embodiment:\n  id: test-hand\n  capabilities:\n    skills: [force.cut]\n";

    #[test]
    fn cut_requires_tool_safety_capability() {
        let skill = Skill::parse_yaml(CUT_SKILL).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(EMB_CUT_NO_TOOL_SAFETY).unwrap();
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: tool_safety"), "got {err}");
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p rfl-core cut_requires_tool_safety 2>&1 | grep -E "error\[|panicked|capability_absent|test result|FAILED" | head`
Expected: FAIL — the gate currently passes `force.cut` (the embodiment declares it), so `retarget`
does not error on `tool_safety` (either it succeeds → `unwrap_err` panics, or it errors for a
different reason → the assert fails). Either way: red.

- [ ] **Step 3: Add the `ToolSafety` capability + parse it in `Aux`** — in
`crates/rfl-core/src/embodiment.rs`, in the `Aux` struct, after the `compliance` field:

```rust
    /// Hazardous-tool safety capability (`spec/03` § Tool-safety capability), required by
    /// `force.cut`. Presence is the gate; the hazard-class validation bench is a later increment.
    #[serde(default)]
    pub tool_safety: Option<ToolSafety>,
```

and add the struct after `Aux` (before `Frames`):

```rust
/// The hazardous-tool safety capability (`spec/03` § Tool-safety capability; `aux.tool_safety`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ToolSafety {
    /// The hazard managed (e.g. `cut` / `shear`).
    pub hazard_class: String,
    /// Reference to the deployment safety standard the capability claims to satisfy.
    #[serde(default)]
    pub standard: Option<String>,
}
```

- [ ] **Step 4: Add the `has_tool_safety` accessor** — in `embodiment.rs`, in the `impl
Embodiment` block, after `has_tactile_sensing` (the method that reads `aux.tactile_sensing`):

```rust
    /// Whether the embodiment declares `aux.tool_safety` (the hazardous-tool capability).
    #[must_use]
    pub fn has_tool_safety(&self) -> bool {
        self.capabilities.aux.tool_safety.is_some()
    }
```

- [ ] **Step 5: Make the `ForceCut` gate conjunctive** — in `crates/rfl-core/src/translation.rs`
`check_capability`, replace the arm:

```rust
        Primitive::ForceCut(_) => "force.cut",
```
with:
```rust
        // force.cut is hazardous: it requires BOTH the force.cut skill AND a declared
        // tool_safety capability (spec/01 § 6.7 precondition). A conjunctive gate.
        Primitive::ForceCut(_) => {
            return if !e.has_skill("force.cut") {
                Err(crate::Error::Translation("capability_absent: force.cut".to_string()))
            } else if !e.has_tool_safety() {
                Err(crate::Error::Translation("capability_absent: tool_safety".to_string()))
            } else {
                Ok(())
            };
        }
```

- [ ] **Step 6: Run the rejection test (green)**

Run: `cargo test -p rfl-core cut_requires_tool_safety 2>&1 | grep -E "test result"`
Expected: PASS (the inline embodiment has `force.cut` but no `tool_safety` → `capability_absent:
tool_safety`).

- [ ] **Step 7: Declare `tool_safety` on the 3 descriptors** — add `tool_safety: { hazard_class:
cut }` to each `capabilities.aux:` block. For allegro, after `compliance: active`:

```yaml
    aux:
      tactile_sensing: true        # distributed fingertip tactile → manifold-tier confirmation
      compliance: active
      tool_safety: { hazard_class: cut }   # hazardous-tool capability required by force.cut
```

For `leap` (same — after its `compliance: active`) and `pneumatic-6f` (after its
`compliance: passive` line), add the same `tool_safety: { hazard_class: cut }` line.

- [ ] **Step 8: Run the existing cut suite (still green — descriptors now declare tool_safety)**

Run: `cargo test -p rfl-core cut 2>&1 | grep -E "test result|cut_"`
Run: `cargo test -p rfl-conformance --test cut 2>&1 | grep -E "test result"`
Run: `cargo test -p rfl-conformance --test envelope_conformance cut 2>&1 | grep -E "test result"`
Expected: all PASS — `cut_lowers_irreversible_and_shear_budget`, `cut_capability_absent_when_not_declared`
(still `force.cut`, checked first), the `cut.rs` golden, and the envelope cut tests all green
(the 3 descriptors now declare `tool_safety`, so the conjunctive gate passes for them).

- [ ] **Step 9: Full suite (separate batch)**

Run: `cargo test 2>&1 | grep -E "test result:|error\[" | tail -20`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2`
Expected: green; `rfl-core` 84 (was 83, +1 cut_requires_tool_safety); conformance unchanged
counts; all goldens byte-identical (tool_safety is input, not in the retarget output). `validate.py`
PASS (the descriptors gain a valid `aux.tool_safety` object; not schema-validated by validate.py
anyway, and C1 is about the skills enum, unaffected).

- [ ] **Step 10: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add crates/rfl-core/src/embodiment.rs crates/rfl-core/src/translation.rs examples/03-screw-fasten/embodiments/allegro.yaml examples/03-screw-fasten/embodiments/leap.yaml examples/03-screw-fasten/embodiments/pneumatic-6f.yaml && \
git commit -m "feat(core): enforce force.cut tool_safety capability gate

force.cut is hazardous (spec/01 § 6.7): require the embodiment declare
capabilities.aux.tool_safety (hazard_class), not just force.cut. Parse
tool_safety in Aux + a conjunctive ForceCut gate (force.cut AND tool_safety;
force.cut checked first). The 3 screw-fasten descriptors declare tool_safety
so the existing cut suite stays green. The dummy-hand validation bench is
deferred. No schema / golden change (tool_safety is input, not output).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only the 5 listed files (embodiment.rs, translation.rs, 3 descriptors).

---

### Task 2: README status line

**Files:**
- Modify: `README.md` (~line 84)

- [ ] **Step 1: Extend the README status line** — note that `force.cut` now enforces the
hazardous-tool capability gate: a cut requires the embodiment declare `tool_safety` (hazard_class),
not just `force.cut` (the hazardous-operation precondition; the dummy-hand validation bench is a
later increment). Match the surrounding phrasing; keep the existing sentence structure. (spec/01
§6.7 / spec/05 already state the obligation, so no spec change.)

- [ ] **Step 2: Verify nothing regressed**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`
Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -1`
Expected: unchanged green.

- [ ] **Step 3: Commit + push**

```bash
git fetch -q && git merge-base --is-ancestor origin/main HEAD && \
git add README.md && \
git commit -m "docs(readme): note force.cut tool_safety capability gate

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>" && \
git push origin main && git rev-list --left-right --count HEAD...origin/main && git show --stat HEAD | head
```
Verify: sync `0 0`; only `README.md`.

---

## Post-increment

- Final full-suite read: expected `rfl-core` 84, conformance unchanged (lib 16, driver_protocol 9,
  envelope_conformance 44, the golden binaries unchanged), `validate.py` PASS.
- Update `project_rfl.md` "Implementation track" (22nd increment, real hashes + test deltas) +
  the `MEMORY.md` RFL line. Move deferred items (tool_safety validation bench,
  human_collaboration_safety, the standard field) into the parked list.

## Self-review (spec coverage)

- The hazardous-tool capability gate (design § 1, § 3) → Task 1 (conjunctive ForceCut gate). ✓
- tool_safety parsed in Embodiment (design § 4) → Task 1 (Aux.tool_safety + ToolSafety +
  has_tool_safety). ✓
- Descriptors declare tool_safety so the cut suite stays green (design § 5) → Task 1 Step 7. ✓
- README (design § 6) → Task 2. ✓
- No schema / golden / conformance-check change (design § 2) → confirmed: Task 1 touches only
  embodiment.rs / translation.rs / the 3 descriptors. ✓
- Type consistency: `Aux.tool_safety: Option<ToolSafety>`; `ToolSafety{hazard_class, standard}`;
  `Embodiment::has_tool_safety`; the `ForceCut` gate calls `has_skill("force.cut")` +
  `has_tool_safety()`. ✓
```
