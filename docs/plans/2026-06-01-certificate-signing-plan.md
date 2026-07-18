# Certificate Signing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rfl sign` attaches an ed25519 signature over a certificate's `content_hash`; `rfl verify` reports the signer; `rfl keygen` makes keys.

**Architecture:** Signing is a `serde_json::Value`-level operation (the typed `Certificate` is untouched): `sign_certificate` inserts a top-level `signature` object; `verify_certificate` recomputes the content hash (now dropping both `content_hash` and `signature`) and, if signed, checks the ed25519 signature against the embedded public key. Scoped to crypto mechanics — trust/distribution is out of scope.

**Tech Stack:** Rust (edition 2024), `ed25519-dalek` 2 (`rand_core` feature), `rand` 0.8 (OsRng), `hex` 0.4, `boon`/`serde_json`, `clap` 4.

**Design doc:** `docs/design/2026-06-01-certificate-signing-design.md` (committed `051240c`).

**Discipline:** explicit `git add <paths>` (never `-A`); Conventional Commits ≤72; after each commit `git fetch origin -q && git rev-list --left-right --count origin/main...HEAD` = `0 0` before AND ff `git push origin main` after; full `cargo test --workspace` + `validate.py` read separate from each commit.

----

## File Structure

- `crates/rfl-conformance/Cargo.toml` — MODIFY. Add `ed25519-dalek`, `rand`, `hex`.
- `schemas/certificate.schema.json` — MODIFY. Add an optional top-level `signature` object.
- `crates/rfl-conformance/src/certificate.rs` — MODIFY. `validate_against_schema` helper; `recompute_content_hash` drops `signature`; `generate_keypair`; `sign_certificate`; `verify_certificate` + `VerifyReport` gain signature checking; `SignatureVerdict`.
- `crates/rfl-cli/src/main.rs` — MODIFY. `Keygen` + `Sign` subcommands; `verify` output extended.
- `crates/rfl-cli/tests/sign_verify_cli.rs` — NEW. keygen → sign → verify CLI smoke.
- `docs/certifying-a-driver.md` — MODIFY. Signing subsection; update the re-verification recipe to `del(.content_hash, .signature)`.

----

## Task 1: Dependencies + schema

**Files:**
- Modify: `crates/rfl-conformance/Cargo.toml`, `schemas/certificate.schema.json`

- [ ] **Step 1: Add the crypto deps**

In `crates/rfl-conformance/Cargo.toml`, under `[dependencies]`, after `sha2 = "0.10"`, add:

```toml
ed25519-dalek = { version = "2", features = ["rand_core"] }
rand = "0.8"
hex = "0.4"
```

- [ ] **Step 2: Add the optional `signature` to the schema**

In `schemas/certificate.schema.json`, in the top-level `properties`, after the `"content_hash"` property, add:

```json
    "content_hash": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" },
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

(`signature` is NOT added to the top-level `required` array — it is optional.)

- [ ] **Step 3: Build + validate.py (existing unsigned certs stay valid)**

Run: `cargo build -p rfl-conformance 2>&1 | tail -3`
Expected: builds clean (the three crates download + compile).

Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | rg "certificate|PASS"`
Expected: `check_schema certificate` ok + the three `… vs certificate-schema` ok + `PASS` (unsigned example certs remain valid; `signature` is optional).

- [ ] **Step 4: Commit**

```bash
git add crates/rfl-conformance/Cargo.toml Cargo.lock schemas/certificate.schema.json
git commit -m "build(certify): add ed25519/rand/hex deps + optional signature schema

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 2: Crypto in `certificate.rs`

**Files:**
- Modify: `crates/rfl-conformance/src/certificate.rs`

- [ ] **Step 1: Add imports**

In `crates/rfl-conformance/src/certificate.rs`, after `use sha2::{Digest, Sha256};` (line 11), add:

```rust
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
```

- [ ] **Step 2: Extract `validate_against_schema` and update `recompute_content_hash` to drop `signature`**

Replace the current `recompute_content_hash` (lines ~146-152) with the schema helper + the updated recompute:

```rust
/// Schema-validate a parsed certificate against the embedded certificate schema.
fn validate_against_schema(value: &serde_json::Value) -> Result<()> {
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
        .validate(value, idx)
        .map_err(|e| anyhow!("certificate schema violation: {e}"))?;
    Ok(())
}

