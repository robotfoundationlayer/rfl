// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors
#![cfg(unix)]

//! Live-mode (`certify::run_live`) tests via shell-script mock drivers. Unix-only; CI is
//! ubuntu + macos.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rfl_conformance::certify::{self, CertResult};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

/// Write an executable `/bin/sh` mock driver that drains stdin (the execute goals) and then runs
/// `body`. Returns its path.
fn mock_driver(tag: &str, body: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("rfl-mock-driver-{}-{}.sh", std::process::id(), tag));
    std::fs::write(&p, format!("#!/bin/sh\ncat >/dev/null\n{body}\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

#[test]
fn live_driver_emitting_committed_report_certifies_pass() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let report = dir.join("driver-report.jsonl");
    let mock = mock_driver("ok", &format!("cat '{}'", report.display()));

    let outcome =
        certify::run_live(&skill, &emb, &mock, Duration::from_secs(30)).expect("valid live run");
    assert_eq!(outcome.result, CertResult::Pass);
    // live == replay for the same report bytes: the content_hash matches the committed cert.
    let committed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("certificate.json")).unwrap())
            .unwrap();
    assert_eq!(
        committed
            .get("content_hash")
            .and_then(serde_json::Value::as_str),
        Some(outcome.certificate.content_hash.as_str())
    );
    std::fs::remove_file(mock).ok();
}

#[test]
fn live_driver_nonzero_exit_is_invalid_run() {
    let dir = example_dir();
    let mock = mock_driver("fail", "exit 3");
    let r = certify::run_live(
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
        &mock,
        Duration::from_secs(30),
    );
    assert!(r.is_err());
    std::fs::remove_file(mock).ok();
}

#[test]
fn live_driver_malformed_stdout_is_invalid_run() {
    let dir = example_dir();
    let mock = mock_driver("garbage", "echo 'not a json line'");
    let r = certify::run_live(
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
        &mock,
        Duration::from_secs(30),
    );
    assert!(r.is_err());
    std::fs::remove_file(mock).ok();
}

#[test]
fn live_driver_timeout_is_invalid_run_and_returns_promptly() {
    let dir = example_dir();
    let mock = mock_driver("slow", "sleep 30");
    let start = std::time::Instant::now();
    let r = certify::run_live(
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
        &mock,
        Duration::from_secs(1),
    );
    assert!(r.is_err());
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "driver should have been killed promptly"
    );
    std::fs::remove_file(mock).ok();
}
