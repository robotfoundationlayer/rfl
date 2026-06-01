// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Smoke test for `rfl certify`: exit code + summary on the committed cable-insertion example.

use std::path::{Path, PathBuf};
use std::process::Command;

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

#[test]
fn certify_nominal_run_exits_zero() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    // generate a nominal JSONL report with the reference driver.
    let reports = rfl_conformance::run_reference_driver(&skill, &emb).unwrap();
    let jsonl = rfl_conformance::reports_to_jsonl(&reports);
    let report = std::env::temp_dir().join(format!("rfl-cli-certify-{}.jsonl", std::process::id()));
    std::fs::write(&report, jsonl).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_rfl"))
        .args(["certify", "--skill"])
        .arg(&skill)
        .arg("--embodiment")
        .arg(&emb)
        .arg("--report")
        .arg(&report)
        .output()
        .expect("run rfl certify");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "exit {:?}, stdout: {stdout}", out.status.code());
    assert!(stdout.contains("RESULT: PASS"), "stdout: {stdout}");
    std::fs::remove_file(report).ok();
}