/// Recompute the content hash of a parsed certificate: drop `content_hash` and `signature` (both
/// excluded from the hashed body — `signature` is added post-seal), then hash the rest in sorted-key
/// canonical form (the same `content_hash_of` `seal` uses).
fn recompute_content_hash(cert: &serde_json::Value) -> String {
    let mut obj = cert.as_object().cloned().unwrap_or_default();
    obj.remove("content_hash");
    obj.remove("signature");
    content_hash_of(&serde_json::Value::Object(obj))
}
```

- [ ] **Step 3: Add `SignatureVerdict`, the keypair/sign/verify helpers, and update `VerifyReport` + `verify_certificate`**

Replace the current `VerifyReport` struct (lines ~136-144) and `verify_certificate` (lines ~154-185) with:

```rust
/// A signature verdict on a certificate (present only when the certificate carries a signature).
pub struct SignatureVerdict {
    /// The signer's ed25519 public key (hex).
    pub public_key: String,
    /// True iff the signature verifies over the certificate's `content_hash`.
    pub valid: bool,
}

/// The result of verifying a certificate's integrity (and signature, if signed).
pub struct VerifyReport {
    /// The `content_hash` declared in the certificate.
    pub declared: String,
    /// The `content_hash` recomputed over the certificate's canonical body.
    pub recomputed: String,
    /// True iff the declared and recomputed hashes match (the certificate is unmodified).
    pub matches: bool,
    /// The signature verdict, if the certificate carries a `signature`.
    pub signature: Option<SignatureVerdict>,
}

/// Generate an ed25519 keypair, returned as `(secret_hex, public_hex)`.
#[must_use]
pub fn generate_keypair() -> (String, String) {
    let sk = SigningKey::generate(&mut OsRng);
    (hex::encode(sk.to_bytes()), hex::encode(sk.verifying_key().to_bytes()))
}

/// Sign `message` with the hex secret key; returns `(sig_hex, public_hex)`.
fn sign_message(secret_hex: &str, message: &[u8]) -> Result<(String, String)> {
    let secret = hex::decode(secret_hex.trim()).context("secret key is not hex")?;
    let secret: [u8; 32] =
        secret.as_slice().try_into().map_err(|_| anyhow!("secret key must be 32 bytes"))?;
    let sk = SigningKey::from_bytes(&secret);
    let sig = sk.sign(message);
    Ok((hex::encode(sig.to_bytes()), hex::encode(sk.verifying_key().to_bytes())))
}

/// Verify a hex ed25519 signature over `message` with a hex public key. Any malformed input is a
/// failed verification, never an error.
fn verify_signature(public_hex: &str, message: &[u8], sig_hex: &str) -> bool {
    let Ok(pk_bytes) = hex::decode(public_hex) else { return false };
    let Ok(pk_arr): std::result::Result<[u8; 32], _> = pk_bytes.as_slice().try_into() else {
        return false;
    };
    let Ok(vk) = VerifyingKey::from_bytes(&pk_arr) else { return false };
    let Ok(sig_bytes) = hex::decode(sig_hex) else { return false };
    let Ok(sig_arr): std::result::Result<[u8; 64], _> = sig_bytes.as_slice().try_into() else {
        return false;
    };
    let sig = Signature::from_bytes(&sig_arr);
    vk.verify_strict(message, &sig).is_ok()
}

