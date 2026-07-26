# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

RFL is a **specification + reference implementation**: a neutral abstraction
layer between VLA foundation models and robotic embodiments.

| Need | Read |
|---|---|
| Project overview, status, milestones | [README.md](README.md) |
| How the code fits together (pipeline, per-file map, tests) | [docs/code-map.md](docs/code-map.md) |
| Per-increment discipline, goldens, CI, parallel sessions | [docs/development-workflow.md](docs/development-workflow.md) |
| CLI flags, exit codes, worked I/O | [docs/cli-reference.md](docs/cli-reference.md) |
| Contribution rules, commit format, SPDX | [CONTRIBUTING.md](CONTRIBUTING.md) / [GOVERNANCE.md](GOVERNANCE.md) |

## Layout

- `spec/00`–`06` — the specification chapters. Design-complete.
- `crates/` — `rfl-core` (Skill ISA + Translation Layer + lowering),
  `rfl-conformance` (battery, reference + adversarial drivers,
  certify/verify/badge/measure), `rfl-cli` (the `rfl` binary).
- `schemas/` — eight JSON Schemas + `validate.py`. The schema is the
  authoritative wire contract, and is stricter than `serde`.
- `examples/` — worked examples with per-embodiment retarget goldens.
- `bindings/` — `c` (cbindgen), `ros2` (`rfl_msgs` IDL), `python` (PyO3, outside
  the workspace).
- `docs/` — guides + `docs/design/` (decision records); `docs/plans/` is
  LOCAL-only, never staged. PDFs and source in `whitepaper/`.

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

Narrower loops while iterating:

```bash
cargo test -p rfl-conformance --test screw_fasten      # one integration test file
cargo test -p rfl-core translation::                    # unit tests by name filter
cargo test -p rfl-cli --test certify_cli
scripts/demo.sh                        # validate→retarget→certify→verify→sign, no hardware
scripts/provisional-epsilon-from-sim.sh # the hardware-free sim→measure ε sweep
```

Local cargo misses rustfmt / clippy / spec-lint / cargo-deny / MSRV — always
`gh run watch` after pushing.

## Working rules (detail in docs/development-workflow.md)

- Record design choices in `docs/design/` before implementing; never
  `git add docs/plans/`.
- TDD, one task at a time. A new enum variant lands with its `lower` / dispatch
  / match arm in the **same commit** (matches are exhaustive).
- Schema-check a new skill YAML *before* generating goldens.
- Re-bless goldens with `INSTA_UPDATE=always` (snapshots) and `RFL_BLESS=1`
  (certificates); delete stray `*.snap.new` before staging.
- Wire-format changes move goldens: prefer additive `#[serde(default)]` /
  `skip_serializing_if` and check the output is byte-identical.
- Stage explicit paths, never `git add -A`. A parallel whitepaper session owns
  `whitepaper/`, `spec/02-translation-layer.md`, sometimes `README.md`;
  `bindings/python` belongs to another session.
- Conventional Commits, subject ≤ 72 chars, imperative, English body, ending
  with: `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

## Honesty bar

The per-skill ε-tolerance values and the contact-geometry thresholds are
**data-dependent**: they must come from real measurement, not invented numbers.
The committed `null`/deferred state is correct until real traces exist — do not
fabricate conformance data. `rfl measure` ingests N driver-report traces into a
*provisional* ε table, and `rfl sim --seed --variation` generates varied traces
from a **declared** noise model (sim-derived, provisional, never promotable —
for ε only σ matters, so the candidate is purely a function of the declared σ).
Real hardware traces are required to promote actual ε values into the committed
table, which stays `null`; the live ROS 2 node awaits a ROS 2 environment.
