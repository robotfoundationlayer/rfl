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
        // Echo the commanded force_budget, or — on a wipe with no budget — the normal_force
        // band setpoint (force.wipe), so the ForceTrajectory band leg has an in-band sample.
        let force_mag = ca.force_budget.as_ref().and_then(|q| q.parse().map(|(v, _)| v)).or_else(|| {
            ca.safety_envelope
                .force_profile
                .as_ref()
                .and_then(|fp| fp.get("normal_force"))
                .and_then(serde_json::Value::as_str)
                .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v))
        });
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
        // Echo a zero station error for an action carrying a station_keeping contract
        // (reach.hover settling, spec/01 § 1.5): the nominal hover holds station perfectly,
        // so the settled-tail leg passes. Absent for every other action.
        let station_error = ca
            .safety_envelope
            .station_keeping
            .as_ref()
            .map(|_| rfl_core::quantity::Quantity("0 mm".to_string()));
        // Echo the detent ForceEvent for an action carrying a detent actuation contract
        // (force.press_button, spec/01 § 6.6): the nominal press detects the actuation click.
        // Absent for every other action (events stays empty).
        let events: Vec<serde_json::Value> = match ca
            .safety_envelope
            .force_profile
            .as_ref()
            .and_then(|fp| fp.get("actuation"))
            .and_then(serde_json::Value::as_str)
        {
            Some("detent") => vec![serde_json::json!({ "kind": "detent" })],
            _ => vec![],
        };
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
                    station_error: station_error.clone(),
                    tactile: vec![],
                    events: events.clone(),
                    fidelity_tier: fidelity_tier.clone(),
                }
            })
            .collect();
        // Engagement-confirmation evidence (force.snap_engage confirm_held, spec/01 § 6.10): the
        // nominal driver's release-test confirmed the bistable connection holds. AUD1 evidence.
        let mut evidence = vec!["nominal reference-driver execution".to_string()];
        if ca
            .safety_envelope
            .force_profile
            .as_ref()
            .and_then(|fp| fp.get("confirm_held"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        {
            evidence.push("held_confirmed".to_string());
        }
        let status = Status {
            message: "status",
            action_id: goal.action_id.clone(),
            outcome: Outcome::Succeeded,
            verdict: Some(Verdict {
                value: true,
                confidence: 1.0,
                evidence,
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
    /// `wrench.force` driven to zero (loss of contact — violates the force.wipe band's lower
    /// edge; the mirror of `OverForce`).
    LoseContact,
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
            Fault::LoseContact => {
                for t in &mut report.telemetry {
                    if let Some(w) = t.wrench.as_mut() {
                        w.force = [0.0, 0.0, 0.0];
                    }
                }
            }
        }
        report
    }
}

/// How a driver responds to an OVER-budget injected disturbance (under budget it always
/// returns the nominal maintained invariant). `Graceful` is the conformant § 4.4 C2 response;
/// the two adversarial responses prove `check_graceful_degradation` bites.
#[derive(Debug, Clone, Copy)]
pub enum DisturbanceResponse {
    /// Conformant: halt to a stable config with the object still secured.
    Graceful,
    /// Adversarial: the object is dropped (securing_force below floor) — a loss, not a halt.
    Drops,
    /// Adversarial: claim success despite the over-budget disturbance.
    ClaimsSuccess,
}

/// The ENV3 disturbance bench (`spec/05` § Disturbance injection): a driver parameterized by
/// the bench-injected disturbance magnitude (N). It reads the action's `disturbance_budget`
/// from the execute message and models a driver's response — maintain the nominal invariant
/// when `injected <= budget`, otherwise respond per `DisturbanceResponse`. Disturbance
/// injection is the bench's input; the response is what the checks judge. Non-carry actions
/// (no `disturbance_budget`) pass through unchanged.
#[derive(Debug)]
pub struct DisturbanceDriver {
    inner: ReferenceDriver,
    injected_n: f64,
    response: DisturbanceResponse,
}

impl DisturbanceDriver {
    /// A disturbance driver injecting `injected_n` newtons with the given response policy.
    #[must_use]
    pub fn new(injected_n: f64, response: DisturbanceResponse) -> Self {
        DisturbanceDriver { inner: ReferenceDriver::default(), injected_n, response }
    }
}

impl Driver for DisturbanceDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        let budget = goal
            .canonical_action
            .safety_envelope
            .force_profile
            .as_ref()
            .and_then(|fp| fp.get("disturbance_budget"))
            .and_then(serde_json::Value::as_str)
            .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v));
        // No budget (non-carry) or injected within budget: the invariant holds (nominal report).
        let Some(budget) = budget else { return report };
        if self.injected_n <= budget {
            return report;
        }
        // Over budget. ClaimsSuccess returns the nominal Succeeded report unchanged (adversarial).
        if matches!(self.response, DisturbanceResponse::ClaimsSuccess) {
            return report;
        }
        // Graceful / Drops: a controlled halt (Failed, blocked, disturbance_exceeded).
        report.status.outcome = Outcome::Failed;
        report.status.failure_class = Some("blocked".to_string());
        report.status.failure_detail = Some("disturbance_exceeded".to_string());
        if let Some(v) = report.status.verdict.as_mut() {
            v.value = false; // honest: the postcondition did not hold
        }
        if matches!(self.response, DisturbanceResponse::Drops) {
            // The object was lost — securing_force falls below the floor (mirrors UnderSecure).
            for t in &mut report.telemetry {
                if t.securing_force.is_some() {
                    t.securing_force = Some(rfl_core::quantity::Quantity("0.1 N".to_string()));
                }
            }
        }
        report
    }
}

