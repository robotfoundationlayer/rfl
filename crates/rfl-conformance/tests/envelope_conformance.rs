// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 3, part 2 (`spec/05` § The envelope-class taxonomy): the
//! envelope-class checkers JUDGE a driver report against each primitive's envelope.
//! The nominal driver conforms to every check; the adversarial `FaultyDriver`s are
//! REJECTED by the matching checker — the non-circular proof that the suite bites.

use rfl_conformance::{
    check_envelope, drive, envelope_class_for, CheckOutcome, EnvelopeClass, Fault, FaultyDriver,
    ReferenceDriver,
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
