# Design — certificate as a validatable + re-verifiable contract (`rfl verify`)

**Date:** 2026-06-01
**Track:** conformance-certify — increment 2 (cert-as-contract)
**Status:** approved
**Scope:** `rfl-conformance` (`certificate` seal change + new `verify_certificate`), `rfl-cli` (new
`verify` subcommand), `schemas/certificate.schema.json` (NEW), `schemas/validate.py` (well-formedness
+ example-cert validation), `examples/01-cable-insertion/` (committed `driver-report.jsonl` +
`certificate.json` fixtures). **No `rfl-core` change**, **no change to `replay` / `battery` / the
`certify` orchestration logic** (only `seal`'s hash recipe moves).

## 0. Why this increment

`certify` v0 *emits* a certificate but gives no one a way to *validate its shape* or *re-verify its
integrity*. A certificate is adoption infrastructure only if a third party (a steward, an insurer, a
CI gate) can mechanically trust it — which today they cannot: there is no schema for the format and no
way to detect tampering. This increment closes that loop. It is the natural completion of the v0
artifact: the cert becomes a published, validatable, tamper-evident contract.

## 1. The canonicalization decision (the crux)

v0 computed `content_hash` over `serde_json::to_vec(&CertificateBody)` — Rust struct
*field-declaration order*, which is opaque to any other language and impossible to reproduce without
the Rust type. Re-verification needs a canonical form a third party can reproduce. The decision:

**Hash over sorted-key canonical JSON.** `serde_json::Value` is a `BTreeMap` (no `preserve_order`
feature in this workspace), so `serde_json::to_vec(&serde_json::to_value(&body))` is sorted-key,
recursively, deterministically — a JCS/RFC-8785-flavored canonical form. The re-verification recipe is
language-agnostic:

> parse the certificate → remove `content_hash` → sort every object's keys recursively →
> compact-serialize → `sha256` → compare to the declared `content_hash`.

Seal and verify share one helper (`content_hash_of(&Value)`) so they cannot drift. This **changes the
v0 hash value**, which is free: v0 shipped hours ago with no certificates in the wild, and the example
fixtures are regenerated here.

**Rejected alternative** — keep struct-order and derive `Deserialize` on the cert types (round-trip to
re-serialize). This keeps a Rust-specific, non-portable canonical order and churns every
`&'static str` cert field to an owned type. The portable recipe is worth more than the saved hash
value.

**Rejected trap** — enabling serde_json's global `preserve_order` feature would reorder the
`force_profile` `Value` keys embedded in the retarget goldens and break the 12 byte-identical golden
binaries. The `to_value`/`BTreeMap` route gets canonical ordering locally without that blast radius.

The emitted file stays pretty / struct-order for human reading; verify re-derives the canonical form
from the parsed `Value`, so the file's whitespace and key order are irrelevant to verification.

## 2. `schemas/certificate.schema.json` (NEW)

A JSON Schema (Draft 2020-12, matching the existing four schemas' style — `$schema`, `$id`, `title`,
`description`, `$defs`):

- top-level `required`: every body field + `content_hash`; `additionalProperties: false`.
- `certificate_schema_version`: `const "0.1"`. `spec_version` / `tool_version`: `string`.
- `skill` / `embodiment`: `{ id: string, sha256: ^[0-9a-f]{64}$ }`, `required [id, sha256]`,
  `additionalProperties: false`.
- `report_sha256`: `^[0-9a-f]{64}$`. `content_hash`: `^sha256:[0-9a-f]{64}$`.
- `result`: `enum [pass, fail]`.
- `covered` / `excluded`: `array` of `string`.
- `actions`: array of `ActionEntry { action_id: string, suffix: string, envelope_class?: enum[...4],
  checks: [CheckEntry], passed: bool }`, `required [action_id, suffix, checks, passed]`,
  `additionalProperties: false`.
- `CheckEntry { name: string, result: enum[pass, fail], reason?: string }`, `required [name, result]`,
  `additionalProperties: false`.
- `sequence_checks`: array of `CheckEntry`.
- `envelope_class` enum: `terminal_postcondition`, `grasp_continuity`, `force_trajectory`,
  `interval_invariant` (the four `certificate::envelope_class_str` emits).

`validate.py` gains: (a) the new schema in its well-formedness loop, and (b) a check that the committed
example certificate validates against it.

## 3. Seal canonicalization (`certificate.rs`)

Add a shared helper and rewrite `seal`:

```rust
/// The content hash of a certificate body Value: sha256 over its sorted-key canonical JSON
/// (serde_json Value is a BTreeMap without preserve_order, so keys serialize sorted, recursively).
/// A third party re-verifies by: parse cert, drop content_hash, sort keys recursively,
/// compact-serialize, sha256.
fn content_hash_of(body: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(body).expect("serialize canonical certificate body");
    format!("sha256:{}", sha256_hex(&bytes))
}

pub fn seal(body: CertificateBody) -> Certificate {
    let value = serde_json::to_value(&body).expect("certificate body to value");
    let content_hash = content_hash_of(&value);
    Certificate { body, content_hash }
}
```

`to_json` is unchanged (pretty, struct order — human-facing). The v0 unit test
`seal_is_deterministic_and_verifiable` updates its recompute leg to the new recipe (via
`to_value`/`content_hash_of`); the determinism + result-change-changes-hash legs are unaffected in
intent.

## 4. `verify_certificate` + `rfl verify`

`certificate.rs` (or a small `verify` section) gains:

```rust
pub struct VerifyReport {
    pub declared: String,    // content_hash read from the certificate
    pub recomputed: String,  // content_hash recomputed over the canonical body
    pub matches: bool,
}

/// Validate a certificate's shape against the embedded certificate schema and re-verify its
/// content hash. Err = malformed (unparseable / schema-invalid). Ok with matches=false = the
/// certificate was altered after sealing.
pub fn verify_certificate(cert_json: &str) -> anyhow::Result<VerifyReport>;
```

It embeds `schemas/certificate.schema.json` via `include_str!` (self-contained binary, mirroring
`replay`'s driver-schema embedding), boon-validates the parsed cert, reads `content_hash`, recomputes
it over `parsed_value − content_hash` via `content_hash_of`, and compares.

`rfl verify <cert.json>` (new subcommand): read the file, call `verify_certificate`, print a one-line
result, exit:

- **0** — schema-valid + `content_hash` matches (a well-formed, unmodified RFL certificate);
- **1** — `content_hash` mismatch (tampered / corrupted; the body no longer hashes to the declared
  value);
- **2** — malformed (unreadable / unparseable / schema-invalid; no verdict possible).

Mirrors `certify`'s ran-and-failed vs couldn't-run split.

## 5. Committed example fixtures (`examples/01-cable-insertion/`)

- `driver-report.jsonl` — the nominal `ReferenceDriver` capture for `skill.yaml` + `allegro.yaml`
  (exactly `reports_to_jsonl(run_reference_driver(...))` bytes).
- `certificate.json` — its sealed certificate (`rfl certify --out`).

These give `rfl verify` a real fixture and document the artifact for adopters. Generated once via the
CLI and committed; guarded by tests (§ 6).

## 6. Testing

- **Round-trip:** `seal` a body → `verify_certificate(to_json(cert))` → `matches == true`.
- **Tamper detection:** flip one byte in a check `reason` (or any body field) of a sealed cert →
  `verify_certificate` → `matches == false` (and `rfl verify` exit 1).
- **Schema rejection:** a cert with a bad `result` enum / missing `content_hash` / extra field →
  `verify_certificate` → `Err` (exit 2).
- **Canonical-recipe parity:** assert `content_hash` equals an independently computed
  `sha256` over `to_vec(to_value(body))` (the v0 recompute test, updated).
- **Example fixtures:** (a) regenerating the report from `ReferenceDriver` byte-equals the committed
  `driver-report.jsonl`; (b) `certify::run` on the committed inputs reproduces the committed cert's
  `content_hash` (catches content drift via the integrity fingerprint, robust to pretty-print
  whitespace); (c) `rfl verify examples/.../certificate.json` exits 0.
- **`validate.py`:** the new schema is well-formed; the committed example cert validates against it.
- **CLI smoke:** `rfl verify` on the committed cert → exit 0 + a "VERIFIED" line.

Full `cargo test --workspace` + `validate.py` are READ in a batch separate from each commit.

## 7. Honest scope boundary

`rfl verify` proves a certificate's *shape* (schema-valid) and *integrity* (unmodified since sealing).
It does **not** prove the *signer's identity* — there is no signature in v0; the content hash is
tamper-*evidence*, not authentication. "Who vouched for this certificate" is the deferred out-of-band
signing / steward-tier work. `rfl verify` answers "is this a well-formed, unmodified RFL
certificate?", not "who issued it?".

## 8. Out of scope (named, each a later track)

Cryptographic signing + a `signature` block; steward Tier-2/3 verifier identity; certificate
revocation / expiry; a registry of issued certificates; live `--driver` stdio mode (separate track);
verifying the cert against a *re-run* of certify (that is just `certify` again, not `verify`).
