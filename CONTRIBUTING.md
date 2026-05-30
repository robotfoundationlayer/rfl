# Contributing to RFL

Thank you for considering a contribution to the Robot Foundation Layer. This document covers how to get set up, the kinds of contributions we welcome, and the conventions we expect.

## Code of Conduct

All participation in this repository is governed by the [Code of Conduct](CODE_OF_CONDUCT.md) (Contributor Covenant v2.1). By participating you agree to uphold it.

## Project status (2026-05-30)

This repository is in the **pre-release scaffold phase**. The specification draft v0.1 is targeted for 2027 Q1. Code contributions to the reference implementation, conformance tests, and bindings are welcome once the spec text stabilizes (expected late 2026).

In the meantime, the most valuable contributions are:

1. **Spec design feedback** — open an Issue under "spec clarification" or "extension proposal"
2. **Use-case submissions** — describe a (VLA, embodiment) pair the current spec would not cleanly support
3. **Comparative-analysis additions** — bibliographic links to adjacent standards or platforms
4. **Translation or proof-reading** — the whitepaper exists in English and Japanese

## Development setup

```bash
# Prerequisites
rustup default stable
rustup component add rustfmt clippy

# Clone
git clone https://github.com/robotfoundationlayer/rfl.git
cd rfl

# Build
cargo build --workspace

# Test
cargo test --workspace

# Format + lint before pushing
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

## Coding conventions

### Rust

- Edition 2024 (pinned via `rust-toolchain.toml`)
- `rustfmt` default style with no custom rules
- `clippy` is treated as gating; warnings fail CI
- Public APIs require rustdoc comments with at least one usage example
- Errors use `thiserror` for definition and `anyhow` only at binary edges

### Python (bindings)

- Python 3.11+ baseline (3.12 recommended)
- Type hints required on all public APIs (`mypy --strict` passes)
- `ruff` for lint and format

### C (bindings)

- C17 minimum, `-Wall -Wextra -Wpedantic` clean
- Bindings generated via `cbindgen` from the Rust core; do not hand-edit generated headers

## Commit message conventions

We follow **Conventional Commits**:

```
<type>(<scope>): <short summary in imperative mood>

<body — explain why, not what; reference Issues or PRs>

<optional footer — BREAKING CHANGE / Co-Authored-By / etc>
```

Allowed types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `perf`, `ci`, `build`, `spec`.

The `spec:` type is RFL-specific and is used for changes to anything under `spec/` or `schemas/`. A `spec:` change requires explicit sign-off from a Maintainer before merge.

## Pull request process

1. Open or join an Issue to discuss the change before submitting code
2. Fork the repository and create a feature branch from `main`
3. Make the change with tests and documentation updates
4. Ensure CI (fmt, clippy, test, spec lint) passes locally
5. Open a PR with a clear description; link to the Issue
6. Address review comments; squash on merge is the default

## Spec-change discipline

The specification (`spec/`) is governed by the five constitutional principles in `spec/00-overview.md`. Any PR that modifies `spec/` must include:

- A "**Principle check**" section in the PR description, addressing each of the five principles
- A "**Compatibility statement**" — is this a v1.x backward-compatible addition, or does it require a major-version bump?
- A "**Cross-vendor evidence**" — at least one realistic example for each of three structurally-distinct embodiment classes (tendon-driven, direct-drive, pneumatic)

The compatibility statement is binding: once v1.0 ships, no breaking change can land in a v1.x release.

## Reporting security issues

If you discover a security vulnerability in RFL (in the spec, reference implementation, or conformance suite), please **do not** open a public Issue. Email <yo@vox.delivery> with the subject prefix `[security]` and we will respond within 72 hours.

## License of contributions

By submitting a contribution you agree that your contribution is licensed under the Apache License 2.0, the same license that covers this repository. See [LICENSE](LICENSE).
