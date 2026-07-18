# Certificate Contract + `rfl verify` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the `rfl certify` certificate a published, validatable, tamper-evident contract: a JSON Schema for the format, a sorted-key canonical content hash, and `rfl verify <cert.json>` that schema-validates a certificate and re-verifies its integrity.

**Architecture:** Move `content_hash` to sha256 over sorted-key canonical JSON (`serde_json::Value` is a `BTreeMap` without `preserve_order`), shared by `seal` and a new `verify_certificate` via one helper so they can't drift. Add `schemas/certificate.schema.json` (boon-validated at verify time, embedded in the binary). Add a `rfl verify` CLI with exit 0 (verified) / 1 (tampered) / 2 (malformed). Commit example fixtures (report + cert) guarded by a bless-pattern test.

**Tech Stack:** Rust (edition 2024), `serde`/`serde_json`, `boon` 0.6, `sha2` 0.10, `clap` 4, `anyhow`; `jsonschema`/`pyyaml` for `validate.py`.

**Design doc:** `docs/design/2026-06-01-certificate-contract-verify-design.md` (committed `7f0488c`).

**Discipline (every task):** inline TDD red→green; explicit `git add <paths>` (never `-A`); Conventional Commits, English imperative ≤72; after each commit `git fetch origin -q && git rev-list --left-right --count origin/main...HEAD` = `0 0` before AND ff `git push origin main` after (a parallel `cobel` docs session shares this tree — re-verify `git log` has your commit each batch); run full `cargo test --workspace` + `validate.py` in a batch separate from the commit and READ it.

----

## File Structure

- `schemas/certificate.schema.json` — NEW. The certificate-format wire contract (Draft 2020-12).
- `schemas/validate.py` — MODIFY. Add the new schema to the well-formedness loop (Task 1) and an example-cert reference-instance validation (Task 5).
- `crates/rfl-conformance/src/certificate.rs` — MODIFY. Add `content_hash_of` helper; rewrite `seal` to sorted-key canonical; add `VerifyReport` + `verify_certificate` (embeds the cert schema, boon-validates, recomputes the hash); update the recompute unit test.
- `crates/rfl-cli/src/main.rs` — MODIFY. Add the `Verify` subcommand + handler (exit 0/1/2).
- `crates/rfl-cli/tests/verify_cli.rs` — NEW. CLI smoke for `rfl verify`.
- `examples/01-cable-insertion/driver-report.jsonl` — NEW. Nominal `ReferenceDriver` capture (fixture).
- `examples/01-cable-insertion/certificate.json` — NEW. Its sealed certificate (fixture).
- `crates/rfl-conformance/tests/certificate_fixtures.rs` — NEW. Bless-pattern fixture guard.

----

## Task 1: `certificate.schema.json` + validate.py well-formedness

**Files:**
- Create: `schemas/certificate.schema.json`
- Modify: `schemas/validate.py`

- [ ] **Step 1: Create the schema**

Create `schemas/certificate.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://rfl.dev/schemas/certificate.schema.json",
  "title": "RFL Conformance Certificate",
  "description": "The artifact emitted by `rfl certify` (Class 3 driver-protocol, JSONL replay). Identifies inputs by RFL id + content sha256; content_hash is sha256 over the sorted-key canonical JSON of every field except content_hash itself. Backs `rfl verify`.",
  "type": "object",
  "required": [
    "certificate_schema_version", "spec_version", "tool_version", "skill", "embodiment",
    "report_sha256", "result", "covered", "excluded", "actions", "sequence_checks", "content_hash"
  ],
  "additionalProperties": false,
  "properties": {
    "certificate_schema_version": { "const": "0.1" },
    "spec_version": { "type": "string" },
    "tool_version": { "type": "string" },
    "skill": { "$ref": "#/$defs/FileRef" },
    "embodiment": { "$ref": "#/$defs/FileRef" },
    "report_sha256": { "$ref": "#/$defs/Sha256Hex" },
    "result": { "enum": ["pass", "fail"] },
    "covered": { "type": "array", "items": { "type": "string" } },
    "excluded": { "type": "array", "items": { "type": "string" } },
    "actions": { "type": "array", "items": { "$ref": "#/$defs/ActionEntry" } },
    "sequence_checks": { "type": "array", "items": { "$ref": "#/$defs/CheckEntry" } },
    "content_hash": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" }
  },
  "$defs": {
    "Sha256Hex": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
    "FileRef": {
      "type": "object",
      "required": ["id", "sha256"],
      "additionalProperties": false,
      "properties": {
        "id": { "type": "string" },
        "sha256": { "$ref": "#/$defs/Sha256Hex" }
      }
    },
    "CheckEntry": {
      "type": "object",
      "required": ["name", "result"],
      "additionalProperties": false,
      "properties": {
        "name": { "type": "string" },
        "result": { "enum": ["pass", "fail"] },
        "reason": { "type": "string" }
      }
    },
    "ActionEntry": {
      "type": "object",
      "required": ["action_id", "suffix", "checks", "passed"],
      "additionalProperties": false,
      "properties": {
        "action_id": { "type": "string" },
        "suffix": { "type": "string" },
        "envelope_class": {
          "enum": ["terminal_postcondition", "grasp_continuity", "force_trajectory", "interval_invariant"]
        },
        "checks": { "type": "array", "items": { "$ref": "#/$defs/CheckEntry" } },
        "passed": { "type": "boolean" }
      }
    }
  }
}
```

