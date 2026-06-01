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
    let verified = rfl_conformance::certificate::verify_certificate(
        &std::fs::read_to_string(&cert_path).unwrap(),
    )
    .expect("committed certificate is schema-valid");
    assert!(verified.matches, "committed certificate.json fails its own content_hash");
}
