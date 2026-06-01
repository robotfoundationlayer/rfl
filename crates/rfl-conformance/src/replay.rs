// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Vendor driver-report ingestion for `rfl certify` (`spec/05` Test Class 3, replay mode).
//!
//! `rfl-core` produces the wire protocol (`Serialize` only, RD1c-deterministic); the verifier
//! *consumes* arbitrary vendor protocol. So the strict `Deserialize` mirror structs +
//! schema-validation live here, in the verifier, not in `rfl-core`. Reconstructs the canonical
//! `DriverReport`s the unchanged `check_*` battery already operates on.
//!
//! The embedded `driver-interface.schema.json` is the authoritative `additionalProperties:false`
//! gate (validated per line *before* deserialization), so the mirror structs need no
//! `deny_unknown_fields`: they model only the fields certify consumes and let serde ignore the
//! rest (already schema-validated). Floored sub-objects: `realized_pose` / `final_pose` are
//! presence-only (no v0 check reads their contents); `wrench` / `verdict` are concretely typed
//! (the force / audit checks read them); `tactile` and `safety_flags.momentary_release` are
//! schema-validated but not carried (no v0 check consumes them — the continuity-break signal is
//! `verdict.evidence`).

use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Context, Result};
use rfl_core::driver::{
    DriverReport, FreedPartDisposition, Outcome, RealizedPose, SafetyFlags, Status, Telemetry,
    Verdict, Wrench,
};
use rfl_core::quantity::Quantity;

/// The driver-interface wire schema, embedded so the validator is self-contained in the binary
/// (the installed `rfl` has no repo checkout to read `schemas/` from).
const DRIVER_SCHEMA: &str = include_str!("../../../schemas/driver-interface.schema.json");

// ---- input mirrors (model only the consumed fields; boon is the additionalProperties gate) ----

/// A driver-report line, discriminated by `message` (the schema's other three message kinds —
/// `execute` and the two clearance messages — deserialize to neither variant and are rejected).
#[derive(serde::Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
enum ReportLine {
    Telemetry(TelemetryIn),
    Status(StatusIn),
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum OutcomeIn {
    Succeeded,
    Failed,
    Indeterminate,
}

/// `wrench` IS consumed by the force-trajectory / contact-band checks, so it is concretely typed.
#[derive(serde::Deserialize)]
struct WrenchIn {
    force: [f64; 3],
    torque: [f64; 3],
}

/// `verdict.evidence` IS consumed (engagement / irreversible / momentary_release / actuation).
#[derive(serde::Deserialize)]
struct VerdictIn {
    #[serde(default)]
    value: bool,
    #[serde(default)]
    confidence: f64,
    #[serde(default)]
    evidence: Vec<String>,
}

#[derive(serde::Deserialize)]
struct FreedPartDispositionIn {
    disposition: String,
    #[serde(default)]
    zone: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct SafetyFlagsIn {
    #[serde(default)]
    freed_part_disposition: Option<FreedPartDispositionIn>,
}

#[derive(serde::Deserialize)]
struct TelemetryIn {
    action_id: String,
    t: f64,
    #[serde(default)]
    realized_pose: Option<serde_json::Value>, // presence-only (contents floored + unread)
    #[serde(default)]
    wrench: Option<WrenchIn>,
    #[serde(default)]
    securing_force: Option<String>,
    #[serde(default)]
    station_error: Option<String>,
    #[serde(default)]
    events: Vec<serde_json::Value>,
    #[serde(default)]
    fidelity_tier: Option<String>,
}

#[derive(serde::Deserialize)]
struct StatusIn {
    action_id: String,
    outcome: OutcomeIn,
    #[serde(default)]
    verdict: Option<VerdictIn>,
    #[serde(default)]
    fidelity_tier: Option<String>,
    #[serde(default)]
    final_pose: Option<serde_json::Value>, // presence-only
    #[serde(default)]
    failure_class: Option<String>,
    #[serde(default)]
    failure_detail: Option<String>,
    #[serde(default)]
    stop_latency: Option<String>,
    #[serde(default)]
    safety_flags: Option<SafetyFlagsIn>,
}

fn to_telemetry(t: TelemetryIn) -> Telemetry {
    Telemetry {
        message: "telemetry",
        action_id: t.action_id,
        t: t.t,
        realized_pose: t.realized_pose.map(|_| RealizedPose::placeholder()),
        wrench: t.wrench.map(|w| Wrench { force: w.force, torque: w.torque }),
        securing_force: t.securing_force.map(Quantity),
        station_error: t.station_error.map(Quantity),
        tactile: vec![],
        events: t.events,
        fidelity_tier: t.fidelity_tier,
    }
}

fn to_status(s: StatusIn) -> Status {
    Status {
        message: "status",
        action_id: s.action_id,
        outcome: match s.outcome {
            OutcomeIn::Succeeded => Outcome::Succeeded,
            OutcomeIn::Failed => Outcome::Failed,
            OutcomeIn::Indeterminate => Outcome::Indeterminate,
        },
        verdict: s.verdict.map(|v| Verdict {
            value: v.value,
            confidence: v.confidence,
            evidence: v.evidence,
        }),
        fidelity_tier: s.fidelity_tier,
        final_pose: s.final_pose.map(|_| RealizedPose::placeholder()),
        failure_class: s.failure_class,
        failure_detail: s.failure_detail,
        stop_latency: s.stop_latency.map(Quantity),
        safety_flags: s.safety_flags.map(|sf| SafetyFlags {
            freed_part_disposition: sf.freed_part_disposition.map(|d| FreedPartDisposition {
                disposition: d.disposition,
                zone: d.zone,
            }),
        }),
    }
}

/// One action's accumulating report during correlation.
#[derive(Default)]
struct Partial {
    telemetry: Vec<Telemetry>,
    status: Option<Status>,
}

/// Ingest a vendor driver-report JSONL stream into per-action `DriverReport`s, keyed by
/// `action_id`. Each line must be a schema-valid `telemetry` or `status` message; every action
/// must have exactly one terminal `status`.
///
/// # Errors
/// A malformed / schema-invalid line, a non-report message kind, a duplicate status, or a
/// telemetry-only action (no status) — each is an invalid run.
pub fn replay_report(jsonl: &str) -> Result<BTreeMap<String, DriverReport>> {
    let schema_value: serde_json::Value =
        serde_json::from_str(DRIVER_SCHEMA).context("parse embedded driver-interface schema")?;
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource("driver-interface.schema.json", schema_value)
        .map_err(|e| anyhow!("add schema resource: {e}"))?;
    let idx = compiler
        .compile("driver-interface.schema.json", &mut schemas)
        .map_err(|e| anyhow!("compile schema: {e}"))?;

    let mut acc: BTreeMap<String, Partial> = BTreeMap::new();

    for (i, line) in jsonl.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value =
            serde_json::from_str(line).with_context(|| format!("line {n}: invalid JSON"))?;
        schemas
            .validate(&v, idx)
            .map_err(|e| anyhow!("line {n}: schema violation: {e}"))?;
        match serde_json::from_value::<ReportLine>(v) {
            Ok(ReportLine::Telemetry(t)) => {
                acc.entry(t.action_id.clone()).or_default().telemetry.push(to_telemetry(t));
            }
            Ok(ReportLine::Status(s)) => {
                let slot = acc.entry(s.action_id.clone()).or_default();
                if slot.status.is_some() {
                    bail!("line {n}: duplicate status for action {}", s.action_id);
                }
                slot.status = Some(to_status(s));
            }
            Err(e) => bail!("line {n}: not a telemetry/status report message: {e}"),
        }
    }