/// Sign a certificate: schema-validate it, sign its `content_hash` with the hex secret key, and
/// return the certificate JSON with an added top-level `signature` object.
///
/// # Errors
/// The certificate is malformed / schema-invalid, or the secret key is not a 32-byte hex string.
pub fn sign_certificate(cert_json: &str, secret_hex: &str) -> Result<String> {
    let mut value: serde_json::Value =
        serde_json::from_str(cert_json).context("certificate is not valid JSON")?;
    validate_against_schema(&value)?;
    let content_hash = value
        .get("content_hash")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("certificate has no content_hash"))?
        .to_string();
    let (sig_hex, public_hex) = sign_message(secret_hex, content_hash.as_bytes())?;
    let obj = value.as_object_mut().ok_or_else(|| anyhow!("certificate is not a JSON object"))?;
    obj.insert(
        "signature".to_string(),
        serde_json::json!({ "alg": "ed25519", "public_key": public_hex, "sig": sig_hex }),
    );
    Ok(serde_json::to_string_pretty(&value).expect("serialize signed certificate"))
}

/// Validate a certificate's shape against the embedded certificate schema, re-verify its content
/// hash, and (if it carries a `signature`) verify the signature against the embedded public key.
///
/// # Errors
/// The certificate is malformed (unparseable JSON, or schema-invalid). A schema-valid certificate
/// whose body was altered after sealing returns `Ok` with `matches == false`; an invalid signature
/// returns `Ok` with `signature.valid == false`.
pub fn verify_certificate(cert_json: &str) -> Result<VerifyReport> {
    let value: serde_json::Value =
        serde_json::from_str(cert_json).context("certificate is not valid JSON")?;
    validate_against_schema(&value)?;
    let declared = value
        .get("content_hash")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("certificate has no content_hash"))?
        .to_string();
    let recomputed = recompute_content_hash(&value);
    let matches = declared == recomputed;
    let signature = value.get("signature").map(|s| {
        let public_key =
            s.get("public_key").and_then(serde_json::Value::as_str).unwrap_or_default().to_string();
        let sig = s.get("sig").and_then(serde_json::Value::as_str).unwrap_or_default();
        let valid = verify_signature(&public_key, declared.as_bytes(), sig);
        SignatureVerdict { public_key, valid }
    });
    Ok(VerifyReport { declared, recomputed, matches, signature })
}
```

- [ ] **Step 4: Write the unit tests**

In the `#[cfg(test)] mod tests` block of `certificate.rs`, append:

```rust
    #[test]
    fn sign_then_verify_round_trips() {
        let (secret, public) = generate_keypair();
        let sealed = to_json(&seal(sample_body("pass")));
        let signed = sign_certificate(&sealed, &secret).expect("sign");
        let report = verify_certificate(&signed).expect("verify signed");
        assert!(report.matches, "integrity must hold after signing");
        let sig = report.signature.expect("signature present");
        assert!(sig.valid, "signature must verify");
        assert_eq!(sig.public_key, public, "embedded public key matches the keypair");
    }

    #[test]
    fn tampered_signature_fails_but_integrity_holds() {
        let (secret, _public) = generate_keypair();
        let sealed = to_json(&seal(sample_body("pass")));
        let signed = sign_certificate(&sealed, &secret).expect("sign");
        // flip one hex char of the sig (still 128 hex chars -> schema-valid, signature invalid).
        let broken = signed.replacen("\"sig\": \"0", "\"sig\": \"1", 1).replacen(
            "\"sig\": \"1",
            "\"sig\": \"1",
            1,
        );
        // ensure we actually changed it; if the sig did not start with 0, mutate the first hex.
        let broken = if broken == signed {
            // mutate the first character after the sig opener
            signed.replacen("\"sig\": \"", "\"sig\": \"0", 1)
        } else {
            broken
        };
        let report = verify_certificate(&broken).expect("still schema-valid");
        assert!(report.matches, "body integrity is unaffected");
        assert!(!report.signature.expect("signature present").valid, "tampered sig must fail");
    }

    #[test]
    fn body_edit_after_signing_fails_integrity() {
        let (secret, _public) = generate_keypair();
        let sealed = to_json(&seal(sample_body("pass")));
        let signed = sign_certificate(&sealed, &secret).expect("sign");
        let tampered = signed.replace("cable-insertion", "evil-skill");
        assert_ne!(tampered, signed);
        let report = verify_certificate(&tampered).expect("still schema-valid");
        assert!(!report.matches, "body tamper must fail integrity");
    }

    #[test]
    fn unsigned_certificate_has_no_signature_verdict() {
        let sealed = to_json(&seal(sample_body("pass")));
        let report = verify_certificate(&sealed).expect("verify unsigned");
        assert!(report.matches);
        assert!(report.signature.is_none());
    }
```

