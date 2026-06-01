// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Guards the committed example fixtures (driver-report*.jsonl + certificate*.json). Run with
//! `RFL_BLESS=1` to (re)generate them; otherwise asserts each is current and verifies.

use std::path::Path;

struct Case {
    dir: &'static str,
    skill: &'static str,
    embodiment: &'static str,
    report: &'static str,
    cert: &'static str,
}

const CASES: &[Case] = &[
    Case {
        dir: "01-cable-insertion",
        skill: "skill.yaml",
        embodiment: "allegro.yaml",
        report: "driver-report.jsonl",
        cert: "certificate.json",
    },
    Case {
        dir: "01-cable-insertion",
        skill: "skill.yaml",
        embodiment: "pneumatic-6f.yaml",
        report: "driver-report-pneumatic.jsonl",
        cert: "certificate-pneumatic.json",
    },
];

#[test]
fn example_fixtures_are_current() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let bless = std::env::var("RFL_BLESS").is_ok();
    for c in CASES {
        let dir = examples.join(c.dir);
        let skill = dir.join(c.skill);
        let emb = dir.join("embodiments").join(c.embodiment);
        let report = rfl_conformance::reports_to_jsonl(
            &rfl_conformance::run_reference_driver(&skill, &emb).unwrap(),
        );
        let report_path = dir.join(c.report);
        let cert_path = dir.join(c.cert);

        if bless {
            std::fs::write(&report_path, &report).unwrap();
            let cert =
                rfl_conformance::certify::run(&skill, &emb, &report_path).unwrap().certificate;
            std::fs::write(&cert_path, rfl_conformance::certificate::to_json(&cert)).unwrap();
            continue;
        }

        let committed_report = std::fs::read_to_string(&report_path)
            .unwrap_or_else(|_| panic!("{} present (run RFL_BLESS=1 to generate)", c.report));
        assert_eq!(committed_report, report, "{} stale; run RFL_BLESS=1 cargo test", c.report);

        let fresh = rfl_conformance::certify::run(&skill, &emb, &report_path).unwrap().certificate;
        let committed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cert_path).unwrap()).unwrap();
        assert_eq!(
            committed.get("content_hash").and_then(serde_json::Value::as_str),
            Some(fresh.content_hash.as_str()),
            "{} stale; run RFL_BLESS=1 cargo test",
            c.cert
        );

        let verified = rfl_conformance::certificate::verify_certificate(
            &std::fs::read_to_string(&cert_path).unwrap(),
        )
        .unwrap_or_else(|e| panic!("{} not schema-valid: {e}", c.cert));
        assert!(verified.matches, "{} fails its own content_hash", c.cert);
    }
}
