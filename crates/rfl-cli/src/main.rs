// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! `rfl` — the Robot Foundation Layer command-line interface.
//!
//! Subcommands:
//! - `rfl validate <skill.yaml>` — parse and validate a Skill ISA composition
//! - `rfl retarget <skill.yaml> --embodiment <descriptor.yaml>` — print the
//!   resolved canonical actions for the target embodiment
//! - `rfl conformance --driver <binary>` — run the conformance test suite

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
            anyhow::bail!(
                "retarget not yet implemented (target: spec v0.1, 2027 Q1) — {skill:?} -> {embodiment:?}"
            );
        }
        Command::Conformance { driver } => {
            anyhow::bail!(
                "conformance not yet implemented (target: spec v0.1, 2027 Q1) — driver {driver:?}"
            );
        }
        Command::SpecVersion => {
            println!("{}", rfl_core::SPEC_VERSION);
            Ok(())
        }
    }
}
