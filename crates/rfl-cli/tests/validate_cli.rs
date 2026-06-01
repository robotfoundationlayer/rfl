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

#[test]
fn validate_rejects_a_transport_inadmissible_composition() {
    // STB3: a surface_bound grasp.pin followed by a free transport is invalid.
    let p = std::env::temp_dir().join(format!("rfl-stb3-skill-{}.yaml", std::process::id()));
    std::fs::write(
        &p,
        "skill: pin-then-carry\nbody:\n  sequence:\n    - grasp.pin:\n        target: part\n        against_surface: workbench\n        force_budget: 8 N\n    - transport.move_to_pose:\n        target_pose: { ref: dest }\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_rfl"))
        .arg("validate")
        .arg(&p)
        .output()
        .expect("run rfl validate");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "stderr: {stderr}");
    assert!(
        stderr.contains("transport_inadmissible"),
        "stderr: {stderr}"
    );
    std::fs::remove_file(p).ok();
}