/// How a driver responds to a bench-injected lateral impulse on a `reach.hover` (`spec/01`
/// § 1.5 C2). `Recovers` / `Aborts` are the two conformant outcomes (recover-and-continue vs
/// abort-to-safe-state); `FailsToRecover` / `ClaimsSuccess` are adversarial.
#[derive(Debug, Clone, Copy)]
pub enum HoverResponse {
    /// Conformant: a transient excursion in the grace window, recovered in the settled tail.
    Recovers,
    /// Adversarial: drifts past `station_tolerance` throughout yet claims success.
    FailsToRecover,
    /// Conformant (over-envelope): abort to a safe state (Failed + `station_exceeded`).
    Aborts,
    /// Adversarial (over-envelope): claim success despite the station-exceeding impulse.
    ClaimsSuccess,
}

/// The `reach.hover` ENV3 bench (`spec/05` § Disturbance injection; `spec/01` § 1.5 C2):
/// models a driver's response to a calibrated lateral impulse. Reads `station_tolerance` from
/// the action's `station_keeping`; mutates the nominal report's `station_error` / outcome per
/// `response`. Non-hover actions (no `station_keeping`) pass through unchanged.
#[derive(Debug)]
pub struct HoverSettlingDriver {
    inner: ReferenceDriver,
    response: HoverResponse,
}

impl HoverSettlingDriver {
    /// A hover settling driver with the given response policy.
    #[must_use]
    pub fn new(response: HoverResponse) -> Self {
        HoverSettlingDriver { inner: ReferenceDriver::default(), response }
    }
}

impl Driver for HoverSettlingDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        let tol = goal
            .canonical_action
            .safety_envelope
            .station_keeping
            .as_ref()
            .and_then(|sk| sk.get("station_tolerance"))
            .and_then(serde_json::Value::as_str)
            .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v));
        let Some(tol) = tol else { return report }; // non-hover: passthrough
        let over = tol + 3.0; // above tolerance (the impulse / failed recovery)
        let under = tol / 2.0; // within tolerance (recovered)
        let q = |v: f64| Some(rfl_core::quantity::Quantity::from_si(v, "mm"));
        match self.response {
            HoverResponse::Recovers => {
                // sample 0 = transient excursion (grace window); tail = recovered.
                for (i, t) in report.telemetry.iter_mut().enumerate() {
                    t.station_error = q(if i == 0 { over } else { under });
                }
            }
            HoverResponse::FailsToRecover => {
                for t in &mut report.telemetry {
                    t.station_error = q(over);
                }
            }
            HoverResponse::Aborts => {
                report.status.outcome = Outcome::Failed;
                report.status.failure_class = Some("blocked".to_string());
                report.status.failure_detail = Some("station_exceeded".to_string());
                if let Some(v) = report.status.verdict.as_mut() {
                    v.value = false; // honest: the station was not held
                }
            }
            HoverResponse::ClaimsSuccess => {
                for t in &mut report.telemetry {
                    t.station_error = q(over); // ignored the impulse, still claims success
                }
            }
        }
        report
    }
}

