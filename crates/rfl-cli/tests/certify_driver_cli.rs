// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors
#![cfg(unix)]

//! Smoke test for `rfl certify --driver`: a mock driver emitting the committed report certifies
//! pass through the live path. Unix-only (CI is ubuntu + macos).

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

#[test]
fn certify_driver_live_run_exits_zero() {
    let dir = example_dir();
    let report = dir.join("driver-report.jsonl");
    let mock = std::env::temp_dir().join(format!("rfl-cli-mock-{}.sh", std::process::id()));
    std::fs::write(
        &mock,
        format!("#!/bin/sh\ncat >/dev/null\ncat '{}'\n", report.display()),
    )
    .unwrap();
    std::fs::set_permissions(&mock, std::fs::Permissions::from_mode(0o755)).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_rfl"))
        .args(["certify", "--skill"])
        .arg(dir.join("skill.yaml"))
        .arg("--embodiment")
        .arg(dir.join("embodiments/allegro.yaml"))
        .arg("--driver")
        .arg(&mock)
        .output()
        .expect("run rfl certify --driver");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "exit {:?}, stdout: {stdout}",
        out.status.code()
    );
    assert!(stdout.contains("RESULT: PASS"), "stdout: {stdout}");
    std::fs::remove_file(mock).ok();
}