- [ ] **Step 2: Add it to validate.py's well-formedness loop**

In `schemas/validate.py`, after the line `adapter = load_schema("tactile-manifold/adapter.schema.json")` (currently line 58), add:

```python
    certificate = load_schema("certificate.schema.json")
```

Then in the well-formedness tuple (currently lines 61-66), add the `("certificate", certificate),` entry so the block reads:

```python
    for label, schema in (
        ("skill-isa", skill),
        ("embodiment-descriptor", descriptor),
        ("driver-interface", driver),
        ("tactile-manifold adapter", adapter),
        ("certificate", certificate),
    ):
```

- [ ] **Step 3: Run validate.py to verify the new schema is well-formed**

Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py`
Expected: a new `[ok  ] check_schema certificate` line; the run ends `PASS — all conformance test class 1 checks passed`.

- [ ] **Step 4: Commit**

```bash
git add schemas/certificate.schema.json schemas/validate.py
git commit -m "feat(certify): add certificate.schema.json (cert format contract)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 2: Sorted-key canonical content hash

**Files:**
- Modify: `crates/rfl-conformance/src/certificate.rs`

- [ ] **Step 1: Update the recompute unit test to the canonical recipe (red)**

In `crates/rfl-conformance/src/certificate.rs`, replace the recompute leg of `seal_is_deterministic_and_verifiable`. Find:

```rust
        // recompute the hash over the compact body and confirm it matches content_hash.
        let cert = seal(sample_body("pass"));
        let compact = serde_json::to_vec(&cert.body).unwrap();
        assert_eq!(cert.content_hash, format!("sha256:{}", sha256_hex(&compact)));
        assert!(cert.content_hash.starts_with("sha256:"));
```

Replace with:

```rust
        // recompute the hash over the sorted-key canonical body and confirm it matches.
        let cert = seal(sample_body("pass"));
        let canonical = serde_json::to_vec(&serde_json::to_value(&cert.body).unwrap()).unwrap();
        assert_eq!(cert.content_hash, format!("sha256:{}", sha256_hex(&canonical)));
        assert!(cert.content_hash.starts_with("sha256:"));
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p rfl-conformance --lib certificate::tests::seal_is_deterministic_and_verifiable`
Expected: FAIL on the recompute assertion (current `seal` hashes struct-order bytes, not sorted-key).

- [ ] **Step 3: Rewrite `seal` to the canonical recipe**

In `certificate.rs`, replace the `seal` function:

```rust
/// Seal a body: attach `content_hash` over its compact serialization.
#[must_use]
pub fn seal(body: CertificateBody) -> Certificate {
    let compact = serde_json::to_vec(&body).expect("serialize certificate body");
    let content_hash = format!("sha256:{}", sha256_hex(&compact));
    Certificate { body, content_hash }
}
```

with a shared helper + the canonical `seal`:

