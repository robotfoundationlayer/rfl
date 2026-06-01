# CLAUDE.md — working in the rfl repo

Guidance for AI coding sessions in this repository. RFL is a **specification +
reference implementation**: a neutral abstraction layer between VLA foundation
models and robotic embodiments. Read the [README](README.md) for the project
overview and [CONTRIBUTING.md](CONTRIBUTING.md) / [GOVERNANCE.md](GOVERNANCE.md)
for contribution and stewardship rules.

## Layout

- `spec/00`–`06` — the specification chapters (Markdown). Design-complete.
- `crates/` — the Rust workspace: `rfl-core` (Skill ISA parser + Translation
  Layer + lowering), `rfl-conformance` (the test-class battery, reference +
  adversarial drivers, certify/verify/badge/measure), `rfl-cli` (the `rfl`
  binary).
- `schemas/` — eight JSON Schemas + `validate.py` (conformance test class 1,
  anti-drift invariants C1–C10). The schema is the authoritative wire contract.
- `examples/` — worked examples (cable-insertion, surface-scan, screw-fasten)
  with per-embodiment retarget goldens.
- `bindings/` — `c` (cbindgen), `ros2` (the `rfl_msgs` IDL), `python` (PyO3).
- `docs/` — guides + `docs/design/` (per-increment design records). The PDFs and
  source live in `whitepaper/`.

## Gates — run all of them, read the real exit code

Prefix every cargo command with `export PATH="$HOME/.cargo/bin:$PATH"` (shell
state does not persist across tool calls). Never read an exit code through a
`| tail`/filtered pipe (`$?` picks up the tail). Use `cmd; echo $?` or
`${PIPESTATUS[0]}`.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets          # grep -c "test result: FAILED"
uv run --with jsonschema --with pyyaml python schemas/validate.py
python3 scripts/md_lint.py                     # when Markdown changed
```

CI runs eight jobs (the five above × platforms, plus `cargo-deny` and an MSRV
check on Rust 1.86). **Local cargo alone misses rustfmt / clippy / spec-lint /
cargo-deny / MSRV — always `gh run watch` after pushing.**

## Per-increment discipline

1. Brainstorm (superpowers) for anything with a design choice; record the
   decision in `docs/design/`. Detailed plans go in `docs/plans/` — **never
   `git add` `docs/plans/`** (LOCAL-only).
2. TDD: red → green, one task at a time. A new Rust enum variant lands with its
   `lower`/dispatch/match arm in the **same commit** (exhaustive match).
3. A new skill YAML gets a one-off `jsonschema` check **before** generating
   goldens — `serde` retarget is lenient, the schema is strict.
4. Goldens: insta `INSTA_UPDATE=always cargo test -p rfl-conformance --test
   <name>`; certs `RFL_BLESS=1 cargo test -p rfl-conformance --test
   certificate_fixtures`. Delete stray `*.snap.new` before staging. Adding a
   capability to a descriptor changes its sha256 → re-bless any cert embedding
   it; a new per-action battery check re-blesses every reference cert (one
   vacuous-pass line).
5. Wire-format note: `rfl-core` produces the canonical wire and now also parses
   it (`canonical::to_jsonl` / `from_jsonl`); changing a serialized form moves
   goldens, so prefer additive/`#[serde(default)]`/`skip_serializing_if` and
   verify the Serialize output is byte-identical.

## Commits, push, parallel sessions

- Conventional Commits; subject ≤ 72 chars, imperative, English body. End with:
  `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- Stage explicit files (`git add <path>`); never `git add -A`/`.`.
- Push fast-forward-safe: `git fetch origin main` → confirm `HEAD~N ==
  origin/main` → push, else rebase. Then `gh run watch` for CI green.
- A **parallel whitepaper session** edits `whitepaper/`, `spec/02-translation-layer.md`,
  and sometimes `README.md`; `bindings/python` belongs to another session. Do
  not stage their files. Before each commit:
  `git diff --cached --name-only | grep -E '^whitepaper/|^spec/02|bindings/python|friend-review'`
  (anchor patterns with `^` — a bare `whitepaper`/`README.md` substring
  false-positives on your own filenames).
- CI-config changes are pre-approved only for: validate.py integration,
  spec-lint depth, cargo-deny/MSRV, action bumps, least-privilege permissions.
- Gotchas seen in practice: a GitHub Action's floating major tag may not exist —
  pin the exact version (`setup-uv@v8.1.0`, `cargo-deny-action@v2.0.20`).
  `${{ env.* }}` is not a valid expression context in a job `name:` (it makes the
  workflow unparseable).

## Honesty bar

The per-skill ε-tolerance values and the contact-geometry thresholds are
**data-dependent**: they must come from real measurement, not invented numbers.
The committed `null`/deferred state is correct until real traces exist — do not
fabricate conformance data. The measurement harness (`rfl-conformance::measure`)
is ready to consume real multi-run traces; the live ROS 2 node awaits a ROS 2
environment.
