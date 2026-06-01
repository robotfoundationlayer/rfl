# Design — certificate signing (`rfl sign` / `rfl keygen`; `verify` reports the signer)

**Date:** 2026-06-01
**Track:** conformance-certify — increment 6 (signer identity)
**Status:** approved
**Scope:** `rfl-conformance` (`certificate` — `generate_keypair`, `sign_certificate`, signature-checking in `verify_certificate`; deps `ed25519-dalek` + `rand` + `hex`), `schemas/certificate.schema.json` (optional `signature`), `rfl-cli` (`keygen` + `sign` subcommands; `verify` output), `docs/certifying-a-driver.md` (Signing subsection; recipe update). **No `rfl-core` change**; **the typed `Certificate` is unchanged** (signing is a `Value`-level operation).

## 0. Why this increment

Every prior increment documented the same boundary: a certificate proves *integrity*, not *signer identity*. This closes it — the certify track's capstone. `rfl sign` attaches an ed25519 signature; `rfl verify` reports *who* signed. It moves the cert from Tier-1 self-certification toward Tier-2 (a steward signs).

**Scoped to mechanics only.** The crypto (sha256 canonical bytes → ed25519) is governance-neutral. *Key distribution and trust* — "is this signer authorized?" — stays out of scope; that is the governance decision the spec deliberately defers. `verify` reports the signer's public key; it does not judge whether that key is trusted.

## 1. What is signed, and where the signature lives

- **Signed message:** the certificate's `content_hash` string (`sha256:<hex>`). Because `content_hash` is the canonical hash of the body, signing it transitively binds the body — integrity verification (the body re-hashes to `content_hash`) makes the signature vouch for the whole certificate. One stable 71-byte string to sign.
- **Where:** a top-level `signature` object added to the certificate JSON *after* sealing: `{ "alg": "ed25519", "public_key": "<64-hex>", "sig": "<128-hex>" }`.
- **`Value`-level, so the typed `Certificate` is untouched.** `certify` emits unsigned certs; `rfl sign` reads the cert JSON as a `serde_json::Value`, inserts `signature`, and re-serializes; `verify_certificate` already works at the `Value` level.

## 2. The correctness point — `recompute_content_hash` drops `signature` too

`content_hash` is computed at seal time over the body, which never contains `content_hash` or `signature` (the latter is added post-seal). So re-verification must drop **both** before re-hashing:

```rust
fn recompute_content_hash(cert: &Value) -> String {
    let mut obj = cert.as_object().cloned().unwrap_or_default();
    obj.remove("content_hash");
    obj.remove("signature"); // added post-seal; not part of the hashed body
    content_hash_of(&Value::Object(obj))
}
```

A signed cert's `content_hash` therefore still verifies. The documented language-agnostic recipe becomes `del(.content_hash, .signature)`.

## 3. Crypto (`certificate.rs`)

Deps: `ed25519-dalek = { version = "2", features = ["rand_core"] }`, `rand = "0.8"` (OsRng), `hex = "0.4"`. Raw 32-byte keys, hex-encoded.

- `generate_keypair() -> (secret_hex, public_hex)` — `SigningKey::generate(&mut rand::rngs::OsRng)`; `hex::encode(sk.to_bytes())` / `hex::encode(sk.verifying_key().to_bytes())`.
- `sign_certificate(cert_json: &str, secret_hex: &str) -> Result<String>` — parse + schema-validate the cert, read `content_hash`, `SigningKey::from_bytes(&secret32).sign(content_hash.as_bytes())`, insert the `signature` object (`public_key` derived from the secret), return pretty JSON.
- `verify_certificate` gains signature checking: after the integrity recompute, if a `signature` is present, `VerifyingKey::from_bytes(&pub32)?.verify_strict(content_hash.as_bytes(), &Signature::from_bytes(&sig64))` → `valid: bool`. `VerifyReport` gains `signature: Option<SignatureVerdict { public_key: String, valid: bool }>`. A malformed `signature` (bad hex / wrong length / bad key) → `valid: false`, never an error (the schema already enforces hex shape; this is defence-in-depth).

## 4. CLI (`rfl-cli`)

- `rfl keygen <out>` — write the secret-key hex to `<out>` (chmod `0600` on unix), print the public-key hex to stdout. Errors → exit 2.
- `rfl sign --key <secret-file> <cert.json>` — print the signed certificate JSON to stdout (composable: `rfl sign --key k cert.json > signed.json`). A bad key / cert → exit 2.
- `rfl verify <cert.json>` — output extended:
  - integrity mismatch → `TAMPERED …`, exit `1`.
  - integrity ok, no signature → `VERIFIED: content_hash … matches (unsigned)`, exit `0`.
  - integrity ok, signature valid → `VERIFIED: content_hash … matches; signed by <pubkey> (ed25519)`, exit `0`.
  - integrity ok, signature invalid → `SIGNATURE INVALID: content_hash matches but the signature for <pubkey> does not verify`, exit `1`.
  - malformed cert → exit `2`.

Overall: exit `0` iff integrity holds AND (unsigned OR signature valid); `1` on integrity mismatch or invalid signature; `2` on malformed.

## 5. Schema

`certificate.schema.json` top-level gains an optional `signature` (not in `required`):

```json
"signature": {
  "type": "object",
  "required": ["alg", "public_key", "sig"],
  "additionalProperties": false,
  "properties": {
    "alg": { "const": "ed25519" },
    "public_key": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
    "sig": { "type": "string", "pattern": "^[0-9a-f]{128}$" }
  }
}
```

`additionalProperties: false` on the cert stays — `signature` is now a declared optional field. The committed example certs are unsigned (no `signature`), so they remain schema-valid and their `content_hash` is unchanged (the new `signature`-drop in recompute is a no-op for unsigned certs). The fixture guard is unaffected.

## 6. Testing

- `certificate.rs` unit tests: keygen → seal a sample cert → `sign_certificate` → `verify_certificate` round-trip (`matches`, `signature.valid`, embedded `public_key` == keygen public); a corrupted `sig` hex → `signature.valid == false` while integrity still `matches`; a post-sign body edit → integrity `matches == false`; an unsigned cert → `signature == None`, `matches == true`.
- CLI smoke (`#[cfg(unix)]` for the chmod): `rfl keygen` → `rfl sign` → `rfl verify` exit 0 + "signed by"; a tampered signed cert → exit 1.
- Full `cargo test --workspace` + `validate.py` read separately; the example fixtures + the 7 certify + 3-case guard stay green.

## 7. Out of scope (YAGNI)

Key distribution / a trust store / signer authorization (governance); PEM / OpenSSH key formats (raw hex only); multiple signatures / countersigning; revocation / expiry; signing the full canonical body instead of `content_hash` (redundant — `content_hash` already binds the body).