```rust
/// The content hash of a certificate body Value: sha256 over its sorted-key canonical JSON
/// (`serde_json::Value` is a `BTreeMap` without `preserve_order`, so keys serialize sorted,
/// recursively). A third party re-verifies by: parse the certificate, drop `content_hash`, sort
/// every object's keys recursively, compact-serialize, sha256.
fn content_hash_of(body: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(body).expect("serialize canonical certificate body");
    format!("sha256:{}", sha256_hex(&bytes))
}

/// Seal a body: attach `content_hash` over its sorted-key canonical serialization.
#[must_use]
pub fn seal(body: CertificateBody) -> Certificate {
    let value = serde_json::to_value(&body).expect("certificate body to value");
    let content_hash = content_hash_of(&value);
    Certificate { body, content_hash }
}
```

- [ ] **Step 4: Run the certificate tests to verify they pass**

Run: `cargo test -p rfl-conformance --lib certificate::`
Expected: all PASS (determinism, recompute-with-canonical, result-change-changes-hash).

- [ ] **Step 5: Confirm the certify integration tests still pass (hash value changed, not determinism)**

Run: `cargo test -p rfl-conformance --test certify`
Expected: all 4 PASS (they assert determinism + structure, never a specific hash value).

- [ ] **Step 6: Commit**

```bash
git add crates/rfl-conformance/src/certificate.rs
git commit -m "feat(certify): hash certificate over sorted-key canonical JSON

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 3: `verify_certificate`

**Files:**
- Modify: `crates/rfl-conformance/src/certificate.rs`

- [ ] **Step 1: Write the failing tests**

In `certificate.rs`, append to the `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn verify_round_trips_a_sealed_certificate() {
        let json = to_json(&seal(sample_body("pass")));
        let report = verify_certificate(&json).expect("schema-valid certificate");
        assert!(report.matches, "declared {} != recomputed {}", report.declared, report.recomputed);
        assert!(report.declared.starts_with("sha256:"));
    }

    #[test]
    fn verify_detects_tampering() {
        let json = to_json(&seal(sample_body("pass")));
        // alter a body field (skill.id + action_id carry "cable-insertion"); content_hash is hex,
        // so it is untouched -> the recomputed hash no longer matches the declared one.
        let tampered = json.replace("cable-insertion", "evil-skill");
        assert_ne!(tampered, json);
        let report = verify_certificate(&tampered).expect("still schema-valid");
        assert!(!report.matches);
    }

    #[test]
    fn verify_rejects_schema_invalid_certificate() {
        // break the content_hash pattern (only content_hash carries the "sha256:" prefix).
        let json = to_json(&seal(sample_body("pass")));
        let bad = json.replace("sha256:", "badhash:");
        assert!(verify_certificate(&bad).is_err());
    }
```

- [ ] **Step 2: Run to verify they fail to compile**

Run: `cargo test -p rfl-conformance --lib certificate::tests::verify_round_trips_a_sealed_certificate`
Expected: compile error — `cannot find function 'verify_certificate'` / `VerifyReport`.

- [ ] **Step 3: Implement `verify_certificate`**

In `certificate.rs`, add near the top (after the existing `use` lines) the embedded schema:

```rust
/// The certificate-format schema, embedded so `verify` is self-contained in the binary.
const CERTIFICATE_SCHEMA: &str = include_str!("../../../schemas/certificate.schema.json");
```

and add `use anyhow::{anyhow, Context, Result};` to the imports at the top of the file (alongside `use serde::Serialize;`). Then add, above the `#[cfg(test)]` block:

