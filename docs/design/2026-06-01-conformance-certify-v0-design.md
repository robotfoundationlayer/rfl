# Design — `rfl certify`: third-party self-certification (JSONL-replay), v0

**Date:** 2026-06-01
**Track:** conformance-certify (new) — v0
**Status:** approved
**Scope:** `rfl-conformance` (new `replay` / `battery` / `certificate` / `certify` modules) +
`rfl-cli` (new `certify` subcommand, new dep edge on `rfl-conformance`). Promote `boon` from
dev-dep to dep; add `sha2`. **No `rfl-core` change** (wire types stay `Serialize`-only), **no
schema change**, **no spec change**, **no change to any existing `check_*` / driver / golden**.

## 0. Why this track

The conformance suite has four envelope-class checkers, a dozen adversarial drivers, and 223 cargo
+ validate.py green. That proves *we hold the tests*. It does not yet let a vendor get a verdict on
*their own* driver. This track turns "we have tests" into "you can certify": a vendor runs their
driver (any language), captures the `telemetry`+`status` it emits as JSONL, and runs one command to
get a structured conformance report plus a deterministic, hashable certificate. That is the
adoption-infrastructure pivot — the move from a test artifact we own to a certification product a
third party uses.

The key enabling fact is already true in the code: every `check_*` is a **pure function of
`(ExecuteGoal, DriverReport)`** (or a slice of pairs for `check_momentary_release`). It does not
care whether the report came from an in-process `Driver::execute` or was replayed from a file. So
`certify` is a thin ingestion + orchestration layer over the existing, unchanged checkers — not a
re-implementation.

## 1. The four locked decisions

1. **Input contract** — three explicit files. The vendor runs their driver out-of-band, captures
   `telemetry`+`status` lines as JSONL, and passes
   `--skill <s> --embodiment <e> --report <j>`. Transport-free, language-neutral. The goal /
   contract side is **not** supplied by the vendor — the tool derives it itself via `retarget`, so
   the vendor cannot forge what they are being judged against.
2. **Placement** — a new `rfl certify` subcommand (product verb), separate from the internal
   `rfl conformance --driver` stub (left untouched for the future live path) and the `cargo test`
   suite. Mirrors the thin `retarget` CLI handler.
3. **Certificate scope** — **Class 3 driver-protocol only** (schema-valid wire + action_id
   correlation + per-action envelope-class conformance + audit obligations). Every certificate
   explicitly disclaims what it does not cover (§ 7).
4. **Certificate artifact** — deterministic canonical JSON identified by RFL id + content sha256;
   signing is out-of-band (detached-sign the canonical bytes). No key management in v0.

## 2. CLI surface (`rfl-cli`)

```
rfl certify --skill <skill.yaml> --embodiment <emb.yaml> --report <driver.jsonl> [--out <certificate.json>]
```

- Prints a human PASS/FAIL summary + per-action table to stdout.
- `--out` writes the canonical certificate JSON to a file (omitted → summary only; the certificate
  is still computed so the exit code is meaningful).
- **Exit codes carry the run's meaning** (the load-bearing "ran-and-failed vs couldn't-run" split):
  - `0` — valid run, every action conforms.
  - `1` — valid run, ≥1 non-conformance. A **FAIL certificate is still emitted** (the vendor needs
    the report of what failed).
  - `2` — run **invalid**: unparseable skill/embodiment, malformed or schema-invalid JSONL line, or
    broken correlation. **No certificate**; a line-numbered error goes to stderr.
- New dependency edge `rfl-cli → rfl-conformance` (today rfl-cli is core-only). The handler is thin:
  parse args, call `rfl_conformance::certify::run(...)`, render, map result → exit code. The stubbed
  `Conformance { driver }` arm is unchanged.

## 3. Unit decomposition (all new code in `rfl-conformance`)

Each unit has one purpose, a typed interface, and is testable in isolation.

### 3.a `replay` — ingest the vendor JSONL

Interface: `replay_report(jsonl: &str) -> anyhow::Result<Vec<(String, DriverReport)>>`
(action_id → reconstructed report, in first-seen order).

1. **Parse** each non-blank line as JSON; an error carries the 1-based line number.
2. **Schema-validate** each line against `schemas/driver-interface.schema.json` via `boon` — the
   authoritative Class 3 wire-compliance leg. This enforces what serde alone cannot: the `Force` /
   `Length` / `Duration` regex patterns, the `outcome` / `disposition` enums, and
   `additionalProperties: false`. The compiled schema is built once per run.
