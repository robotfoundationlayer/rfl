// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for grasp.precision_tripod: the retarget output is byte-deterministic,
//! matches a committed golden, and every line is a valid driver-interface execute message. The
//! three-point force closure lowers like pinch but declares the rotation_constrained flag. The
//! allegro and leap hands declare grasp.precision_tripod; the pneumatic bellows hand does not, so
//! it is rejected at the capability gate.

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-tripod.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_tripod_allegro() {
    insta::assert_snapshot!("tripod_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_tripod_leap() {
    insta::assert_snapshot!("tripod_leap", jsonl_for("leap"));
}

#[test]
fn tripod_rejected_on_pneumatic_without_capability() {
    let dir = example_dir();
    let err = retarget_example_to_jsonl(
        &dir.join("skill-tripod.yaml"),
        &dir.join("embodiments/pneumatic-6f.yaml"),
    )
    .unwrap_err()
    .to_string();
    assert!(
        err.contains("capability_absent: grasp.precision_tripod"),
        "expected capability gate, got: {err}"
    );
}

#[test]
fn tripod_generation_is_byte_identical() {
    for stem in ["allegro", "leap"] {
        assert_eq!(
            jsonl_for(stem),
            jsonl_for(stem),
            "non-deterministic for {stem}"
        );
    }
}

#[test]
fn tripod_every_line_is_a_valid_execute_message() {
    let schema_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/driver-interface.schema.json");
    let schema: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&schema_path).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource("driver-interface.schema.json", schema)
        .unwrap();
    let idx = compiler
        .compile("driver-interface.schema.json", &mut schemas)
        .unwrap();
    for stem in ["allegro", "leap"] {
        for line in jsonl_for(stem).lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            schemas
                .validate(&v, idx)
                .unwrap_or_else(|e| panic!("{stem}: {e}"));
        }
    }
}
