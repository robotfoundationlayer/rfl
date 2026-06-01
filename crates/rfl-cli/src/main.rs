// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! `rfl` — the Robot Foundation Layer command-line interface.
//!
//! Subcommands:
//! - `rfl validate <skill.yaml>` — parse and validate a Skill ISA composition
//! - `rfl retarget <skill.yaml> --embodiment <descriptor.yaml>` — print the
//!   resolved canonical actions for the target embodiment
//! - `rfl certify --skill <s> --embodiment <e> (--report <j> | --driver <bin>)` — certify a
//!   vendor driver against the Class 3 driver-protocol obligations (replay a captured report, or
//!   spawn the driver live)
//! - `rfl verify <certificate.json>` — schema-validate a certificate and re-verify its
//!   content hash (tamper detection)
//! - `rfl keygen <out>` — generate an ed25519 keypair (secret to `<out>`, public to stdout)
//! - `rfl sign --key <secret> <certificate.json>` — attach an ed25519 signature to a certificate
//! - `rfl badge <certificate.json>` — derive a conformance badge (regime + fidelity tier + trademark gate)
//! - `rfl sim --skill <s> --embodiment <e>` — the reference simulator driver: retarget + execute,
//!   emitting a conformant driver-report JSONL (the supply-side reference; feed it to `rfl certify --report`)
//! - `rfl measure --skill <s> --embodiment <e> --run <t> --run <t> …` — measure a provisional
//!   ε-tolerance table from N captured driver-report traces (never the committed normative table)

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
    /// Certify a vendor driver (replay via --report, or live via --driver) against the Class 3
    /// driver-protocol obligations and emit a deterministic certificate.
    Certify {
        /// Path to the skill YAML file.
        #[arg(long)]
        skill: std::path::PathBuf,
        /// Path to the embodiment descriptor YAML file.
        #[arg(long)]
        embodiment: std::path::PathBuf,
        /// Path to a captured driver-report JSONL (replay mode). Mutually exclusive with --driver.
        #[arg(long)]
        report: Option<std::path::PathBuf>,
        /// Path to a driver binary to spawn (live mode). Mutually exclusive with --report.
        #[arg(long)]
        driver: Option<std::path::PathBuf>,
        /// Timeout in seconds for live (--driver) mode.
        #[arg(long, default_value_t = 30)]
        timeout: u64,
        /// Optional path to write the canonical certificate JSON.
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    /// Schema-validate a certificate and re-verify its content hash (tamper detection).
    Verify {
        /// Path to the certificate JSON file.
        certificate: std::path::PathBuf,
    },
    /// Generate an ed25519 keypair: write the secret key (hex) to `<out>`, print the public key.
    Keygen {
        /// Path to write the secret key (hex) to.
        out: std::path::PathBuf,
    },
    /// Sign a certificate (ed25519 over its content_hash); print the signed certificate to stdout.
    Sign {
        /// Path to the secret-key hex file.
        #[arg(long)]
        key: std::path::PathBuf,
        /// Path to the certificate JSON file.
        certificate: std::path::PathBuf,
    },
    /// Derive a conformance badge from a certificate (regime + fidelity tier + trademark gate).
    Badge {
        /// Path to the certificate JSON file.
        certificate: std::path::PathBuf,
    },
    /// The reference simulator driver. With --skill/--embodiment it retargets and executes,
    /// emitting a conformant driver-report JSONL. With neither, it reads canonical execute goals
    /// from stdin (the live `--driver` protocol) and emits the report for those goals.
    Sim {
        /// Path to the skill YAML file (regenerate mode). Omit both for stdin mode.
        #[arg(long)]
        skill: Option<std::path::PathBuf>,
        /// Path to the embodiment descriptor YAML file (regenerate mode).
        #[arg(long)]
        embodiment: Option<std::path::PathBuf>,
    },
    /// Measure a provisional epsilon-tolerance table from N captured driver-report traces
    /// for one skill+embodiment. Emits a clearly-marked provisional document; never the
    /// committed normative table.
    Measure {
        /// Path to the skill YAML file.
        #[arg(long)]
        skill: std::path::PathBuf,
        /// Path to the embodiment descriptor YAML file.
        #[arg(long)]
        embodiment: std::path::PathBuf,
        /// Path to a captured driver-report JSONL trace. Repeatable; at least two are required
        /// (a single run has no run-to-run variation).
        #[arg(long = "run", required = true)]
        runs: Vec<std::path::PathBuf>,
        /// The percentile (in [0, 1]) for the epsilon candidate.
        #[arg(long, default_value_t = 0.95)]
        percentile: f64,
        /// The safety factor applied to the percentile.
        #[arg(long, default_value_t = 1.2)]
        safety: f64,
        /// Optional path to write the provisional table to (default: stdout).
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    /// Print the specification version this CLI implements.
    SpecVersion,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { path } => {
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("validate: read {path:?}: {e}");
                    std::process::exit(2);
                }
            };
            let skill = match rfl_core::skill_isa::Skill::parse_yaml(&text) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("validate: invalid skill: {e}");
                    std::process::exit(2);
                }
            };
            if let Err(e) = skill.validate() {
                eprintln!("validate: invalid composition: {e}");
                std::process::exit(2);
            }
            let stmts = &skill.body.sequence;
            let lets = stmts
                .iter()
                .filter(|s| matches!(s, rfl_core::skill_isa::Statement::LetBind(_)))
                .count();
            println!(
                "VALID: skill '{}' — {} statement(s) ({} primitive call(s), {} let-bind(s))",
                skill.skill,
                stmts.len(),
                stmts.len() - lets,
                lets
            );
            std::process::exit(0);
        }
        Command::Retarget { skill, embodiment } => {
            let skill_text = std::fs::read_to_string(&skill)
                .map_err(|e| anyhow::anyhow!("read skill {skill:?}: {e}"))?;
            let emb_text = std::fs::read_to_string(&embodiment)
                .map_err(|e| anyhow::anyhow!("read embodiment {embodiment:?}: {e}"))?;
            let parsed_skill = rfl_core::skill_isa::Skill::parse_yaml(&skill_text)?;
            let emb = rfl_core::embodiment::Embodiment::parse_yaml(&emb_text)?;
            let out = rfl_core::translation::retarget(&parsed_skill, &emb)?;
            let jsonl = rfl_core::canonical::to_jsonl(
                &parsed_skill.skill,
                &emb.id,
                &out.actions,
                &out.suffixes,
            );
            print!("{jsonl}");
            Ok(())
        }
        Command::Certify {
            skill,
            embodiment,
            report,
            driver,
            timeout,
            out,
        } => {
            let result = match (report, driver) {
                (Some(r), None) => rfl_conformance::certify::run(&skill, &embodiment, &r),
                (None, Some(d)) => rfl_conformance::certify::run_live(
                    &skill,
                    &embodiment,
                    &d,
                    std::time::Duration::from_secs(timeout),
                ),
                _ => {
                    eprintln!("certify: exactly one of --report or --driver is required");
                    std::process::exit(2);
                }
            };
            let outcome = match result {
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
                    if !report.matches {
                        println!(
                            "TAMPERED: declared {} != recomputed {}",
                            report.declared, report.recomputed
                        );
                        std::process::exit(1);
                    }
                    match &report.signature {
                        None => {
                            println!(
                                "VERIFIED: content_hash {} matches (unsigned)",
                                report.declared
                            );
                            std::process::exit(0);
                        }
                        Some(s) if s.valid => {
                            println!(
                                "VERIFIED: content_hash {} matches; signed by {} (ed25519)",
                                report.declared, s.public_key
                            );
                            std::process::exit(0);
                        }
                        Some(s) => {
                            println!(
                                "SIGNATURE INVALID: content_hash matches but the signature for {} does not verify",
                                s.public_key
                            );
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        Command::Keygen { out } => {
            let (secret, public) = rfl_conformance::certificate::generate_keypair();
            if let Err(e) = std::fs::write(&out, &secret) {
                eprintln!("keygen: write {out:?}: {e}");
                std::process::exit(2);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o600));
            }
            println!("public_key {public}");
            eprintln!("secret key written to {}", out.display());
            Ok(())
        }
        Command::Sign { key, certificate } => {
            let cert_text = match std::fs::read_to_string(&certificate) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("sign: read {certificate:?}: {e}");
                    std::process::exit(2);
                }
            };
            let secret = match std::fs::read_to_string(&key) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("sign: read key {key:?}: {e}");
                    std::process::exit(2);
                }
            };
            match rfl_conformance::certificate::sign_certificate(&cert_text, secret.trim()) {
                Ok(signed) => {
                    println!("{signed}");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("sign: {e:#}");
                    std::process::exit(2);
                }
            }
        }
        Command::Badge { certificate } => {
            let text = match std::fs::read_to_string(&certificate) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("badge: read {certificate:?}: {e}");
                    std::process::exit(2);
                }
            };
            let badge = match rfl_conformance::badge::derive_badge(&text) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("badge: malformed certificate: {e:#}");
                    std::process::exit(2);
                }
            };
            println!(
                "RFL conformance badge — {} on {}",
                badge.skill, badge.embodiment
            );
            println!("  result: {}", badge.result.to_uppercase());
            println!(
                "  regime tier: Tier {} (self-certification)",
                badge.regime_tier
            );
            println!(
                "  achieved fidelity: {}",
                badge.achieved_fidelity.as_deref().unwrap_or("n/a")
            );
            println!(
                "  RFL(TM) trademark: {}",
                if badge.trademark_permitted {
                    "permitted"
                } else {
                    "not permitted (Tier 1 self-certification)"
                }
            );
            for a in &badge.actions {
                let mark = if a.passed { "PASS" } else { "FAIL" };
                let class = a.envelope_class.as_deref().unwrap_or("perception");
                let tier = a.fidelity_tier.as_deref().unwrap_or("-");
                println!("  [{mark}] {} ({class}) — fidelity {tier}", a.action_id);
            }
            std::process::exit(0);
        }
        Command::Sim { skill, embodiment } => {
            let reports = match (skill, embodiment) {
                // Regenerate mode: retarget + execute with the nominal reference driver.
                (Some(s), Some(e)) => rfl_conformance::run_reference_driver(&s, &e)?,
                // Stdin (live `--driver`) mode: parse the canonical execute goals from
                // stdin and execute them — the driver-input side of the wire.
                (None, None) => {
                    use rfl_core::driver::Driver;
                    use std::io::Read;
                    let mut input = String::new();
                    std::io::stdin().read_to_string(&mut input)?;
                    let goals = rfl_core::canonical::from_jsonl(&input)?;
                    let mut driver = rfl_conformance::ReferenceDriver::default();
                    goals.iter().map(|g| driver.execute(g)).collect()
                }
                _ => {
                    eprintln!(
                        "sim: pass both --skill and --embodiment (regenerate), or neither (read goals from stdin)"
                    );
                    std::process::exit(2);
                }
            };
            print!("{}", rfl_conformance::reports_to_jsonl(&reports));
            Ok(())
        }
        Command::Measure {
            skill,
            embodiment,
            runs,
            percentile,
            safety,
            out,
        } => {
            if runs.len() < 2 {
                eprintln!(
                    "measure: at least two --run traces are required (a single run has no run-to-run variation)"
                );
                std::process::exit(2);
            }
            let skill_text = std::fs::read_to_string(&skill)
                .map_err(|e| anyhow::anyhow!("read skill {skill:?}: {e}"))?;
            let emb_text = std::fs::read_to_string(&embodiment)
                .map_err(|e| anyhow::anyhow!("read embodiment {embodiment:?}: {e}"))?;
            let parsed_skill = rfl_core::skill_isa::Skill::parse_yaml(&skill_text)?;
            let emb = rfl_core::embodiment::Embodiment::parse_yaml(&emb_text)?;
            let mut runs_jsonl = Vec::with_capacity(runs.len());
            for r in &runs {
                runs_jsonl.push(
                    std::fs::read_to_string(r)
                        .map_err(|e| anyhow::anyhow!("read run {r:?}: {e}"))?,
                );
            }
            let table = match rfl_conformance::measure::measure_traces(
                &parsed_skill,
                &emb,
                &runs_jsonl,
                percentile,
                safety,
            ) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("measure: {e:#}");
                    std::process::exit(2);
                }
            };
            let yaml = table.to_provisional_yaml();
            match out {
                Some(p) => {
                    std::fs::write(&p, yaml).map_err(|e| anyhow::anyhow!("write {p:?}: {e}"))?
                }
                None => print!("{yaml}"),
            }
            Ok(())
        }
        Command::SpecVersion => {
            println!("{}", rfl_core::SPEC_VERSION);
            Ok(())
        }
    }
}
