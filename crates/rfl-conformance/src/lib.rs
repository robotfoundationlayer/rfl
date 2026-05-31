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
        let ca = &goal.canonical_action;
        let fidelity_tier = match &ca.tactile_target {
            Some(TactileTargetOut::Auto) => Some("manifold".to_string()),
            Some(TactileTargetOut::Proxy { .. }) => Some("proxy".to_string()),
            _ => None,
        };
        // Echo any commanded force budget as plausible in-protocol content (carries
        // increment 3's bounds through; C2's envelope checkers will sample it).
        // Echo any commanded force budget into wrench.force / securing_force and any
        // commanded torque (force.screw, in force_profile.torque) into wrench.torque
        // (within budget) — so the C2 / E3 envelope checkers have something to sample.
        let force_mag = ca.force_budget.as_ref().and_then(|q| q.parse().map(|(v, _)| v));
        let torque_mag = ca
            .safety_envelope
            .force_profile
            .as_ref()
            .and_then(|fp| fp.get("torque"))
            .and_then(serde_json::Value::as_str)
            .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v));
        let wrench = if force_mag.is_some() || torque_mag.is_some() {
            Some(Wrench {
                force: [0.0, 0.0, force_mag.unwrap_or(0.0)],
                torque: [0.0, 0.0, torque_mag.unwrap_or(0.0)],
            })
        } else {
            None
        };
        // Echo the commanded grip budget, or — on a held carry with no commanded budget
        // (transport.move_to_pose) — the declared min_holding_force floor, representing the
        // grip maintained at its securing minimum (GC1 base continuity).
        let securing_force = ca.force_budget.clone().or_else(|| {
            ca.safety_envelope
                .force_profile
                .as_ref()
                .and_then(|fp| fp.get("min_holding_force"))
                .and_then(serde_json::Value::as_str)
                .map(|s| rfl_core::quantity::Quantity(s.to_string()))
        });
        // Interval-invariant actions (reach.hover) are sampled over the interval (ENV2,
        // spec/05); every other action emits a single terminal-ish sample. N is decided
        // by the envelope class so future interval-invariant primitives inherit it.
        let n_samples = if envelope_class_for(suffix_of(&goal.action_id))
            == Some(EnvelopeClass::IntervalInvariant)
        {
            3
        } else {
            1
        };
        let telemetry: Vec<Telemetry> = (0..n_samples)
            .map(|_| {
                self.step += 1;
                Telemetry {
                    message: "telemetry",
                    action_id: goal.action_id.clone(),
                    t: f64::from(self.step),
                    realized_pose: Some(RealizedPose::placeholder()),
                    wrench: wrench.clone(),
                    securing_force: securing_force.clone(),
                    tactile: vec![],
                    events: vec![],
                    fidelity_tier: fidelity_tier.clone(),
                }
            })
            .collect();
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
        DriverReport { telemetry, status }
    }
}

/// A fault a `FaultyDriver` injects into the nominal report — schema-valid but
/// envelope-violating, to verify the checkers reject violations (the non-circular
/// proof of Test Class 3).
#[derive(Debug, Clone, Copy)]
pub enum Fault {
    /// `securing_force` below any derived floor (violates grasp-continuity GC1).
    UnderSecure,
    /// `wrench.force` above any budget (violates the force-trajectory bound, ENV4).
    OverForce,
    /// `wrench.torque` above any torque budget (violates the force-trajectory bound, ENV4).
    OverTorque,
    /// `outcome = indeterminate`, no `final_pose` (violates terminal-postcondition).
    NeverSettle,
    /// Drop the middle telemetry sample's `realized_pose` (violates interval-invariant,
    /// ENV2 — a mid-interval gap while the endpoint still conforms).
    MidIntervalDrop,
}

/// Wraps the nominal `ReferenceDriver` and injects one `Fault` into every report it
/// returns. The report still validates against the driver-interface schema; the
/// violation is semantic (caught by `check_envelope`, not the schema).
#[derive(Debug)]
pub struct FaultyDriver {
    inner: ReferenceDriver,
    fault: Fault,
}

impl FaultyDriver {
    /// A faulty driver injecting `fault`.
    #[must_use]
    pub fn new(fault: Fault) -> Self {
        FaultyDriver { inner: ReferenceDriver::default(), fault }
    }
}

