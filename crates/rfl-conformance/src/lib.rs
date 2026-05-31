// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Robot Foundation Layer — conformance test suite.
//!
//! Implementations of the four conformance test classes defined in
//! `spec/05-conformance.md`:
//!
//! 1. Skill ISA parser conformance
//! 2. Translation Layer retargeting determinism
//! 3. Driver Interface protocol compliance
//! 4. End-to-end execution conformance (Translation Layer → Driver Interface
//!    → embodiment)
//!
//! The test runner targets driver binaries that expose the Driver Interface
//! over ROS 2 (or the local in-process trait, for unit testing).

use std::path::Path;

/// Retarget the example skill onto the named embodiment descriptor and return the
/// JSONL stream (`execute` messages, one per line). The cable-insertion reference
/// lives under `examples/`.
///
/// # Errors
/// Propagates parse / retarget errors.
pub fn retarget_example_to_jsonl(
    skill_path: &Path,
    embodiment_path: &Path,
) -> anyhow::Result<String> {
    let skill = rfl_core::skill_isa::Skill::parse_yaml(&std::fs::read_to_string(skill_path)?)?;
    let emb =
        rfl_core::embodiment::Embodiment::parse_yaml(&std::fs::read_to_string(embodiment_path)?)?;
    let out = rfl_core::translation::retarget(&skill, &emb)?;
    Ok(rfl_core::canonical::to_jsonl(&skill.skill, &emb.id, &out.actions, &out.suffixes))
}

use rfl_core::canonical::{ExecuteGoal, TactileTargetOut};
use rfl_core::driver::{Driver, DriverReport, Outcome, RealizedPose, Status, Telemetry, Verdict, Wrench};

/// A nominal-echo reference driver: it does not simulate physics; it returns the
/// in-protocol report a conformant driver would produce on a nominal execution,
/// deterministically from the `execute` message. The in-process Class 3 target
/// (`spec/05` § Four test classes). One telemetry sample + one terminal status per
/// action; `t` is the 1-based step counter.
#[derive(Debug, Default)]
pub struct ReferenceDriver {
    step: u32,
}

impl Driver for ReferenceDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        self.step += 1;
        let ca = &goal.canonical_action;
        let fidelity_tier = match &ca.tactile_target {
            Some(TactileTargetOut::Auto) => Some("manifold".to_string()),
            Some(TactileTargetOut::Proxy { .. }) => Some("proxy".to_string()),
            _ => None,
        };
        // Echo any commanded force budget as plausible in-protocol content (carries
        // increment 3's bounds through; C2's envelope checkers will sample it).
        let (wrench, securing_force) = match &ca.force_budget {
            Some(q) => {
                let mag = q.parse().map_or(0.0, |(v, _)| v);
                (
                    Some(Wrench { force: [0.0, 0.0, mag], torque: [0.0, 0.0, 0.0] }),
                    Some(q.clone()),
                )
            }
            None => (None, None),
        };
        let telemetry = Telemetry {
            message: "telemetry",
            action_id: goal.action_id.clone(),
            t: f64::from(self.step),
            realized_pose: Some(RealizedPose::placeholder()),
            wrench,
            securing_force,
            tactile: vec![],
            events: vec![],
            fidelity_tier: fidelity_tier.clone(),
        };
        let status = Status {
            message: "status",
            action_id: goal.action_id.clone(),
            outcome: Outcome::Succeeded,
            verdict: Some(Verdict {
                value: true,
                confidence: 1.0,
                evidence: vec!["nominal reference-driver execution".to_string()],
            }),
            fidelity_tier,
            final_pose: Some(RealizedPose::placeholder()),
            failure_class: None,
            failure_detail: None,
        };
        DriverReport { telemetry: vec![telemetry], status }
    }
}

/// Retarget the skill onto the embodiment and drive every `execute` message through a
/// fresh `ReferenceDriver`, returning the per-action reports. Action ids match the
/// `{skill}/{embodiment_id}/{NNNN}-{suffix}` form `canonical::to_jsonl` emits.
///
/// # Errors
/// Propagates parse / retarget errors.
pub fn run_reference_driver(
    skill_path: &Path,
    embodiment_path: &Path,
) -> anyhow::Result<Vec<DriverReport>> {
    let skill = rfl_core::skill_isa::Skill::parse_yaml(&std::fs::read_to_string(skill_path)?)?;
    let emb =
        rfl_core::embodiment::Embodiment::parse_yaml(&std::fs::read_to_string(embodiment_path)?)?;
    let out = rfl_core::translation::retarget(&skill, &emb)?;
    let mut driver = ReferenceDriver::default();
    let mut reports = Vec::new();
    for (i, (action, suffix)) in out.actions.iter().zip(&out.suffixes).enumerate() {
        let action_id = format!("{}/{}/{:04}-{}", skill.skill, emb.id, i + 1, suffix);
        let goal = ExecuteGoal::wrap(action_id, action.clone());
        reports.push(driver.execute(&goal));
    }
    Ok(reports)
}

/// Render a report stream as JSON Lines (each telemetry sample, then the status, per
/// action) for golden snapshots and determinism checks.
#[must_use]
pub fn reports_to_jsonl(reports: &[DriverReport]) -> String {
    let mut out = String::new();
    for r in reports {
        for t in &r.telemetry {
            out.push_str(&serde_json::to_string(t).expect("serialize telemetry"));
            out.push('\n');
        }
        out.push_str(&serde_json::to_string(&r.status).expect("serialize status"));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn example_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
    }

    #[test]
    fn reference_driver_succeeds_and_echoes_fidelity() {
        let dir = example_dir();
        let reports = run_reference_driver(
            &dir.join("skill.yaml"),
            &dir.join("embodiments/pneumatic-6f.yaml"),
        )
        .expect("drive");
        assert_eq!(reports.len(), 8); // 8 cable-insertion actions
        for r in &reports {
            assert!(matches!(r.status.outcome, rfl_core::driver::Outcome::Succeeded));
            for t in &r.telemetry {
                assert_eq!(t.action_id, r.status.action_id); // correlation
            }
        }
        // pneumatic has no tactile sensing -> grasp.pinch confirmation degrades to proxy
        assert_eq!(reports[1].status.fidelity_tier.as_deref(), Some("proxy"));
    }
}
