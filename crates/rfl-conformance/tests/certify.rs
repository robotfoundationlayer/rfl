// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! End-to-end `certify::run` tests: a nominal report certifies pass; a faulty report certifies
//! fail; broken correlation is an invalid run; the certificate is deterministic.

use std::path::{Path, PathBuf};

use rfl_conformance::certify::{self, CertResult};
use rfl_conformance::{drive, reports_to_jsonl, run_reference_driver, Fault, FaultyDriver};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

/// Write `jsonl` to a unique temp file and return its path.
fn temp_report(tag: &str, jsonl: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("rfl-certify-{}-{}.jsonl", std::process::id(), tag));
    std::fs::write(&p, jsonl).unwrap();
    p
}

#[test]
fn nominal_report_certifies_pass() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let reports = run_reference_driver(&skill, &emb).unwrap();
    let report = temp_report("nominal", &reports_to_jsonl(&reports));

    let outcome = certify::run(&skill, &emb, &report).expect("valid run");
    assert_eq!(outcome.result, CertResult::Pass);
    let body = &outcome.certificate.body;
    assert_eq!(body.result, "pass");
    assert_eq!(body.actions.len(), 8);
    assert_eq!(body.covered, vec!["class3_driver_protocol"]);
    assert!(body.excluded.contains(&"env3_disturbance"));
    assert!(body.excluded.contains(&"class4_physical"));
    std::fs::remove_file(report).ok();
}

#[test]
fn faulty_report_certifies_fail_with_reason() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    // generate a non-conformant report (UnderSecure lowers grasp.pinch securing_force).
    let pairs = drive(FaultyDriver::new(Fault::UnderSecure), &skill, &emb).unwrap();
    let reports: Vec<_> = pairs.into_iter().map(|(_, r)| r).collect();
    let report = temp_report("faulty", &reports_to_jsonl(&reports));

    let outcome = certify::run(&skill, &emb, &report).expect("valid run");
    assert_eq!(outcome.result, CertResult::Fail);
    let pinch = outcome
        .certificate
        .body
        .actions
        .iter()
        .find(|a| a.suffix == "pinch")
        .unwrap();
    assert!(!pinch.passed);
    let env = pinch.checks.iter().find(|c| c.name == "envelope").unwrap();
    assert_eq!(env.result, "fail");
    assert!(env.reason.as_ref().unwrap().contains("securing_force"));
    std::fs::remove_file(report).ok();
}

#[test]
fn missing_status_is_invalid_run() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let reports = run_reference_driver(&skill, &emb).unwrap();
    // drop the status line for the last action -> correlation gap.
    let full = reports_to_jsonl(&reports);
    let last_id = reports.last().unwrap().status.action_id.clone();
    let pruned: String = full
        .lines()
        .filter(|l| !(l.contains("\"status\"") && l.contains(last_id.as_str())))
        .collect::<Vec<_>>()
        .join("\n");
    let report = temp_report("missing", &pruned);

    let err = match certify::run(&skill, &emb, &report) {
        Ok(_) => panic!("expected an invalid run for a missing status"),
        Err(e) => e.to_string(),
    };
    assert!(
        err.contains("no driver report") || err.contains("no terminal status"),
        "got: {err}"
    );
    std::fs::remove_file(report).ok();
}

#[test]
fn certificate_records_per_action_fidelity_tier() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let reports = run_reference_driver(&skill, &emb).unwrap();
    let report = temp_report("fidelity", &reports_to_jsonl(&reports));

    let outcome = certify::run(&skill, &emb, &report).expect("valid run");
    let actions = &outcome.certificate.body.actions;
    // grasp.pinch confirms at manifold tier on allegro (tactile sensing present).
    let pinch = actions.iter().find(|a| a.suffix == "pinch").unwrap();
    assert_eq!(pinch.fidelity_tier.as_deref(), Some("manifold"));
    // a pure reach has no confirmation tier.
    let reach = actions.iter().find(|a| a.suffix == "align" || a.suffix == "retract").unwrap();
    assert_eq!(reach.fidelity_tier, None);
    std::fs::remove_file(report).ok();
}

#[test]
fn certifies_proxy_fidelity_on_pneumatic() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/pneumatic-6f.yaml");
    let reports = run_reference_driver(&skill, &emb).unwrap();
    let report = temp_report("pneumatic", &reports_to_jsonl(&reports));

    let outcome = certify::run(&skill, &emb, &report).expect("valid run");
    assert_eq!(outcome.result, CertResult::Pass, "honest proxy must still pass");
    // pneumatic-6f has no tactile sensing -> grasp.pinch confirms at proxy tier.
    let pinch = outcome.certificate.body.actions.iter().find(|a| a.suffix == "pinch").unwrap();
    assert_eq!(pinch.fidelity_tier.as_deref(), Some("proxy"));
    // and the audit-honesty check passed for it (a proxy claim against a proxy lowering).
    let audit = pinch.checks.iter().find(|c| c.name == "audit_honesty").unwrap();
    assert_eq!(audit.result, "pass");
    std::fs::remove_file(report).ok();
}

#[test]
fn certificate_is_deterministic() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let reports = run_reference_driver(&skill, &emb).unwrap();
    let report = temp_report("determinism", &reports_to_jsonl(&reports));

    let a = certify::run(&skill, &emb, &report).unwrap().certificate.content_hash;
    let b = certify::run(&skill, &emb, &report).unwrap().certificate.content_hash;
    assert_eq!(a, b);
    std::fs::remove_file(report).ok();
}