- [ ] **Step 5: Run the certificate tests**

Run: `cargo test -p rfl-conformance --lib certificate::`
Expected: all PASS (the prior verify/seal tests + the four new signing tests). If the `tampered_signature` mutation does not change the string, the test's fallback handles it; if it still fails, adjust to mutate a definite hex position (e.g. replace the 9th char of the sig).

- [ ] **Step 6: Confirm the existing verify tests + fixture guard still pass**

Run: `cargo test -p rfl-conformance --test certificate_fixtures && cargo test -p rfl-cli --test verify_cli`
Expected: both PASS (unsigned certs unaffected; recompute dropping `signature` is a no-op on them).

- [ ] **Step 7: Clippy**

Run: `cargo clippy -p rfl-conformance --lib --all-features 2>&1 | rg -c "certificate\.rs" || echo "0 certificate.rs warnings"`
Expected: `0 certificate.rs warnings`.

- [ ] **Step 8: Commit**

```bash
git add crates/rfl-conformance/src/certificate.rs
git commit -m "feat(certify): ed25519 sign + verify the signer

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 3: CLI `keygen` + `sign` + `verify` output

**Files:**
- Modify: `crates/rfl-cli/src/main.rs`
- Create: `crates/rfl-cli/tests/sign_verify_cli.rs`

- [ ] **Step 1: Extend the doc comment + add the `Keygen` / `Sign` variants**

In `crates/rfl-cli/src/main.rs`, add to the module doc comment (after the `rfl verify` line):

```rust
//! - `rfl keygen <out>` — generate an ed25519 keypair (secret to <out>, public to stdout)
//! - `rfl sign --key <secret> <certificate.json>` — attach an ed25519 signature to a certificate
```

In the `Command` enum, after the `Verify { … }` variant and before `SpecVersion`, add:

```rust
    /// Generate an ed25519 keypair: write the secret key (hex) to <out>, print the public key.
    Keygen {
        /// Path to write the secret key (hex) to.
        out: std::path::PathBuf,
    },
    /// Sign a certificate (ed25519 over its content_hash); print the signed certificate to stdout.
    Sign {
        /// Path to the secret-key hex file.
        #[arg(long)]
        key: std::path::PathBuf,
        /// Path to the certificate JSON file.
        certificate: std::path::PathBuf,
    },
```

- [ ] **Step 2: Add the `Keygen` + `Sign` handler arms**

In `fn main`, after the `Command::Verify { … } => { … }` arm and before `Command::SpecVersion`, add:

```rust
        Command::Keygen { out } => {
            let (secret, public) = rfl_conformance::certificate::generate_keypair();
            if let Err(e) = std::fs::write(&out, &secret) {
                eprintln!("keygen: write {out:?}: {e}");
                std::process::exit(2);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o600));
            }
            println!("public_key {public}");
            eprintln!("secret key written to {}", out.display());
            Ok(())
        }
        Command::Sign { key, certificate } => {
            let cert_text = match std::fs::read_to_string(&certificate) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("sign: read {certificate:?}: {e}");
                    std::process::exit(2);
                }
            };
            let secret = match std::fs::read_to_string(&key) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("sign: read key {key:?}: {e}");
                    std::process::exit(2);
                }
            };
            match rfl_conformance::certificate::sign_certificate(&cert_text, secret.trim()) {
                Ok(signed) => {
                    println!("{signed}");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("sign: {e:#}");
                    std::process::exit(2);
                }
            }
        }
