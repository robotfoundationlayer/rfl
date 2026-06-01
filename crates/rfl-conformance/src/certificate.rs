// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! The `rfl certify` certificate: a deterministic, content-hashed artifact. Inputs are identified
//! by RFL id + content sha256 (never a filesystem path), so the certificate is machine-independent.
//! `content_hash` covers the compact serialization of every field except itself; signing is
//! out-of-band (detached-sign the canonical bytes).

use anyhow::{Context, Result, anyhow};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::EnvelopeClass;

/// The certificate-format schema, embedded so `rfl verify` is self-contained in the binary
/// (the installed `rfl` has no repo checkout to read `schemas/` from).
const CERTIFICATE_SCHEMA: &str = include_str!("../../../schemas/certificate.schema.json");

/// An input file reference: its RFL id + content hash.
#[derive(Serialize)]
pub struct FileRef {
    /// The RFL id (skill name / embodiment id).
    pub id: String,
    /// Lowercase hex sha256 of the file bytes.
    pub sha256: String,
}

/// One obligation's result in the certificate.
#[derive(Serialize)]
pub struct CheckEntry {
    /// The obligation name.
    pub name: &'static str,
    /// `"pass"` or `"fail"`.
    pub result: &'static str,
    /// The failure reason (present only on `fail`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One action's certified result.
#[derive(Serialize)]
pub struct ActionEntry {
    /// The correlated action id.
    pub action_id: String,
    /// The primitive suffix.
    pub suffix: String,
    /// The envelope class verified (absent for perception primitives).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope_class: Option<&'static str>,
    /// The fidelity tier achieved (`spec/05` badge); absent for actions with no confirmation tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fidelity_tier: Option<String>,
    /// Every obligation run for this action.
    pub checks: Vec<CheckEntry>,
    /// True iff every check passed.
    pub passed: bool,
}

/// The certificate body (everything the `content_hash` covers).
#[derive(Serialize)]
pub struct CertificateBody {
    /// The certificate-format version.
    pub certificate_schema_version: &'static str,
    /// The RFL spec version the suite implements.
    pub spec_version: &'static str,
    /// The `rfl` tool version.
    pub tool_version: &'static str,
    /// The certified skill.
    pub skill: FileRef,
    /// The certified embodiment.
    pub embodiment: FileRef,
    /// sha256 of the vendor report bytes.
    pub report_sha256: String,
    /// `"pass"` or `"fail"`.
    pub result: &'static str,
    /// The test classes / dimensions this certificate covers.
    pub covered: Vec<&'static str>,
    /// What it explicitly does NOT cover (honesty boundary).
    pub excluded: Vec<&'static str>,
    /// Per-action results, in retarget order.
    pub actions: Vec<ActionEntry>,
    /// Sequence-level obligations (e.g. momentary_release propagation).
    pub sequence_checks: Vec<CheckEntry>,
}

/// A sealed certificate: the body plus its content hash.
#[derive(Serialize)]
pub struct Certificate {
    /// The hashed body.
    #[serde(flatten)]
    pub body: CertificateBody,
    /// `"sha256:<hex>"` over the compact serialization of `body`.
    pub content_hash: String,
}

/// Lowercase hex sha256 of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// The stable string for an envelope class (certificate vocabulary).
#[must_use]
pub fn envelope_class_str(class: EnvelopeClass) -> &'static str {
    match class {
        EnvelopeClass::TerminalPostcondition => "terminal_postcondition",
        EnvelopeClass::GraspContinuity => "grasp_continuity",
        EnvelopeClass::ForceTrajectory => "force_trajectory",
        EnvelopeClass::IntervalInvariant => "interval_invariant",
    }
}

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

/// Render a sealed certificate as pretty canonical JSON.
#[must_use]
pub fn to_json(cert: &Certificate) -> String {
    serde_json::to_string_pretty(cert).expect("serialize certificate")
}

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
/// excluded from the hashed body — `signature` is added post-seal), then hash the rest in
/// sorted-key canonical form (the same `content_hash_of` `seal` uses).
fn recompute_content_hash(cert: &serde_json::Value) -> String {
    let mut obj = cert.as_object().cloned().unwrap_or_default();
    obj.remove("content_hash");
    obj.remove("signature");
    content_hash_of(&serde_json::Value::Object(obj))
}

/// Generate an ed25519 keypair, returned as `(secret_hex, public_hex)`.
#[must_use]
pub fn generate_keypair() -> (String, String) {
    let sk = SigningKey::generate(&mut OsRng);
    (
        hex::encode(sk.to_bytes()),
        hex::encode(sk.verifying_key().to_bytes()),
    )
}

/// Sign `message` with the hex secret key; returns `(sig_hex, public_hex)`.
fn sign_message(secret_hex: &str, message: &[u8]) -> Result<(String, String)> {
    let secret = hex::decode(secret_hex.trim()).context("secret key is not hex")?;
    let secret: [u8; 32] = secret
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("secret key must be 32 bytes"))?;
    let sk = SigningKey::from_bytes(&secret);
    let sig = sk.sign(message);
    Ok((
        hex::encode(sig.to_bytes()),
        hex::encode(sk.verifying_key().to_bytes()),
    ))
}