```rust
/// The result of verifying a certificate's integrity.
pub struct VerifyReport {
    /// The `content_hash` declared in the certificate.
    pub declared: String,
    /// The `content_hash` recomputed over the certificate's canonical body.
    pub recomputed: String,
    /// True iff the declared and recomputed hashes match (the certificate is unmodified).
    pub matches: bool,
}

/// Recompute the content hash of a parsed certificate: drop `content_hash`, then hash the rest in
/// sorted-key canonical form (the same `content_hash_of` `seal` uses).
fn recompute_content_hash(cert: &serde_json::Value) -> String {
    let mut obj = cert.as_object().cloned().unwrap_or_default();
    obj.remove("content_hash");
    content_hash_of(&serde_json::Value::Object(obj))
}

/// Validate a certificate's shape against the embedded certificate schema and re-verify its
/// content hash.
///
/// # Errors
/// The certificate is malformed (unparseable JSON, or schema-invalid). A schema-valid certificate
/// whose body was altered after sealing returns `Ok` with `matches == false`.
pub fn verify_certificate(cert_json: &str) -> Result<VerifyReport> {
    let value: serde_json::Value =
        serde_json::from_str(cert_json).context("certificate is not valid JSON")?;
    let schema_value: serde_json::Value =
        serde_json::from_str(CERTIFICATE_SCHEMA).context("parse embedded certificate schema")?;
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource("certificate.schema.json", schema_value)
        .map_err(|e| anyhow!("add schema resource: {e}"))?;
    let idx = compiler
        .compile("certificate.schema.json", &mut schemas)
        .map_err(|e| anyhow!("compile schema: {e}"))?;
    schemas
        .validate(&value, idx)
        .map_err(|e| anyhow!("certificate schema violation: {e}"))?;

    let declared = value
        .get("content_hash")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("certificate has no content_hash"))?
        .to_string();
    let recomputed = recompute_content_hash(&value);
    let matches = declared == recomputed;
    Ok(VerifyReport { declared, recomputed, matches })
}
```

- [ ] **Step 4: Run the verify tests**

Run: `cargo test -p rfl-conformance --lib certificate::`
Expected: all PASS.

- [ ] **Step 5: Confirm clippy clean for the crate's new code**

Run: `cargo clippy -p rfl-conformance --lib --all-features 2>&1 | rg -c "certificate\.rs" || echo "0 certificate.rs warnings"`
Expected: `0 certificate.rs warnings`.

- [ ] **Step 6: Commit**

```bash
git add crates/rfl-conformance/src/certificate.rs
git commit -m "feat(certify): verify_certificate (schema-validate + re-hash)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 4: `rfl verify` CLI

**Files:**
- Modify: `crates/rfl-cli/src/main.rs`
- Create: `crates/rfl-cli/tests/verify_cli.rs`

- [ ] **Step 1: Add the `Verify` subcommand variant**

In `crates/rfl-cli/src/main.rs`, add to the doc-comment block (after the `rfl certify` line):

```rust
//! - `rfl verify <certificate.json>` — schema-validate a certificate and re-verify its
//!   content hash (tamper detection)
```

Then in the `Command` enum, after the `Certify { … }` variant and before `SpecVersion`, add:

```rust
    /// Schema-validate a certificate and re-verify its content hash (tamper detection).
    Verify {
        /// Path to the certificate JSON file.
        certificate: std::path::PathBuf,
    },
```

- [ ] **Step 2: Add the handler arm**

In `fn main`, after the `Command::Certify { … } => { … }` arm and before `Command::SpecVersion`, add:

```rust
        Command::Verify { certificate } => {
            let text = match std::fs::read_to_string(&certificate) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("verify: read {certificate:?}: {e}");
                    std::process::exit(2);
                }
            };
            match rfl_conformance::certificate::verify_certificate(&text) {
                Err(e) => {
                    eprintln!("verify: malformed certificate: {e:#}");
                    std::process::exit(2);
                }
                Ok(report) => {
                    if report.matches {
                        println!("VERIFIED: content_hash {} matches", report.declared);
                        std::process::exit(0);
                    }
                    println!(
                        "TAMPERED: declared {} != recomputed {}",
                        report.declared, report.recomputed
                    );
                    std::process::exit(1);
                }
            }
        }
```

- [ ] **Step 3: Build to verify it compiles**

Run: `cargo build -p rfl-cli`
Expected: builds clean.

- [ ] **Step 4: Write the CLI smoke test**

Create `crates/rfl-cli/tests/verify_cli.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Smoke test for `rfl verify`: a freshly sealed certificate verifies (exit 0); a tampered one
//! does not (exit 1).

use std::path::{Path, PathBuf};
use std::process::Command;

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

fn run_verify(cert_path: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rfl"))
        .arg("verify")
        .arg(cert_path)
        .output()
        .expect("run rfl verify")
}