```

- [ ] **Step 3: Extend the `Verify` handler to report the signer**

In the `Command::Verify { certificate } => { … }` arm, replace the `Ok(report) => { … }` block:

```rust
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
```

with:

```rust
                Ok(report) => {
                    if !report.matches {
                        println!(
                            "TAMPERED: declared {} != recomputed {}",
                            report.declared, report.recomputed
                        );
                        std::process::exit(1);
                    }
                    match &report.signature {
                        None => {
                            println!("VERIFIED: content_hash {} matches (unsigned)", report.declared);
                            std::process::exit(0);
                        }
                        Some(s) if s.valid => {
                            println!(
                                "VERIFIED: content_hash {} matches; signed by {} (ed25519)",
                                report.declared, s.public_key
                            );
                            std::process::exit(0);
                        }
                        Some(s) => {
                            println!(
                                "SIGNATURE INVALID: content_hash matches but the signature for {} does not verify",
                                s.public_key
                            );
                            std::process::exit(1);
                        }
                    }
                }
```

- [ ] **Step 4: Build**

Run: `cargo build -p rfl-cli`
Expected: builds clean.

- [ ] **Step 5: Write the CLI smoke test**

Create `crates/rfl-cli/tests/sign_verify_cli.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Smoke test for `rfl keygen` / `rfl sign` / `rfl verify`: a signed certificate verifies and
//! reports the signer; tampering the signature fails.

use std::path::{Path, PathBuf};
use std::process::Command;

fn rfl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rfl"))
}

fn example_cert() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion/certificate.json")
}

#[test]
fn keygen_sign_verify_reports_signer() {
    let tmp = std::env::temp_dir();
    let key = tmp.join(format!("rfl-signer-{}.key", std::process::id()));
    let signed = tmp.join(format!("rfl-signed-{}.json", std::process::id()));

    // keygen -> public_key on stdout, secret in the file.
    let kg = rfl().arg("keygen").arg(&key).output().expect("keygen");
    assert!(kg.status.success(), "keygen exit {:?}", kg.status.code());
    let public = String::from_utf8_lossy(&kg.stdout);
    assert!(public.contains("public_key "), "stdout: {public}");

    // sign the committed certificate -> stdout.
    let sg = rfl()
        .arg("sign")
        .arg("--key")
        .arg(&key)
        .arg(example_cert())
        .output()
        .expect("sign");
    assert!(sg.status.success(), "sign exit {:?}", sg.status.code());
    std::fs::write(&signed, &sg.stdout).unwrap();

    // verify the signed cert -> exit 0 + "signed by".
    let vf = rfl().arg("verify").arg(&signed).output().expect("verify");
    let out = String::from_utf8_lossy(&vf.stdout);
    assert!(vf.status.success(), "verify exit {:?}, stdout: {out}", vf.status.code());
    assert!(out.contains("signed by"), "stdout: {out}");

    // tamper the signature -> exit 1.
    let tampered = String::from_utf8_lossy(&sg.stdout).replacen("\"sig\": \"", "\"sig\": \"0", 1);
    let tampered_path = tmp.join(format!("rfl-tampered-{}.json", std::process::id()));
    std::fs::write(&tampered_path, tampered.as_bytes()).unwrap();
    let vf2 = rfl().arg("verify").arg(&tampered_path).output().expect("verify tampered");
    assert_eq!(vf2.status.code(), Some(1), "stdout: {}", String::from_utf8_lossy(&vf2.stdout));

    for p in [key, signed, tampered_path] {
        std::fs::remove_file(p).ok();
    }
}
```

Note: the tamper inserts a `0` after `"sig": "`, making the sig 129 hex chars — which the schema's `^[0-9a-f]{128}$` rejects, so `verify` exits `2` (malformed), not `1`. To get a clean signature-invalid (`1`), instead replace one interior hex char without changing length. Use a length-preserving mutation in the test: replace the substring after `"sig": "` first char. Implement as: find `"sig": "` then flip the next char between `0`<->`1`. Simpler robust form (use in the test):

