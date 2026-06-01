// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Driver Interface protocol types.
//!
//! The report side of the Driver Interface (`spec/03` § Canonical driver messages,
//! `schemas/driver-interface.schema.json` TelemetryFeedback / StatusResult): the
//! `Telemetry` (Feedback) and `Status` (Result) a driver returns, and the in-process
//! `Driver` trait a reference / test driver implements. Representation-owned content
//! (pose, wrench, verdict) is concrete here but validates against the schema's open
//! floors, mirroring the execute side. Serialization is deterministic (RD1c): fixed
//! field order, optionals skipped when empty, no float reformatting beyond round6.
//!
//! See `spec/03-driver-interface.md` and `spec/05-conformance.md` § Four test classes.

use crate::canonical::ExecuteGoal;
use crate::quantity::Quantity;

/// A realized pose report (`Pose6DFloor`). v0 retarget targets are symbolic (the
/// concrete `Pose6D` representation is `spec/02`'s), so a reference driver emits the
/// deterministic placeholder; concrete tracking arrives with the pose representation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RealizedPose {
    /// Position in R^3 (metres).
    pub position: [f64; 3],
    /// Orientation unit quaternion `[x, y, z, w]`.
    pub orientation: [f64; 4],
}

impl RealizedPose {
    /// The deterministic placeholder pose (identity orientation at the origin).
    #[must_use]
    pub fn placeholder() -> Self {
        RealizedPose {
            position: [0.0, 0.0, 0.0],
            orientation: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// A wrench reading (`WrenchFloor`): force (N) and torque (N·m) vectors.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Wrench {
    /// Force vector (N).
    pub force: [f64; 3],
    /// Torque vector (N·m).
    pub torque: [f64; 3],
}

/// A tactile feature reading.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TactileReading {
    /// The manifold feature name.
    pub feature: String,
    /// The reading (a unit-suffixed quantity or a qualitative token).
    pub reading: String,
}

/// A three-valued verdict with evidence (`VerdictFloor`; v0 carries a two-valued
/// `value` — the `outcome` enum already supplies the indeterminate case).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Verdict {
    /// The postcondition verdict.
    pub value: bool,
    /// Confidence in `[0, 1]`.
    pub confidence: f64,
    /// Evidence supporting the verdict.
    pub evidence: Vec<String>,
}

/// The terminal outcome of an action (`StatusResult.outcome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The action completed and its postcondition holds.
    Succeeded,
    /// The action failed (a `failure_class` accompanies it).
    Failed,
    /// The postcondition could not be determined.
    Indeterminate,
}

/// A telemetry sample (`TelemetryFeedback`; required: message, action_id, t).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Telemetry {
    /// The message discriminant (`telemetry`).
    pub message: &'static str,
    /// The correlated action id.
    pub action_id: String,
    /// Timestamp on the `04` manifold timebase (seconds).
    pub t: f64,
    /// The realized pose at this sample.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realized_pose: Option<RealizedPose>,
    /// The contact wrench at this sample.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrench: Option<Wrench>,
    /// The securing force on the held object (grasp-continuity, GC1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub securing_force: Option<Quantity>,
    /// The positional station error vs. the `reach.hover` setpoint (`spec/01` § 1.5
    /// "externally measured station error"; the interval-invariant settling leg / ENV3
    /// samples it). Present only for a hover carrying a `station_keeping` contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub station_error: Option<Quantity>,
    /// Tactile feature readings (manifold confirmation).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tactile: Vec<TactileReading>,
    /// ForceEvents fired this sample (`04` § Force events; `ForceEventFloor` objects:
    /// `kind` / `at` / `magnitude`, e.g. a detent click). Read by `check_actuation`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<serde_json::Value>,
    /// The fidelity tier in effect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fidelity_tier: Option<String>,
}