/// How a driver reports a `force.press_button` press (`spec/01` § 6.6). `Actuates` is the
/// nominal detent + success; `Bottoms` is the conformant `no_actuation` (a force rise with no
/// detent — a stuck / absent button); `ClaimsActuation` is adversarial (success, no detent).
#[derive(Debug, Clone, Copy)]
pub enum PressButtonResponse {
    /// Conformant: the actuation detent fired and the press succeeded (ReferenceDriver nominal).
    Actuates,
    /// Conformant: no detent fired -> no_actuation reported honestly (force kept within budget).
    Bottoms,
    /// Adversarial: claim success though no detent fired.
    ClaimsActuation,
}

/// The `force.press_button` bench: models a driver's actuation outcome. Reuses the nominal
/// `ReferenceDriver` (which echoes the detent for an actuation contract) and mutates it per
/// `response`. Non-press actions pass through unchanged.
#[derive(Debug)]
pub struct PressButtonDriver {
    inner: ReferenceDriver,
    response: PressButtonResponse,
}

impl PressButtonDriver {
    /// A press-button driver with the given actuation outcome.
    #[must_use]
    pub fn new(response: PressButtonResponse) -> Self {
        PressButtonDriver { inner: ReferenceDriver::default(), response }
    }
}

impl Driver for PressButtonDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        match self.response {
            PressButtonResponse::Actuates => {} // nominal: detent echoed + Succeeded
            PressButtonResponse::Bottoms => {
                for t in &mut report.telemetry {
                    t.events.clear(); // no detent fired
                }
                report.status.outcome = Outcome::Failed;
                report.status.failure_class = Some("blocked".to_string());
                report.status.failure_detail = Some("no_actuation".to_string());
                if let Some(v) = report.status.verdict.as_mut() {
                    v.value = false; // honest: the button was not actuated
                }
            }
            PressButtonResponse::ClaimsActuation => {
                for t in &mut report.telemetry {
                    t.events.clear(); // no detent, yet claims success
                }
            }
        }
        report
    }
}

/// How a driver reports a `force.snap_engage` (`spec/01` § 6.10). `Engages` is the nominal
/// detent + held-confirmed + success; `NoSnap` is the conformant `no_snap` (force rise, no
/// detent); `ClaimsHeld` is adversarial (success + detent but the hold was never confirmed).
#[derive(Debug, Clone, Copy)]
pub enum SnapEngageResponse {
    /// Conformant: the snap-in detent fired, the hold was confirmed, and the engagement succeeded.
    Engages,
    /// Conformant: no snap fired -> no_snap reported honestly (force kept within budget).
    NoSnap,
    /// Adversarial: claim success though the connection was never confirmed held.
    ClaimsHeld,
}

/// The `force.snap_engage` bench: models a driver's engagement outcome. Reuses the nominal
/// `ReferenceDriver` (which echoes the detent + held-confirmed evidence) and mutates it per
/// `response`. Non-snap actions pass through unchanged.
#[derive(Debug)]
pub struct SnapEngageDriver {
    inner: ReferenceDriver,
    response: SnapEngageResponse,
}

impl SnapEngageDriver {
    /// A snap-engage driver with the given engagement outcome.
    #[must_use]
    pub fn new(response: SnapEngageResponse) -> Self {
        SnapEngageDriver { inner: ReferenceDriver::default(), response }
    }
}

