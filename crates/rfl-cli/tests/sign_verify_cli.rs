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
    let tampered_path = tmp.join(format!("rfl-tampered-{}.json", std::process::id()));

    // keygen -> public_key on stdout, secret in the file.
    let kg = rfl().arg("keygen").arg(&key).output().expect("keygen");
    assert!(kg.status.success(), "keygen exit {:?}", kg.status.code());
    let public = String::from_utf8_lossy(&kg.stdout);
    assert!(public.contains("public_key "), "stdout: {public}");

    // sign the committed certificate -> stdout.
    let sg = rfl().arg("sign").arg("--key").arg(&key).arg(example_cert()).output().expect("sign");
    assert!(sg.status.success(), "sign exit {:?}", sg.status.code());
    std::fs::write(&signed, &sg.stdout).unwrap();

    // verify the signed cert -> exit 0 + "signed by".
    let vf = rfl().arg("verify").arg(&signed).output().expect("verify");
    let out = String::from_utf8_lossy(&vf.stdout);
    assert!(vf.status.success(), "verify exit {:?}, stdout: {out}", vf.status.code());
    assert!(out.contains("signed by"), "stdout: {out}");

    // length-preserving sig tamper: flip the first hex digit of the signature -> exit 1.
    let s = String::from_utf8(sg.stdout).unwrap();
    let marker = "\"sig\": \"";
    let at = s.find(marker).expect("sig field") + marker.len();
    let mut bytes = s.into_bytes();
    bytes[at] = if bytes[at] == b'0' { b'1' } else { b'0' };
    std::fs::write(&tampered_path, &bytes).unwrap();
    let vf2 = rfl().arg("verify").arg(&tampered_path).output().expect("verify tampered");
    assert_eq!(vf2.status.code(), Some(1), "stdout: {}", String::from_utf8_lossy(&vf2.stdout));

    for p in [key, signed, tampered_path] {
        std::fs::remove_file(p).ok();
    }
}
