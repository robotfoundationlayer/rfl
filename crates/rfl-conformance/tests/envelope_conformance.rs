// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 3, part 2 (`spec/05` § The envelope-class taxonomy): the
//! envelope-class checkers JUDGE a driver report against each primitive's envelope.
//! The nominal driver conforms to every check; the adversarial `FaultyDriver`s are
//! REJECTED by the matching checker — the non-circular proof that the suite bites.

use rfl_conformance::{
    check_actuation, check_audit_honesty, check_engagement, check_envelope,
    check_freed_part_disposition, check_graceful_degradation, check_irreversible,
    check_momentary_release, check_settling, drive, envelope_class_for, CheckOutcome, CutDriver,
    CutResponse, DisturbanceDriver, DisturbanceResponse, EnvelopeClass, Fault, FaultyDriver,
    FlipDriver, FlipResponse, FreeingDriver, FreeingResponse, HoverResponse, HoverSettlingDriver,
    PressButtonDriver, PressButtonResponse, ReferenceDriver, SnapEngageDriver, SnapEngageResponse,
};
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

/// The primitive suffix of an action id (`.../NNNN-<suffix>`; suffixes contain no `-`).
fn suffix_of(action_id: &str) -> &str {
    action_id.rsplit('-').next().unwrap_or(action_id)
}

#[test]
fn nominal_driver_passes_every_envelope_check() {
    let dir = example_dir();
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join(format!("embodiments/{stem}.yaml")),
        )
        .expect("drive");
        for (goal, report) in &pairs {
            if let Some(class) = envelope_class_for(suffix_of(&goal.action_id)) {
                assert_eq!(
                    check_envelope(class, goal, report),
                    CheckOutcome::Pass,
                    "{stem} {}",
                    goal.action_id
                );
            }
        }
    }
}

#[test]
fn under_secure_driver_fails_grasp_continuity() {
    let dir = example_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::UnderSecure),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // grasp.pinch (index 1) carries min_holding_force -> GC1 must reject the lowered securing_force.
    let (goal, report) = &pairs[1];
    assert_eq!(suffix_of(&goal.action_id), "pinch");
    assert!(matches!(
        check_envelope(EnvelopeClass::GraspContinuity, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn over_force_driver_fails_force_trajectory() {
    let dir = example_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverForce),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // force.insert_fit (index 5) -> force-trajectory must reject the over-budget wrench.
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "insert_fit");
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn never_settle_driver_fails_terminal_postcondition() {
    let dir = example_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::NeverSettle),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // reach.align (index 4) -> terminal-postcondition must reject indeterminate / no final pose.
    let (goal, report) = &pairs[4];
    assert_eq!(suffix_of(&goal.action_id), "align");
    assert!(matches!(
        check_envelope(EnvelopeClass::TerminalPostcondition, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn nominal_transport_grasp_continuity_is_non_vacuous() {
    let dir = example_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // transport.move_to_pose (index 2) carries the propagated min_holding_force.
    let (goal, report) = &pairs[2];
    assert_eq!(suffix_of(&goal.action_id), "transport");
    // The carry must report a maintained securing_force (non-vacuous) that meets the floor.
    assert!(
        report.telemetry.iter().any(|t| t.securing_force.is_some()),
        "transport telemetry must carry securing_force"
    );
    assert_eq!(check_envelope(EnvelopeClass::GraspContinuity, goal, report), CheckOutcome::Pass);
}

#[test]
fn under_secure_driver_fails_transport_grasp_continuity() {
    let dir = example_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::UnderSecure),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // transport (index 2): the held carry's GC1 must reject the lowered securing_force.
    let (goal, report) = &pairs[2];
    assert_eq!(suffix_of(&goal.action_id), "transport");
    assert!(matches!(
        check_envelope(EnvelopeClass::GraspContinuity, goal, report),
        CheckOutcome::Fail(_)
    ));
}

fn screw_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten")
}

fn surface_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/02-surface-scan")
}

#[test]
fn nominal_screw_passes_force_trajectory() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // force.screw is action index 5.
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "screw");
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
}

