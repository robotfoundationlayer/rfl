// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! The per-action and per-sequence verification battery for `rfl certify`. Runs every applicable
//! Class 3 obligation (each unchanged `check_*` is vacuous-pass when its contract is absent) and
//! returns a structured verdict. The ENV3 disturbance checks (`check_graceful_degradation` /
//! `check_settling`) are excluded: a nominal vendor replay cannot inject perturbations (the
//! certificate discloses `env3` as not-covered).

use rfl_core::canonical::ExecuteGoal;
use rfl_core::driver::DriverReport;

use crate::{
    CheckOutcome, EnvelopeClass, check_actuation, check_audit_honesty, check_audit_record,
    check_contact_geometry, check_controlled_under_actuation, check_engagement, check_envelope,
    check_flip_bounded_window, check_freed_part_disposition, check_hold_test, check_irreversible,
    check_make_before_break, check_momentary_release, check_support_safe_state,
    check_two_party_handoff, envelope_class_for, suffix_of,
};

/// A named check outcome.
pub struct NamedCheck {
    /// The obligation name (stable, used in the certificate).
    pub name: &'static str,
    /// Its outcome.
    pub outcome: CheckOutcome,
}

/// The full verdict for one action.
pub struct ActionVerdict {
    /// The correlated action id.
    pub action_id: String,
    /// The primitive suffix.
    pub suffix: String,
    /// The envelope class verified (None for perception primitives with no motion envelope).
    pub envelope_class: Option<EnvelopeClass>,
    /// The fidelity tier achieved (`report.status.fidelity_tier`); None for actions with no
    /// confirmation tier (reach / perception).
    pub fidelity_tier: Option<String>,
    /// Every obligation run for this action.
    pub checks: Vec<NamedCheck>,
    /// True iff every check passed.
    pub passed: bool,
}

/// Run the full per-action obligation battery.
#[must_use]
pub fn verify_action(goal: &ExecuteGoal, report: &DriverReport) -> ActionVerdict {
    let suffix = suffix_of(&goal.action_id).to_string();
    let envelope_class = envelope_class_for(&suffix);
    let mut checks = Vec::new();
    if let Some(class) = envelope_class {
        checks.push(NamedCheck {
            name: "envelope",
            outcome: check_envelope(class, goal, report),
        });
    }
    checks.push(NamedCheck {
        name: "actuation",
        outcome: check_actuation(goal, report),
    });
    checks.push(NamedCheck {
        name: "engagement",
        outcome: check_engagement(goal, report),
    });
    checks.push(NamedCheck {
        name: "irreversible",
        outcome: check_irreversible(goal, report),
    });
    checks.push(NamedCheck {
        name: "freed_part_disposition",
        outcome: check_freed_part_disposition(goal, report),
    });
    checks.push(NamedCheck {
        name: "audit_honesty",
        outcome: check_audit_honesty(goal, report),
    });
    checks.push(NamedCheck {
        name: "audit_record",
        outcome: check_audit_record(report),
    });
    checks.push(NamedCheck {
        name: "support_safe_state",
        outcome: check_support_safe_state(goal),
    });
    checks.push(NamedCheck {
        name: "contact_geometry",
        outcome: check_contact_geometry(goal, report),
    });
    checks.push(NamedCheck {
        name: "hold_test",
        outcome: check_hold_test(goal, report),
    });
    checks.push(NamedCheck {
        name: "flip_bounded_window",
        outcome: check_flip_bounded_window(goal, report),
    });
    checks.push(NamedCheck {
        name: "make_before_break",
        outcome: check_make_before_break(goal, report),
    });
    checks.push(NamedCheck {
        name: "controlled_under_actuation",
        outcome: check_controlled_under_actuation(goal, report),
    });
    checks.push(NamedCheck {
        name: "two_party_handoff",
        outcome: check_two_party_handoff(goal, report),
    });
    let passed = checks
        .iter()
        .all(|c| matches!(c.outcome, CheckOutcome::Pass));
    let fidelity_tier = report.status.fidelity_tier.clone();
    ActionVerdict {
        action_id: goal.action_id.clone(),
        suffix,
        envelope_class,
        fidelity_tier,
        checks,
        passed,
    }
}