    let mut out = BTreeMap::new();
    for (id, p) in acc {
        let status = p
            .status
            .ok_or_else(|| anyhow!("action {id} has telemetry but no terminal status"))?;
        out.insert(id, DriverReport { telemetry: p.telemetry, status });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{reports_to_jsonl, run_reference_driver};
    use std::path::Path;

    fn example_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
    }

    #[test]
    fn round_trips_reference_driver_output() {
        let dir = example_dir();
        let reports = run_reference_driver(
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .expect("drive");
        let jsonl = reports_to_jsonl(&reports);
        let replayed = replay_report(&jsonl).expect("replay");
        // 8 cable-insertion actions, each reconstructed with its status + telemetry.
        assert_eq!(replayed.len(), 8);
        for r in &reports {
            let back = replayed.get(&r.status.action_id).expect("action present");
            assert_eq!(back.telemetry.len(), r.telemetry.len());
            assert_eq!(back.status.outcome, r.status.outcome);
            assert_eq!(back.status.action_id, r.status.action_id);
        }
    }

    #[test]
    fn rejects_unknown_field() {
        // an extra key the schema forbids (additionalProperties:false), caught by boon.
        let bad = r#"{"message":"status","action_id":"a/b/0001-x","outcome":"succeeded","bogus":1}"#;
        assert!(replay_report(bad).is_err());
    }

    #[test]
    fn rejects_bad_outcome_enum() {
        let bad = r#"{"message":"status","action_id":"a/b/0001-x","outcome":"maybe"}"#;
        assert!(replay_report(bad).is_err());
    }

    #[test]
    fn rejects_non_report_message() {
        // execute is an RFL->driver message, never present in a report stream
        let bad = r#"{"message":"execute","action_id":"a/b/0001-x","canonical_action":{}}"#;
        assert!(replay_report(bad).is_err());
    }

    #[test]
    fn rejects_telemetry_without_status() {
        let bad = r#"{"message":"telemetry","action_id":"a/b/0001-x","t":1.0}"#;
        let err = replay_report(bad).unwrap_err().to_string();
        assert!(err.contains("no terminal status"), "got: {err}");
    }

    #[test]
    fn rejects_duplicate_status() {
        let bad = concat!(
            r#"{"message":"status","action_id":"a/b/0001-x","outcome":"succeeded"}"#,
            "\n",
            r#"{"message":"status","action_id":"a/b/0001-x","outcome":"succeeded"}"#,
        );
        let err = replay_report(bad).unwrap_err().to_string();
        assert!(err.contains("duplicate status"), "got: {err}");
    }
}
