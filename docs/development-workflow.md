# Development workflow — per-increment discipline and CI

Maintainer-facing detail behind the short rules in
[CLAUDE.md](../CLAUDE.md). Public contribution rules (commit message format, PR
process, coding conventions, SPDX headers) live in
[CONTRIBUTING.md](../CONTRIBUTING.md).

## Per-increment discipline

1. **Design first.** Anything with a design choice gets a brainstorm and a
   decision record in [`docs/design/`](design/). Detailed plans go in
   `docs/plans/` — **never `git add` `docs/plans/`** (LOCAL-only, and excluded
   from the Markdown lint).
2. **TDD, one task at a time**: red → green. A new Rust enum variant lands with
   its `lower` / dispatch / match arm in the **same commit**, because the
   matches are exhaustive.
3. **Schema-check new skill YAML before generating goldens.** A one-off
   `jsonschema` run against `schemas/skill-isa.schema.json` catches what
   retargeting will not: `serde` is lenient, the schema is strict.
4. **Goldens.**
   - insta snapshots: `INSTA_UPDATE=always cargo test -p rfl-conformance --test <name>`
   - certificates: `RFL_BLESS=1 cargo test -p rfl-conformance --test certificate_fixtures`
   - Delete stray `*.snap.new` before staging.
   - Adding a capability to an embodiment descriptor changes its sha256, so
     re-bless every certificate that embeds it.
   - Adding a per-action battery check re-blesses **every** reference
     certificate (each gains one vacuous-pass line).
5. **Wire format.** `rfl-core` both produces and parses the canonical wire
   (`canonical::to_jsonl` / `from_jsonl`), so changing a serialized form moves
   goldens. Prefer additive fields with `#[serde(default)]` /
   `skip_serializing_if`, and verify the Serialize output is byte-identical.

## Push protocol

- Push fast-forward-safe: `git fetch origin main`, confirm `HEAD~N ==
  origin/main`, then push; otherwise rebase.
- Stage explicit paths (`git add <path>`); never `git add -A` or `git add .`.
- After pushing, `gh run watch` — local cargo alone misses rustfmt, clippy,
  spec-lint, cargo-deny, and the MSRV check.

## Parallel sessions — file ownership

A **whitepaper session** edits `whitepaper/`, `spec/02-translation-layer.md`,
and sometimes `README.md`. `bindings/python` belongs to another session. Do not
stage their files. Before each commit:

```bash
git diff --cached --name-only | grep -E '^whitepaper/|^spec/02|bindings/python|friend-review'
```

Anchor the patterns with `^`: a bare `whitepaper` or `README.md` substring
false-positives on your own filenames.

## CI

Eight jobs run on push and pull request against `main`:

| Job | What it runs |
|---|---|
| `fmt` | `cargo fmt --all -- --check` |
| `clippy` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| `test` (×2) | `cargo build` + `cargo test --workspace --all-targets` on Linux and macOS |
| `spec-lint` | spec chapter presence + `python3 scripts/md_lint.py` |
| `schema-validate` | `schemas/validate.py` (conformance test class 1, invariants C1–C11) |
| `cargo-deny` | licenses + advisories |
| `msrv` | `cargo check` on Rust 1.86 |

CI-config changes are pre-approved only for: `validate.py` integration,
spec-lint depth, cargo-deny / MSRV, action version bumps, and least-privilege
permissions.

Gotchas seen in practice:

- A GitHub Action's floating major tag may not exist — pin the exact version
  (`setup-uv@v8.1.0`, `cargo-deny-action@v2.0.20`).
- `${{ env.* }}` is not a valid expression context in a job `name:`; using it
  makes the whole workflow unparseable.
