// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for the remaining force primitives (force.push / pull / scrub),
//! completing the force category (10/10) and the full 51-primitive Skill ISA: the retarget output
//! is byte-deterministic, matches a committed golden, and every line is a valid driver-interface
//! execute message. All three are force-trajectory (the force/torque profile is the envelope).

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-force.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_force_allegro() {
    insta::assert_snapshot!("force_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_force_leap() {
    insta::assert_snapshot!("force_leap", jsonl_for("leap"));
}

#[test]
fn golden_force_pneumatic() {
    insta::assert_snapshot!("force_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn force_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(
            jsonl_for(stem),
            jsonl_for(stem),
            "non-deterministic for {stem}"
        );
    }
}

#[test]
fn force_every_line_is_a_valid_execute_message() {
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
