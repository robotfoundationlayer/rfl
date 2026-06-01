// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for grasp.power: the retarget output is byte-deterministic, matches a
//! committed golden, and every line is a valid driver-interface execute message. The whole-volume
//! force-closure power grasp lowers like pinch over the power payload; all three reference hands
//! declare grasp.power.

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-power.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_power_allegro() {
    insta::assert_snapshot!("power_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_power_leap() {
    insta::assert_snapshot!("power_leap", jsonl_for("leap"));
}

#[test]
fn golden_power_pneumatic() {
    insta::assert_snapshot!("power_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn power_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn power_every_line_is_a_valid_execute_message() {
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
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        for line in jsonl_for(stem).lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            schemas
                .validate(&v, idx)
                .unwrap_or_else(|e| panic!("{stem}: {e}"));
        }
    }
}
