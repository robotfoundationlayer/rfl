// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Smoke test for `rfl validate`: a valid skill is accepted; a malformed one is rejected.

use std::path::{Path, PathBuf};
use std::process::Command;

fn skill() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion/skill.yaml")
}

#[test]
fn validate_accepts_a_valid_skill() {
    let out = Command::new(env!("CARGO_BIN_EXE_rfl"))
        .arg("validate")
        .arg(skill())
        .output()
        .expect("run rfl validate");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "exit {:?}, stdout: {stdout}",
        out.status.code()
    );
    assert!(
        stdout.contains("VALID: skill 'cable-insertion'"),
        "stdout: {stdout}"
    );
}

#[test]
fn validate_rejects_a_malformed_skill() {
    let p = std::env::temp_dir().join(format!("rfl-bad-skill-{}.yaml", std::process::id()));
    std::fs::write(&p, "not: a skill\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_rfl"))
        .arg("validate")
        .arg(&p)
        .output()
        .expect("run rfl validate");
    assert_eq!(
        out.status.code(),
        Some(2),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::remove_file(p).ok();
}