#[test]
fn verify_accepts_sealed_and_rejects_tampered() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let reports = rfl_conformance::run_reference_driver(&skill, &emb).unwrap();
    let report_path =
        std::env::temp_dir().join(format!("rfl-verify-report-{}.jsonl", std::process::id()));
    std::fs::write(&report_path, rfl_conformance::reports_to_jsonl(&reports)).unwrap();

    let cert = rfl_conformance::certify::run(&skill, &emb, &report_path).unwrap().certificate;
    let json = rfl_conformance::certificate::to_json(&cert);
    let cert_path =
        std::env::temp_dir().join(format!("rfl-verify-cert-{}.json", std::process::id()));
    std::fs::write(&cert_path, &json).unwrap();

    // sealed -> exit 0 + VERIFIED.
    let ok = run_verify(&cert_path);
    let ok_out = String::from_utf8_lossy(&ok.stdout);
    assert!(ok.status.success(), "exit {:?}, stdout: {ok_out}", ok.status.code());
    assert!(ok_out.contains("VERIFIED"), "stdout: {ok_out}");

    // tampered -> exit 1 + TAMPERED.
    let tampered_path =
        std::env::temp_dir().join(format!("rfl-verify-tampered-{}.json", std::process::id()));
    std::fs::write(&tampered_path, json.replace("cable-insertion", "evil-skill")).unwrap();
    let bad = run_verify(&tampered_path);
    let bad_out = String::from_utf8_lossy(&bad.stdout);
    assert_eq!(bad.status.code(), Some(1), "stdout: {bad_out}");
    assert!(bad_out.contains("TAMPERED"), "stdout: {bad_out}");

    std::fs::remove_file(report_path).ok();
    std::fs::remove_file(cert_path).ok();
    std::fs::remove_file(tampered_path).ok();
}
```

- [ ] **Step 5: Run the CLI smoke test**

Run: `cargo test -p rfl-cli --test verify_cli`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/rfl-cli/src/main.rs crates/rfl-cli/tests/verify_cli.rs
git commit -m "feat(cli): add rfl verify subcommand (schema + hash check)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 5: Committed example fixtures + validate.py example check

**Files:**
- Create: `crates/rfl-conformance/tests/certificate_fixtures.rs`
- Create: `examples/01-cable-insertion/driver-report.jsonl` (generated, then committed)
- Create: `examples/01-cable-insertion/certificate.json` (generated, then committed)
- Modify: `schemas/validate.py`

- [ ] **Step 1: Write the bless-pattern fixture guard**

Create `crates/rfl-conformance/tests/certificate_fixtures.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Guards the committed example fixtures (driver-report.jsonl + certificate.json). Run with
//! `RFL_BLESS=1` to (re)generate them; otherwise asserts they are current.

use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

#[test]
fn example_fixtures_are_current() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let report = rfl_conformance::reports_to_jsonl(
        &rfl_conformance::run_reference_driver(&skill, &emb).unwrap(),
    );
    let report_path = dir.join("driver-report.jsonl");
    let cert_path = dir.join("certificate.json");

    if std::env::var("RFL_BLESS").is_ok() {
        std::fs::write(&report_path, &report).unwrap();
        let cert = rfl_conformance::certify::run(&skill, &emb, &report_path).unwrap().certificate;
        std::fs::write(&cert_path, rfl_conformance::certificate::to_json(&cert)).unwrap();
        return;
    }

    // the committed report is exactly what the reference driver emits (its sha256 feeds the cert).
    let committed_report = std::fs::read_to_string(&report_path)
        .expect("driver-report.jsonl present (run RFL_BLESS=1 to generate)");
    assert_eq!(committed_report, report, "driver-report.jsonl stale; run RFL_BLESS=1 cargo test");

    // a fresh certify over the committed inputs reproduces the committed cert's content_hash
    // (the integrity fingerprint; robust to pretty-print whitespace).
    let fresh = rfl_conformance::certify::run(&skill, &emb, &report_path).unwrap().certificate;
    let committed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&cert_path).unwrap()).unwrap();
    assert_eq!(
        committed.get("content_hash").and_then(serde_json::Value::as_str),
        Some(fresh.content_hash.as_str()),
        "certificate.json stale; run RFL_BLESS=1 cargo test"
    );

    // and the committed certificate verifies.
    let report =
        rfl_conformance::certificate::verify_certificate(&std::fs::read_to_string(&cert_path).unwrap())
            .expect("committed certificate is schema-valid");
    assert!(report.matches, "committed certificate.json fails its own content_hash");
}
```

- [ ] **Step 2: Generate the fixtures**

Run: `RFL_BLESS=1 cargo test -p rfl-conformance --test certificate_fixtures`
Expected: PASS; `examples/01-cable-insertion/driver-report.jsonl` and `certificate.json` now exist.

- [ ] **Step 3: Run the guard without bless to confirm it asserts current**

Run: `cargo test -p rfl-conformance --test certificate_fixtures`
Expected: PASS (fixtures are current).

- [ ] **Step 4: Add the example-cert reference-instance validation to validate.py**

In `schemas/validate.py`, after the tactile-manifold adapter validation block (currently ends line 93, before `print("\nCross-schema consistency (anti-drift)")`), add:

```python
    cert_validator = Draft202012Validator(certificate)
    cert_path = EXAMPLES / "certificate.json"
    errs = list(cert_validator.iter_errors(json.loads(cert_path.read_text())))
    check("certificate.json vs certificate-schema", not errs, errs[0].message if errs else "")
