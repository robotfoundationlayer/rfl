// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! `rfl measure` (the ε-tolerance ingestion path): it consumes N captured
//! driver-report traces for one skill+embodiment and emits a *provisional* table.
//! The honesty firewall is load-bearing: the committed normative
//! `schemas/epsilon-tolerances.yaml` (all `null`) must stay byte-unchanged.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cable_example() -> PathBuf {
    repo_root().join("examples/01-cable-insertion")
}

#[test]
fn measure_emits_a_provisional_table_and_never_touches_the_committed_one() {
    let dir = cable_example();
    let report = dir.join("driver-report.jsonl");
    let committed = repo_root().join("schemas/epsilon-tolerances.yaml");
    let before = std::fs::read(&committed).expect("read committed epsilon table");

    // Two copies of the committed conformant trace: identical runs -> ε = 0 (a true
    // value), exercising the full action_id -> primitive -> quantity pipeline.
    let out = Command::new(env!("CARGO_BIN_EXE_rfl"))
        .arg("measure")
        .arg("--skill")
        .arg(dir.join("skill.yaml"))
        .arg("--embodiment")
        .arg(dir.join("embodiments/allegro.yaml"))
        .arg("--run")
        .arg(&report)
        .arg("--run")
        .arg(&report)
        .output()
        .expect("run rfl measure");
    assert!(
        out.status.success(),
        "rfl measure failed: {:?}",
        out.status.code()
    );
    let stdout = String::from_utf8(out.stdout).unwrap();

    // The honesty firewall + a real mapped primitive from the committed trace.
    assert!(stdout.starts_with("# PROVISIONAL"), "stdout: {stdout}");
    assert!(stdout.contains("provisional: true"));
    assert!(stdout.contains("source: measured"));
    assert!(stdout.contains("force.insert_fit"), "stdout: {stdout}");
    assert!(stdout.contains("final_position"));
    assert!(stdout.contains("tolerance: 0")); // identical runs -> zero, never fabricated

    // The committed normative table is untouched.
    let after = std::fs::read(&committed).expect("re-read committed epsilon table");
    assert_eq!(
        before, after,
        "rfl measure must not modify the committed table"
    );
}

#[test]
fn measure_requires_at_least_two_runs() {
    let dir = cable_example();
    let out = Command::new(env!("CARGO_BIN_EXE_rfl"))
        .arg("measure")
        .arg("--skill")
        .arg(dir.join("skill.yaml"))
        .arg("--embodiment")
        .arg(dir.join("embodiments/allegro.yaml"))
        .arg("--run")
        .arg(dir.join("driver-report.jsonl"))
        .output()
        .expect("run rfl measure");
    assert_eq!(
        out.status.code(),
        Some(2),
        "a single run must be a usage error"
    );
}