3. **Deserialize** into strict input mirror structs (`TelemetryIn` / `StatusIn`,
   `#[serde(deny_unknown_fields)]`), with the `message` discriminant checked (`telemetry` /
   `status`; the three non-driver→RFL message kinds — `execute` / clearance — are rejected from a
   report stream). Convert into the canonical `rfl_core::driver::{Telemetry, Status}` (setting the
   `message: &'static str` literal during conversion — see § 4 for why this lives here, not in
   rfl-core).
4. **Correlate**: group by `action_id`. Exactly one `status` per action; its telemetry samples
   attach to it. Extra status, missing status, or a telemetry line for an action with no status →
   invalid-run error. (Whether *every expected* action_id is present is checked in § 3.d against the
   retarget output, since `replay` alone does not know the expected set.)

The dedicated `*In` structs are not redundant duplication — they *are* the wire contract a third
party must conform to, and the right place to make that contract strict.

### 3.b `battery` — run every obligation, structured verdict

Pure functions over the canonical types.

- Per action: `verify_action(goal: &ExecuteGoal, report: &DriverReport) -> ActionVerdict`.
  Runs the full applicable battery; each existing checker is already vacuous-pass when its contract
  is absent, so the battery simply runs them all:
  - `check_envelope(envelope_class_for(suffix), goal, report)` when the suffix maps to a class
    (`sense.*` → no envelope, recorded as `perception / no envelope`);
  - `check_actuation`, `check_engagement`, `check_irreversible`, `check_freed_part_disposition`,
    `check_audit_honesty`.
- Sequence-level: `check_momentary_release(&pairs)`.
- **Deliberately excluded**: `check_graceful_degradation` and `check_settling`. Both verify the
  failure *shape* under a bench-**injected** over-budget disturbance / impulse (ENV3). A vendor's
  nominal self-run cannot exercise them — injection is a property of an active bench, not of replay.
  This is exactly the `env3` disclaimer on the certificate, kept honest by construction.

```
ActionVerdict {
    action_id: String,
    suffix: String,
    envelope_class: Option<EnvelopeClass>,
    checks: Vec<(/* name */ &'static str, CheckOutcome)>,
    passed: bool,   // all checks Pass
}
```

### 3.c `certificate` — the signable artifact

Pure given the three file byte-strings + the verdicts.

- Inputs are identified by **RFL id + content sha256, never by filesystem path** — the cert is
  machine-independent (the point of "canonical"). Skill id from the parsed skill, embodiment id from
  the parsed descriptor.
- Deterministic serialization: fixed-field-order `Serialize` structs (serde_json emits fields in
  declaration order); no floats anywhere in the cert, so the bytes are reproducible run-to-run and
  machine-to-machine. `content_hash` = `sha256` over the cert body serialized *without*
  `content_hash`, then attached.

```json
{
  "certificate_schema_version": "0.1",
  "spec_version": "0.1",
  "tool_version": "0.0.1",
  "skill": { "id": "cable-insertion", "sha256": "..." },
  "embodiment": { "id": "allegro", "sha256": "..." },
  "report_sha256": "...",
  "result": "pass",
  "covered": ["class3_driver_protocol"],
  "excluded": ["class4_physical", "class2_loose_epsilon", "env3_disturbance", "fidelity_tier_physical_truth"],
  "actions": [
    { "action_id": "cable-insertion/allegro/0002-pinch", "suffix": "pinch",
      "envelope_class": "grasp_continuity",
      "checks": [ { "name": "envelope", "result": "pass" }, { "name": "audit_honesty", "result": "pass" } ],
      "passed": true }
  ],
  "content_hash": "sha256:..."
}
```

### 3.d `certify` — orchestration

`run(skill_path, embodiment_path, report_path) -> anyhow::Result<CertifyOutcome>`, where
`CertifyOutcome { certificate, result: Pass | Fail }` and the invalid-run cases surface as `Err`.

1. Read + `sha256` each of the three files (bytes).
2. `retarget(skill, embodiment)` → the ordered canonical goals (this is also the implicit Class 1
   parse + Class 2 generation *precondition* — if the skill cannot be parsed/retargeted, the run is
   invalid; the certificate claims only Class 3, but the run cannot proceed without a valid goal
   set). Reuses the same id format `drive()` uses: `{skill}/{embodiment_id}/{NNNN}-{suffix}`.
3. `replay_report(report_jsonl)` → action_id → report.
4. **Expected-set correlation**: every retarget goal has a matching replayed report and vice-versa
   (no missing, no orphan). Mismatch → invalid run.
