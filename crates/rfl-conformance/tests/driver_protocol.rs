// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 3 (`spec/05` § Four test classes): the driver-protocol
//! round-trip. A driver consumes the `execute` messages and reports back in-protocol
//! (`telemetry` + `status`), schema-valid, action-correlated, and deterministic.
//! C1 verifies the round-trip; the envelope-class checkers + adversarial drivers are
//! C2.

use rfl_conformance::{reports_to_jsonl, run_reference_driver};
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

fn reports_jsonl(stem: &str) -> String {
    let dir = example_dir();
    let reports = run_reference_driver(
        &dir.join("skill.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("drive");
    reports_to_jsonl(&reports)
}

#[test]
fn golden_allegro() {
    insta::assert_snapshot!("driver_allegro", reports_jsonl("allegro"));
}

#[test]
fn golden_leap() {
    insta::assert_snapshot!("driver_leap", reports_jsonl("leap"));
}

#[test]
fn golden_pneumatic() {
    insta::assert_snapshot!("driver_pneumatic", reports_jsonl("pneumatic-6f"));
}

#[test]
fn report_stream_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(reports_jsonl(stem), reports_jsonl(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn all_actions_correlate_and_succeed() {
    let dir = example_dir();
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        let reports = run_reference_driver(
            &dir.join("skill.yaml"),
            &dir.join(format!("embodiments/{stem}.yaml")),
        )
        .expect("drive");
        assert_eq!(reports.len(), 8, "stem {stem}");
        for r in &reports {
            assert!(matches!(r.status.outcome, rfl_core::driver::Outcome::Succeeded), "stem {stem}");
            for t in &r.telemetry {
                assert_eq!(t.action_id, r.status.action_id, "stem {stem}");
            }
        }
    }
}

#[test]
fn fidelity_tier_echoes_tactile_degradation() {
    let dir = example_dir();
    // allegro + leap declare tactile sensing -> manifold; pneumatic-6f does not -> proxy.
    for (stem, tier) in [("allegro", "manifold"), ("leap", "manifold"), ("pneumatic-6f", "proxy")] {
        let reports = run_reference_driver(
            &dir.join("skill.yaml"),
            &dir.join(format!("embodiments/{stem}.yaml")),
        )
        .expect("drive");
        // action index 1 is grasp.pinch (the tactile-confirmed action).
        assert_eq!(reports[1].status.fidelity_tier.as_deref(), Some(tier), "stem {stem}");
    }
}

#[test]
fn every_report_message_is_schema_valid() {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/driver-interface.schema.json");
    let schema: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&schema_path).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource("driver-interface.schema.json", schema).unwrap();
    let idx = compiler.compile("driver-interface.schema.json", &mut schemas).unwrap();

    let dir = example_dir();
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        let reports = run_reference_driver(
            &dir.join("skill.yaml"),
            &dir.join(format!("embodiments/{stem}.yaml")),
        )
        .expect("drive");
        for r in &reports {
            for t in &r.telemetry {
                let v = serde_json::to_value(t).unwrap();
                schemas.validate(&v, idx).unwrap_or_else(|e| panic!("{stem} telemetry: {e}"));
            }
            let v = serde_json::to_value(&r.status).unwrap();
            schemas.validate(&v, idx).unwrap_or_else(|e| panic!("{stem} status: {e}"));
        }
    }
}

#[test]
fn telemetry_with_station_error_is_schema_valid() {
    use rfl_core::driver::{RealizedPose, Telemetry};
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/driver-interface.schema.json");
    let schema: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&schema_path).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource("driver-interface.schema.json", schema).unwrap();
    let idx = compiler.compile("driver-interface.schema.json", &mut schemas).unwrap();

    let t = Telemetry {
        message: "telemetry",
        action_id: "s/e/0001-hover".to_string(),
        t: 1.0,
        realized_pose: Some(RealizedPose::placeholder()),
        wrench: None,
        securing_force: None,
        station_error: Some(rfl_core::quantity::Quantity("1 mm".to_string())),
        tactile: vec![],
        events: vec![],
        fidelity_tier: None,
    };
    let v = serde_json::to_value(&t).unwrap();
    schemas.validate(&v, idx).expect("telemetry with station_error must be schema-valid");
}

#[test]
fn telemetry_with_detent_event_is_schema_valid() {
    use rfl_core::driver::{RealizedPose, Telemetry};
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/driver-interface.schema.json");
    let schema: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&schema_path).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource("driver-interface.schema.json", schema).unwrap();
    let idx = compiler.compile("driver-interface.schema.json", &mut schemas).unwrap();

    let t = Telemetry {
        message: "telemetry",
        action_id: "s/e/0001-press_button".to_string(),
        t: 1.0,
        realized_pose: Some(RealizedPose::placeholder()),
        wrench: None,
        securing_force: None,
        station_error: None,
        tactile: vec![],
        events: vec![serde_json::json!({ "kind": "detent" })],
        fidelity_tier: None,
    };
    let v = serde_json::to_value(&t).unwrap();
    schemas.validate(&v, idx).expect("a detent ForceEvent object must be schema-valid");
}