/// Verify a hex ed25519 signature over `message` with a hex public key. Any malformed input is a
/// failed verification, never an error.
fn verify_signature(public_hex: &str, message: &[u8], sig_hex: &str) -> bool {
    let Ok(pk_bytes) = hex::decode(public_hex) else {
        return false;
    };
    let Ok(pk_arr): std::result::Result<[u8; 32], _> = pk_bytes.as_slice().try_into() else {
        return false;
    };
    let Ok(vk) = VerifyingKey::from_bytes(&pk_arr) else {
        return false;
    };
    let Ok(sig_bytes) = hex::decode(sig_hex) else {
        return false;
    };
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
    let obj = value
        .as_object_mut()
        .ok_or_else(|| anyhow!("certificate is not a JSON object"))?;
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
        let public_key = s
            .get("public_key")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let sig = s
            .get("sig")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let valid = verify_signature(&public_key, declared.as_bytes(), sig);
        SignatureVerdict { public_key, valid }
    });
    Ok(VerifyReport {
        declared,
        recomputed,
        matches,
        signature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_body(result: &'static str) -> CertificateBody {
        CertificateBody {
            certificate_schema_version: "0.1",
            spec_version: "v0.1-draft",
            tool_version: "0.0.1",
            skill: FileRef {
                id: "cable-insertion".into(),
                sha256: "a".repeat(64),
            },
            embodiment: FileRef {
                id: "allegro".into(),
                sha256: "b".repeat(64),
            },
            report_sha256: "c".repeat(64),
            result,
            covered: vec!["class3_driver_protocol"],
            excluded: vec![
                "class4_physical",
                "class2_loose_epsilon",
                "env3_disturbance",
            ],
            actions: vec![ActionEntry {
                action_id: "cable-insertion/allegro/0002-pinch".into(),
                suffix: "pinch".into(),
                envelope_class: Some("grasp_continuity"),
                fidelity_tier: Some("manifold".to_string()),
                checks: vec![CheckEntry {
                    name: "envelope",
                    result: "pass",
                    reason: None,
                }],
                passed: true,
            }],
            sequence_checks: vec![CheckEntry {
                name: "momentary_release",
                result: "pass",
                reason: None,
            }],
        }
    }

    #[test]
    fn seal_is_deterministic_and_verifiable() {
        let a = to_json(&seal(sample_body("pass")));
        let b = to_json(&seal(sample_body("pass")));
        assert_eq!(a, b, "certificate must be byte-identical across runs");

        // recompute the hash over the sorted-key canonical body and confirm it matches.
        let cert = seal(sample_body("pass"));
        let canonical = serde_json::to_vec(&serde_json::to_value(&cert.body).unwrap()).unwrap();
        assert_eq!(
            cert.content_hash,
            format!("sha256:{}", sha256_hex(&canonical))
        );
        assert!(cert.content_hash.starts_with("sha256:"));
    }

    #[test]
    fn result_change_changes_hash() {
        let pass = seal(sample_body("pass")).content_hash;
        let fail = seal(sample_body("fail")).content_hash;
        assert_ne!(pass, fail);
    }

    #[test]
    fn verify_round_trips_a_sealed_certificate() {
        let json = to_json(&seal(sample_body("pass")));
        let report = verify_certificate(&json).expect("schema-valid certificate");
        assert!(
            report.matches,
            "declared {} != recomputed {}",
            report.declared, report.recomputed
        );
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

    #[test]
    fn sign_then_verify_round_trips() {
        let (secret, public) = generate_keypair();
        let sealed = to_json(&seal(sample_body("pass")));
        let signed = sign_certificate(&sealed, &secret).expect("sign");
        let report = verify_certificate(&signed).expect("verify signed");
        assert!(report.matches, "integrity must hold after signing");
        let sig = report.signature.expect("signature present");
        assert!(sig.valid, "signature must verify");
        assert_eq!(
            sig.public_key, public,
            "embedded public key matches the keypair"
        );
    }

    #[test]
    fn tampered_signature_fails_but_integrity_holds() {
        let (secret, _public) = generate_keypair();
        let sealed = to_json(&seal(sample_body("pass")));
        let signed = sign_certificate(&sealed, &secret).expect("sign");
        // length-preserving sig tamper: flip the first hex digit of the signature.
        let marker = "\"sig\": \"";
        let at = signed.find(marker).expect("sig field") + marker.len();
        let mut bytes = signed.into_bytes();
        bytes[at] = if bytes[at] == b'0' { b'1' } else { b'0' };
        let tampered = String::from_utf8(bytes).unwrap();
        let report = verify_certificate(&tampered).expect("still schema-valid");
        assert!(report.matches, "body integrity is unaffected by a sig edit");
        assert!(
            !report.signature.expect("signature present").valid,
            "tampered sig must fail"
        );
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
}