/// The terminal action result (`StatusResult`; required: message, action_id, outcome).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Status {
    /// The message discriminant (`status`).
    pub message: &'static str,
    /// The correlated action id.
    pub action_id: String,
    /// The terminal outcome.
    pub outcome: Outcome,
    /// The postcondition verdict + evidence (audit record, `05` AUD1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
    /// The achieved fidelity tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fidelity_tier: Option<String>,
    /// The final pose at rest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_pose: Option<RealizedPose>,
    /// The protocol failure class (present only when `outcome != succeeded`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_class: Option<String>,
    /// The primitive-specific failure detail token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_detail: Option<String>,
    /// The externally measured time from envelope breach to the at-rest safe state, bounded by
    /// `embodiment.limits.stop_time` (`spec/01` § 1.5 C2 abort timing). Present only on an
    /// aborted action (the bench measures it); a nominal success omits it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_latency: Option<Quantity>,
    /// Safety-critical facts the audit record propagates (`spec/05` AUD2 / AUD3, `spec/04`
    /// freed-part handling). Present only when a safety-relevant fact must be disclosed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_flags: Option<SafetyFlags>,
}

/// The `safety_flags` audit object (`driver-interface` schema `StatusResult.safety_flags`).
/// v0 carries the freed-part disposition; `momentary_release` (schema-declared) is currently
/// disclosed via `verdict.evidence` (increment 21) — reconciling the two is deferred.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SafetyFlags {
    /// The freed-part disposition disclosed by a freeing operation (`spec/04` TM21c).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freed_part_disposition: Option<FreedPartDisposition>,
}

/// A freeing operation's disposition (`spec/04` TM21c): `retained` or `safe_zone_release(zone)`,
/// never an uncontrolled drop.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FreedPartDisposition {
    /// `retained` or `safe_zone_release`.
    pub disposition: String,
    /// The declared safe zone (present for `safe_zone_release`; owned by `04`, floored).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<serde_json::Value>,
}

/// One action's driver report: the telemetry samples plus the terminal status.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DriverReport {
    /// The telemetry samples emitted during the action.
    pub telemetry: Vec<Telemetry>,
    /// The terminal status.
    pub status: Status,
}

/// An in-process driver: consumes an `execute` message and reports back in-protocol.
/// The conformance test target (`spec/05` § Four test classes, the local in-process
/// path); a ROS 2 transport binding is a later increment.
pub trait Driver {
    /// Execute one canonical action and produce its telemetry samples + terminal status.
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_serializes_required_fields_and_skips_empty() {
        let t = Telemetry {
            message: "telemetry",
            action_id: "a/b/0001-x".into(),
            t: 1.0,
            realized_pose: Some(RealizedPose::placeholder()),
            wrench: None,
            securing_force: None,
            station_error: None,
            tactile: vec![],
            events: vec![],
            fidelity_tier: Some("manifold".into()),
        };
        let j = serde_json::to_string(&t).unwrap();
        assert!(j.contains("\"message\":\"telemetry\""));
        assert!(j.contains("\"action_id\":\"a/b/0001-x\""));
        assert!(j.contains("\"t\":1.0"));
        assert!(j.contains("\"fidelity_tier\":\"manifold\""));
        assert!(!j.contains("wrench")); // skipped when None
        assert!(!j.contains("tactile")); // skipped when empty
    }

    #[test]
    fn status_outcome_serializes_snake_case() {
        let s = Status {
            message: "status",
            action_id: "a/b/0001-x".into(),
            outcome: Outcome::Succeeded,
            verdict: None,
            fidelity_tier: None,
            final_pose: None,
            failure_class: None,
            failure_detail: None,
            stop_latency: None,
            safety_flags: None,
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("\"outcome\":\"succeeded\""));
        assert!(!j.contains("failure_class")); // skipped when None
    }

    #[test]
    fn status_safety_flags_serializes_freed_part_disposition() {
        let s = Status {
            message: "status",
            action_id: "a/b/0005-unscrew".into(),
            outcome: Outcome::Succeeded,
            verdict: None,
            fidelity_tier: None,
            final_pose: None,
            failure_class: None,
            failure_detail: None,
            stop_latency: None,
            safety_flags: Some(SafetyFlags {
                freed_part_disposition: Some(FreedPartDisposition {
                    disposition: "retained".into(),
                    zone: None,
                }),
            }),
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("\"freed_part_disposition\""));
        assert!(j.contains("\"disposition\":\"retained\""));
        assert!(!j.contains("\"zone\"")); // skipped when None
    }
}