impl Driver for SnapEngageDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        match self.response {
            SnapEngageResponse::Engages => {} // nominal: detent + held_confirmed + Succeeded
            SnapEngageResponse::NoSnap => {
                for t in &mut report.telemetry {
                    t.events.clear(); // no snap detent fired
                }
                report.status.outcome = Outcome::Failed;
                report.status.failure_class = Some("blocked".to_string());
                report.status.failure_detail = Some("no_snap".to_string());
                if let Some(v) = report.status.verdict.as_mut() {
                    v.value = false;
                    v.evidence.retain(|e| e != "held_confirmed"); // nothing engaged to confirm
                }
            }
            SnapEngageResponse::ClaimsHeld => {
                // snap detected (detent kept), claims success, but the hold was never confirmed.
                if let Some(v) = report.status.verdict.as_mut() {
                    v.evidence.retain(|e| e != "held_confirmed");
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

/// Map an action-id suffix to its envelope class (`spec/05` ENV1). Every primitive maps
/// to exactly one class; `sense.*` (locate / inspect) has no motion envelope. `hover` and
/// `carry` are interval-invariant — a held `carry` additionally checks the securing floor
/// over the interval (`check_envelope`), but it remains one class (ENV1).
#[must_use]
pub fn envelope_class_for(suffix: &str) -> Option<EnvelopeClass> {
    match suffix {
        "align" | "retract" | "scan" => Some(EnvelopeClass::TerminalPostcondition),
        "pinch" | "release" | "transport" => Some(EnvelopeClass::GraspContinuity),
        "insert_fit" | "screw" | "unscrew" | "press_button" | "wipe" | "snap_engage" => {
            Some(EnvelopeClass::ForceTrajectory)
        }
        "hover" | "carry" => Some(EnvelopeClass::IntervalInvariant),
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

/// If the action carries a `min_holding_force` floor and any telemetry sample's
/// `securing_force` is below it, the failure reason; else `None`. Shared by the
/// grasp-continuity check and the held leg of the interval-invariant check (`05` GC1).
fn securing_floor_violation(goal: &ExecuteGoal, report: &DriverReport) -> Option<String> {
    let floor = goal
        .canonical_action
        .safety_envelope
        .force_profile
        .as_ref()
        .and_then(|fp| fp.get("min_holding_force"))
        .and_then(serde_json::Value::as_str)
        .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v))?;
    for t in &report.telemetry {
        if let Some(sf) = t.securing_force.as_ref().and_then(quantity_mag) {
            if sf < floor {
                return Some(format!("securing_force {sf} < min_holding_force {floor}"));
            }
        }
    }
    None
}

/// If the action carries a `station_keeping` contract (`reach.hover` settling, `spec/01`
/// § 1.5) and any settled-tail sample — one stamped at `t >= first_t + settling_time`, after
/// the recovery grace window — is missing `station_error` or exceeds `station_tolerance`, the
/// failure reason; else `None`. Vacuous when `station_keeping` is absent (carry / bare hover
/// unaffected — the held-floor pattern). Requires >= 1 tail sample (non-vacuous).
fn station_keeping_violation(goal: &ExecuteGoal, report: &DriverReport) -> Option<String> {
    let sk = goal.canonical_action.safety_envelope.station_keeping.as_ref()?;
    let tol = sk
        .get("station_tolerance")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v))?;
    let settle = sk
        .get("settling_time")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v))?;
    let deadline = report.telemetry.first()?.t + settle;
    let mut tail_seen = false;
    for t in &report.telemetry {
        if t.t < deadline {
            continue; // recovery grace window — excursion permitted here
        }
        tail_seen = true;
        match t.station_error.as_ref().and_then(quantity_mag) {
            None => return Some(format!("settled-tail sample at t={} missing station_error", t.t)),
            Some(err) if err > tol => {
                return Some(format!("station_error {err} > station_tolerance {tol} at t={}", t.t));
            }
            _ => {}
        }
    }
    if !tail_seen {
        return Some(format!("no settled-tail sample at t >= {deadline}"));
    }
    None
}