#[test]
fn over_torque_driver_fails_screw_force_trajectory() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverTorque),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // force.screw (index 5) -> torque-trajectory must reject the over-budget wrench torque.
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "screw");
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn nominal_unscrew_passes_force_trajectory() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-unscrew.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // force.unscrew is action index 5.
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "unscrew");
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
}

#[test]
fn over_torque_driver_fails_unscrew_force_trajectory() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverTorque),
        &dir.join("skill-unscrew.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // force.unscrew (index 5) -> torque-trajectory must reject the over-budget wrench torque.
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "unscrew");
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}

// --- freed-part disposition (spec/04 TM21c, safety_flags) --------------------------------------

#[test]
fn nominal_unscrew_discloses_freed_part_disposition() {
    let dir = screw_dir();
    let pairs = drive(
        FreeingDriver::new(FreeingResponse::Discloses),
        &dir.join("skill-unscrew.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "unscrew");
    assert_eq!(check_freed_part_disposition(goal, report), CheckOutcome::Pass);
}

#[test]
fn unscrew_uncontrolled_drop_fails_disposition() {
    let dir = screw_dir();
    let pairs = drive(
        FreeingDriver::new(FreeingResponse::DropsUncontrolled),
        &dir.join("skill-unscrew.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[5];
    assert_eq!(suffix_of(&goal.action_id), "unscrew");
    assert!(matches!(check_freed_part_disposition(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn unscrew_false_disposition_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FreeingDriver::new(FreeingResponse::FalseDisposition),
        &dir.join("skill-unscrew.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[5];
    assert!(matches!(check_freed_part_disposition(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn nominal_hover_passes_interval_invariant() {
    let dir = surface_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // reach.hover is action index 0; the driver samples the interval (3 samples).
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "hover");
    assert_eq!(report.telemetry.len(), 3, "the interval must be sampled (non-vacuous)");
    assert_eq!(check_envelope(EnvelopeClass::IntervalInvariant, goal, report), CheckOutcome::Pass);
}

#[test]
fn mid_interval_drop_fails_interval_but_passes_terminal() {
    // The ENV2 property, made executable: a mid-interval violation fails the interval
    // check even though the endpoint (terminal) conforms.
    let dir = surface_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::MidIntervalDrop),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "hover");
    assert!(matches!(
        check_envelope(EnvelopeClass::IntervalInvariant, goal, report),
        CheckOutcome::Fail(_)
    ));
    assert_eq!(
        check_envelope(EnvelopeClass::TerminalPostcondition, goal, report),
        CheckOutcome::Pass
    );
}

// --- ENV3 disturbance injection (transport.carry C2) ---------------------------------------
// The carry emits disturbance_budget = 0.4 N; injected magnitudes straddle it.

#[test]
fn carry_invariant_holds_under_sub_budget_disturbance() {
    // Injected 0.3 N <= the 0.4 N budget: the held interval invariant still holds.
    let dir = example_dir();
    let pairs = drive(
        DisturbanceDriver::new(0.3, DisturbanceResponse::Graceful),
        &dir.join("skill-carry.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[2]; // locate, pinch, carry
    assert_eq!(suffix_of(&goal.action_id), "carry");
    assert_eq!(report.telemetry.len(), 3, "interval-sampled under perturbation");
    assert_eq!(check_envelope(EnvelopeClass::IntervalInvariant, goal, report), CheckOutcome::Pass);
}

#[test]
fn over_budget_disturbance_degrades_gracefully() {
    // Injected 0.6 N > 0.4 N budget: NOT IntervalInvariant (did not succeed), but graceful
    // (halt with object secured) — the opposite-verdicts property.
    let dir = example_dir();
    let pairs = drive(
        DisturbanceDriver::new(0.6, DisturbanceResponse::Graceful),
        &dir.join("skill-carry.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[2];
    assert_eq!(suffix_of(&goal.action_id), "carry");
    assert_eq!(check_graceful_degradation(goal, report), CheckOutcome::Pass);
    assert!(matches!(
        check_envelope(EnvelopeClass::IntervalInvariant, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn over_budget_drop_fails_graceful_degradation() {
    // Adversarial: over budget AND drops the object -> not graceful (a loss). The non-circular bite.
    let dir = example_dir();
    let pairs = drive(
        DisturbanceDriver::new(0.6, DisturbanceResponse::Drops),
        &dir.join("skill-carry.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[2];
    assert!(matches!(check_graceful_degradation(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn over_budget_false_success_fails_graceful_degradation() {
    // Adversarial: over budget but claims success (ignores the disturbance) -> rejected.
    let dir = example_dir();
    let pairs = drive(
        DisturbanceDriver::new(0.6, DisturbanceResponse::ClaimsSuccess),
        &dir.join("skill-carry.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[2];
    assert!(matches!(check_graceful_degradation(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn nominal_carry_passes_interval_with_held_floor() {
    // transport.carry is interval-invariant (spec/05 ENV1), and being held its interval
    // invariant ALSO requires the securing floor at every sample (§ 4.4 — the held leg).
    let dir = example_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-carry.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // skill-carry: locate (0), pinch (1), carry (2).
    let (goal, report) = &pairs[2];
    assert_eq!(suffix_of(&goal.action_id), "carry");
    assert_eq!(report.telemetry.len(), 3, "carry is interval-sampled");
    assert_eq!(check_envelope(EnvelopeClass::IntervalInvariant, goal, report), CheckOutcome::Pass);
    // non-vacuous: the held floor IS present and IS being checked over the interval.
    assert!(report.telemetry[0].securing_force.is_some());
}

#[test]
fn under_secure_fails_carry_interval_held_floor() {
    // The held leg of the carry interval invariant: a lowered securing_force is rejected.
    let dir = example_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::UnderSecure),
        &dir.join("skill-carry.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[2];
    assert_eq!(suffix_of(&goal.action_id), "carry");
    assert!(matches!(
        check_envelope(EnvelopeClass::IntervalInvariant, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn mid_interval_drop_fails_carry_interval_but_passes_terminal() {
    // The station leg: a mid-interval pose gap fails the carry interval invariant even
    // though the endpoint (terminal) conforms (ENV2).
    let dir = example_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::MidIntervalDrop),
        &dir.join("skill-carry.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[2];
    assert_eq!(suffix_of(&goal.action_id), "carry");
    assert!(matches!(
        check_envelope(EnvelopeClass::IntervalInvariant, goal, report),
        CheckOutcome::Fail(_)
    ));
    assert_eq!(
        check_envelope(EnvelopeClass::TerminalPostcondition, goal, report),
        CheckOutcome::Pass
    );
}

// --- ENV3 disturbance injection (reach.hover C2, spec/01 § 1.5) -----------------------------
// The hover emits station_keeping{station_tolerance: 2 mm, settling_time: 1 s}; with hover as
// action 1 the samples land at t = 1, 2, 3, so the settling deadline (t0 + 1 s = 2) leaves
// the tail {t = 2, 3} and the grace window {t = 1}.

#[test]
fn hover_recovers_within_settling_passes_interval() {
    let dir = surface_dir();
    let pairs = drive(
        HoverSettlingDriver::new(HoverResponse::Recovers),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "hover");
    assert_eq!(report.telemetry.len(), 3, "interval-sampled under perturbation");
    // sample 0 exceeds tolerance (grace window) but the settled tail recovered.
    assert_eq!(check_envelope(EnvelopeClass::IntervalInvariant, goal, report), CheckOutcome::Pass);
}

#[test]
fn hover_fails_to_recover_fails_interval() {
    let dir = surface_dir();
    let pairs = drive(
        HoverSettlingDriver::new(HoverResponse::FailsToRecover),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // drifts past station_tolerance throughout yet claims success -> the settled-tail leg bites.
    assert!(matches!(
        check_envelope(EnvelopeClass::IntervalInvariant, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn hover_over_envelope_aborts_gracefully() {
    let dir = surface_dir();
    let pairs = drive(
        HoverSettlingDriver::new(HoverResponse::Aborts),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // opposite-verdicts: aborts (Failed + station_exceeded) -> settling Pass, interval Fail.
    assert_eq!(check_settling(goal, report), CheckOutcome::Pass);
    assert!(matches!(
        check_envelope(EnvelopeClass::IntervalInvariant, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn hover_over_envelope_abort_too_slow_fails_timing() {
    let dir = surface_dir();
    let pairs = drive(
        HoverSettlingDriver::new(HoverResponse::AbortsTooSlow),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // non-circular: the abort is honest (Failed + station_exceeded) yet overruns stop_time,
    // so the timing leg fails while the outcome leg alone would have passed.
    assert_eq!(report.status.failure_detail.as_deref(), Some("station_exceeded"));
    assert!(matches!(check_settling(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn hover_over_envelope_false_success_fails_settling() {
    let dir = surface_dir();
    let pairs = drive(
        HoverSettlingDriver::new(HoverResponse::ClaimsSuccess),
        &dir.join("skill-hover.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert!(matches!(check_settling(goal, report), CheckOutcome::Fail(_)));
}

// --- force.press_button event-gated actuation (spec/01 § 6.6) --------------------------------

#[test]
fn nominal_press_button_passes_actuation_and_force_trajectory() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-press.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "press_button");
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
    assert_eq!(check_actuation(goal, report), CheckOutcome::Pass);
}

#[test]
fn press_button_bottoming_out_is_honest() {
    let dir = screw_dir();
    let pairs = drive(
        PressButtonDriver::new(PressButtonResponse::Bottoms),
        &dir.join("skill-press.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // no detent fired, force held: not a success, and check_actuation does not falsely fail it.
    assert_eq!(check_actuation(goal, report), CheckOutcome::Pass);
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
    assert!(!matches!(report.status.outcome, rfl_core::driver::Outcome::Succeeded));
}

#[test]
fn press_button_false_actuation_fails() {
    let dir = screw_dir();
    let pairs = drive(
        PressButtonDriver::new(PressButtonResponse::ClaimsActuation),
        &dir.join("skill-press.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // claims success but emitted no detent -> the events channel bites.
    assert!(matches!(check_actuation(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn press_button_over_force_fails_trajectory() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverForce),
        &dir.join("skill-press.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}

// --- force.wipe contact-maintenance band (spec/01 § 6.8) -------------------------------------

#[test]
fn nominal_wipe_holds_the_contact_band() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-wipe.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "wipe");
    // nominal wrench echoes the 5 N setpoint -> in [4, 6] band.
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
}

#[test]
fn wipe_loss_of_contact_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::LoseContact),
        &dir.join("skill-wipe.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // wrench.force -> 0, below the band lower edge -> the new lower-bound bite.
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}

#[test]
fn wipe_over_force_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverForce),
        &dir.join("skill-wipe.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // wrench.force -> 999, above the band upper edge.
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}

// --- force.snap_engage snap-in + engagement-confirmation (spec/01 § 6.10) --------------------

#[test]
fn nominal_snap_engage_passes_all_legs() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-snap.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "snap_engage");
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
    assert_eq!(check_actuation(goal, report), CheckOutcome::Pass); // reused detent leg
    assert_eq!(check_engagement(goal, report), CheckOutcome::Pass); // the new confirm_held leg
}

#[test]
fn snap_engage_no_snap_is_honest() {
    let dir = screw_dir();
    let pairs = drive(
        SnapEngageDriver::new(SnapEngageResponse::NoSnap),
        &dir.join("skill-snap.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // no snap fired, force held: not a success; the checks do not falsely fail it.
    assert!(!matches!(report.status.outcome, rfl_core::driver::Outcome::Succeeded));
    assert_eq!(check_engagement(goal, report), CheckOutcome::Pass);
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
}

#[test]
fn snap_engage_unconfirmed_hold_fails() {
    let dir = screw_dir();
    let pairs = drive(
        SnapEngageDriver::new(SnapEngageResponse::ClaimsHeld),
        &dir.join("skill-snap.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // claims success (snap detected) but the connection was never confirmed held -> the bite.
    assert!(matches!(check_engagement(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn snap_engage_over_force_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverForce),
        &dir.join("skill-snap.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}

// --- force.cut irreversibility / partial-state on interruption (spec/01 § 6.7) --------------

#[test]
fn nominal_cut_passes() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-cut.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert_eq!(suffix_of(&goal.action_id), "cut");
    assert_eq!(check_envelope(EnvelopeClass::ForceTrajectory, goal, report), CheckOutcome::Pass);
    assert_eq!(check_irreversible(goal, report), CheckOutcome::Pass); // Succeeded -> vacuous
}

#[test]
fn cut_partial_state_reported_is_honest() {
    let dir = screw_dir();
    let pairs = drive(
        CutDriver::new(CutResponse::PartialReported),
        &dir.join("skill-cut.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // interrupted, but the precise partial state is reported -> honest.
    assert!(!matches!(report.status.outcome, rfl_core::driver::Outcome::Succeeded));
    assert_eq!(check_irreversible(goal, report), CheckOutcome::Pass);
}

#[test]
fn cut_binary_halt_fails() {
    let dir = screw_dir();
    let pairs = drive(
        CutDriver::new(CutResponse::BinaryHalt),
        &dir.join("skill-cut.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    // interrupted irreversible op reported a binary failure without the partial state -> the bite.
    assert!(matches!(check_irreversible(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn cut_over_force_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::OverForce),
        &dir.join("skill-cut.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[0];
    assert!(matches!(
        check_envelope(EnvelopeClass::ForceTrajectory, goal, report),
        CheckOutcome::Fail(_)
    ));
}

// --- AUD3 fidelity-tier honesty (degradation disclosure, spec/05 AUD3) ----------------------

#[test]
fn nominal_proxy_tier_is_disclosed() {
    let dir = example_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/pneumatic-6f.yaml"),
    )
    .expect("drive");
    // grasp.pinch (index 1) degrades to proxy on the no-tactile pneumatic hand.
    let (goal, report) = &pairs[1];
    assert_eq!(suffix_of(&goal.action_id), "pinch");
    assert_eq!(report.status.fidelity_tier.as_deref(), Some("proxy"));
    assert_eq!(check_audit_honesty(goal, report), CheckOutcome::Pass);
}

#[test]
fn false_manifold_claim_on_proxy_fails() {
    let dir = example_dir();
    let pairs = drive(
        FaultyDriver::new(Fault::FalseTier),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/pneumatic-6f.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[1];
    // claims manifold on a proxy-degraded action -> undisclosed degradation (the bite).
    assert!(matches!(check_audit_honesty(goal, report), CheckOutcome::Fail(_)));
}

#[test]
fn manifold_tier_passes() {
    let dir = example_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    let (goal, report) = &pairs[1];
    // allegro declares tactile -> manifold; honest.
    assert_eq!(report.status.fidelity_tier.as_deref(), Some("manifold"));
    assert_eq!(check_audit_honesty(goal, report), CheckOutcome::Pass);
}

// --- AUD2 momentary_release propagation (in_hand.flip, spec/05) ------------------------------

#[test]
fn nominal_flip_declares_and_propagates() {
    let dir = screw_dir();
    let pairs = drive(
        ReferenceDriver::default(),
        &dir.join("skill-flip.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // locate(0), pinch(1), flip(2), release(3).
    assert_eq!(suffix_of(&pairs[2].0.action_id), "flip");
    assert_eq!(suffix_of(&pairs[3].0.action_id), "release");
    assert_eq!(check_momentary_release(&pairs), CheckOutcome::Pass);
}

#[test]
fn flip_suppressed_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FlipDriver::new(FlipResponse::SuppressesFlip),
        &dir.join("skill-flip.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // the flip omits momentary_release -> transparency violation.
    assert!(matches!(check_momentary_release(&pairs), CheckOutcome::Fail(_)));
}

#[test]
fn flip_propagation_dropped_fails() {
    let dir = screw_dir();
    let pairs = drive(
        FlipDriver::new(FlipResponse::DropsDownstream),
        &dir.join("skill-flip.yaml"),
        &dir.join("embodiments/allegro.yaml"),
    )
    .expect("drive");
    // the flip declares it but the downstream release drops the propagated flag -> the
    // sequence-level bite (each report looks fine in isolation).
    assert!(matches!(check_momentary_release(&pairs), CheckOutcome::Fail(_)));
}