```rust
    // length-preserving sig tamper: flip the first hex digit of the signature (0<->1, else ->0).
    let s = String::from_utf8_lossy(&sg.stdout).to_string();
    let marker = "\"sig\": \"";
    let at = s.find(marker).unwrap() + marker.len();
    let mut bytes = s.into_bytes();
    bytes[at] = if bytes[at] == b'0' { b'1' } else { b'0' };
    let tampered = String::from_utf8(bytes).unwrap();
```

Replace the two tamper lines above with this block (it keeps the sig 128 hex chars → schema-valid → signature-invalid → exit 1).

- [ ] **Step 6: Run the CLI smoke**

Run: `cargo test -p rfl-cli --test sign_verify_cli`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/rfl-cli/src/main.rs crates/rfl-cli/tests/sign_verify_cli.rs
git commit -m "feat(cli): rfl keygen + sign; verify reports the signer

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 4: Document signing

**Files:**
- Modify: `docs/certifying-a-driver.md`

- [ ] **Step 1: Update the re-verification recipe to drop `signature`**

In `docs/certifying-a-driver.md`, in the "Re-verifying without the `rfl` binary" subsection, change the jq command `del(.content_hash)` to `del(.content_hash, .signature)` and the Python `d.pop("content_hash")` to also drop `signature`:

```bash
printf '%s' "$(jq -S -c 'del(.content_hash, .signature)' certificate.json)" | shasum -a 256
```

```python
import json, hashlib
d = json.load(open("certificate.json")); d.pop("content_hash", None); d.pop("signature", None)
print("sha256:" + hashlib.sha256(json.dumps(d, sort_keys=True, separators=(",", ":")).encode()).hexdigest())
```

Add a sentence: "An unsigned certificate has no `signature` key, so dropping it is a harmless no-op; a signed certificate's `content_hash` is over the body *excluding* both `content_hash` and `signature`."

- [ ] **Step 2: Add a Signing subsection**

After the "Verifying a certificate" section (before "What a certificate does not claim"), add:

```markdown
## Signing a certificate (optional)

A self-certified certificate carries integrity but no signer. To attach a signer identity, sign it
with an ed25519 key:

​```bash
rfl keygen signer.key                 # writes the secret key to signer.key, prints the public key
rfl sign --key signer.key certificate.json > certificate-signed.json
​```

`rfl sign` adds a top-level `signature` object — `{ "alg": "ed25519", "public_key": "<hex>",
"sig": "<hex>" }` — signing the certificate's `content_hash`. `rfl verify` then reports the signer:

​```text
VERIFIED: content_hash sha256:… matches; signed by <public_key> (ed25519)
​```

`rfl verify` confirms the signature is **valid** and reports **which key** signed. It does **not**
decide whether that key is **trusted** — mapping a public key to an authorized steward is a
governance question (the Tier-2 / Tier-3 regime in
[spec/05-conformance.md](../spec/05-conformance.md)), out of scope for the tool.
```

(The inner ```bash / ```text fences are real fenced blocks in the doc.)

- [ ] **Step 3: Reframe the boundary line**

In "What a certificate does not claim", change the **signer identity** bullet to note that a signature *attests the signer's key* but the tool does not establish *trust* in that key (authorization remains the governance question).

- [ ] **Step 4: Verify the documented commands work**

```bash
cd ~/Documents/GitHub/rfl
cargo run -q -p rfl-cli -- keygen /tmp/s.key
cargo run -q -p rfl-cli -- sign --key /tmp/s.key examples/01-cable-insertion/certificate.json > /tmp/signed.json
cargo run -q -p rfl-cli -- verify /tmp/signed.json ; echo "exit=$?"
printf '%s' "$(jq -S -c 'del(.content_hash, .signature)' /tmp/signed.json)" | shasum -a 256
jq -r '.content_hash' /tmp/signed.json
rm -f /tmp/s.key /tmp/signed.json
```
Expected: `verify` prints `… signed by … (ed25519)` and `exit=0`; the shasum equals the signed cert's `content_hash` hex (the recipe drops `signature`, so the signed cert's hash still reproduces).

