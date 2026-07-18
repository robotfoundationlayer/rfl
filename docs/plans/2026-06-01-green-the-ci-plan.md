# Green the CI (rustfmt + clippy) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the `rustfmt` and `clippy` CI jobs on `main` pass.

**Architecture:** Two mechanical commits — `cargo fmt --all` (whole-workspace format) and clippy-to-green (`--fix` + targeted `float_cmp` allows on deterministic test assertions), each gated on `cargo test --workspace` staying green; then verify the actual CI run goes green via `gh`.

**Tech Stack:** `cargo fmt`, `cargo clippy` (Rust 1.96 / stable), `gh` CLI.

**Design doc:** `docs/design/2026-06-01-green-the-ci-design.md` (committed `3c34288`).

**Discipline:** explicit `git add <paths>` (or `-u` for whole-tree formatting where every change is a known fmt edit — see Task 1); Conventional Commits ≤72; after each commit `git fetch origin -q && git rev-list --left-right --count origin/main...HEAD` = `0 0` before AND ff `git push origin main` after.

----

## Task 1: rustfmt — `cargo fmt --all`

**Files:** the whole workspace (all `crates/**/*.rs`) — formatting only.

- [ ] **Step 1: Confirm the repo is currently fmt-dirty**

Run: `cargo fmt --all -- --check >/dev/null 2>&1 ; echo "check exit=$?"`
Expected: `check exit=1` (dirty).

- [ ] **Step 2: Apply the format**

Run: `cargo fmt --all`
Then: `cargo fmt --all -- --check >/dev/null 2>&1 ; echo "check exit=$?"`
Expected: `check exit=0` (clean).

- [ ] **Step 3: Confirm no behavior changed**

Run: `cargo test --workspace 2>&1 | rg "test result:|FAILED|error\[" | rg -c "FAILED|error\[" || echo "0 failures"`
Expected: `0 failures` (formatting is non-semantic; all suites stay green).

- [ ] **Step 4: Confirm only .rs source files changed (no fixtures/goldens)**

Run: `git status --short | rg -v '\.rs$' | rg -v '^\?\? docs/plans/' || echo "only .rs files changed"`
Expected: `only .rs files changed` (no `examples/`, `schemas/`, or golden/data file touched by fmt).

- [ ] **Step 5: Stage all formatted .rs files + commit**

