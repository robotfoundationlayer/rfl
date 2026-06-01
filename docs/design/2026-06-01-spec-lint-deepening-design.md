# Deepen the spec-lint CI job: a markdown doc linter

Status: design-complete (2026-06-01)

## Problem

The `spec-lint` CI job checks only that the seven spec chapters *exist*. A
broken internal link (a renamed file, a typo'd anchor), an accidental bare `---`
thematic break (which breaks Ulysses HTML/PDF export, the vault convention the
repo inherits), or a JP double em-dash `——` (the top AI-generated prose tell)
all land on `main` green.

## Decision

Add `scripts/md_lint.py` — a **pure-stdlib** (no pip deps, runs on the runner's
`python3`) markdown linter — and run it from the `spec-lint` job. It lints every
tracked `*.md` **except** `docs/design/**`, `docs/plans/**`, and `whitepaper/**`
(internal design records, gitignored plans, and the other-owned whitepaper
source with its own export-time de-AI pipeline).

Three checks, each chosen to be correct on the *current* corpus (audited first,
so the new gate is green on landing):

1. **De-AI / `——`**: no JP double em-dash in outward-facing prose. A *single*
   `—` is legitimate English punctuation and pervasive (1369 uses in `spec/`),
   so it is **not** banned — only the doubled form, which should never appear in
   English technical docs. Inline-code spans (`` `...` ``) and fenced code blocks
   are stripped before checking so a doc may still *quote* `——` as a literal.

2. **Divider convention**: no bare `^---$` thematic break (exactly three
   hyphens). The repo uses `----` (four). No file carries YAML frontmatter, so a
   `---` line is unambiguously a thematic break. (Audit: zero current
   violations.)

3. **Internal-link integrity**: for every `[text](target)` whose target is a
   local relative path (no `http(s)`/`mailto`/`tel` scheme), the file part must
   resolve relative to the linking file, and a `#anchor` (same-file or
   cross-file) must match a heading slug in the target (GitHub-style slugger:
   lowercase, drop punctuation, spaces→hyphens, de-duplicate). This is the
   "heading / cross-reference integrity" check.

## Why a script, not inline bash

Link resolution + anchor slugging + code-span stripping is too much for a
readable `run:` block. A committed script is testable on its own and reusable
locally (`python3 scripts/md_lint.py`). It exits non-zero with a per-violation
report.

## Iteration contract

The linter is run against the live corpus during development; every flag is
triaged as either a real broken link (fix the doc) or a linter false positive
(fix the linter) until green. The audit found the corpus already clean for
checks 1–2; check 3 is the discovery surface.