- [ ] **Step 5: Commit**

```bash
git add docs/certifying-a-driver.md
git commit -m "docs: document certificate signing (rfl sign / keygen)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 5: Full-suite verification (READ)

- [ ] **Step 1: Full workspace test (READ)**

Run: `cargo test --workspace`
Expected: all green, including the four new signing unit tests + `sign_verify_cli` + the unchanged certify/verify/fixture suites.

- [ ] **Step 2: Clippy (READ — my crates)**

Run: `cargo clippy -p rfl-conformance -p rfl-cli --all-targets --all-features 2>&1 | rg -n "certificate\.rs|main\.rs|sign_verify_cli\.rs" -A2 | head`
Expected: no warnings attributed to the changed files.

- [ ] **Step 3: validate.py (READ)**

Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -3`
Expected: ends `PASS`.

- [ ] **Step 4: End-to-end signed-cert smoke (READ)**

```bash
cd ~/Documents/GitHub/rfl
cargo run -q -p rfl-cli -- keygen /tmp/s.key
cargo run -q -p rfl-cli -- sign --key /tmp/s.key examples/01-cable-insertion/certificate.json > /tmp/signed.json
cargo run -q -p rfl-cli -- verify /tmp/signed.json ; echo "exit=$?"
# integrity still recomputes (recipe drops content_hash + signature):
printf '%s' "$(jq -S -c 'del(.content_hash, .signature)' /tmp/signed.json)" | shasum -a 256 | awk '{print "sha256:"$1}'
jq -r '.content_hash' /tmp/signed.json
rm -f /tmp/s.key /tmp/signed.json
```
Expected: `signed by … (ed25519)`, `exit=0`, and the recomputed hash equals the declared `content_hash`.

----

## Self-Review

**Spec coverage** (design doc → tasks):
- deps + optional `signature` schema → Task 1. ✓
- `recompute_content_hash` drops `signature` → Task 2 Step 2. ✓
- `generate_keypair` / `sign_certificate` / `verify_certificate` signature checking / `SignatureVerdict` → Task 2. ✓
- CLI `keygen` / `sign` / `verify` output + exit codes → Task 3. ✓
- docs signing subsection + recipe `del(.content_hash, .signature)` → Task 4. ✓
- tests (round-trip / tampered sig / body tamper / unsigned / CLI smoke) → Tasks 2–3. ✓
- trust out of scope (verify reports key, not authorization) → Task 3 output + Task 4 boundary. ✓

**Placeholder scan:** no TBD/TODO; every code step shows complete code; the tamper mutation has a length-preserving definitive form (Task 3 Step 5 note). ✓

**Type consistency:** `generate_keypair() -> (String, String)`; `sign_certificate(&str, &str) -> Result<String>`; `verify_certificate(&str) -> Result<VerifyReport>` with `VerifyReport.signature: Option<SignatureVerdict { public_key, valid }>`; CLI reads `report.{matches, declared, recomputed, signature}`. The signed message is `content_hash.as_bytes()` in `sign_message` and `declared.as_bytes()` in `verify_certificate` (identical — `declared` is the cert's `content_hash`). ✓

**Crypto API (confirmed against ed25519-dalek 2.2 docs):** `SigningKey::generate(&mut OsRng)` (rand_core feature, `rand::rngs::OsRng`); `SigningKey::from_bytes(&[u8;32])` (infallible); `sk.sign(msg)` (`Signer`); `sk.verifying_key().to_bytes()`; `VerifyingKey::from_bytes(&[u8;32]) -> Result`; `Signature::from_bytes(&[u8;64])`; `vk.verify_strict(msg, &sig) -> Result`. ✓

**Unaffected:** committed example certs are unsigned → `signature`-drop in recompute is a no-op → `content_hash` unchanged → the fixture guard + the 7 certify + verify tests stay green. ✓
