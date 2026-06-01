// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! `rfl` — the Robot Foundation Layer command-line interface.
//!
//! Subcommands:
//! - `rfl validate <skill.yaml>` — parse and validate a Skill ISA composition
//! - `rfl retarget <skill.yaml> --embodiment <descriptor.yaml>` — print the
//!   resolved canonical actions for the target embodiment
//! - `rfl conformance --driver <binary>` — run the conformance test suite
//! - `rfl certify --skill <s> --embodiment <e> --report <j>` — certify a vendor
//!   driver report (JSONL replay) against the Class 3 driver-protocol obligations
//! - `rfl verify <certificate.json>` — schema-validate a certificate and re-verify its
//!   content hash (tamper detection)

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rfl", version, about = "Robot Foundation Layer CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse and validate a Skill ISA composition.
    Validate {
        /// Path to the skill YAML file.
        path: std::path::PathBuf,
    },
    /// Retarget a skill onto a specific embodiment.
    Retarget {
        /// Path to the skill YAML file.
        skill: std::path::PathBuf,
        /// Path to the embodiment descriptor YAML file.
        #[arg(long)]
        embodiment: std::path::PathBuf,
    },
    /// Run the conformance test suite against a driver implementation.
    Conformance {
        /// Path to the driver binary to test.
        #[arg(long)]
        driver: std::path::PathBuf,
    },
    /// Certify a vendor driver report (JSONL replay) against the Class 3 driver-protocol
    /// obligations and emit a deterministic certificate.
    Certify {
        /// Path to the skill YAML file.
        #[arg(long)]
        skill: std::path::PathBuf,
        /// Path to the embodiment descriptor YAML file.
        #[arg(long)]
        embodiment: std::path::PathBuf,
        /// Path to the captured driver-report JSONL (telemetry + status lines).
        #[arg(long)]
        report: std::path::PathBuf,
        /// Optional path to write the canonical certificate JSON.
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    /// Schema-validate a certificate and re-verify its content hash (tamper detection).
    Verify {
        /// Path to the certificate JSON file.
        certificate: std::path::PathBuf,
    },
    /// Print the specification version this CLI implements.
    SpecVersion,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { path } => {
            anyhow::bail!("validate not yet implemented (target: spec v0.1, 2027 Q1) — {path:?}");
        }
        Command::Retarget { skill, embodiment } => {
            let skill_text = std::fs::read_to_string(&skill)
                .map_err(|e| anyhow::anyhow!("read skill {skill:?}: {e}"))?;
            let emb_text = std::fs::read_to_string(&embodiment)
                .map_err(|e| anyhow::anyhow!("read embodiment {embodiment:?}: {e}"))?;
            let parsed_skill = rfl_core::skill_isa::Skill::parse_yaml(&skill_text)?;
            let emb = rfl_core::embodiment::Embodiment::parse_yaml(&emb_text)?;
            let out = rfl_core::translation::retarget(&parsed_skill, &emb)?;
            let jsonl =
                rfl_core::canonical::to_jsonl(&parsed_skill.skill, &emb.id, &out.actions, &out.suffixes);
            print!("{jsonl}");
            Ok(())
        }
        Command::Conformance { driver } => {
            anyhow::bail!(
                "conformance not yet implemented (target: spec v0.1, 2027 Q1) — driver {driver:?}"
            );
        }
        Command::Certify { skill, embodiment, report, out } => {
            let outcome = match rfl_conformance::certify::run(&skill, &embodiment, &report) {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("certify: invalid run: {e:#}");
                    std::process::exit(2);
                }
            };
            let cert = &outcome.certificate;
            let body = &cert.body;
            println!(
                "RFL conformance certificate — {} on {} (spec {})",
                body.skill.id, body.embodiment.id, body.spec_version
            );
            for a in &body.actions {
                let mark = if a.passed { "PASS" } else { "FAIL" };
                let class = a.envelope_class.unwrap_or("perception");
                println!("  [{mark}] {} ({class})", a.action_id);
                for c in &a.checks {
                    if let Some(reason) = &c.reason {
                        println!("        - {} FAIL: {reason}", c.name);
                    }
                }
            }
            for c in &body.sequence_checks {
                let mark = if c.result == "pass" { "PASS" } else { "FAIL" };
                println!("  [{mark}] sequence:{}", c.name);
                if let Some(reason) = &c.reason {
                    println!("        - {reason}");
                }
            }
            println!(
                "RESULT: {} ({} covered, env3/class4 excluded) — {}",
                body.result.to_uppercase(),
                body.covered.join("+"),
                cert.content_hash
            );
            if let Some(path) = out {
                if let Err(e) = std::fs::write(&path, rfl_conformance::certificate::to_json(cert)) {
                    eprintln!("certify: write {path:?}: {e}");
                    std::process::exit(2);
                }
                println!("certificate -> {}", path.display());
            }
            match outcome.result {
                rfl_conformance::certify::CertResult::Pass => std::process::exit(0),
                rfl_conformance::certify::CertResult::Fail => std::process::exit(1),
            }
        }
        Command::Verify { certificate } => {
            let text = match std::fs::read_to_string(&certificate) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("verify: read {certificate:?}: {e}");
                    std::process::exit(2);
                }
            };
            match rfl_conformance::certificate::verify_certificate(&text) {
                Err(e) => {
                    eprintln!("verify: malformed certificate: {e:#}");
                    std::process::exit(2);
                }
                Ok(report) => {
                    if report.matches {
                        println!("VERIFIED: content_hash {} matches", report.declared);
                        std::process::exit(0);
                    }
                    println!(
                        "TAMPERED: declared {} != recomputed {}",
                        report.declared, report.recomputed
                    );
                    std::process::exit(1);
                }
            }
        }
        Command::SpecVersion => {
            println!("{}", rfl_core::SPEC_VERSION);
            Ok(())
        }
    }
}