/// If the action carries a `normal_force` band (`force.wipe`, `spec/01` § 6.8) and any
/// `wrench` sample's `|force|` falls outside `[normal_force − tol, normal_force + tol]`, the
/// failure reason (below = loss of contact, above = over-force); else `None`. Vacuous when no
/// `normal_force` band (insert_fit / screw / press_button) — the two-sided counterpart of the
/// securing floor, on contact wrench.
fn contact_band_violation(goal: &ExecuteGoal, report: &DriverReport) -> Option<String> {
    let fp = goal.canonical_action.safety_envelope.force_profile.as_ref()?;
    let setpoint = fp
        .get("normal_force")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v))?;
    let tol = fp
        .get("normal_force_tolerance")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| rfl_core::quantity::Quantity(s.to_string()).parse().map(|(v, _)| v))?;
    let (lo, hi) = (setpoint - tol, setpoint + tol);
    for t in &report.telemetry {
        if let Some(w) = &t.wrench {
            let mag = w.force.iter().map(|x| x * x).sum::<f64>().sqrt();
            if mag < lo {
                return Some(format!("contact lost: |wrench.force| {mag} < {lo} (normal_force {setpoint} − tol {tol})"));
            }
            if mag > hi {
                return Some(format!("over-force: |wrench.force| {mag} > {hi} (normal_force {setpoint} + tol {tol})"));
            }
        }
    }
    None
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
        EnvelopeClass::GraspContinuity => match securing_floor_violation(goal, report) {
            // GF1c floor from the execute message's force_profile, when present (transport /
            // release carry no floor in v0 -> vacuously Pass).
            Some(reason) => CheckOutcome::Fail(reason),
            None => CheckOutcome::Pass,
        },
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
            // Contact-maintenance band (force.wipe, spec/01 § 6.8): the two-sided lower+upper
            // edge. Vacuous without a normal_force band (insert_fit / screw / press_button).
            if let Some(reason) = contact_band_violation(goal, report) {
                return CheckOutcome::Fail(reason);
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
            // Held interval (transport.carry, § 4.4): if the action carries a
            // min_holding_force floor, the grasp must stay secured at >= floor at EVERY
            // interval sample — the held leg of the interval invariant. Vacuous for
            // reach.hover (no floor), so hover is unaffected.
            if let Some(reason) = securing_floor_violation(goal, report) {
                return CheckOutcome::Fail(reason);
            }
            // Station-keeping recovery (reach.hover settling, spec/01 § 1.5 line 668): the
            // settled tail must be within station_tolerance. Vacuous for transport.carry.
            if let Some(reason) = station_keeping_violation(goal, report) {
                return CheckOutcome::Fail(reason);
            }
            CheckOutcome::Pass
        }
    }
}

/// Verify the § 4.4 C2 graceful-degradation contract on an OVER-budget disturbance report:
/// the carry must NOT claim success, must report the `disturbance_exceeded` halt reason, and
/// must keep the object secured (`securing_force >= min_holding_force` at every sample — a
/// controlled halt, not a drop). Distinct from `check_envelope`: this judges the *failure
/// shape* of an over-budget injection (`spec/05` ENV3), not correct execution.
#[must_use]
pub fn check_graceful_degradation(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    if matches!(report.status.outcome, Outcome::Succeeded) {
        return CheckOutcome::Fail("claimed success under an over-budget disturbance".to_string());
    }
    if report.status.failure_detail.as_deref() != Some("disturbance_exceeded") {
        return CheckOutcome::Fail(format!(
            "expected failure_detail disturbance_exceeded, got {:?}",
            report.status.failure_detail
        ));
    }
    if let Some(reason) = securing_floor_violation(goal, report) {
        return CheckOutcome::Fail(format!("object not secured during halt: {reason}"));
    }
    CheckOutcome::Pass
}

/// Verify the § 1.5 C2 over-envelope contract for `reach.hover`: the hover must NOT claim
/// success and must report the `station_exceeded` halt reason (abort to a safe state). There
/// is no held object to secure, so failure-shape correctness IS the contract. Distinct from
/// `check_envelope`: this judges the failure shape of an over-envelope impulse (`05` ENV3),
/// not correct recovery.
#[must_use]
pub fn check_settling(report: &DriverReport) -> CheckOutcome {
    if matches!(report.status.outcome, Outcome::Succeeded) {
        return CheckOutcome::Fail(
            "claimed success under a station-exceeding disturbance".to_string(),
        );
    }
    if report.status.failure_detail.as_deref() != Some("station_exceeded") {
        return CheckOutcome::Fail(format!(
            "expected failure_detail station_exceeded, got {:?}",
            report.status.failure_detail
        ));
    }
    CheckOutcome::Pass
}

