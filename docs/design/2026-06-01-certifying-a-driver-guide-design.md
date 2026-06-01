# Design — `docs/certifying-a-driver.md` (third-party adoption guide)

**Date:** 2026-06-01
**Track:** conformance-certify — increment 4 (adoption documentation)
**Status:** approved
**Scope:** one new file `docs/certifying-a-driver.md`; optionally one discoverability link in `README.md` (only if clean). **No code change** — pure documentation, grounded in the committed `rfl certify` / `rfl verify` tooling and the committed example fixtures.

## 0. Why this increment

Three certify increments shipped a feature-rich product (certify → verify → fidelity), but it has **zero adoption documentation** — a third party cannot use it without reading the source. For RFL's standard / conformance-regime play, the documentation *is* the adoption surface: a conformance regime nobody can implement against is nothing. This increment writes the on-ramp.

## 1. The verified re-verification recipe (the load-bearing fact)

The guide tells a third party how to independently re-verify a certificate's integrity. That recipe must be **provably correct**, not asserted. Confirmed against the committed `examples/01-cable-insertion/certificate.json` (`content_hash` `sha256:4c9ff53d…`):

- **Recipe:** drop `content_hash`, sort every object's keys recursively, compact-serialize (no whitespace, **no trailing newline**), sha256, prefix `sha256:`.
- **`jq` (newline-safe — the trailing newline `jq -c` adds is the trap):**
  ```
  printf '%s' "$(jq -S -c 'del(.content_hash)' certificate.json)" | shasum -a 256
  ```
  → reproduces `4c9ff53d…` (the bare `jq -S -c … | shasum` form is WRONG: `efab16…`, poisoned by the trailing newline).
- **Python:**
  ```python
  import json, hashlib
  d = json.load(open("certificate.json")); d.pop("content_hash")
  print("sha256:" + hashlib.sha256(json.dumps(d, sort_keys=True, separators=(",", ":")).encode()).hexdigest())
  ```
  → reproduces `4c9ff53d…`.

Both verified equal to the declared hash. The guide documents both with the newline caveat called out.

## 2. Structure (six sections)

1. **Overview** — the certificate = Class 3 driver-protocol self-certification; the three-step flow (capture a driver report → `rfl certify` → `rfl verify`); link to `spec/05-conformance.md` for the normative test-class definitions.
2. **The driver-report wire format** — what a vendor's driver must emit. Pointer to `schemas/driver-interface.schema.json` (the normative contract) — NOT re-documented (DRY) — plus 2–3 annotated real lines from `examples/01-cable-insertion/driver-report.jsonl` (a `telemetry` line and a `status` line), and the correlation rules: each line is a `telemetry` or `status` message (the `message` field), correlated by `action_id`, with exactly one terminal `status` per action.
3. **Running `rfl certify`** — the command with its three inputs; the human summary + `--out <cert.json>`; the exit codes **0** pass / **1** fail (certificate still emitted) / **2** invalid run (no certificate) — the ran-and-failed vs couldn't-run distinction. Shows real output from the committed example.
4. **Reading the certificate** — an annotated walk of a real `certificate.json`: `skill`/`embodiment`/`report_sha256` (id + content sha256 identity, never a path), `result`, `covered` / `excluded` (what it **claims** and **disclaims**), per-action `checks`, `fidelity_tier` (the spec/05 badge: how well a capability confirmed — `manifold` > `proxy` > `proxy_reactive`), `content_hash`. Uses the three committed reference certs to show manifold vs proxy vs a non-vacuous sequence check.
5. **`rfl verify` + the re-verification recipe** — the command + exit codes (**0** verified / **1** tampered / **2** malformed); then the § 1 verified recipe (jq + Python) so a steward / insurer / CI in any language can re-check the hash without the `rfl` binary.
6. **What this does NOT cover** — the honest boundary: Class 4 (physical / high-fidelity-sim end-to-end), Class 2-loose ε (data-dependent), ENV3 disturbance-rejection, the *physical truth* of a fidelity-tier claim (only self-report honesty is checked), and **signer identity** — there is no signature; `content_hash` is tamper-*evidence*, not authentication. "Who vouched for this" is the deferred signing / steward-tier work.

## 3. Discoverability

`docs/certifying-a-driver.md` is standalone. I will add a single link from `README.md` only if `git status` shows it clean (the `cobel` session that owns the README status narrative is now finished); the link goes in a natural spot (near the conformance / status mention). If README is dirty, skip the link and note it — never stage a file whose cadence another session owns.

## 4. Testing (the rigor for a docs increment)

Every command in the guide is run and its output confirmed against the committed fixtures:

- `rfl certify … examples/01-cable-insertion/{skill,embodiments/allegro}.yaml --report driver-report.jsonl` → the documented summary + `RESULT: PASS`.
- `rfl verify examples/01-cable-insertion/certificate.json` → `VERIFIED … exit 0`.
- A tamper demo → `TAMPERED … exit 1`; a malformed input → `exit 2`.
- **The § 1 recipe** (jq newline-safe + Python) reproduces the committed cert's `content_hash` byte-for-byte (already confirmed; re-run during execution as the doc's checkable invariant).
- Doc hygiene: every referenced path exists; no `----`/`---` divider issues (GitHub-rendered markdown); the wire-format example lines are copied verbatim from the committed report.

## 5. Out of scope (YAGNI)

A from-scratch driver-building tutorial; per-language SDK snippets beyond the prose + jq/Python recipe; flow diagrams (three linear commands need none); a certificate-format field reference beyond the annotated walk (the schema is the reference).
