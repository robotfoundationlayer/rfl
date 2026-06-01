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
