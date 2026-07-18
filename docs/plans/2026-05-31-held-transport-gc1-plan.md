# Held-Transport GC1 Propagation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A held `transport.move_to_pose` declares its static securing floor (`force_profile.min_holding_force`) so the grasp-continuity (GC1) checker verifies the maintained grip across the carry, and an under-secure-mid-carry driver is rejected.

**Architecture:** rfl-core `lower_transport_move_to_pose` emits `min_holding_force` from the `ctx.held` it already reads (Approach 2, the static counterpart of the GF2c `a_max` it already emits). The conformance `ReferenceDriver` echoes that floor into `securing_force`; the per-action GraspContinuity checker and the `UnderSecure` `FaultyDriver` are unchanged and now bite the transport.

**Tech Stack:** Rust (rfl-core / rfl-conformance), serde_json (Envelope.force_profile), insta (goldens), boon (schema), uv + jsonschema (validate.py).

**Process discipline (this repo):**
- Prefix every cargo command with `export PATH="$HOME/.cargo/bin:$PATH"`.
- Run validation (`cargo test` / `validate.py`) and read the result in a batch **physically separate** from the `git commit`.
- `git add` explicit paths only (never `-A`); never stage `docs/plans/`. Parallel session shares the tree — `git status` immediately before each stage.
- Per commit: branch == `main`; before push `git merge-base --is-ancestor origin/main HEAD`; after push `git rev-list --left-right --count origin/main...HEAD` == `0 0`.
- Conventional Commits + `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. Real `git log` hashes, never predicted.
- insta regen: `INSTA_UPDATE=always cargo test -p rfl-conformance --test <name>`, then `rm -f crates/rfl-conformance/tests/snapshots/*.snap.new`, then `git diff` to eyeball.

**Facts (from the repo + spec):** cable transport is action index 2 (`locate`0, `pinch`1, `transport`2, `locate`3, `align`4, `insert_fit`5, `release`6, `retract`7). Connector `estimated_mass 1.45 N`, pinch → `min_holding_force = 1.45 · 2.0 = 2.9 N`. Screw driver `estimated_mass 1.0 N` → `2.0 N`. `ctx.held` is set at pinch, cleared at release, so the floor lands on the held transport and not on release.

---

## Task 1: rfl-core — emit `min_holding_force` on the held transport

**Files:**
- Modify: `crates/rfl-core/src/translation.rs` (`lower_transport_move_to_pose` ~line 286; add one test in `mod tests`)
- Regenerate: `crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_{allegro,leap,pneumatic}.snap` and `screw_fasten__screw_{allegro,leap,pneumatic}.snap`

- [ ] **Step 1: Write the failing translation test**

Append to the `mod tests` block in `crates/rfl-core/src/translation.rs` (the block already has `load`, `retarget`, `Skill`, `Embodiment` in scope):

```rust
    #[test]
    fn transport_emits_min_holding_force_on_held_carry() {
        let (skill, emb) = load("allegro");
        let out = retarget(&skill, &emb).expect("retarget");
        // suffixes: locate, pinch, transport(2), locate, align, insert_fit, release, retract
        assert_eq!(out.suffixes[2], "transport");
        let fp = out.actions[2]
            .safety_envelope
            .force_profile
            .as_ref()
            .expect("held transport carries force_profile");
        // connector estimated_mass 1.45 N, pinch -> min_holding_force = 1.45 * 2.0 = 2.9 N
        assert_eq!(fp.get("min_holding_force").and_then(|v| v.as_str()), Some("2.9 N"));
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core transport_emits_min_holding_force_on_held_carry`
Expected: FAIL — panics `held transport carries force_profile` (the transport's `force_profile` is currently `None`).

- [ ] **Step 3: Emit the floor in `lower_transport_move_to_pose`**

In `crates/rfl-core/src/translation.rs`, the held block currently reads:

```rust
    let mut env = base_envelope(e);
    if let Some(held) = &ctx.held {
        let payload = e.scalar_limit(held.mode.payload_key()).and_then(|q| q.parse());
```

Insert the static-floor emit as the first statements inside the `if let`:

```rust
    let mut env = base_envelope(e);
    if let Some(held) = &ctx.held {
        // GC1 static floor: the held carry maintains min_holding_force (the static
        // counterpart of the dynamic a_max clamp below; spec/05 GC1 base continuity for
        // transport.move_to_pose, spec/02 § min_holding_force). Mirrors lower_grasp_pinch.
        let mhf = grasp_force::min_holding_force(held.weight_n, held.mode);
        env.force_profile =
            Some(serde_json::json!({ "min_holding_force": Quantity::from_si(mhf, "N").0 }));
        let payload = e.scalar_limit(held.mode.payload_key()).and_then(|q| q.parse());
```

(`grasp_force`, `Quantity`, and `serde_json::json!` are already used elsewhere in this file; `held.weight_n` / `held.mode` are already in scope here.)

- [ ] **Step 4: Run the test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core transport_emits_min_holding_force_on_held_carry`
Expected: PASS.

- [ ] **Step 5: Run the affected conformance goldens — they now MISMATCH (expected)**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test retarget_determinism --test screw_fasten 2>&1 | grep -E "golden_|test result:"`
Expected: the three `golden_*` (retarget) and three `screw_fasten` golden tests FAIL with a snapshot diff showing the `transport` execute line gained `"force_profile":{"min_holding_force":"2.9 N"}` (cable) / `"2 N"` (screw). The `generation_is_byte_identical` / boon tests still PASS.

- [ ] **Step 6: Regenerate and review the goldens**

Run:

```bash
export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance --test retarget_determinism --test screw_fasten
rm -f ~/Documents/GitHub/rfl/crates/rfl-conformance/tests/snapshots/*.snap.new
cd ~/Documents/GitHub/rfl && git diff --stat crates/rfl-conformance/tests/snapshots/ && echo "--- transport lines (cable=2.9 N, screw=2 N) ---" && grep -h "min_holding_force" crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_allegro.snap crates/rfl-conformance/tests/snapshots/screw_fasten__screw_allegro.snap
```

Expected: exactly 6 snapshots changed (`retarget_determinism__retarget_{allegro,leap,pneumatic}`, `screw_fasten__screw_{allegro,leap,pneumatic}`); the diff is the `transport` execute line gaining `force_profile.min_holding_force` (and the existing pinch line still has its own `min_holding_force`). Confirm the `driver_protocol` snapshots are NOT in the diff (the report side is untouched until Task 2):
`git diff --stat crates/rfl-conformance/tests/snapshots/driver_protocol__*.snap` → empty.

- [ ] **Step 7: Full validation batch (separate from the commit)**

Run:

```bash
export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | grep "test result:" | head -1 && cargo test -p rfl-conformance 2>&1 | grep -E "Running|test result:" && ls crates/rfl-conformance/tests/snapshots/*.snap.new 2>/dev/null && echo "STALE PENDING" || echo "no pending snapshots"
```

Expected: rfl-core all pass; every rfl-conformance suite passes (incl. the regenerated retarget_determinism + screw_fasten, and `envelope_conformance` still green because the driver is unchanged → transport securing_force still `None` → GC1 still vacuous-passes at this commit); no pending snapshots. Read and confirm before staging.

- [ ] **Step 8: Commit**

```bash
cd ~/Documents/GitHub/rfl && git fetch origin main --quiet && git rev-parse --abbrev-ref HEAD && git status --short
git add crates/rfl-core/src/translation.rs \
  crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_allegro.snap \
  crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_leap.snap \
  crates/rfl-conformance/tests/snapshots/retarget_determinism__retarget_pneumatic.snap \
  crates/rfl-conformance/tests/snapshots/screw_fasten__screw_allegro.snap \
  crates/rfl-conformance/tests/snapshots/screw_fasten__screw_leap.snap \
  crates/rfl-conformance/tests/snapshots/screw_fasten__screw_pneumatic.snap
git commit -F - <<'EOF'
feat(core): emit min_holding_force on the held transport (GC1 static floor)

A held transport.move_to_pose now declares force_profile.min_holding_force —
the static securing floor (spec/05 GC1 base continuity), the static counterpart
of the GF2c dynamic a_max already on motion_bounds (spec/02 § min_holding_force).
Read from the ctx.held the a_max code already uses; mirrors lower_grasp_pinch.
Cable carry = 2.9 N, screw-driver carry = 2 N. Regenerates the cable retarget
and screw-fasten goldens (transport line gains the field); the driver-protocol
report goldens are untouched.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git show --stat --oneline HEAD | head -9
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```

Expected: `git show --stat` lists exactly 7 files (translation.rs + 6 snapshots), no `driver_protocol`, no `docs/plans/`; push succeeds; final count `0 0`. Record the real hash.

---

## Task 2: conformance — `ReferenceDriver` maintains `securing_force` on the carry

**Files:**
- Modify: `crates/rfl-conformance/src/lib.rs` (`ReferenceDriver::execute`, the `securing_force` line ~line 80)
- Modify: `crates/rfl-conformance/tests/envelope_conformance.rs` (add two tests)
- Regenerate: `crates/rfl-conformance/tests/snapshots/driver_protocol__driver_{allegro,leap,pneumatic}.snap`

- [ ] **Step 1: Write the failing conformance tests**

Append to `crates/rfl-conformance/tests/envelope_conformance.rs` (after `never_settle_driver_fails_terminal_postcondition`, before `screw_dir`):

```rust
#[test]
fn nominal_transport_grasp_continuity_is_non_vacuous() {
    let dir = example_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // transport.move_to_pose (index 2) carries the propagated min_holding_force.
    let (goal, report) = &pairs[2];
    assert_eq!(suffix_of(&goal.action_id), "transport");
    // The carry must report a maintained securing_force (non-vacuous) that meets the floor.
    assert!(
        report.telemetry.iter().any(|t| t.securing_force.is_some()),
        "transport telemetry must carry securing_force"
    );
    assert_eq!(check_envelope(EnvelopeClass::GraspContinuity, goal, report), CheckOutcome::Pass);
}

#[test]
fn under_secure_driver_fails_transport_grasp_continuity() {
    let dir = example_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::UnderSecure),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // transport (index 2): the held carry's GC1 must reject the lowered securing_force.
    let (goal, report) = &pairs[2];
    assert_eq!(suffix_of(&goal.action_id), "transport");
    assert!(matches!(
        check_envelope(EnvelopeClass::GraspContinuity, goal, report),
        CheckOutcome::Fail(_)
    ));
}
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test envelope_conformance transport`
Expected: BOTH FAIL — the `ReferenceDriver` does not yet echo `securing_force` on the transport (no commanded budget), so `nominal_transport...` fails the `any(securing_force.is_some())` assert and `under_secure...` sees a vacuous Pass instead of Fail.

- [ ] **Step 3: Make `ReferenceDriver` fall back to the floor**

In `crates/rfl-conformance/src/lib.rs`, replace the line:

```rust
        let securing_force = ca.force_budget.clone();
```

with:

```rust
        // Echo the commanded grip budget, or — on a held carry with no commanded budget
        // (transport.move_to_pose) — the declared min_holding_force floor, representing the
        // grip maintained at its securing minimum (GC1 base continuity).
        let securing_force = ca.force_budget.clone().or_else(|| {
            ca.safety_envelope
                .force_profile
                .as_ref()
                .and_then(|fp| fp.get("min_holding_force"))
                .and_then(serde_json::Value::as_str)
                .map(|s| rfl_core::quantity::Quantity(s.to_string()))
        });
```

- [ ] **Step 4: Run the new tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test envelope_conformance transport`
Expected: BOTH PASS — the transport telemetry now carries `securing_force = "2.9 N"` (nominal: `2.9 ≥ 2.9` Pass; under-secure: `0.1 < 2.9` Fail).

- [ ] **Step 5: Driver-protocol goldens now MISMATCH (expected) — regenerate and review**

Run:

```bash
export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test driver_protocol 2>&1 | grep -E "golden_|test result:" || true
INSTA_UPDATE=always cargo test -p rfl-conformance --test driver_protocol
rm -f ~/Documents/GitHub/rfl/crates/rfl-conformance/tests/snapshots/*.snap.new
cd ~/Documents/GitHub/rfl && git diff crates/rfl-conformance/tests/snapshots/driver_protocol__driver_allegro.snap | grep -E "^\+|^-" | grep -i "securing_force" | head
```

Expected: the three `driver_protocol` goldens changed; the diff adds `"securing_force":"2.9 N"` to the `transport` telemetry line only (the pinch/insert_fit lines already had securing_force; `release` stays without). Confirm exactly 3 snapshots changed: `git diff --stat crates/rfl-conformance/tests/snapshots/driver_protocol__*.snap`.

- [ ] **Step 6: Full validation batch (separate from the commit)**

Run:

```bash
export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | grep "test result:" | head -1 && cargo test -p rfl-conformance 2>&1 | grep -E "Running|test result:" && cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -2 && echo "--- surface goldens unchanged? ---" && git diff --stat crates/rfl-conformance/tests/snapshots/surface_scan*.snap | tail -1 || echo "(surface unchanged)" && ls crates/rfl-conformance/tests/snapshots/*.snap.new 2>/dev/null && echo STALE || echo "no pending snapshots"
```

Expected: rfl-core all pass; every rfl-conformance suite passes (incl. `envelope_conformance` with the two new transport tests, and the regenerated `driver_protocol`); validate.py C1–C7 PASS; `surface_scan*` snapshots NOT changed (no transport); no pending snapshots. Read and confirm before staging.

- [ ] **Step 7: Commit**

```bash
cd ~/Documents/GitHub/rfl && git fetch origin main --quiet && git rev-parse --abbrev-ref HEAD && git status --short
git add crates/rfl-conformance/src/lib.rs \
  crates/rfl-conformance/tests/envelope_conformance.rs \
  crates/rfl-conformance/tests/snapshots/driver_protocol__driver_allegro.snap \
  crates/rfl-conformance/tests/snapshots/driver_protocol__driver_leap.snap \
  crates/rfl-conformance/tests/snapshots/driver_protocol__driver_pneumatic.snap
git commit -F - <<'EOF'
feat(conformance): ReferenceDriver maintains securing_force on the held carry

The driver now echoes the held transport's force_profile.min_holding_force into
securing_force when no grip budget is commanded — the grip maintained at its
securing minimum. The per-action GraspContinuity checker (unchanged) now verifies
the carry, and the UnderSecure FaultyDriver (unchanged) bites it: two new
envelope_conformance tests prove the transport GC1 is non-vacuous (nominal passes
with securing_force present) and rejects an under-secured carry (0.1 N < 2.9 N).
Regenerates the cable driver-protocol goldens (transport telemetry gains
securing_force).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git show --stat --oneline HEAD | head -8
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```

Expected: `git show --stat` lists exactly 5 files (lib.rs + envelope_conformance.rs + 3 driver_protocol snapshots), no `docs/plans/`; push succeeds; final count `0 0`. Record the real hash.

---

## Final verification gate

- [ ] Both feat commits on `main`, each pushed with post-push `0 0`.
- [ ] `cargo test -p rfl-core` green; `cargo test -p rfl-conformance` green (all 7 suites); `validate.py` C1–C7 EXIT 0.
- [ ] `surface_scan` / `surface_scan_spiral` goldens unchanged (no transport).
- [ ] No `schemas/` change; no new `Primitive`/enum variant; `force_profile.min_holding_force` rides the open Envelope floor (boon still passes).
- [ ] Update README (if status-bearing) / `project_rfl.md` Implementation track / `MEMORY.md` with the two real hashes.

## Self-review (run after writing, fix inline)

**Spec coverage** (design §2 in-scope):
- rfl-core emits `force_profile.min_holding_force` on the held transport → Task 1. ✓
- `ReferenceDriver` `securing_force` falls back to the floor; checker + FaultyDriver unchanged → Task 2. ✓
- translation test (carry emits floor) → Task 1 Step 1; conformance tests (nominal non-vacuous + UnderSecure bite) → Task 2 Step 1. ✓
- regenerated cable retarget + screw goldens (Task 1) + cable driver_protocol goldens (Task 2). ✓
- Deferred (full held-interval GC1, GC2/3/4/5/6, interval-invariant) → no task, intentionally out of scope per design §2. ✓

**Placeholder scan:** every code step shows full code; every run step gives an exact command + expected output; values concrete (2.9 N / 2 N / 0.1 N, index 2, snapshot names). No TBD/TODO. ✓

**Type consistency:** `grasp_force::min_holding_force(held.weight_n, held.mode)` matches the signature read from the tree; `Quantity::from_si(mhf, "N").0` matches `lower_grasp_pinch`; `ca.safety_envelope.force_profile` is `Option<serde_json::Value>` (read from canonical.rs); `securing_force` is `Option<Quantity>`; `check_envelope(EnvelopeClass::GraspContinuity, goal, report)` and `suffix_of` match `envelope_conformance.rs`. ✓

**Golden churn split:** Task 1 = retarget_determinism ×3 + screw_fasten ×3 (execute side), driver_protocol explicitly unchanged; Task 2 = driver_protocol ×3 (report side). No overlap. ✓
