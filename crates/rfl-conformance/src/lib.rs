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

pub mod battery;
pub mod certificate;
pub mod certify;
pub mod replay;

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
    Ok(rfl_core::canonical::to_jsonl(
        &skill.skill,
        &emb.id,
        &out.actions,
        &out.suffixes,
    ))
}

use rfl_core::canonical::{ExecuteGoal, TactileTargetOut};
use rfl_core::driver::{
    Driver, DriverReport, FreedPartDisposition, Outcome, RealizedPose, SafetyFlags, Status,
    Telemetry, Verdict, Wrench,
};

/// A nominal-echo reference driver: it does not simulate physics; it returns the
/// in-protocol report a conformant driver would produce on a nominal execution,
/// deterministically from the `execute` message. The in-process Class 3 target
/// (`spec/05` § Four test classes). One telemetry sample + one terminal status per
/// action; `t` is the 1-based step counter.
#[derive(Debug, Default)]
pub struct ReferenceDriver {
    step: u32,
    /// AUD2 cross-action state: set once an `in_hand.flip` is seen, so `momentary_release`
    /// propagates into every downstream action's audit record.
    momentary_release_seen: bool,
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
        let force_mag = ca
            .force_budget
            .as_ref()
            .and_then(|q| q.parse().map(|(v, _)| v))
            .or_else(|| {
                ca.safety_envelope
                    .force_profile
                    .as_ref()
                    .and_then(|fp| fp.get("normal_force"))
                    .and_then(serde_json::Value::as_str)
                    .and_then(|s| {
                        rfl_core::quantity::Quantity(s.to_string())
                            .parse()
                            .map(|(v, _)| v)
                    })
            });
        let torque_mag = ca
            .safety_envelope
            .force_profile
            .as_ref()
            .and_then(|fp| fp.get("torque"))
            .and_then(serde_json::Value::as_str)
            .and_then(|s| {
                rfl_core::quantity::Quantity(s.to_string())
                    .parse()
                    .map(|(v, _)| v)
            });
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
        // GC2 (hold-test closure, spec/05): a grasp's success is confirmed by a hold test
        // whose perturbation profile is selected by the closure type. The nominal driver
        // applies the closure-appropriate profile and the object is retained. Only
        // grasp-establishing actions (pinch / pin / platform) carry a closure.
        if let Some(st) = &ca.grasp_stability {
            let profile = match st.closure {
                rfl_core::stability::Closure::Force => "omnidirectional",
                rfl_core::stability::Closure::Form => "load_direction",
                rfl_core::stability::Closure::Support => "level_gentle",
            };
            evidence.push(format!("hold_test:{profile}"));
        }
        // AUD2 (in_hand.flip, spec/05): the flip declares momentary_release and the flag
        // propagates into every downstream action's audit record (the first cross-action state).
        let is_flip = suffix_of(&goal.action_id) == "flip";
        if is_flip || self.momentary_release_seen {
            evidence.push("momentary_release".to_string());
        }
        if is_flip {
            self.momentary_release_seen = true;
        }
        // GC5 (bounded continuity-exception, spec/05): for a flip the nominal driver measures
        // an unsecured window within the declared max_release_time and confirms the re-catch.
        // v0 models the measured window as 80% of the bound (no physics, the ENV3 / GC2 posture).
        if let Some((bound, unit)) = ca
            .safety_envelope
            .force_profile
            .as_ref()
            .and_then(|fp| fp.get("max_release_time"))
            .and_then(serde_json::Value::as_str)
            .and_then(|s| {
                rfl_core::quantity::Quantity(s.to_string())
                    .parse()
                    .map(|(v, u)| (v, u.to_string()))
            })
        {
            let measured = rfl_core::quantity::Quantity::from_si(bound * 0.8, &unit);
            evidence.push(format!("unsecured_window:{}", measured.0));
            evidence.push("recatch_confirmed".to_string());
        }
        // Freed-part disposition (spec/04 TM21c): a freeing operation (force.unscrew, carrying
        // force_profile.on_disengagement) discloses where the freed part went — retained, or
        // released into a declared safe zone. Absent for every non-freeing action.
        let safety_flags = ca.safety_envelope.force_profile.as_ref().and_then(|fp| {
            if let Some(od) = fp
                .get("on_disengagement")
                .and_then(serde_json::Value::as_str)
            {
                let (disposition, zone) = if od == "drop_safe" {
                    (
                        "safe_zone_release",
                        Some(serde_json::json!({ "zone": "discard_bin" })),
                    )
                } else {
                    ("retained", None)
                };
                Some(SafetyFlags {
                    freed_part_disposition: Some(FreedPartDisposition {
                        disposition: disposition.to_string(),
                        zone,
                    }),
                })
            } else if fp.get("irreversible").and_then(serde_json::Value::as_bool) == Some(true) {
                // force.cut: the cut-off piece is retained (v0 conservative default).
                Some(SafetyFlags {
                    freed_part_disposition: Some(FreedPartDisposition {
                        disposition: "retained".to_string(),
                        zone: None,
                    }),
                })
            } else {
                None
            }
        });
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
            stop_latency: None,
            safety_flags,
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
    /// `status.fidelity_tier` over-claimed as `manifold` (undisclosed degradation, AUD3).
    FalseTier,
    /// The hold-test perturbation profile replaced with one wrong for the closure
    /// (`level_gentle` on a force-closure grasp — violates GC2 hold-test closure).
    WrongHoldTest,
    /// The flip's measured unsecured window driven over `max_release_time` (the object held
    /// unsecured too long — violates GC5 bounded continuity-exception).
    FlipWindowExceeded,
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
        FaultyDriver {
            inner: ReferenceDriver::default(),
            fault,
        }
    }
}