/// Run the sequence-level obligation (`momentary_release` propagation, AUD2).
#[must_use]
pub fn verify_sequence(pairs: &[(ExecuteGoal, DriverReport)]) -> NamedCheck {
    NamedCheck {
        name: "momentary_release",
        outcome: check_momentary_release(pairs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fault, FaultyDriver, ReferenceDriver, drive};
    use std::path::Path;

    fn example_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
    }

    #[test]
    fn nominal_action_passes_full_battery() {
        let dir = example_dir();
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // index 1 = grasp.pinch (grasp-continuity).
        let (g, r) = &pairs[1];
        let v = verify_action(g, r);
        assert_eq!(v.suffix, "pinch");
        assert_eq!(v.envelope_class, Some(EnvelopeClass::GraspContinuity));
        assert!(
            v.passed,
            "checks: {:?}",
            v.checks
                .iter()
                .map(|c| (c.name, &c.outcome))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn under_secure_action_fails_envelope() {
        let dir = example_dir();
        let pairs = drive(
            FaultyDriver::new(Fault::UnderSecure),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        let (g, r) = &pairs[1]; // grasp.pinch
        let v = verify_action(g, r);
        assert!(!v.passed);
        let env = v.checks.iter().find(|c| c.name == "envelope").unwrap();
        assert!(matches!(env.outcome, CheckOutcome::Fail(_)));
    }

    #[test]
    fn nominal_pinch_passes_hold_test_but_wrong_profile_fails() {
        let dir = example_dir();
        // nominal: the force-closure pinch reports an omnidirectional hold test -> passes GC2.
        let nominal = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        let (g, r) = &nominal[1]; // grasp.pinch
        let v = verify_action(g, r);
        let ht = v.checks.iter().find(|c| c.name == "hold_test").unwrap();
        assert!(
            matches!(ht.outcome, CheckOutcome::Pass),
            "checks: {:?}",
            v.checks
                .iter()
                .map(|c| (c.name, &c.outcome))
                .collect::<Vec<_>>()
        );
        // adversarial: a driver running the wrong-closure perturbation profile -> GC2 fails.
        let wrong = drive(
            FaultyDriver::new(Fault::WrongHoldTest),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        let (g, r) = &wrong[1];
        let v = verify_action(g, r);
        assert!(!v.passed);
        let ht = v.checks.iter().find(|c| c.name == "hold_test").unwrap();
        assert!(matches!(ht.outcome, CheckOutcome::Fail(_)));
    }

    #[test]
    fn nominal_hook_passes_form_hold_test_but_wrong_profile_fails() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill = dir.join("skill-hook.yaml");
        let emb = dir.join("embodiments/allegro.yaml");
        // nominal: the FORM-closure hook reports a load_direction hold test -> passes GC2. This is
        // the first form-closure grasp to drive the load_direction branch end-to-end.
        let nominal = drive(ReferenceDriver::default(), &skill, &emb).unwrap();
        let (g, r) = &nominal[1]; // locate(0), hook(1), release(2)
        assert_eq!(g.action_id.rsplit('-').next(), Some("hook"));
        let v = verify_action(g, r);
        let ht = v.checks.iter().find(|c| c.name == "hold_test").unwrap();
        assert!(
            matches!(ht.outcome, CheckOutcome::Pass),
            "checks: {:?}",
            v.checks
                .iter()
                .map(|c| (c.name, &c.outcome))
                .collect::<Vec<_>>()
        );
        // adversarial: a driver running a wrong-closure (level_gentle) profile -> GC2 fails for the
        // form-closure hook just as it does for a force-closure pinch.
        let wrong = drive(FaultyDriver::new(Fault::WrongHoldTest), &skill, &emb).unwrap();
        let (g, r) = &wrong[1];
        let v = verify_action(g, r);
        assert!(!v.passed);
        let ht = v.checks.iter().find(|c| c.name == "hold_test").unwrap();
        assert!(matches!(ht.outcome, CheckOutcome::Fail(_)));
    }

    #[test]
    fn nominal_flip_passes_bounded_window_but_over_window_fails() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill = dir.join("skill-flip.yaml");
        let emb = dir.join("embodiments/allegro.yaml");
        // nominal: the flip's measured unsecured window stays within max_release_time -> GC5 pass.
        let nominal = drive(ReferenceDriver::default(), &skill, &emb).unwrap();
        let (g, r) = &nominal[2]; // locate(0), pinch(1), flip(2), release(3)
        assert_eq!(g.action_id.rsplit('-').next(), Some("flip"));
        let v = verify_action(g, r);
        let bw = v
            .checks
            .iter()
            .find(|c| c.name == "flip_bounded_window")
            .unwrap();
        assert!(
            matches!(bw.outcome, CheckOutcome::Pass),
            "checks: {:?}",
            v.checks
                .iter()
                .map(|c| (c.name, &c.outcome))
                .collect::<Vec<_>>()
        );
        // adversarial: a driver that held the object unsecured past the bound -> GC5 fails.
        let over = drive(FaultyDriver::new(Fault::FlipWindowExceeded), &skill, &emb).unwrap();
        let (g, r) = &over[2];
        let v = verify_action(g, r);
        assert!(!v.passed);
        let bw = v
            .checks
            .iter()
            .find(|c| c.name == "flip_bounded_window")
            .unwrap();
        assert!(matches!(bw.outcome, CheckOutcome::Fail(_)));
    }

    #[test]
    fn nominal_regrasp_passes_make_before_break_but_a_gap_fails() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill = dir.join("skill-regrasp.yaml");
        let emb = dir.join("embodiments/allegro.yaml");
        // nominal: the regrasp confirms the new grasp before releasing the old -> GC3 pass.
        let nominal = drive(ReferenceDriver::default(), &skill, &emb).unwrap();
        let (g, r) = &nominal[2]; // locate(0), pinch(1), regrasp(2), release(3)
        assert_eq!(g.action_id.rsplit('-').next(), Some("regrasp"));
        let v = verify_action(g, r);
        let mbb = v
            .checks
            .iter()
            .find(|c| c.name == "make_before_break")
            .unwrap();
        assert!(
            matches!(mbb.outcome, CheckOutcome::Pass),
            "checks: {:?}",
            v.checks
                .iter()
                .map(|c| (c.name, &c.outcome))
                .collect::<Vec<_>>()
        );
        // adversarial: a driver that released the old grasp before confirming the new -> GC3 fails.
        let gap = drive(FaultyDriver::new(Fault::BreakBeforeMake), &skill, &emb).unwrap();
        let (g, r) = &gap[2];
        let v = verify_action(g, r);
        assert!(!v.passed);
        let mbb = v
            .checks
            .iter()
            .find(|c| c.name == "make_before_break")
            .unwrap();
        assert!(matches!(mbb.outcome, CheckOutcome::Fail(_)));
    }

    #[test]
    fn nominal_pivot_passes_under_actuation_but_unresecured_fails() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill = dir.join("skill-pivot.yaml");
        let emb = dir.join("embodiments/allegro.yaml");
        // nominal: the pivot releases exactly the pivot DOF and re-secures it -> GC4 pass.
        let nominal = drive(ReferenceDriver::default(), &skill, &emb).unwrap();
        let (g, r) = &nominal[2]; // locate(0), pinch(1), pivot(2), release(3)
        assert_eq!(g.action_id.rsplit('-').next(), Some("pivot"));
        let v = verify_action(g, r);
        let cua = v
            .checks
            .iter()
            .find(|c| c.name == "controlled_under_actuation")
            .unwrap();
        assert!(
            matches!(cua.outcome, CheckOutcome::Pass),
            "checks: {:?}",
            v.checks
                .iter()
                .map(|c| (c.name, &c.outcome))
                .collect::<Vec<_>>()
        );
        // adversarial: a driver that left the released DOF un-resecured -> GC4 fails.
        let bad = drive(FaultyDriver::new(Fault::DofNotResecured), &skill, &emb).unwrap();
        let (g, r) = &bad[2];
        let v = verify_action(g, r);
        assert!(!v.passed);
        let cua = v
            .checks
            .iter()
            .find(|c| c.name == "controlled_under_actuation")
            .unwrap();
        assert!(matches!(cua.outcome, CheckOutcome::Fail(_)));
    }

    #[test]
    fn nominal_handoff_passes_two_party_but_cograsp_overforce_fails() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill = dir.join("skill-handoff.yaml");
        let emb = dir.join("embodiments/allegro.yaml");
        // nominal: at least one party secures + combined force within budget -> GC6 pass.
        let nominal = drive(ReferenceDriver::default(), &skill, &emb).unwrap();
        let (g, r) = &nominal[2]; // locate(0), pinch(1), handoff(2), release(3)
        assert_eq!(g.action_id.rsplit('-').next(), Some("handoff"));
        let v = verify_action(g, r);
        let tph = v
            .checks
            .iter()
            .find(|c| c.name == "two_party_handoff")
            .unwrap();
        assert!(
            matches!(tph.outcome, CheckOutcome::Pass),
            "checks: {:?}",
            v.checks
                .iter()
                .map(|c| (c.name, &c.outcome))
                .collect::<Vec<_>>()
        );
        // adversarial: the two effectors crushed / fought the object past the budget -> GC6 fails.
        let bad = drive(FaultyDriver::new(Fault::CograspOverforce), &skill, &emb).unwrap();
        let (g, r) = &bad[2];
        let v = verify_action(g, r);
        assert!(!v.passed);
        let tph = v
            .checks
            .iter()
            .find(|c| c.name == "two_party_handoff")
            .unwrap();
        assert!(matches!(tph.outcome, CheckOutcome::Fail(_)));
    }

    #[test]
    fn sequence_check_runs_momentary_release() {
        let dir = example_dir();
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // cable-insertion has no flip -> vacuously Pass.
        let seq = verify_sequence(&pairs);
        assert_eq!(seq.name, "momentary_release");
        assert!(matches!(seq.outcome, CheckOutcome::Pass));
    }
}