/// Verify the § 6.6 actuation postcondition for `force.press_button`: an actuated (Succeeded)
/// press MUST show the detent ForceEvent that marks actuation. Vacuous unless the action
/// carries an `actuation` contract (every non-press_button action passes). Makes the `events`
/// ForceEvent channel falsifiable — a success claimed without a detent fails.
#[must_use]
pub fn check_actuation(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    if goal
        .canonical_action
        .safety_envelope
        .force_profile
        .as_ref()
        .and_then(|fp| fp.get("actuation"))
        .is_none()
    {
        return CheckOutcome::Pass; // not an actuated press -> vacuous
    }
    if matches!(report.status.outcome, Outcome::Succeeded) {
        let has_detent = report.telemetry.iter().any(|t| {
            t.events
                .iter()
                .any(|e| e.get("kind").and_then(serde_json::Value::as_str) == Some("detent"))
        });
        if !has_detent {
            return CheckOutcome::Fail(
                "press_button claimed success without a detent actuation event".to_string(),
            );
        }
    }
    CheckOutcome::Pass
}

/// Verify the § 6.10 engagement-confirmation for `force.snap_engage`: a snap that succeeded
/// under a `confirm_held` contract MUST carry the held-confirmation evidence (the release-test
/// confirmed the bistable connection holds). Vacuous unless the action declares `confirm_held`.
/// Makes the AUD1 `verdict.evidence` channel falsifiable — a `false_engagement` claiming success
/// (snap detected but the connection does not hold) fails.
#[must_use]
pub fn check_engagement(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    let confirm = goal
        .canonical_action
        .safety_envelope
        .force_profile
        .as_ref()
        .and_then(|fp| fp.get("confirm_held"))
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    if !confirm {
        return CheckOutcome::Pass; // no confirm_held contract -> vacuous
    }
    if matches!(report.status.outcome, Outcome::Succeeded) {
        let held = report
            .status
            .verdict
            .as_ref()
            .is_some_and(|v| v.evidence.iter().any(|e| e == "held_confirmed"));
        if !held {
            return CheckOutcome::Fail(
                "snap_engage claimed success without confirming the connection holds (confirm_held)"
                    .to_string(),
            );
        }
    }
    CheckOutcome::Pass
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
        assert_eq!(envelope_class_for("carry"), Some(EnvelopeClass::IntervalInvariant));
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
                station_keeping: None,
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
            station_error: None,
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

    #[test]
    fn graceful_degradation_accepts_secured_halt_rejects_pretended_success() {
        use rfl_core::driver::{Outcome, RealizedPose, Status, Telemetry, Verdict};
        let goal = ExecuteGoal::wrap("s/e/0003-carry".to_string(), sample_action());
        // sample_action carries no force_profile, so the object-secured clause is vacuous here
        // (it is exercised end-to-end by the Drops integration test); this covers clauses 1-2.
        let report = |outcome: Outcome, detail: Option<&str>| {
            let status = Status {
                message: "status",
                action_id: "s/e/0003-carry".to_string(),
                outcome,
                verdict: Some(Verdict { value: false, confidence: 1.0, evidence: vec![] }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: detail.map(|_| "blocked".to_string()),
                failure_detail: detail.map(str::to_string),
            };
            DriverReport {
                telemetry: vec![Telemetry {
                    message: "telemetry",
                    action_id: "s/e/0003-carry".to_string(),
                    t: 1.0,
                    realized_pose: Some(RealizedPose::placeholder()),
                    wrench: None,
                    securing_force: None,
                    station_error: None,
                    tactile: vec![],
                    events: vec![],
                    fidelity_tier: None,
                }],
                status,
            }
        };
        // graceful halt: Failed + disturbance_exceeded -> Pass.
        assert_eq!(
            check_graceful_degradation(&goal, &report(Outcome::Failed, Some("disturbance_exceeded"))),
            CheckOutcome::Pass
        );
        // pretended success -> Fail (clause 1).
        assert!(matches!(
            check_graceful_degradation(&goal, &report(Outcome::Succeeded, None)),
            CheckOutcome::Fail(_)
        ));
        // wrong halt reason -> Fail (clause 2).
        assert!(matches!(
            check_graceful_degradation(&goal, &report(Outcome::Failed, Some("blocked"))),
            CheckOutcome::Fail(_)
        ));
    }

    #[test]
    fn check_settling_accepts_station_exceeded_abort_rejects_pretended_success() {
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        let status = |outcome: Outcome, detail: Option<&str>| Status {
            message: "status",
            action_id: "s/e/0001-hover".to_string(),
            outcome,
            verdict: Some(Verdict { value: false, confidence: 1.0, evidence: vec![] }),
            fidelity_tier: None,
            final_pose: Some(RealizedPose::placeholder()),
            failure_class: detail.map(|_| "blocked".to_string()),
            failure_detail: detail.map(str::to_string),
        };
        let report = |o, d| DriverReport { telemetry: vec![], status: status(o, d) };
        // abort to safe state: Failed + station_exceeded -> Pass.
        assert_eq!(check_settling(&report(Outcome::Failed, Some("station_exceeded"))), CheckOutcome::Pass);
        // pretended success -> Fail.
        assert!(matches!(check_settling(&report(Outcome::Succeeded, None)), CheckOutcome::Fail(_)));
        // wrong halt reason -> Fail.
        assert!(matches!(check_settling(&report(Outcome::Failed, Some("blocked"))), CheckOutcome::Fail(_)));
    }

    #[test]
    fn check_actuation_requires_a_detent_on_success() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode,
        };
        use rfl_core::driver::{Outcome, RealizedPose, Status, Telemetry, Verdict};
        let action = CanonicalAction {
            target_frame: "control".into(),
            target_pose: PoseExpr::Ref { r#ref: "button".into() },
            force_budget: Some(rfl_core::quantity::Quantity("5 N".into())),
            timing: TimingHints {
                nominal_duration: None,
                timing_mode: TimingMode::TimeScalable,
                stop_at_goal: true,
            },
            tactile_target: None,
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: Some(serde_json::json!({ "actuation": "detent" })),
                station_keeping: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
        };
        let goal = ExecuteGoal::wrap("s/e/0001-press_button".to_string(), action);
        let sample = |events: Vec<serde_json::Value>| Telemetry {
            message: "telemetry",
            action_id: "s/e/0001-press_button".to_string(),
            t: 1.0,
            realized_pose: Some(RealizedPose::placeholder()),
            wrench: None,
            securing_force: None,
            station_error: None,
            tactile: vec![],
            events,
            fidelity_tier: None,
        };
        let status = |outcome: Outcome| Status {
            message: "status",
            action_id: "s/e/0001-press_button".to_string(),
            outcome,
            verdict: Some(Verdict { value: true, confidence: 1.0, evidence: vec![] }),
            fidelity_tier: None,
            final_pose: Some(RealizedPose::placeholder()),
            failure_class: None,
            failure_detail: None,
        };
        // Succeeded + detent -> Pass.
        let ok = DriverReport {
            telemetry: vec![sample(vec![serde_json::json!({ "kind": "detent" })])],
            status: status(Outcome::Succeeded),
        };
        assert_eq!(check_actuation(&goal, &ok), CheckOutcome::Pass);
        // Succeeded + no detent -> Fail (the events bite).
        let claims =
            DriverReport { telemetry: vec![sample(vec![])], status: status(Outcome::Succeeded) };
        assert!(matches!(check_actuation(&goal, &claims), CheckOutcome::Fail(_)));
        // Not succeeded -> vacuously Pass.
        let bottoms =
            DriverReport { telemetry: vec![sample(vec![])], status: status(Outcome::Failed) };
        assert_eq!(check_actuation(&goal, &bottoms), CheckOutcome::Pass);
    }

    #[test]
    fn contact_band_rejects_loss_of_contact_and_over_force() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode,
        };
        use rfl_core::driver::{Outcome, RealizedPose, Status, Telemetry, Verdict, Wrench};
        let action = CanonicalAction {
            target_frame: "control".into(),
            target_pose: PoseExpr::Ref { r#ref: "panel".into() },
            force_budget: None,
            timing: TimingHints {
                nominal_duration: None,
                timing_mode: TimingMode::TimeScalable,
                stop_at_goal: true,
            },
            tactile_target: None,
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: Some(serde_json::json!({ "normal_force": "5 N", "normal_force_tolerance": "1 N" })),
                station_keeping: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
        };
        let goal = ExecuteGoal::wrap("s/e/0001-wipe".to_string(), action);
        let report = |fz: f64| {
            let t = Telemetry {
                message: "telemetry",
                action_id: "s/e/0001-wipe".to_string(),
                t: 1.0,
                realized_pose: Some(RealizedPose::placeholder()),
                wrench: Some(Wrench { force: [0.0, 0.0, fz], torque: [0.0, 0.0, 0.0] }),
                securing_force: None,
                station_error: None,
                tactile: vec![],
                events: vec![],
                fidelity_tier: None,
            };
            DriverReport {
                telemetry: vec![t],
                status: Status {
                    message: "status",
                    action_id: "s/e/0001-wipe".to_string(),
                    outcome: Outcome::Succeeded,
                    verdict: Some(Verdict { value: true, confidence: 1.0, evidence: vec![] }),
                    fidelity_tier: None,
                    final_pose: Some(RealizedPose::placeholder()),
                    failure_class: None,
                    failure_detail: None,
                },
            }
        };
        // in band (5 N) -> Pass.
        assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, &goal, &report(5.0)), CheckOutcome::Pass);
        // loss of contact (0 N, below 4) -> Fail.
        assert!(matches!(
            check_envelope(EnvelopeClass::ForceTrajectory, &goal, &report(0.0)),
            CheckOutcome::Fail(_)
        ));
        // over-force (9 N, above 6) -> Fail.
        assert!(matches!(
            check_envelope(EnvelopeClass::ForceTrajectory, &goal, &report(9.0)),
            CheckOutcome::Fail(_)
        ));
    }

    #[test]
    fn check_engagement_requires_held_confirmation_on_success() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode,
        };
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        let action = CanonicalAction {
            target_frame: "grasp".into(),
            target_pose: PoseExpr::Ref { r#ref: "clip".into() },
            force_budget: Some(rfl_core::quantity::Quantity("25 N".into())),
            timing: TimingHints {
                nominal_duration: None,
                timing_mode: TimingMode::TimeScalable,
                stop_at_goal: true,
            },
            tactile_target: None,
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: Some(serde_json::json!({ "actuation": "detent", "confirm_held": true })),
                station_keeping: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
        };
        let goal = ExecuteGoal::wrap("s/e/0001-snap_engage".to_string(), action);
        let report = |outcome: Outcome, evidence: Vec<String>| DriverReport {
            telemetry: vec![],
            status: Status {
                message: "status",
                action_id: "s/e/0001-snap_engage".to_string(),
                outcome,
                verdict: Some(Verdict { value: true, confidence: 1.0, evidence }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
            },
        };
        // Succeeded + held_confirmed -> Pass.
        assert_eq!(
            check_engagement(&goal, &report(Outcome::Succeeded, vec!["held_confirmed".to_string()])),
            CheckOutcome::Pass
        );
        // Succeeded + no held_confirmed -> Fail.
        assert!(matches!(
            check_engagement(&goal, &report(Outcome::Succeeded, vec![])),
            CheckOutcome::Fail(_)
        ));
        // Not succeeded -> vacuously Pass.
        assert_eq!(
            check_engagement(&goal, &report(Outcome::Failed, vec![])),
            CheckOutcome::Pass
        );
    }
}
