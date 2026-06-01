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

`cargo clippy --fix --workspace --all-targets --all-features` auto-applies the machine-applicable lints (`redundant_closure_for_method_calls` → method reference, …). **(Reality vs the original assumption: `--fix` resolved the `float_cmp` items the CI log first showed; what remained was dominated by `doc_markdown`.)** The remainder (confirmed via `cargo clippy … -- -D warnings`) is resolved by category:

- **Genuinely actionable — fixed by hand:** `map_unwrap_or` → `map_or` (2 sites), `unnecessary_map_or` → `is_none_or` (1), and a missing `# Panics` doc section on `canonical::to_jsonl` (1).
- **`clippy::pedantic` false positives — curated with targeted `#[allow]` + justification** (the idiomatic use of the pedantic group: these fire on this codebase's deliberate conventions, not real defects, and every *other* pedantic lint stays active):
  - crate-level in `rfl-core/src/lib.rs`: `doc_markdown` (docs reference spec sections like `spec/02`, invariant tags `RD1c`/`GF1c`/`TM21c`, and primitive names `force.insert_fit` in prose), plus `cast_precision_loss` / `cast_possible_truncation` / `cast_sign_loss` / `many_single_char_names` (bounded non-negative geometry / grasp-force math with single-letter variables);
  - test-module-scoped in the 5 affected files (`sigma`, `canonical`, `grasp_force`, `translation`, `region`): `float_cmp` + `unreadable_literal` on deterministic golden-value assertions — exact comparison is intended, and production `rfl-core` does no float equality (the lib built clean), so the allow stays confined to tests;
  - one targeted allow on the `Statement` AST enum: `large_enum_variant` (a parse-once node; boxing would obscure the `serde(untagged)` shape for negligible gain).

Iterate `cargo clippy … -- -D warnings` until clean, then re-run `cargo fmt --all -- --check` (the `--fix` pass can re-drift formatting) and `cargo test --workspace`. One commit.

## 3. Verify CI actually goes green

Local clean is necessary but not sufficient — the success criterion is the CI jobs. After pushing, `gh run watch` (or poll `gh run list`) the new run and confirm the `rustfmt` and `clippy` jobs pass (alongside the already-green `test` job).

## 4. Out of scope (YAGNI)

Modifying the CI config (dropping `-D warnings`, loosening `fmt`) or *wholesale* disabling `#![warn(clippy::pedantic)]` — that would "fix" CI by lowering the bar. **Per-lint curation** of `clippy::pedantic` with justification (§ 2) is not that: it is the documented, intended use of the pedantic group (clippy itself recommends `#[allow]`-ing the lints a project disagrees with), and every other pedantic lint stays active. The `actions/checkout@v4` Node.js 20 deprecation notice (a warning, not a failure). Any non-formatting, non-lint refactor of `rfl-core`.
