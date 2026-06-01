# CI: enforce conformance test class 1 (schema validation)

Status: design-complete (2026-06-01)

## Problem

`schemas/validate.py` is the committed conformance-test-class-1 runner (schema
well-formedness + reference-instance validation + the C1–C7 cross-schema
anti-drift invariants). It is currently run only by hand. CI's `spec-lint` job
checks *file existence* of the seven spec chapters and nothing more, so a
schema regression — a malformed schema, a reference instance that drifts from
its schema, or a C1–C7 invariant break (e.g. the descriptor capability enum
drifting from `skill-isa`'s `PrimitiveId` set) — lands on `main` green. That is
the largest unguarded surface in the repo: the Rust workspace is fully gated,
the schemas are not.

## Decision

Add a dedicated `schema-validate` job to `.github/workflows/ci.yml` that runs
`validate.py` exactly the way the docs prescribe:

    uv run --with jsonschema --with pyyaml python schemas/validate.py

- **Toolchain**: install `uv` via `astral-sh/setup-uv` (the project's canonical
  Python runner per the global tool-preference rule; consistent with the
  already-third-party `dtolnay/rust-toolchain` + `Swatinem/rust-cache` actions).
  `uv run --with` supplies `jsonschema` + `pyyaml` ephemerally — no
  `requirements.txt`, no environment to maintain.
- **Separate job, not a step on `spec-lint`**: schema validation and the
  markdown spec-lint are independent concerns with independent toolchains
  (Python vs none). A separate job parallelizes and gives a clear red signal
  naming the failure.
- `validate.py` already exits non-zero on any failure, so no wrapper logic is
  needed.

## Non-goals

The per-skill ε-tolerance table (Class 2-loose) is data-dependent and out of
scope. This job enforces only Class 1, which is what `validate.py` covers.
