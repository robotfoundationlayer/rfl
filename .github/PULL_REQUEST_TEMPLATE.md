<!--
Thanks for contributing to RFL. Keep PRs to one logical change; split unrelated
work. See CONTRIBUTING.md and GOVERNANCE.md.
-->

## What and why

<!-- What does this change, and why? Link the Issue / RFC it addresses. -->

Closes #

## Type of change

- [ ] Specification (`spec/*.md`) — note any constitutional-principle impact
- [ ] Reference implementation (`crates/*`)
- [ ] Schema (`schemas/*`) — ran `schemas/validate.py`
- [ ] Extension registry entry (`extensions/*`) — includes the Principle-3 artifact
- [ ] Documentation (`docs/*`, `README`, …)
- [ ] Tooling / CI

## Checklist

- [ ] Commits follow Conventional Commits; subject ≤ 72 chars, imperative
- [ ] `cargo fmt --all -- --check` clean
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean
- [ ] `cargo test --workspace` green
- [ ] `uv run --with jsonschema --with pyyaml python schemas/validate.py` green (if schemas/examples changed)
- [ ] `python3 scripts/md_lint.py` green (if Markdown changed)
- [ ] Docs / CHANGELOG updated as needed
- [ ] No breaking change to a stabilized contract (or it is called out + justified)

## Notes for reviewers

<!-- Anything that needs special attention: a normative construction, a golden
re-bless, a spec ↔ whitepaper convergence implication, etc. -->
