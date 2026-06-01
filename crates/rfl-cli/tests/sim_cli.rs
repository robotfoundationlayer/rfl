// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! `rfl sim` (the reference simulator driver): its emitted driver report is
//! conformant — feeding it straight back to `rfl certify --report` passes, for a
//! skill with no committed report, proving the supply-side reference generates
//! (not replays) reports for arbitrary skills.

use std::path::{Path, PathBuf};
use std::process::Command;

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn sim_then_certify(skill: &Path, emb: &Path) {
    // rfl sim --skill --embodiment  ->  a driver-report JSONL.
    let sim = Command::new(env!("CARGO_BIN_EXE_rfl"))
        .args(["sim", "--skill"])
        .arg(skill)
        .arg("--embodiment")
        .arg(emb)
        .output()
        .expect("run rfl sim");
    assert!(
        sim.status.success(),
        "rfl sim failed: {:?}",
        sim.status.code()
    );
    assert!(!sim.stdout.is_empty(), "rfl sim emitted no report");

    let report = std::env::temp_dir().join(format!(
        "rfl-sim-{}-{}.jsonl",
        std::process::id(),
        skill.file_stem().unwrap().to_string_lossy()
    ));
    std::fs::write(&report, &sim.stdout).unwrap();

    // rfl certify --report <that>  ->  PASS.
    let certify = Command::new(env!("CARGO_BIN_EXE_rfl"))
        .args(["certify", "--skill"])
        .arg(skill)
        .arg("--embodiment")
        .arg(emb)
        .arg("--report")
        .arg(&report)
        .output()
        .expect("run rfl certify");
    let stdout = String::from_utf8_lossy(&certify.stdout);
    assert!(
        certify.status.success(),
        "certify exit {:?}: {stdout}",
        certify.status.code()
    );
    assert!(stdout.contains("RESULT: PASS"), "stdout: {stdout}");
    std::fs::remove_file(report).ok();
}

#[test]
fn sim_report_certifies_for_the_cable_example() {
    let d = examples().join("01-cable-insertion");
    sim_then_certify(&d.join("skill.yaml"), &d.join("embodiments/allegro.yaml"));
}

#[test]
fn sim_report_certifies_for_a_skill_with_no_committed_report() {
    // skill-power has no committed driver-report; the reference sim generates one.
    let d = examples().join("03-screw-fasten");
    sim_then_certify(
        &d.join("skill-power.yaml"),
        &d.join("embodiments/leap.yaml"),
    );
}