5. Pair `(goal, report)` in retarget order; `battery::verify_action` each; `check_momentary_release`
   the sequence; assemble `actions` + overall `result`.
6. `certificate::build(...)`.

This mirrors `drive()` exactly, except reports come from `replay` instead of a live `Driver`.

## 4. Why ingestion lives in `rfl-conformance`, not `rfl-core`

The wire types derive `serde::Serialize` only, by design: `driver.rs` documents deterministic
serialization (RD1c) and uses `message: &'static str`, which is not a `Deserialize` target. rfl-core
*produces* the protocol; the verifier *consumes* arbitrary, possibly-hostile vendor protocol. Adding
`Deserialize` to rfl-core would (a) risk the green determinism goldens and (b) put the strict,
adversarial input-validation concern in the wrong layer. So the `*In` mirror structs + conversion
live in `rfl-conformance::replay`, and rfl-core is untouched. The asymmetry (Serialize in core,
Deserialize in the verifier) is intentional and correct, not a gap.

## 5. Dependencies

- `rfl-conformance`: promote `boon = "0.6"` from `[dev-dependencies]` to `[dependencies]` (already
  in the lockfile via tests — low risk); add `sha2 = "0.10"`.
- `rfl-cli`: add `rfl-conformance = { path = "../rfl-conformance" }`.

No other crates. No network, no key material.

## 6. Error handling

Strict only at the external boundary (the three files + the JSONL): line-numbered JSON-parse errors,
schema-validation errors naming the failing keyword/path, and correlation errors naming the
offending action_id. Past the schema-validation gate, conversion into canonical types trusts the
input. The cardinal distinction: a parse / schema / correlation failure makes the run **invalid**
(exit 2, no certificate) — categorically different from a checker **FAIL** (exit 1, certificate
issued recording the non-conformance).

## 7. Honest boundaries (disclaimed on every certificate)

- **Class 4** physical / high-fidelity-simulator end-to-end — out of scope (needs hardware or a
  recursively-conformant simulator).
- **Class 2-loose ε** — the per-skill tolerance table is still data-dependent and undefined; no
  realized-execution equivalence claim is made.
- **ENV3 disturbance-rejection** — requires an injecting bench; a nominal replay cannot exercise it.
- **Fidelity-tier physical truth** — `check_audit_honesty` verifies the self-reported tier is not
  *over-claimed* relative to the lowering decision; it does **not** verify the tier is physically
  achieved. The cert claims honesty, not physical fidelity.
- **Cryptographic signing** — the canonical bytes are detached-signable out-of-band; v0 ships no
  signing / key management.
- **A JSON Schema for the certificate format itself** — a follow-on, not v0.

## 8. Testing (TDD, inline red→green)

- **Closed-loop ingestion**: `ReferenceDriver` → `reports_to_jsonl` (existing) → `replay_report`
  (new) → reconstructs identical reports + correlation. Proves serialize/ingest symmetry.
- **Schema rejection**: hand-crafted bad lines (unknown field, bad `outcome` enum, malformed `Force`
  pattern, missing required `action_id`) → `replay_report` errors at the right line.
- **Certify happy path**: nominal cable-insertion run → `result: pass`, 8/8 actions, exact
  `covered` / `excluded`.
- **Certify fail path**: a `FaultyDriver(UnderSecure)` report file → `result: fail`, the
  `grasp.pinch` action carries the `securing_force < min_holding_force` reason. (FaultyDriver used
  only to *generate a non-conformant fixture* — a test use, never the product surface.)
- **Determinism**: certify twice on the same inputs → byte-identical certificate (equal
  `content_hash`).
- **Correlation**: drop a `status` line, or add an orphan action_id → invalid-run error (exit 2),
  distinct from a FAIL.
- **Golden**: insta snapshot of the nominal certificate. The example files are committed and fixed,
  so their sha256 (and thus the whole cert) is stable and snapshot-able.

Full `cargo test` (workspace) + `validate.py` are READ in a batch separate from each commit. The CLI
arm is covered by an integration test invoking `rfl certify` on the committed example + a captured
nominal JSONL fixture.

## 9. Out of scope for this track (named, not silently dropped)

Live driver spawning (`--driver`, the existing stub's future); ROS 2 transport; Class 4 benches;
the Class 2-loose ε table; certificate signing and a steward verification tier (Tier 2/3 of the
regime); a certificate-format JSON Schema. Each is a later track; none is blocked by v0.