```bash
git add -u                       # stages only tracked modifications (all are fmt edits, verified in Step 4)
git status --short               # sanity: every staged path ends in .rs
git commit -m "style: cargo fmt --all

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

- [ ] **Step 6: ff-push**

```bash
git fetch origin -q && git rev-list --left-right --count origin/main...HEAD   # expect 0 1
git push origin main
git fetch origin -q && git rev-list --left-right --count origin/main...HEAD   # expect 0 0
```

----

## Task 2: clippy — to green under `-D warnings`

**Files:** primarily `crates/rfl-core/**` (the 109 lib + 132 lib-test lints); `--fix` may also touch the other crates.

- [ ] **Step 1: Auto-apply the machine-applicable lints**

Run: `cargo clippy --fix --workspace --all-targets --all-features --allow-dirty 2>&1 | tail -5`
(`--allow-dirty` is needed only if the tree is dirty; after Task 1 it is clean, so it may be omitted. `--fix` applies `map_unwrap_or` → `map_or`, `redundant_closure_for_method_calls` → method ref, and other machine-applicable fixes.)

- [ ] **Step 2: See what clippy still rejects under `-D warnings`**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | rg "^error: " | sed -E 's/.*error: //' | sort | uniq -c | sort -rn`
Expected: the remaining error categories. The dominant one will be `strict comparison of \`f32\` or \`f64\`` (`float_cmp`) on deterministic test assertions.

- [ ] **Step 3: Resolve `float_cmp` on deterministic test assertions**

For each `rfl-core` source file whose `#[cfg(test)] mod tests` contains the flagged `assert_eq!(... f64 array ...)` comparisons (the CI log named `crates/rfl-core/src/translation.rs`; Step 2 lists any others), add a module-scoped allow at the top of that test module:

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)] // deterministic retarget output: exact golden comparison is intended
    // ... existing test code ...
}
```

Justification: the retarget pipeline is byte-deterministic, so `assert_eq!(poses[0].position, [0.1, 0.075, 0.1])` compares exact, reproducible values — `float_cmp` is a false positive here. Confine the allow to the test module, not the crate.

If `float_cmp` fires in **non-test** (production) code, do NOT blanket-allow: read the comparison. Keep exact comparison only where a value is provably exact (e.g. `x == 0.0` right after normalizing); otherwise rewrite as `(a - b).abs() < f64::EPSILON` (or an appropriate tolerance) following clippy's own note.

- [ ] **Step 4: Resolve any remaining non-auto-fixable lints by hand**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | rg "^error" | head -20`
For each remaining error, apply clippy's suggested fix (shown in its `help:` line) at the named `--> file:line`. Re-run until the next step is clean.

- [ ] **Step 5: Confirm clippy is clean under the exact CI command**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -3`
Expected: `Finished` with no `error:` lines (exit 0).

- [ ] **Step 6: Confirm `--fix` changed no behavior**

Run: `cargo test --workspace 2>&1 | rg "test result:|FAILED|error\[" | rg -c "FAILED|error\[" || echo "0 failures"`
Expected: `0 failures`.

Also re-confirm formatting survived `--fix`: `cargo fmt --all -- --check >/dev/null 2>&1 ; echo "fmt exit=$?"` → `fmt exit=0` (if `--fix` introduced fmt drift, run `cargo fmt --all` again and include those edits in this commit).

- [ ] **Step 7: Stage + commit**

```bash
git add -u
git status --short                 # sanity: only .rs files
git commit -m "style(rfl-core): fix clippy pedantic + float_cmp drift

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

- [ ] **Step 8: ff-push**

```bash
git fetch origin -q && git rev-list --left-right --count origin/main...HEAD   # expect 0 1
git push origin main
git fetch origin -q && git rev-list --left-right --count origin/main...HEAD   # expect 0 0
```

----

## Task 3: Verify the CI actually goes green (READ)

- [ ] **Step 1: Watch the new run for the push**

Run: `gh run list --limit 1` to get the latest run id for the `style(rfl-core)` push, then:
`gh run watch <id> --exit-status ; echo "ci exit=$?"`
Expected: `ci exit=0` — all jobs (spec lint, rustfmt, clippy, test ubuntu, test macos) pass.

- [ ] **Step 2: Confirm the previously-red jobs are green**

Run: `gh run view <id> 2>&1 | rg -i "rustfmt|clippy|test"`
Expected: `✓ rustfmt`, `✓ clippy`, `✓ test (ubuntu-latest)`, `✓ test (macos-latest)`.

----

## Self-Review

**Spec coverage** (design doc → tasks):
- rustfmt whole-workspace → Task 1. ✓
- clippy `--fix` + float_cmp targeted allow + iterate-to-green → Task 2. ✓
- verify CI jobs go green via gh → Task 3. ✓
- CI config untouched; goldens/fixtures untouched (Task 1 Step 4 asserts only `.rs` changed). ✓

**Placeholder scan:** the lint-resolution steps are inherently iterative (can't pre-enumerate 132 sites), but each step gives the exact command to discover the remaining set and the concrete fix pattern (module `#![allow(clippy::float_cmp)]` for deterministic tests; clippy's own `help:` for the rest). No `TODO`/`TBD`. ✓

**Safety invariants:** (1) `cargo test --workspace` gated after both the fmt commit and the clippy commit; (2) Task 1 Step 4 proves fmt touched no data files; (3) `git add -u` stages only tracked modifications (no stray new files), with a `git status --short` sanity check that every staged path is `.rs`. ✓