```

- [ ] **Step 5: Run validate.py to confirm the example cert validates**

Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py`
Expected: a new `[ok  ] certificate.json vs certificate-schema` line; run ends `PASS`.

- [ ] **Step 6: Commit**

```bash
git add crates/rfl-conformance/tests/certificate_fixtures.rs examples/01-cable-insertion/driver-report.jsonl examples/01-cable-insertion/certificate.json schemas/validate.py
git commit -m "test(certify): commit example certificate fixtures + validate them

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 6: Full-suite verification (READ)

- [ ] **Step 1: Run the full workspace test suite (READ, separate from any commit)**

Run: `cargo test --workspace`
Expected: all green — the prior suites + the new certificate verify/fixture tests + the verify CLI smoke.

- [ ] **Step 2: Run validate.py (READ)**

Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py`
Expected: `check_schema certificate` ok + `certificate.json vs certificate-schema` ok + C1–C7 ok; ends `PASS`.

- [ ] **Step 3: Manual `rfl verify` smoke on the committed example (READ)**

```bash
cd ~/Documents/GitHub/rfl
cargo run -q -p rfl-cli -- verify examples/01-cable-insertion/certificate.json ; echo "exit=$?"
```
Expected: `VERIFIED: content_hash sha256:… matches`, `exit=0`.

- [ ] **Step 4: Manual tamper smoke (READ)**

```bash
cd ~/Documents/GitHub/rfl
sed 's/cable-insertion/evil-skill/' examples/01-cable-insertion/certificate.json > /tmp/tampered-cert.json
cargo run -q -p rfl-cli -- verify /tmp/tampered-cert.json ; echo "exit=$?"
```
Expected: `TAMPERED: declared … != recomputed …`, `exit=1`.

----

## Self-Review

**Spec coverage** (design doc → tasks):
- `schemas/certificate.schema.json` → Task 1. ✓
- sorted-key canonical content hash (shared `content_hash_of`) → Task 2. ✓
- `verify_certificate` (boon schema-validate + recompute) → Task 3. ✓
- `rfl verify` CLI, exit 0/1/2 → Task 4. ✓
- committed example fixtures + validate.py example validation → Task 5. ✓
- testing (round-trip, tamper, schema-reject, fixture golden, CLI smoke) → Tasks 2–5. ✓
- honest boundary (integrity not identity) → documented in design + the cert's own `excluded`/scope; no signature field introduced. ✓

**Placeholder scan:** no TBD/TODO; every code step shows complete code; every command has an expected result. ✓

**Type consistency:** `content_hash_of(&Value) -> String`; `seal(CertificateBody) -> Certificate`; `recompute_content_hash(&Value) -> String`; `verify_certificate(&str) -> Result<VerifyReport>`; `VerifyReport { declared, recomputed, matches }`; CLI reads `report.{matches, declared, recomputed}`. The fixture guard uses `rfl_conformance::{run_reference_driver, reports_to_jsonl, certify, certificate}` — all `pub`. Consistent across tasks. ✓

**Canonical-recipe parity (the load-bearing invariant):** `seal` hashes `to_value(&body)`; `verify` hashes `parsed_cert − content_hash`. Both route through `content_hash_of` → `to_vec(&Value)` with `serde_json`'s `BTreeMap` (sorted, recursive, no floats in the cert), so the bytes are identical. Task 5's fixture test exercises seal↔verify end-to-end. ✓
