# Design — green the CI (rustfmt + clippy)

**Date:** 2026-06-01
**Track:** repo health (CI)
**Status:** approved
**Scope:** whole workspace formatting (`cargo fmt --all`) + `rfl-core` clippy fixes. No behavior change; no spec / schema / golden / example change. The CI config is **not** modified.

## 0. Why this

`main`'s CI has been red on **every** commit this session — not the `test` job (green on ubuntu+macos), but two others:

- **`rustfmt`** (`cargo fmt --all -- --check`) — the entire repo (~50 files, all of `rfl-core` and the certify additions) is not `cargo fmt`-formatted.
- **`clippy`** (`cargo clippy --workspace --all-targets --all-features -- -D warnings`) — `rfl-core` has 109 lib + 132 lib-test lints under `-D warnings`: `map_unwrap_or`, `redundant_closure_for_method_calls`, and `float_cmp` (deterministic test assertions).

Local `cargo test --workspace` passes, which masked this (the `test` job is the only green one). A red CI on a public standard repo is a higher priority than any new feature.

## 1. rustfmt — `cargo fmt --all`

Apply the project's own declared format to the whole workspace. This is conformance to the project's CI rule (it runs `fmt --check`), not a refactor. `rustfmt` only changes whitespace and line-wrapping — never semantics, never string-literal contents — so the example fixtures, the byte-identical retarget goldens, and the insta snapshots are unaffected. After formatting, `cargo test --workspace` must stay green (it will; formatting is non-semantic). One commit: `style: cargo fmt --all`.

## 2. clippy — to green under `-D warnings`

`cargo clippy --fix --workspace --all-targets --all-features` auto-applies the machine-applicable lints (`map_unwrap_or` → `map_or`, `redundant_closure_for_method_calls` → method reference, and the other style/complexity fixes). Then resolve the remainder by category:

- **`float_cmp`** — the flagged sites are `assert_eq!` on `f64` arrays against exact literal golden values (e.g. `assert_eq!(poses[0].position, [0.1, 0.075, 0.1])`). The retarget pipeline is byte-deterministic, so exact comparison is *intended* — this is a `float_cmp` false positive in deterministic tests. Fix with a targeted `#[allow(clippy::float_cmp)]` on the affected test module(s), not a blanket crate allow. Any `float_cmp` in *production* code (if present) is reviewed individually — exact comparison kept only where the values are provably exact (e.g. comparing against `0.0` after a normalize), otherwise switched to an epsilon comparison.
- **Anything `--fix` could not resolve** — fixed by hand following clippy's own suggestion.

Iterate `cargo clippy --workspace --all-targets --all-features -- -D warnings` until it exits clean, then `cargo test --workspace` to confirm the `--fix` edits changed no behavior. One commit: `style(rfl-core): fix clippy pedantic + float_cmp drift` (scope may extend to the other crates if `--fix` touches them).

## 3. Verify CI actually goes green

Local clean is necessary but not sufficient — the success criterion is the CI jobs. After pushing, `gh run watch` (or poll `gh run list`) the new run and confirm the `rustfmt` and `clippy` jobs pass (alongside the already-green `test` job).

## 4. Out of scope (YAGNI)

Modifying the CI config — dropping `-D warnings`, relaxing `#![warn(clippy::pedantic)]`, or loosening `fmt` — would "fix" CI by lowering the bar; the project chose strict lints and we honor them. The `actions/checkout@v4` Node.js 20 deprecation notice (a warning, not a failure). Any non-formatting, non-lint refactor of `rfl-core`.