impl Driver for FaultyDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        match self.fault {
            Fault::UnderSecure => {
                for t in &mut report.telemetry {
                    if t.securing_force.is_some() {
                        t.securing_force =
                            Some(rfl_core::quantity::Quantity("0.1 N".to_string()));
                    }
                }
            }
            Fault::OverForce => {
                for t in &mut report.telemetry {
                    if let Some(w) = t.wrench.as_mut() {
                        w.force = [0.0, 0.0, 999.0];
                    }
                }
            }
            Fault::OverTorque => {
                for t in &mut report.telemetry {
                    if let Some(w) = t.wrench.as_mut() {
                        w.torque = [0.0, 0.0, 999.0];
                    }
                }
            }
            Fault::NeverSettle => {
                report.status.outcome = Outcome::Indeterminate;
                report.status.final_pose = None;
            }
            Fault::MidIntervalDrop => {
                let mid = report.telemetry.len() / 2;
                if let Some(t) = report.telemetry.get_mut(mid) {
                    t.realized_pose = None;
                }
            }
        }
        report
    }
}

/// Retarget the skill onto the embodiment and drive every `execute` message through
/// `driver`, returning the `(goal, report)` pair per action. Generic over any
/// `Driver` (the nominal `ReferenceDriver` or a `FaultyDriver`). Action ids match the
/// `{skill}/{embodiment_id}/{NNNN}-{suffix}` form `canonical::to_jsonl` emits.
///
/// # Errors
/// Propagates parse / retarget errors.
pub fn drive<D: Driver>(
    mut driver: D,
    skill_path: &Path,
    embodiment_path: &Path,
) -> anyhow::Result<Vec<(ExecuteGoal, DriverReport)>> {
    let skill = rfl_core::skill_isa::Skill::parse_yaml(&std::fs::read_to_string(skill_path)?)?;
    let emb =
        rfl_core::embodiment::Embodiment::parse_yaml(&std::fs::read_to_string(embodiment_path)?)?;
    let out = rfl_core::translation::retarget(&skill, &emb)?;
    let mut pairs = Vec::new();
    for (i, (action, suffix)) in out.actions.iter().zip(&out.suffixes).enumerate() {
        let action_id = format!("{}/{}/{:04}-{}", skill.skill, emb.id, i + 1, suffix);
        let goal = ExecuteGoal::wrap(action_id, action.clone());
        let report = driver.execute(&goal);
        pairs.push((goal, report));
    }
    Ok(pairs)
}

/// Drive the example with the nominal `ReferenceDriver`, returning the reports
/// (goals dropped) for golden / JSONL rendering.
///
/// # Errors
/// Propagates parse / retarget errors.
pub fn run_reference_driver(
    skill_path: &Path,
    embodiment_path: &Path,
) -> anyhow::Result<Vec<DriverReport>> {
    Ok(drive(ReferenceDriver::default(), skill_path, embodiment_path)?
        .into_iter()
        .map(|(_, report)| report)
        .collect())
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

/// The conformance envelope class a primitive is verified against (`spec/05` ENV1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeClass {
    /// `reach.*` (except hover): the end state, pose at rest, endpoint only.
    TerminalPostcondition,
    /// `grasp.*` / `in_hand.*` / `transport.*` / `place.*`: securing force >= floor.
    GraspContinuity,
    /// `force.*`: the force/torque profile against per-axis budgets.
    ForceTrajectory,
    /// `reach.hover` / `transport.carry`: a maintained invariant sampled over the
    /// interval (`05` ENV2). A mid-interval violation fails even when the endpoint conforms.
    IntervalInvariant,
}

/// Map an action-id suffix to its envelope class (`spec/05` ENV1). `sense.*`
/// (locate / inspect) has no motion envelope; the interval-invariant primitives
/// (hover / carry) are not in the v0 worked example.
#[must_use]
pub fn envelope_class_for(suffix: &str) -> Option<EnvelopeClass> {
    match suffix {
        "align" | "retract" | "scan" => Some(EnvelopeClass::TerminalPostcondition),
        "pinch" | "release" | "transport" => Some(EnvelopeClass::GraspContinuity),
        "insert_fit" | "screw" | "unscrew" => Some(EnvelopeClass::ForceTrajectory),
        "hover" => Some(EnvelopeClass::IntervalInvariant),
        _ => None, // locate / inspect: perception, no envelope
    }
}

/// The primitive suffix of an action id (`.../NNNN-<suffix>`; suffixes contain no `-`).
fn suffix_of(action_id: &str) -> &str {
    action_id.rsplit('-').next().unwrap_or(action_id)
}

/// The result of an envelope-class check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    /// The report conforms to the envelope.
    Pass,
    /// The report violates the envelope, with a human-readable reason.
    Fail(String),
}

/// Magnitude of a `"<x> <unit>"` quantity.
fn quantity_mag(q: &rfl_core::quantity::Quantity) -> Option<f64> {
    q.parse().map(|(v, _)| v)
}