impl Driver for FaultyDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        match self.fault {
            Fault::UnderSecure => {
                for t in &mut report.telemetry {
                    if t.securing_force.is_some() {
                        t.securing_force = Some(rfl_core::quantity::Quantity("0.1 N".to_string()));
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
            Fault::FalseTier => {
                report.status.fidelity_tier = Some("manifold".to_string());
            }
            Fault::WrongHoldTest => {
                // Replace the closure-appropriate hold-test profile with a wrong one
                // (level_gentle would not certify a force-closure grasp), violating GC2.
                if let Some(v) = report.status.verdict.as_mut() {
                    for e in &mut v.evidence {
                        if e.starts_with("hold_test:") {
                            *e = "hold_test:level_gentle".to_string();
                        }
                    }
                }
            }
            Fault::FlipWindowExceeded => {
                // Drive the measured unsecured window over the declared bound (held unsecured
                // too long), violating GC5. The bound is read back from the goal's force_profile.
                let bound = goal
                    .canonical_action
                    .safety_envelope
                    .force_profile
                    .as_ref()
                    .and_then(|fp| fp.get("max_release_time"))
                    .and_then(serde_json::Value::as_str)
                    .and_then(|s| {
                        rfl_core::quantity::Quantity(s.to_string())
                            .parse()
                            .map(|(v, u)| (v, u.to_string()))
                    });
                if let (Some((bound, unit)), Some(v)) = (bound, report.status.verdict.as_mut()) {
                    let over = rfl_core::quantity::Quantity::from_si(bound * 1.5, &unit);
                    for e in &mut v.evidence {
                        if e.starts_with("unsecured_window:") {
                            *e = format!("unsecured_window:{}", over.0);
                        }
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
        DisturbanceDriver {
            inner: ReferenceDriver::default(),
            injected_n,
            response,
        }
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
            .and_then(|s| {
                rfl_core::quantity::Quantity(s.to_string())
                    .parse()
                    .map(|(v, _)| v)
            });
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
    /// Conformant (over-envelope): abort to a safe state (Failed + `station_exceeded`) within
    /// `stop_time`.
    Aborts,
    /// Adversarial (over-envelope): aborts honestly (`station_exceeded`) but overruns `stop_time`.
    AbortsTooSlow,
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
        HoverSettlingDriver {
            inner: ReferenceDriver::default(),
            response,
        }
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
            .and_then(|s| {
                rfl_core::quantity::Quantity(s.to_string())
                    .parse()
                    .map(|(v, _)| v)
            });
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
            HoverResponse::Aborts | HoverResponse::AbortsTooSlow => {
                report.status.outcome = Outcome::Failed;
                report.status.failure_class = Some("blocked".to_string());
                report.status.failure_detail = Some("station_exceeded".to_string());
                if let Some(v) = report.status.verdict.as_mut() {
                    v.value = false; // honest: the station was not held
                }
                // Emit the measured abort latency relative to the envelope stop_time: a
                // conformant abort halts within it; AbortsTooSlow overruns it.
                let stop = goal
                    .canonical_action
                    .safety_envelope
                    .stop_time
                    .as_ref()
                    .and_then(|q| q.parse().map(|(v, _)| v))
                    .unwrap_or(0.1);
                let factor = if matches!(self.response, HoverResponse::AbortsTooSlow) {
                    5.0
                } else {
                    0.5
                };
                report.status.stop_latency =
                    Some(rfl_core::quantity::Quantity::from_si(stop * factor, "s"));
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
        PressButtonDriver {
            inner: ReferenceDriver::default(),
            response,
        }
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
        SnapEngageDriver {
            inner: ReferenceDriver::default(),
            response,
        }
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

/// How a driver reports a `force.cut` (`spec/01` § 6.7). `Completes` is the nominal success;
/// `PartialReported` is the conformant interruption (the precise partial state is reported);
/// `BinaryHalt` is adversarial (an interrupted cut hides the partial state behind a bare failure).
#[derive(Debug, Clone, Copy)]
pub enum CutResponse {
    /// Conformant: the cut completed and separated (Succeeded).
    Completes,
    /// Conformant: interrupted, but reports the precise partial state (how far it progressed).
    PartialReported,
    /// Adversarial: interrupted, reports a bare binary failure with no partial state.
    BinaryHalt,
}

/// The `force.cut` bench: models a driver's cut outcome. Reuses the nominal `ReferenceDriver`
/// (a Succeeded cut) and mutates it per `response`. Non-cut actions pass through unchanged.
#[derive(Debug)]
pub struct CutDriver {
    inner: ReferenceDriver,
    response: CutResponse,
}

impl CutDriver {
    /// A cut driver with the given outcome.
    #[must_use]
    pub fn new(response: CutResponse) -> Self {
        CutDriver {
            inner: ReferenceDriver::default(),
            response,
        }
    }
}

impl Driver for CutDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        match self.response {
            CutResponse::Completes => {} // nominal: Succeeded
            CutResponse::PartialReported => {
                report.status.outcome = Outcome::Failed;
                report.status.failure_class = Some("blocked".to_string());
                report.status.failure_detail = Some("incomplete_cut".to_string());
                if let Some(v) = report.status.verdict.as_mut() {
                    v.value = false;
                    v.evidence.push("partial_cut: 0.6".to_string()); // the precise irreversible state
                }
            }
            CutResponse::BinaryHalt => {
                report.status.outcome = Outcome::Failed;
                report.status.failure_class = Some("blocked".to_string());
                report.status.failure_detail = Some("incomplete_cut".to_string());
                if let Some(v) = report.status.verdict.as_mut() {
                    v.value = false; // no partial_cut evidence -> a binary halt (adversarial)
                }
            }
        }
        report
    }
}

/// How a driver discloses a freeing operation's disposition (`spec/04` TM21c). `Discloses` is
/// conformant; `DropsUncontrolled` omits the disclosure (an uncontrolled drop); `FalseDisposition`
/// reports the opposite disposition (e.g. releases a part that should have been retained).
#[derive(Debug, Clone, Copy)]
pub enum FreeingResponse {
    /// Conformant: discloses the freed-part disposition (the nominal driver already does).
    Discloses,
    /// Adversarial: a freeing op with no disclosure — an uncontrolled drop.
    DropsUncontrolled,
    /// Adversarial: discloses the opposite disposition (released what should be retained).
    FalseDisposition,
}

/// The freed-part bench: reuses the nominal `ReferenceDriver` (which discloses the disposition for
/// a freeing op) and mutates that disclosure per `response`. Non-freeing actions have no
/// disclosure, so the mutations are no-ops on them.
#[derive(Debug)]
pub struct FreeingDriver {
    inner: ReferenceDriver,
    response: FreeingResponse,
}

impl FreeingDriver {
    /// A freeing driver with the given disclosure policy.
    #[must_use]
    pub fn new(response: FreeingResponse) -> Self {
        FreeingDriver {
            inner: ReferenceDriver::default(),
            response,
        }
    }
}

impl Driver for FreeingDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        match self.response {
            FreeingResponse::Discloses => {} // nominal: the disclosure stands
            FreeingResponse::DropsUncontrolled => {
                report.status.safety_flags = None; // freeing with no disposition recorded
            }
            FreeingResponse::FalseDisposition => {
                if let Some(d) = report
                    .status
                    .safety_flags
                    .as_mut()
                    .and_then(|sf| sf.freed_part_disposition.as_mut())
                {
                    d.disposition = if d.disposition == "retained" {
                        "safe_zone_release".to_string()
                    } else {
                        "retained".to_string()
                    };
                }
            }
        }
        report
    }
}

/// How a driver reports an `in_hand.flip` sequence (`spec/05` AUD2). `Propagates` is conformant
/// (the flip declares momentary_release and it propagates downstream); `SuppressesFlip` omits the
/// flag on the flip; `DropsDownstream` keeps it on the flip but strips it from later actions.
#[derive(Debug, Clone, Copy)]
pub enum FlipResponse {
    /// Conformant: declared on the flip and propagated downstream.
    Propagates,
    /// Adversarial: the flip omits momentary_release (claims continuity preserved).
    SuppressesFlip,
    /// Adversarial: declared on the flip but dropped from every downstream action.
    DropsDownstream,
}

/// The `in_hand.flip` AUD2 bench: reuses the nominal `ReferenceDriver` (which declares +
/// propagates momentary_release) and mutates the audit trail per `response`.
#[derive(Debug)]
pub struct FlipDriver {
    inner: ReferenceDriver,
    response: FlipResponse,
}

impl FlipDriver {
    /// A flip driver with the given audit-trail response.
    #[must_use]
    pub fn new(response: FlipResponse) -> Self {
        FlipDriver {
            inner: ReferenceDriver::default(),
            response,
        }
    }
}

impl Driver for FlipDriver {
    fn execute(&mut self, goal: &ExecuteGoal) -> DriverReport {
        let mut report = self.inner.execute(goal);
        let is_flip = suffix_of(&goal.action_id) == "flip";
        let strip = |report: &mut DriverReport| {
            if let Some(v) = report.status.verdict.as_mut() {
                v.evidence.retain(|e| e != "momentary_release");
            }
        };
        match self.response {
            FlipResponse::Propagates => {} // passthrough — ReferenceDriver declares + propagates
            FlipResponse::SuppressesFlip => {
                if is_flip {
                    strip(&mut report); // the flip omits the flag
                }
            }
            FlipResponse::DropsDownstream => {
                if !is_flip {
                    strip(&mut report); // pre-flip have nothing; post-flip lose the propagated flag
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
    Ok(
        drive(ReferenceDriver::default(), skill_path, embodiment_path)?
            .into_iter()
            .map(|(_, report)| report)
            .collect(),
    )
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
        "pinch" | "release" | "transport" | "flip" => Some(EnvelopeClass::GraspContinuity),
        "insert_fit" | "screw" | "unscrew" | "press_button" | "wipe" | "snap_engage" | "cut" => {
            Some(EnvelopeClass::ForceTrajectory)
        }
        "hover" | "carry" => Some(EnvelopeClass::IntervalInvariant),
        _ => None, // locate / inspect: perception, no envelope
    }
}

/// The primitive suffix of an action id (`.../NNNN-<suffix>`; suffixes contain no `-`).
pub(crate) fn suffix_of(action_id: &str) -> &str {
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
        .and_then(|s| {
            rfl_core::quantity::Quantity(s.to_string())
                .parse()
                .map(|(v, _)| v)
        })?;
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
    let sk = goal
        .canonical_action
        .safety_envelope
        .station_keeping
        .as_ref()?;
    let tol = sk
        .get("station_tolerance")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| {
            rfl_core::quantity::Quantity(s.to_string())
                .parse()
                .map(|(v, _)| v)
        })?;
    let settle = sk
        .get("settling_time")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| {
            rfl_core::quantity::Quantity(s.to_string())
                .parse()
                .map(|(v, _)| v)
        })?;
    let deadline = report.telemetry.first()?.t + settle;
    let mut tail_seen = false;
    for t in &report.telemetry {
        if t.t < deadline {
            continue; // recovery grace window — excursion permitted here
        }
        tail_seen = true;
        match t.station_error.as_ref().and_then(quantity_mag) {
            None => {
                return Some(format!(
                    "settled-tail sample at t={} missing station_error",
                    t.t
                ));
            }
            Some(err) if err > tol => {
                return Some(format!(
                    "station_error {err} > station_tolerance {tol} at t={}",
                    t.t
                ));
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
    let fp = goal
        .canonical_action
        .safety_envelope
        .force_profile
        .as_ref()?;
    let setpoint = fp
        .get("normal_force")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| {
            rfl_core::quantity::Quantity(s.to_string())
                .parse()
                .map(|(v, _)| v)
        })?;
    let tol = fp
        .get("normal_force_tolerance")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| {
            rfl_core::quantity::Quantity(s.to_string())
                .parse()
                .map(|(v, _)| v)
        })?;
    let (lo, hi) = (setpoint - tol, setpoint + tol);
    for t in &report.telemetry {
        if let Some(w) = &t.wrench {
            let mag = w.force.iter().map(|x| x * x).sum::<f64>().sqrt();
            if mag < lo {
                return Some(format!(
                    "contact lost: |wrench.force| {mag} < {lo} (normal_force {setpoint} − tol {tol})"
                ));
            }
            if mag > hi {
                return Some(format!(
                    "over-force: |wrench.force| {mag} > {hi} (normal_force {setpoint} + tol {tol})"
                ));
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
            if let Some(budget) = goal
                .canonical_action
                .force_budget
                .as_ref()
                .and_then(quantity_mag)
            {
                for t in &report.telemetry {
                    if let Some(w) = &t.wrench {
                        let mag = w.force.iter().map(|x| x * x).sum::<f64>().sqrt();
                        if mag > budget {
                            return CheckOutcome::Fail(format!(
                                "|wrench.force| {mag} > budget {budget}"
                            ));
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
                .and_then(|s| {
                    rfl_core::quantity::Quantity(s.to_string())
                        .parse()
                        .map(|(v, _)| v)
                });
            if let Some(tb) = torque_budget {
                for t in &report.telemetry {
                    if let Some(w) = &t.wrench {
                        let mag = w.torque.iter().map(|x| x * x).sum::<f64>().sqrt();
                        if mag > tb {
                            return CheckOutcome::Fail(format!(
                                "|wrench.torque| {mag} > torque budget {tb}"
                            ));
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
                    return CheckOutcome::Fail(format!(
                        "interval sample {i} missing realized_pose"
                    ));
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
pub fn check_settling(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
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
    // Timing leg (spec/01 § 1.5 C2): the abort must reach a safe state within stop_time.
    // Non-vacuous for the station_exceeded abort — an abort that omits its measured time, or
    // that overruns stop_time, is malformed.
    let bound = goal
        .canonical_action
        .safety_envelope
        .stop_time
        .as_ref()
        .and_then(quantity_mag);
    let Some(bound) = bound else {
        return CheckOutcome::Fail("envelope missing stop_time for an aborting hover".to_string());
    };
    match report.status.stop_latency.as_ref().and_then(quantity_mag) {
        None => CheckOutcome::Fail("station_exceeded abort reported no stop_latency".to_string()),
        Some(lat) if lat > bound => CheckOutcome::Fail(format!(
            "abort stop_latency {lat} s exceeds stop_time {bound} s"
        )),
        Some(_) => CheckOutcome::Pass,
    }
}

/// Verify the `spec/04` TM21c freed-part disposition contract: a freeing operation (an action
/// whose `force_profile.on_disengagement` is present — `force.unscrew`) must disclose
/// `status.safety_flags.freed_part_disposition` matching the authored intent (`retain` →
/// `retained`, `drop_safe` → `safe_zone_release`). An undisclosed freeing is an uncontrolled
/// drop (forbidden). Vacuous for any action without `on_disengagement`.
#[must_use]
pub fn check_freed_part_disposition(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    // A freeing completes only on success; an interrupted op froze nothing.
    if !matches!(report.status.outcome, Outcome::Succeeded) {
        return CheckOutcome::Pass;
    }
    let fp = goal.canonical_action.safety_envelope.force_profile.as_ref();
    let on_diseng = fp
        .and_then(|fp| fp.get("on_disengagement"))
        .and_then(serde_json::Value::as_str);
    let irreversible = fp
        .and_then(|fp| fp.get("irreversible"))
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    // Expected disposition: Some for an authored on_disengagement (force.unscrew, expected-match),
    // None for a presence-only freeing (force.cut, no authored intent). Not a freeing op -> Pass.
    let expected: Option<&str> = match on_diseng {
        Some("drop_safe") => Some("safe_zone_release"),
        Some(_) => Some("retained"),
        None if irreversible => None,
        None => return CheckOutcome::Pass,
    };
    let disclosed = report
        .status
        .safety_flags
        .as_ref()
        .and_then(|sf| sf.freed_part_disposition.as_ref());
    match (disclosed, expected) {
        (None, _) => CheckOutcome::Fail(
            "uncontrolled drop: a freeing operation disclosed no freed_part_disposition"
                .to_string(),
        ),
        (Some(d), Some(e)) if d.disposition != e => CheckOutcome::Fail(format!(
            "freed_part_disposition {} does not match the authored intent {e}",
            d.disposition
        )),
        (Some(d), None) if d.disposition != "retained" && d.disposition != "safe_zone_release" => {
            CheckOutcome::Fail(format!("unknown freed_part_disposition {}", d.disposition))
        }
        (Some(_), _) => CheckOutcome::Pass,
    }
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

/// Verify the § 175 / § 6.7 irreversibility obligation for `force.cut`: an irreversible op that
/// is INTERRUPTED (outcome != Succeeded) MUST report the precise partial state (how far it
/// progressed) in `verdict.evidence`, never a bare binary failure. Vacuous unless the action
/// declares `irreversible`; a completed (Succeeded) cut is vacuous (nothing partial to report).
/// The mirror of `check_actuation`/`check_engagement`: those bite the success path, this bites
/// the failure path.
#[must_use]
pub fn check_irreversible(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    let irreversible = goal
        .canonical_action
        .safety_envelope
        .force_profile
        .as_ref()
        .and_then(|fp| fp.get("irreversible"))
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    if !irreversible {
        return CheckOutcome::Pass; // not an irreversible op -> vacuous
    }
    if !matches!(report.status.outcome, Outcome::Succeeded) {
        let reported = report
            .status
            .verdict
            .as_ref()
            .is_some_and(|v| v.evidence.iter().any(|e| e.starts_with("partial_cut")));
        if !reported {
            return CheckOutcome::Fail(
                "interrupted irreversible cut reported a binary failure without the precise partial state"
                    .to_string(),
            );
        }
    }
    CheckOutcome::Pass
}

/// Verify the AUD3 fidelity-tier honesty obligation (`spec/05`): a degraded execution must
/// disclose it — a result reported at full `manifold` tier when the action lowered to `proxy`
/// is malformed. The expected tier is the lowering's `tactile_target` degradation decision
/// (`Auto` => manifold, `Proxy` => proxy); the claimed tier is `status.fidelity_tier`.
/// Over-claiming (manifold claimed when proxy was the truth) is the violation; under-claiming is
/// conservative. Vacuous when the action carries no auto-confirmation (`Explicit` / none).
#[must_use]
pub fn check_audit_honesty(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    let expected = match &goal.canonical_action.tactile_target {
        Some(TactileTargetOut::Auto) => "manifold",
        Some(TactileTargetOut::Proxy { .. }) => "proxy",
        _ => return CheckOutcome::Pass, // no auto-confirmation -> nothing to disclose
    };
    if report.status.fidelity_tier.as_deref() == Some("manifold") && expected == "proxy" {
        return CheckOutcome::Fail(
            "degraded (proxy) execution claimed manifold tier — undisclosed degradation (AUD3)"
                .to_string(),
        );
    }
    CheckOutcome::Pass
}

/// Verify the AUD2 obligation (`spec/05`): the continuity-suspending `in_hand.flip` declares
/// `momentary_release`, and the flag is **propagated** to every downstream action's audit record
/// (so the L4 / L8 loops trace the continuity break several primitives later). The FIRST
/// sequence-level check — a pure function of the whole `drive()` sequence, not a single pair.
/// Vacuous when the sequence contains no flip.
#[must_use]
pub fn check_momentary_release(pairs: &[(ExecuteGoal, DriverReport)]) -> CheckOutcome {
    let declares = |r: &DriverReport| {
        r.status
            .verdict
            .as_ref()
            .is_some_and(|v| v.evidence.iter().any(|e| e == "momentary_release"))
    };
    let Some(flip_idx) = pairs
        .iter()
        .position(|(g, _)| suffix_of(&g.action_id) == "flip")
    else {
        return CheckOutcome::Pass; // no continuity break -> nothing to trace
    };
    if !declares(&pairs[flip_idx].1) {
        return CheckOutcome::Fail(
            "in_hand.flip did not declare momentary_release (transparency violation)".to_string(),
        );
    }
    for (g, r) in &pairs[flip_idx + 1..] {
        if !declares(r) {
            return CheckOutcome::Fail(format!(
                "momentary_release not propagated to downstream action {}",
                g.action_id
            ));
        }
    }
    CheckOutcome::Pass
}

/// Verify the AUD1 audit-record obligation (`spec/05`): every primitive result contributes an
/// audit record — a `verdict` with evidence. A status with no verdict, or a verdict with no
/// evidence, leaves the L4 / L8 loops nothing to read.
#[must_use]
pub fn check_audit_record(report: &DriverReport) -> CheckOutcome {
    match &report.status.verdict {
        None => CheckOutcome::Fail("no audit record: status carries no verdict".to_string()),
        Some(v) if v.evidence.is_empty() => {
            CheckOutcome::Fail("audit record has no evidence".to_string())
        }
        Some(_) => CheckOutcome::Pass,
    }
}

/// Verify the STB2 support-safe-state obligation (`spec/05` § Closure/stability): a
/// support-closure grasp (`grasp.platform`) must declare a controlled-lowering safe state,
/// never an open-release — opening drops a balanced object. Decidable from the goal alone
/// (static). Vacuous-pass for non-grasp actions and for force/form-closure grasps, whose
/// open-and-withdraw safe state is correct.
#[must_use]
pub fn check_support_safe_state(goal: &ExecuteGoal) -> CheckOutcome {
    let ca = &goal.canonical_action;
    let Some(st) = &ca.grasp_stability else {
        return CheckOutcome::Pass; // non-grasp action
    };
    if st.closure != rfl_core::stability::Closure::Support {
        return CheckOutcome::Pass; // force / form closure: open-withdraw safe state is fine
    }
    let safe_state = ca
        .safety_envelope
        .force_profile
        .as_ref()
        .and_then(|fp| fp.get("safe_state"))
        .and_then(serde_json::Value::as_str);
    match safe_state {
        Some("controlled_lowering") => CheckOutcome::Pass,
        other => CheckOutcome::Fail(format!(
            "a support grasp must declare a controlled-lowering safe state, not {other:?}"
        )),
    }
}

/// Verify the GC2 hold-test-closure obligation (`spec/05` § The hold test): a successful
/// grasp's closure is confirmed by a hold test whose perturbation profile is branched on the
/// closure type — `force → omnidirectional`, `form → load_direction`, `support → level_gentle`.
/// Reads the `hold_test:<profile>` evidence the driver records (`verdict.evidence`). Vacuous-
/// pass for non-grasp actions and for a grasp that did not report `Succeeded` (a non-success
/// has its own failure path).
#[must_use]
pub fn check_hold_test(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    let Some(st) = &goal.canonical_action.grasp_stability else {
        return CheckOutcome::Pass; // non-grasp action
    };
    if report.status.outcome != rfl_core::driver::Outcome::Succeeded {
        return CheckOutcome::Pass;
    }
    let expected = match st.closure {
        rfl_core::stability::Closure::Force => "omnidirectional",
        rfl_core::stability::Closure::Form => "load_direction",
        rfl_core::stability::Closure::Support => "level_gentle",
    };
    let reported = report
        .status
        .verdict
        .as_ref()
        .and_then(|v| v.evidence.iter().find_map(|e| e.strip_prefix("hold_test:")));
    match reported {
        Some(p) if p == expected => CheckOutcome::Pass,
        Some(p) => CheckOutcome::Fail(format!(
            "hold test used a {p} perturbation but {expected} is required for {:?} closure",
            st.closure
        )),
        None => CheckOutcome::Fail(
            "a successful grasp must report a hold test (no hold_test \
                                evidence)"
                .to_string(),
        ),
    }
}

/// Verify the GC5 bounded-continuity-exception obligation (`spec/05` § Bounded continuity-
/// exception): `in_hand.flip` suspends grasp continuity, but the suspension is bounded — the
/// measured unsecured window must be `≤ max_release_time` and the re-catch confirmed. Keyed on
/// the action carrying a `max_release_time` bound (only a flip does), so vacuous-pass for every
/// other action. A non-`Succeeded` flip is the controlled-failure path (`recatch_failed` →
/// safe drop), verified elsewhere, so it is vacuous here.
#[must_use]
pub fn check_flip_bounded_window(goal: &ExecuteGoal, report: &DriverReport) -> CheckOutcome {
    let Some((bound, _)) = goal
        .canonical_action
        .safety_envelope
        .force_profile
        .as_ref()
        .and_then(|fp| fp.get("max_release_time"))
        .and_then(serde_json::Value::as_str)
        .and_then(|s| {
            rfl_core::quantity::Quantity(s.to_string())
                .parse()
                .map(|(v, u)| (v, u.to_string()))
        })
    else {
        return CheckOutcome::Pass; // not a bounded-exception action
    };
    if report.status.outcome != rfl_core::driver::Outcome::Succeeded {
        return CheckOutcome::Pass; // controlled-failure (safe-drop) path is verified elsewhere
    }
    let evidence = report.status.verdict.as_ref().map(|v| &v.evidence);
    let measured = evidence.and_then(|ev| {
        ev.iter()
            .find_map(|e| e.strip_prefix("unsecured_window:"))
            .and_then(|s| {
                rfl_core::quantity::Quantity(s.to_string())
                    .parse()
                    .map(|(v, _)| v)
            })
    });
    let Some(measured) = measured else {
        return CheckOutcome::Fail(
            "a successful flip must report its measured unsecured window".to_string(),
        );
    };
    if measured > bound {
        return CheckOutcome::Fail(format!(
            "unsecured window {measured} exceeds max_release_time {bound}"
        ));
    }
    let recatch = evidence.is_some_and(|ev| ev.iter().any(|e| e == "recatch_confirmed"));
    if !recatch {
        return CheckOutcome::Fail("a successful flip must confirm the re-catch".to_string());
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
            assert!(matches!(
                r.status.outcome,
                rfl_core::driver::Outcome::Succeeded
            ));
            for t in &r.telemetry {
                assert_eq!(t.action_id, r.status.action_id); // correlation
            }
        }
        // pneumatic has no tactile sensing -> grasp.pinch confirmation degrades to proxy
        assert_eq!(reports[1].status.fidelity_tier.as_deref(), Some("proxy"));
    }

    #[test]
    fn envelope_class_mapping_follows_env1() {
        assert_eq!(
            envelope_class_for("align"),
            Some(EnvelopeClass::TerminalPostcondition)
        );
        assert_eq!(
            envelope_class_for("retract"),
            Some(EnvelopeClass::TerminalPostcondition)
        );
        assert_eq!(
            envelope_class_for("pinch"),
            Some(EnvelopeClass::GraspContinuity)
        );
        assert_eq!(
            envelope_class_for("transport"),
            Some(EnvelopeClass::GraspContinuity)
        );
        assert_eq!(
            envelope_class_for("insert_fit"),
            Some(EnvelopeClass::ForceTrajectory)
        );
        assert_eq!(
            envelope_class_for("unscrew"),
            Some(EnvelopeClass::ForceTrajectory)
        );
        assert_eq!(
            envelope_class_for("hover"),
            Some(EnvelopeClass::IntervalInvariant)
        );
        assert_eq!(
            envelope_class_for("carry"),
            Some(EnvelopeClass::IntervalInvariant)
        );
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
        assert_eq!(
            check_envelope(EnvelopeClass::GraspContinuity, goal, report),
            CheckOutcome::Pass
        );
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
        assert_eq!(
            check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
            CheckOutcome::Pass
        );
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
        assert_eq!(
            check_envelope(EnvelopeClass::TerminalPostcondition, goal, report),
            CheckOutcome::Pass
        );
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
        assert_eq!(
            pairs[1].1.telemetry[0].securing_force.as_ref().unwrap().0,
            "0.1 N"
        );
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
        assert_eq!(
            check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
            CheckOutcome::Pass
        );
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
            target_pose: PoseExpr::Ref {
                r#ref: "panel".into(),
            },
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
            grasp_stability: None,
        }
    }

    #[test]
    fn support_safe_state_requires_controlled_lowering() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode,
        };
        use rfl_core::grasp_force::GraspMode;
        use rfl_core::stability::StabilityMetadata;
        let goal = |mode: Option<GraspMode>, safe: Option<&str>| {
            let force_profile = safe.map(|s| serde_json::json!({ "safe_state": s }));
            let ca = CanonicalAction {
                target_frame: "support".into(),
                target_pose: PoseExpr::Ref {
                    r#ref: "tray".into(),
                },
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
                    force_profile,
                    station_keeping: None,
                    clearance: None,
                    compliance: None,
                    stop_time: None,
                },
                grasp_stability: mode.map(StabilityMetadata::for_mode),
            };
            ExecuteGoal::wrap("s/e/0001-platform".to_string(), ca)
        };
        // a real support grasp declares controlled lowering -> pass.
        assert_eq!(
            check_support_safe_state(&goal(
                Some(GraspMode::Platform),
                Some("controlled_lowering")
            )),
            CheckOutcome::Pass
        );
        // a support grasp that would open-release a balanced object -> fail (the bite).
        assert!(matches!(
            check_support_safe_state(&goal(Some(GraspMode::Platform), Some("open_withdraw"))),
            CheckOutcome::Fail(_)
        ));
        // a support grasp with no declared safe state -> fail.
        assert!(matches!(
            check_support_safe_state(&goal(Some(GraspMode::Platform), None)),
            CheckOutcome::Fail(_)
        ));
        // force-closure (pinch) and non-grasp actions are vacuous-pass.
        assert_eq!(
            check_support_safe_state(&goal(Some(GraspMode::Pinch), None)),
            CheckOutcome::Pass
        );
        assert_eq!(
            check_support_safe_state(&goal(None, None)),
            CheckOutcome::Pass
        );
    }

    #[test]
    fn hold_test_must_match_the_closure_perturbation_profile() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode,
        };
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        use rfl_core::grasp_force::GraspMode;
        use rfl_core::stability::StabilityMetadata;
        let goal = |mode: Option<GraspMode>| {
            let ca = CanonicalAction {
                target_frame: "grip".into(),
                target_pose: PoseExpr::Ref {
                    r#ref: "part".into(),
                },
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
                grasp_stability: mode.map(StabilityMetadata::for_mode),
            };
            ExecuteGoal::wrap("s/e/0001-pinch".to_string(), ca)
        };
        let report = |outcome: Outcome, evidence: &[&str]| DriverReport {
            telemetry: vec![],
            status: Status {
                message: "status",
                action_id: "s/e/0001-pinch".to_string(),
                outcome,
                verdict: Some(Verdict {
                    value: true,
                    confidence: 1.0,
                    evidence: evidence.iter().map(|s| (*s).to_string()).collect(),
                }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
                stop_latency: None,
                safety_flags: None,
            },
        };
        // force closure confirmed by an omnidirectional hold test -> pass.
        assert_eq!(
            check_hold_test(
                &goal(Some(GraspMode::Pinch)),
                &report(Outcome::Succeeded, &["hold_test:omnidirectional"])
            ),
            CheckOutcome::Pass
        );
        // wrong profile for a force closure (level_gentle would not certify a force grasp) -> fail.
        assert!(matches!(
            check_hold_test(
                &goal(Some(GraspMode::Pinch)),
                &report(Outcome::Succeeded, &["hold_test:level_gentle"])
            ),
            CheckOutcome::Fail(_)
        ));
        // a successful grasp with no hold test reported -> fail.
        assert!(matches!(
            check_hold_test(
                &goal(Some(GraspMode::Pinch)),
                &report(Outcome::Succeeded, &["nominal"])
            ),
            CheckOutcome::Fail(_)
        ));
        // support closure is confirmed by a level, gentle hold test -> pass.
        assert_eq!(
            check_hold_test(
                &goal(Some(GraspMode::Platform)),
                &report(Outcome::Succeeded, &["hold_test:level_gentle"])
            ),
            CheckOutcome::Pass
        );
        // non-grasp action, and a non-Succeeded grasp, are vacuous-pass.
        assert_eq!(
            check_hold_test(&goal(None), &report(Outcome::Succeeded, &[])),
            CheckOutcome::Pass
        );
        assert_eq!(
            check_hold_test(&goal(Some(GraspMode::Pinch)), &report(Outcome::Failed, &[])),
            CheckOutcome::Pass
        );
    }

    #[test]
    fn flip_bounded_window_must_stay_within_max_release_time() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode,
        };
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        // a flip goal carries max_release_time; a non-flip goal does not.
        let goal = |bounded: bool| {
            let force_profile = bounded.then(
                || serde_json::json!({ "max_release_time": "0.3 s", "safe_drop_zone": "tray" }),
            );
            let ca = CanonicalAction {
                target_frame: "grip".into(),
                target_pose: PoseExpr::AxisRelative {
                    direction: serde_json::json!("+x"),
                    distance: rfl_core::quantity::Quantity("0 mm".into()),
                },
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
                    force_profile,
                    station_keeping: None,
                    clearance: None,
                    compliance: None,
                    stop_time: None,
                },
                grasp_stability: None,
            };
            ExecuteGoal::wrap("s/e/0003-flip".to_string(), ca)
        };
        let report = |outcome: Outcome, evidence: &[&str]| DriverReport {
            telemetry: vec![],
            status: Status {
                message: "status",
                action_id: "s/e/0003-flip".to_string(),
                outcome,
                verdict: Some(Verdict {
                    value: true,
                    confidence: 1.0,
                    evidence: evidence.iter().map(|s| (*s).to_string()).collect(),
                }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
                stop_latency: None,
                safety_flags: None,
            },
        };
        // window within bound + recatch confirmed -> pass.
        assert_eq!(
            check_flip_bounded_window(
                &goal(true),
                &report(
                    Outcome::Succeeded,
                    &["unsecured_window:0.24 s", "recatch_confirmed"]
                )
            ),
            CheckOutcome::Pass
        );
        // window over the 0.3 s bound -> fail (the bite).
        assert!(matches!(
            check_flip_bounded_window(
                &goal(true),
                &report(
                    Outcome::Succeeded,
                    &["unsecured_window:0.45 s", "recatch_confirmed"]
                )
            ),
            CheckOutcome::Fail(_)
        ));
        // no measured window, and no recatch confirmation -> fail.
        assert!(matches!(
            check_flip_bounded_window(
                &goal(true),
                &report(Outcome::Succeeded, &["recatch_confirmed"])
            ),
            CheckOutcome::Fail(_)
        ));
        assert!(matches!(
            check_flip_bounded_window(
                &goal(true),
                &report(Outcome::Succeeded, &["unsecured_window:0.24 s"])
            ),
            CheckOutcome::Fail(_)
        ));
        // a non-flip action (no bound), and a non-Succeeded flip (controlled failure), are vacuous.
        assert_eq!(
            check_flip_bounded_window(&goal(false), &report(Outcome::Succeeded, &[])),
            CheckOutcome::Pass
        );
        assert_eq!(
            check_flip_bounded_window(&goal(true), &report(Outcome::Failed, &[])),
            CheckOutcome::Pass
        );
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
            verdict: Some(Verdict {
                value: true,
                confidence: 1.0,
                evidence: vec![],
            }),
            fidelity_tier: None,
            final_pose: Some(RealizedPose::placeholder()),
            failure_class: None,
            failure_detail: None,
            stop_latency: None,
            safety_flags: None,
        };
        let ok = DriverReport {
            telemetry: vec![sample(Some(RealizedPose::placeholder())); 3],
            status: status.clone(),
        };
        assert_eq!(
            check_envelope(EnvelopeClass::IntervalInvariant, &goal, &ok),
            CheckOutcome::Pass
        );
        // Drop the middle sample's pose: interval fails, but the endpoint (terminal) is fine.
        let mut bad = ok.clone();
        bad.telemetry[1].realized_pose = None;
        assert!(matches!(
            check_envelope(EnvelopeClass::IntervalInvariant, &goal, &bad),
            CheckOutcome::Fail(_)
        ));
        assert_eq!(
            check_envelope(EnvelopeClass::TerminalPostcondition, &goal, &bad),
            CheckOutcome::Pass
        );
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
                verdict: Some(Verdict {
                    value: false,
                    confidence: 1.0,
                    evidence: vec![],
                }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: detail.map(|_| "blocked".to_string()),
                failure_detail: detail.map(str::to_string),
                stop_latency: None,
                safety_flags: None,
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
            check_graceful_degradation(
                &goal,
                &report(Outcome::Failed, Some("disturbance_exceeded"))
            ),
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
        use rfl_core::quantity::Quantity;
        let mut action = sample_action();
        action.safety_envelope.stop_time = Some(Quantity("0.1 s".into()));
        let goal = ExecuteGoal::wrap("s/e/0001-hover".to_string(), action);
        let status = |outcome: Outcome, detail: Option<&str>, lat: Option<&str>| Status {
            message: "status",
            action_id: "s/e/0001-hover".to_string(),
            outcome,
            verdict: Some(Verdict {
                value: false,
                confidence: 1.0,
                evidence: vec![],
            }),
            fidelity_tier: None,
            final_pose: Some(RealizedPose::placeholder()),
            failure_class: detail.map(|_| "blocked".to_string()),
            failure_detail: detail.map(str::to_string),
            stop_latency: lat.map(|s| Quantity(s.to_string())),
            safety_flags: None,
        };
        let report = |o, d, l| DriverReport {
            telemetry: vec![],
            status: status(o, d, l),
        };
        // abort within stop_time -> Pass.
        assert_eq!(
            check_settling(
                &goal,
                &report(Outcome::Failed, Some("station_exceeded"), Some("0.05 s"))
            ),
            CheckOutcome::Pass
        );
        // abort too slow (> stop_time) -> Fail (the new timing leg).
        assert!(matches!(
            check_settling(
                &goal,
                &report(Outcome::Failed, Some("station_exceeded"), Some("0.5 s"))
            ),
            CheckOutcome::Fail(_)
        ));
        // abort with no measured latency -> Fail (malformed abort; non-vacuous timing leg).
        assert!(matches!(
            check_settling(
                &goal,
                &report(Outcome::Failed, Some("station_exceeded"), None)
            ),
            CheckOutcome::Fail(_)
        ));
        // pretended success -> Fail (outcome leg, unchanged).
        assert!(matches!(
            check_settling(&goal, &report(Outcome::Succeeded, None, None)),
            CheckOutcome::Fail(_)
        ));
        // wrong halt reason -> Fail (outcome leg, unchanged).
        assert!(matches!(
            check_settling(&goal, &report(Outcome::Failed, Some("blocked"), None)),
            CheckOutcome::Fail(_)
        ));
    }

    #[test]
    fn check_freed_part_disposition_requires_a_disclosure_matching_intent() {
        use rfl_core::driver::{
            FreedPartDisposition, Outcome, RealizedPose, SafetyFlags, Status, Verdict,
        };
        // a freeing action: force_profile.on_disengagement = "retain" -> expected "retained".
        let mut action = sample_action();
        action.safety_envelope.force_profile =
            Some(serde_json::json!({ "on_disengagement": "retain" }));
        let goal = ExecuteGoal::wrap("s/e/0005-unscrew".to_string(), action);
        let report = |flags: Option<SafetyFlags>| {
            let status = Status {
                message: "status",
                action_id: "s/e/0005-unscrew".to_string(),
                outcome: Outcome::Succeeded,
                verdict: Some(Verdict {
                    value: true,
                    confidence: 1.0,
                    evidence: vec![],
                }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
                stop_latency: None,
                safety_flags: flags,
            };
            DriverReport {
                telemetry: vec![],
                status,
            }
        };
        let disp = |d: &str| {
            Some(SafetyFlags {
                freed_part_disposition: Some(FreedPartDisposition {
                    disposition: d.to_string(),
                    zone: None,
                }),
            })
        };
        // discloses retained (matches retain) -> Pass.
        assert_eq!(
            check_freed_part_disposition(&goal, &report(disp("retained"))),
            CheckOutcome::Pass
        );
        // no disclosure on a freeing op -> uncontrolled drop -> Fail.
        assert!(matches!(
            check_freed_part_disposition(&goal, &report(None)),
            CheckOutcome::Fail(_)
        ));
        // wrong disposition (released what should be retained) -> Fail.
        assert!(matches!(
            check_freed_part_disposition(&goal, &report(disp("safe_zone_release"))),
            CheckOutcome::Fail(_)
        ));
        // non-freeing action (no on_disengagement) -> vacuous Pass.
        let plain = ExecuteGoal::wrap("s/e/0001-align".to_string(), sample_action());
        assert_eq!(
            check_freed_part_disposition(&plain, &report(None)),
            CheckOutcome::Pass
        );
    }

    #[test]
    fn check_freed_part_disposition_cut_is_presence_only_and_succeeded_gated() {
        use rfl_core::driver::{
            FreedPartDisposition, Outcome, RealizedPose, SafetyFlags, Status, Verdict,
        };
        let mut action = sample_action();
        action.safety_envelope.force_profile = Some(serde_json::json!({ "irreversible": true }));
        let goal = ExecuteGoal::wrap("s/e/0001-cut".to_string(), action);
        let report = |outcome: Outcome, flags: Option<SafetyFlags>| {
            let status = Status {
                message: "status",
                action_id: "s/e/0001-cut".to_string(),
                outcome,
                verdict: Some(Verdict {
                    value: true,
                    confidence: 1.0,
                    evidence: vec![],
                }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
                stop_latency: None,
                safety_flags: flags,
            };
            DriverReport {
                telemetry: vec![],
                status,
            }
        };
        let disp = |d: &str| {
            Some(SafetyFlags {
                freed_part_disposition: Some(FreedPartDisposition {
                    disposition: d.to_string(),
                    zone: None,
                }),
            })
        };
        // presence-only: either valid disposition on a succeeded cut -> Pass.
        assert_eq!(
            check_freed_part_disposition(&goal, &report(Outcome::Succeeded, disp("retained"))),
            CheckOutcome::Pass
        );
        assert_eq!(
            check_freed_part_disposition(
                &goal,
                &report(Outcome::Succeeded, disp("safe_zone_release"))
            ),
            CheckOutcome::Pass
        );
        // succeeded cut with no disclosure -> uncontrolled drop -> Fail.
        assert!(matches!(
            check_freed_part_disposition(&goal, &report(Outcome::Succeeded, None)),
            CheckOutcome::Fail(_)
        ));
        // interrupted (Failed) cut froze nothing -> vacuous Pass even with no disclosure.
        assert_eq!(
            check_freed_part_disposition(&goal, &report(Outcome::Failed, None)),
            CheckOutcome::Pass
        );
    }

    #[test]
    fn check_actuation_requires_a_detent_on_success() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode,
        };
        use rfl_core::driver::{Outcome, RealizedPose, Status, Telemetry, Verdict};
        let action = CanonicalAction {
            target_frame: "control".into(),
            target_pose: PoseExpr::Ref {
                r#ref: "button".into(),
            },
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
            grasp_stability: None,
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
            verdict: Some(Verdict {
                value: true,
                confidence: 1.0,
                evidence: vec![],
            }),
            fidelity_tier: None,
            final_pose: Some(RealizedPose::placeholder()),
            failure_class: None,
            failure_detail: None,
            stop_latency: None,
            safety_flags: None,
        };
        // Succeeded + detent -> Pass.
        let ok = DriverReport {
            telemetry: vec![sample(vec![serde_json::json!({ "kind": "detent" })])],
            status: status(Outcome::Succeeded),
        };
        assert_eq!(check_actuation(&goal, &ok), CheckOutcome::Pass);
        // Succeeded + no detent -> Fail (the events bite).
        let claims = DriverReport {
            telemetry: vec![sample(vec![])],
            status: status(Outcome::Succeeded),
        };
        assert!(matches!(
            check_actuation(&goal, &claims),
            CheckOutcome::Fail(_)
        ));
        // Not succeeded -> vacuously Pass.
        let bottoms = DriverReport {
            telemetry: vec![sample(vec![])],
            status: status(Outcome::Failed),
        };
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
            target_pose: PoseExpr::Ref {
                r#ref: "panel".into(),
            },
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
                force_profile: Some(
                    serde_json::json!({ "normal_force": "5 N", "normal_force_tolerance": "1 N" }),
                ),
                station_keeping: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
            grasp_stability: None,
        };
        let goal = ExecuteGoal::wrap("s/e/0001-wipe".to_string(), action);
        let report = |fz: f64| {
            let t = Telemetry {
                message: "telemetry",
                action_id: "s/e/0001-wipe".to_string(),
                t: 1.0,
                realized_pose: Some(RealizedPose::placeholder()),
                wrench: Some(Wrench {
                    force: [0.0, 0.0, fz],
                    torque: [0.0, 0.0, 0.0],
                }),
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
                    verdict: Some(Verdict {
                        value: true,
                        confidence: 1.0,
                        evidence: vec![],
                    }),
                    fidelity_tier: None,
                    final_pose: Some(RealizedPose::placeholder()),
                    failure_class: None,
                    failure_detail: None,
                    stop_latency: None,
                    safety_flags: None,
                },
            }
        };
        // in band (5 N) -> Pass.
        assert_eq!(
            check_envelope(EnvelopeClass::ForceTrajectory, &goal, &report(5.0)),
            CheckOutcome::Pass
        );
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
            target_pose: PoseExpr::Ref {
                r#ref: "clip".into(),
            },
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
                force_profile: Some(
                    serde_json::json!({ "actuation": "detent", "confirm_held": true }),
                ),
                station_keeping: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
            grasp_stability: None,
        };
        let goal = ExecuteGoal::wrap("s/e/0001-snap_engage".to_string(), action);
        let report = |outcome: Outcome, evidence: Vec<String>| DriverReport {
            telemetry: vec![],
            status: Status {
                message: "status",
                action_id: "s/e/0001-snap_engage".to_string(),
                outcome,
                verdict: Some(Verdict {
                    value: true,
                    confidence: 1.0,
                    evidence,
                }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
                stop_latency: None,
                safety_flags: None,
            },
        };
        // Succeeded + held_confirmed -> Pass.
        assert_eq!(
            check_engagement(
                &goal,
                &report(Outcome::Succeeded, vec!["held_confirmed".to_string()])
            ),
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

    #[test]
    fn check_irreversible_requires_partial_state_on_interruption() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, TimingHints, TimingMode,
        };
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        let action = CanonicalAction {
            target_frame: "grasp".into(),
            target_pose: PoseExpr::Ref {
                r#ref: "seam".into(),
            },
            force_budget: Some(rfl_core::quantity::Quantity("30 N".into())),
            timing: TimingHints {
                nominal_duration: None,
                timing_mode: TimingMode::TimeScalable,
                stop_at_goal: true,
            },
            tactile_target: None,
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: Some(serde_json::json!({ "irreversible": true })),
                station_keeping: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
            grasp_stability: None,
        };
        let goal = ExecuteGoal::wrap("s/e/0001-cut".to_string(), action);
        let report = |outcome: Outcome, evidence: Vec<String>| DriverReport {
            telemetry: vec![],
            status: Status {
                message: "status",
                action_id: "s/e/0001-cut".to_string(),
                outcome,
                verdict: Some(Verdict {
                    value: true,
                    confidence: 1.0,
                    evidence,
                }),
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
                stop_latency: None,
                safety_flags: None,
            },
        };
        // Succeeded -> vacuously Pass (completed; nothing partial).
        assert_eq!(
            check_irreversible(&goal, &report(Outcome::Succeeded, vec![])),
            CheckOutcome::Pass
        );
        // Interrupted + partial state reported -> Pass.
        assert_eq!(
            check_irreversible(
                &goal,
                &report(Outcome::Failed, vec!["partial_cut: 0.6".to_string()])
            ),
            CheckOutcome::Pass
        );
        // Interrupted + binary halt (no partial state) -> Fail.
        assert!(matches!(
            check_irreversible(&goal, &report(Outcome::Failed, vec![])),
            CheckOutcome::Fail(_)
        ));
    }

    #[test]
    fn check_audit_honesty_rejects_undisclosed_degradation() {
        use rfl_core::canonical::{
            CanonicalAction, Envelope, MotionBounds, PoseExpr, ProxySpec, TactileTargetOut,
            TimingHints, TimingMode,
        };
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        let action = |tt: Option<TactileTargetOut>| CanonicalAction {
            target_frame: "tcp".into(),
            target_pose: PoseExpr::Ref {
                r#ref: "obj".into(),
            },
            force_budget: None,
            timing: TimingHints {
                nominal_duration: None,
                timing_mode: TimingMode::Strict,
                stop_at_goal: true,
            },
            tactile_target: tt,
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: None,
                station_keeping: None,
                clearance: None,
                compliance: None,
                stop_time: None,
            },
            grasp_stability: None,
        };
        let report = |tier: &str| DriverReport {
            telemetry: vec![],
            status: Status {
                message: "status",
                action_id: "s/e/0001-pinch".to_string(),
                outcome: Outcome::Succeeded,
                verdict: Some(Verdict {
                    value: true,
                    confidence: 1.0,
                    evidence: vec![],
                }),
                fidelity_tier: Some(tier.to_string()),
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
                stop_latency: None,
                safety_flags: None,
            },
        };
        let proxy_goal = ExecuteGoal::wrap(
            "s/e/0001-pinch".to_string(),
            action(Some(TactileTargetOut::Proxy {
                proxy: ProxySpec {
                    tier: "proxy",
                    criterion: "force_position",
                },
            })),
        );
        let manifold_goal = ExecuteGoal::wrap(
            "s/e/0001-pinch".to_string(),
            action(Some(TactileTargetOut::Auto)),
        );
        // proxy action + manifold claim -> Fail (undisclosed degradation).
        assert!(matches!(
            check_audit_honesty(&proxy_goal, &report("manifold")),
            CheckOutcome::Fail(_)
        ));
        // proxy action + proxy claim -> Pass (honest).
        assert_eq!(
            check_audit_honesty(&proxy_goal, &report("proxy")),
            CheckOutcome::Pass
        );
        // manifold action + manifold claim -> Pass.
        assert_eq!(
            check_audit_honesty(&manifold_goal, &report("manifold")),
            CheckOutcome::Pass
        );
    }

    #[test]
    fn check_momentary_release_requires_declaration_and_propagation() {
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        let pair = |suffix: &str, momentary: bool| {
            let mut evidence = vec!["nominal".to_string()];
            if momentary {
                evidence.push("momentary_release".to_string());
            }
            let id = format!("s/e/0001-{suffix}");
            let goal = ExecuteGoal::wrap(id.clone(), sample_action());
            let report = DriverReport {
                telemetry: vec![],
                status: Status {
                    message: "status",
                    action_id: id,
                    outcome: Outcome::Succeeded,
                    verdict: Some(Verdict {
                        value: true,
                        confidence: 1.0,
                        evidence,
                    }),
                    fidelity_tier: None,
                    final_pose: Some(RealizedPose::placeholder()),
                    failure_class: None,
                    failure_detail: None,
                    stop_latency: None,
                    safety_flags: None,
                },
            };
            (goal, report)
        };
        // flip declares + downstream release carries -> Pass.
        let ok = vec![
            pair("pinch", false),
            pair("flip", true),
            pair("release", true),
        ];
        assert_eq!(check_momentary_release(&ok), CheckOutcome::Pass);
        // flip omits the flag -> Fail.
        let suppressed = vec![pair("flip", false), pair("release", false)];
        assert!(matches!(
            check_momentary_release(&suppressed),
            CheckOutcome::Fail(_)
        ));
        // flip declares but downstream dropped -> Fail (the sequence-level bite).
        let dropped = vec![pair("flip", true), pair("release", false)];
        assert!(matches!(
            check_momentary_release(&dropped),
            CheckOutcome::Fail(_)
        ));
        // no flip -> vacuous Pass.
        let no_flip = vec![pair("pinch", false), pair("release", false)];
        assert_eq!(check_momentary_release(&no_flip), CheckOutcome::Pass);
    }

    #[test]
    fn check_audit_record_requires_a_verdict_with_evidence() {
        use rfl_core::driver::{Outcome, RealizedPose, Status, Verdict};
        let report = |verdict: Option<Verdict>| DriverReport {
            telemetry: vec![],
            status: Status {
                message: "status",
                action_id: "s/e/0001-x".to_string(),
                outcome: Outcome::Succeeded,
                verdict,
                fidelity_tier: None,
                final_pose: Some(RealizedPose::placeholder()),
                failure_class: None,
                failure_detail: None,
                stop_latency: None,
                safety_flags: None,
            },
        };
        let with_evidence = Verdict {
            value: true,
            confidence: 1.0,
            evidence: vec!["nominal".to_string()],
        };
        assert_eq!(
            check_audit_record(&report(Some(with_evidence))),
            CheckOutcome::Pass
        );
        assert!(matches!(
            check_audit_record(&report(None)),
            CheckOutcome::Fail(_)
        ));
        let no_evidence = Verdict {
            value: true,
            confidence: 1.0,
            evidence: vec![],
        };
        assert!(matches!(
            check_audit_record(&report(Some(no_evidence))),
            CheckOutcome::Fail(_)
        ));
    }
}
