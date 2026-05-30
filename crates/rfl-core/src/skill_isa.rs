// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Skill Instruction Set Architecture (Skill ISA).
//!
//! The Skill ISA defines fifty manipulation primitives organized into seven
//! categories, plus a compositional algebra (sequence, parallel, reactive,
//! repeat, branch) that combines primitives into multi-step manipulations.
//!
//! See `spec/01-skill-isa.md` for the formal definition.

/// One of the seven primitive categories defined by the Skill ISA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Category {
    /// Reaching and approach primitives.
    Reach,
    /// Grasping primitives (pinch, power, hook, ...).
    Grasp,
    /// In-hand manipulation primitives.
    InHand,
    /// Transport primitives (carry, hand-off, ...).
    Transport,
    /// Placement primitives.
    Place,
    /// Force-controlled interaction primitives (insertion, screwing, ...).
    Force,
    /// Sensing-only primitives (probe, inspect, ...).
    Sense,
}

/// A canonical Skill ISA primitive identifier.
///
/// Concrete primitives are defined in `spec/01-skill-isa.md` and loaded from
/// the JSON schemas under `schemas/skill-isa.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PrimitiveId(pub String);

// TODO: implement the compositional algebra (Sequence, Parallel, Reactive,
// Repeat, Branch) per spec/01-skill-isa.md once the spec text stabilizes.
