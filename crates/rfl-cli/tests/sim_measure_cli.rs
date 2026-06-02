// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! The stochastic reference sim → measure loop: `rfl sim --seed --variation` over N seeds
//! produces varied traces, and `rfl measure` over them yields a *non-zero* provisional ε —
//! the meaningful-ε proof the deterministic sim (ε = 0) cannot give. The committed normative
//! `epsilon-tolerances.yaml` must stay byte-unchanged.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rfl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rfl"))
}

#[test]
fn stochastic_sim_then_measure_yields_nonzero_provisional_epsilon() {
    let dir = repo_root().join("examples/01-cable-insertion");
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let decl = repo_root().join("schemas/simulator-declaration.yaml");
    let committed = repo_root().join("schemas/epsilon-tolerances.yaml");
    let before = std::fs::read(&committed).expect("read committed table");

    // Generate three varied runs (seeds 0..3).
    let mut runs = Vec::new();
    for seed in 0..3u64 {
        let out = rfl()
            .arg("sim")
            .arg("--skill")
            .arg(&skill)
            .arg("--embodiment")
            .arg(&emb)
            .arg("--variation")
            .arg(&decl)
            .arg("--seed")
            .arg(seed.to_string())
            .output()
            .expect("run rfl sim");
        assert!(out.status.success(), "rfl sim seed {seed} failed");
        let p = std::env::temp_dir().join(format!("rfl-sim-{}-{seed}.jsonl", std::process::id()));
        std::fs::write(&p, out.stdout).unwrap();
        runs.push(p);
    }

    let mut cmd = rfl();
    cmd.arg("measure")
        .arg("--skill")
        .arg(&skill)
        .arg("--embodiment")
        .arg(&emb);
    for r in &runs {
        cmd.arg("--run").arg(r);
    }
    let out = cmd.output().expect("run rfl measure");
    assert!(out.status.success(), "rfl measure failed");
    let stdout = String::from_utf8(out.stdout).unwrap();

    // A meaningful provisional ε: at least one tolerance is a non-zero number (a decimal
    // point distinguishes "0.5192…" from "tolerance: 0" / "tolerance: null").
    let has_nonzero = stdout
        .lines()
        .any(|l| l.trim_start().starts_with("tolerance:") && l.contains('.'));
    assert!(
        has_nonzero,
        "expected a non-zero provisional tolerance:\n{stdout}"
    );

    // The committed normative table is untouched.
    let after = std::fs::read(&committed).expect("re-read committed table");
    assert_eq!(before, after, "the committed epsilon table must stay null");

    for r in runs {
        std::fs::remove_file(r).ok();
    }
}

#[test]
fn sim_seed_requires_variation() {
    let dir = repo_root().join("examples/01-cable-insertion");
    let out = rfl()
        .arg("sim")
        .arg("--skill")
        .arg(dir.join("skill.yaml"))
        .arg("--embodiment")
        .arg(dir.join("embodiments/allegro.yaml"))
        .arg("--seed")
        .arg("0")
        .output()
        .expect("run rfl sim");
    assert_eq!(
        out.status.code(),
        Some(2),
        "--seed without --variation is a usage error"
    );
}