/// Verify a driver report against the action's envelope class (`spec/05` ENV1–ENV4,
/// GC1). A pure function of the commanded `execute` goal and the returned report.
#[must_use]
pub fn check_envelope(
    class: EnvelopeClass,
    goal: &ExecuteGoal,
    report: &DriverReport,
) -> CheckOutcome {
    match class {
        EnvelopeClass::TerminalPostcondition => {
            if !matches!(report.status.outcome, Outcome::Succeeded) {
                return CheckOutcome::Fail(format!(
                    "outcome not succeeded: {:?}",
                    report.status.outcome
                ));
            }
            if report.status.final_pose.is_none() {
                return CheckOutcome::Fail("no final_pose at rest".to_string());
            }
            CheckOutcome::Pass
        }
        EnvelopeClass::GraspContinuity => {
            // GF1c floor from the execute message's force_profile, if present.
            let floor = goal
                .canonical_action
                .safety_envelope
                .force_profile
                .as_ref()
                .and_then(|fp| fp.get("min_holding_force"))
                .and_then(serde_json::Value::as_str)
                .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v));
            let Some(floor) = floor else {
                return CheckOutcome::Pass; // transport / release carry no floor in v0
            };
            for t in &report.telemetry {
                if let Some(sf) = t.securing_force.as_ref().and_then(quantity_mag) {
                    if sf < floor {
                        return CheckOutcome::Fail(format!(
                            "securing_force {sf} < min_holding_force {floor}"
                        ));
                    }
                }
            }
            CheckOutcome::Pass
        }
        EnvelopeClass::ForceTrajectory => {
            // Force budget (linear) — when present (e.g. force.insert_fit).
            if let Some(budget) = goal.canonical_action.force_budget.as_ref().and_then(quantity_mag) {
                for t in &report.telemetry {
                    if let Some(w) = &t.wrench {
                        let mag = w.force.iter().map(|x| x * x).sum::<f64>().sqrt();
                        if mag > budget {
                            return CheckOutcome::Fail(format!("|wrench.force| {mag} > budget {budget}"));
                        }
                    }
                }
            }
            // Torque budget (rotational) — when present (e.g. force.screw, in force_profile.torque).
            let torque_budget = goal
                .canonical_action
                .safety_envelope
                .force_profile
                .as_ref()
                .and_then(|fp| fp.get("torque"))
                .and_then(serde_json::Value::as_str)
                .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v));
            if let Some(tb) = torque_budget {
                for t in &report.telemetry {
                    if let Some(w) = &t.wrench {
                        let mag = w.torque.iter().map(|x| x * x).sum::<f64>().sqrt();
                        if mag > tb {
                            return CheckOutcome::Fail(format!("|wrench.torque| {mag} > torque budget {tb}"));
                        }
                    }
                }
            }
            CheckOutcome::Pass
        }
        EnvelopeClass::IntervalInvariant => {
            // v0 structural interval check (concrete poses are spec/02's): the action
            // settled (Succeeded) and EVERY interval telemetry sample carries a
            // realized_pose — the station was maintained at each sampled instant. A
            // mid-interval sample missing its pose fails here even when the endpoint
            // (TerminalPostcondition) conforms — the ENV2 property.
            if !matches!(report.status.outcome, Outcome::Succeeded) {
                return CheckOutcome::Fail(format!(
                    "outcome not succeeded: {:?}",
                    report.status.outcome
                ));
            }
            for (i, t) in report.telemetry.iter().enumerate() {
                if t.realized_pose.is_none() {
                    return CheckOutcome::Fail(format!("interval sample {i} missing realized_pose"));
                }
            }
            CheckOutcome::Pass
        }
    }
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

    #[test]
    fn envelope_class_mapping_follows_env1() {
        assert_eq!(envelope_class_for("align"), Some(EnvelopeClass::TerminalPostcondition));
        assert_eq!(envelope_class_for("retract"), Some(EnvelopeClass::TerminalPostcondition));
        assert_eq!(envelope_class_for("pinch"), Some(EnvelopeClass::GraspContinuity));
        assert_eq!(envelope_class_for("transport"), Some(EnvelopeClass::GraspContinuity));
        assert_eq!(envelope_class_for("insert_fit"), Some(EnvelopeClass::ForceTrajectory));
        assert_eq!(envelope_class_for("unscrew"), Some(EnvelopeClass::ForceTrajectory));
        assert_eq!(envelope_class_for("hover"), Some(EnvelopeClass::IntervalInvariant));
        assert_eq!(envelope_class_for("locate"), None); // sense: perception
        assert_eq!(envelope_class_for("inspect"), None);
    }

    #[test]
    fn nominal_grasp_continuity_passes_and_under_secure_fails() {
        let dir = example_dir();
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // index 1 is grasp.pinch (force_profile.min_holding_force present).
        let (goal, report) = &pairs[1];
        assert_eq!(check_envelope(EnvelopeClass::GraspContinuity, goal, report), CheckOutcome::Pass);
        let mut bad = report.clone();
        bad.telemetry[0].securing_force = Some(rfl_core::quantity::Quantity("0.1 N".to_string()));
        assert!(matches!(
            check_envelope(EnvelopeClass::GraspContinuity, goal, &bad),
            CheckOutcome::Fail(_)
        ));
    }

    #[test]
    fn force_trajectory_passes_nominal_and_fails_over_budget() {
        let dir = example_dir();
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // index 5 is force.insert_fit.
        let (goal, report) = &pairs[5];
        assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
        let mut bad = report.clone();
        if let Some(w) = bad.telemetry[0].wrench.as_mut() {
            w.force = [0.0, 0.0, 999.0];
        }
        assert!(matches!(
            check_envelope(EnvelopeClass::ForceTrajectory, goal, &bad),
            CheckOutcome::Fail(_)
        ));
    }

    #[test]
    fn terminal_postcondition_passes_nominal_and_fails_indeterminate() {
        let dir = example_dir();
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // index 4 is reach.align.
        let (goal, report) = &pairs[4];
        assert_eq!(check_envelope(EnvelopeClass::TerminalPostcondition, goal, report), CheckOutcome::Pass);
        let mut bad = report.clone();
        bad.status.outcome = rfl_core::driver::Outcome::Indeterminate;
        bad.status.final_pose = None;
        assert!(matches!(
            check_envelope(EnvelopeClass::TerminalPostcondition, goal, &bad),
            CheckOutcome::Fail(_)
        ));
    }

    #[test]
    fn faulty_under_secure_lowers_securing_force() {
        let dir = example_dir();
        let pairs = drive(
            FaultyDriver::new(Fault::UnderSecure),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // grasp.pinch (index 1) has a securing_force -> lowered to the violating value.
        assert_eq!(pairs[1].1.telemetry[0].securing_force.as_ref().unwrap().0, "0.1 N");
    }

    #[test]
    fn nominal_screw_passes_torque_trajectory_and_echoes_torque() {
        let ex = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let pairs = drive(
            ReferenceDriver::default(),
            &ex.join("skill.yaml"),
            &ex.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // force.screw is action index 5 (locate, pinch, transport, locate, align, screw, ...).
        let (goal, report) = &pairs[5];
        // the driver echoes the clamped torque (0.2 N·m) into wrench.torque.
        assert!((report.telemetry[0].wrench.as_ref().unwrap().torque[2] - 0.2).abs() < 1e-9);
        assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
        // an over-budget torque is rejected.
        let mut bad = report.clone();
        bad.telemetry[0].wrench.as_mut().unwrap().torque = [0.0, 0.0, 999.0];
        assert!(matches!(
            check_envelope(EnvelopeClass::ForceTrajectory, goal, &bad),
            CheckOutcome::Fail(_)
        ));
    }

    fn sample_action() -> rfl_core::canonical::CanonicalAction {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode,
        };
        CanonicalAction {
            target_frame: "control".into(),
            target_pose: PoseExpr::Ref { r#ref: "panel".into() },
            force_budget: None,
            timing: TimingHints {
                nominal_duration: None,
                timing_mode: TimingMode::Strict,
                stop_at_goal: true,
            },
            tactile_target: None,
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
        }
    }

    #[test]
    fn interval_invariant_checks_every_sample() {
        use rfl_core::driver::{Outcome, RealizedPose, Status, Telemetry, Verdict};
        let goal = ExecuteGoal::wrap("s/e/0001-hover".to_string(), sample_action());
        let sample = |pose: Option<RealizedPose>| Telemetry {
            message: "telemetry",
            action_id: "s/e/0001-hover".to_string(),
            t: 1.0,
            realized_pose: pose,
            wrench: None,
            securing_force: None,
            tactile: vec![],
            events: vec![],
            fidelity_tier: None,
        };
        let status = Status {
            message: "status",
            action_id: "s/e/0001-hover".to_string(),
            outcome: Outcome::Succeeded,
            verdict: Some(Verdict { value: true, confidence: 1.0, evidence: vec![] }),
            fidelity_tier: None,
            final_pose: Some(RealizedPose::placeholder()),
            failure_class: None,
            failure_detail: None,
        };
        let ok = DriverReport {
            telemetry: vec![sample(Some(RealizedPose::placeholder())); 3],
            status: status.clone(),
        };
        assert_eq!(check_envelope(EnvelopeClass::IntervalInvariant, &goal, &ok), CheckOutcome::Pass);
        // Drop the middle sample's pose: interval fails, but the endpoint (terminal) is fine.
        let mut bad = ok.clone();
        bad.telemetry[1].realized_pose = None;
        assert!(matches!(
            check_envelope(EnvelopeClass::IntervalInvariant, &goal, &bad),
            CheckOutcome::Fail(_)
        ));
        assert_eq!(check_envelope(EnvelopeClass::TerminalPostcondition, &goal, &bad), CheckOutcome::Pass);
    }
}
