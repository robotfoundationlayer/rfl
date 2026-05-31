// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Robot Foundation Layer — core specification implementation.
//!
//! This crate hosts the three layers of the RFL specification in Rust form:
//!
//! - [`skill_isa`] — the Skill Instruction Set Architecture: fifty manipulation
//!   primitives across seven categories, plus the compositional algebra that
//!   combines them into multi-step manipulations.
//! - [`translation`] — the Translation Layer: the canonical embodiment-agnostic
//!   action representation and the deterministic retargeting algorithm that
//!   resolves a high-level skill into per-embodiment canonical actions.
//! - [`driver`] — the Driver Interface protocol types (ROS 2-compatible).
//!
//! See the project specification under `spec/` and the white paper for the
//! formal definitions these modules implement.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub mod driver;
pub mod embodiment;
pub mod pose;
pub mod quantity;
pub mod skill_isa;
pub mod translation;

/// RFL specification version this crate implements.
pub const SPEC_VERSION: &str = "v0.1-draft";

/// Top-level error type for RFL core operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Skill ISA parse or composition error.
    #[error("skill ISA error: {0}")]
    SkillIsa(String),

    /// Translation Layer retargeting error.
    #[error("translation error: {0}")]
    Translation(String),

    /// Driver Interface protocol error.
    #[error("driver interface error: {0}")]
    Driver(String),

    /// Underlying I/O failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Underlying serialization failure.
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

/// Standard `Result` alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;
