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
    check_actuation, check_audit_honesty, check_engagement, check_envelope,
    check_freed_part_disposition, check_irreversible, check_momentary_release, envelope_class_for,
    suffix_of, CheckOutcome, EnvelopeClass,
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
        checks.push(NamedCheck { name: "envelope", outcome: check_envelope(class, goal, report) });
    }
    checks.push(NamedCheck { name: "actuation", outcome: check_actuation(goal, report) });
    checks.push(NamedCheck { name: "engagement", outcome: check_engagement(goal, report) });
    checks.push(NamedCheck { name: "irreversible", outcome: check_irreversible(goal, report) });
    checks.push(NamedCheck {
        name: "freed_part_disposition",
        outcome: check_freed_part_disposition(goal, report),
    });
    checks.push(NamedCheck { name: "audit_honesty", outcome: check_audit_honesty(goal, report) });
    let passed = checks.iter().all(|c| matches!(c.outcome, CheckOutcome::Pass));
    ActionVerdict { action_id: goal.action_id.clone(), suffix, envelope_class, checks, passed }
}

/// Run the sequence-level obligation (`momentary_release` propagation, AUD2).
#[must_use]
pub fn verify_sequence(pairs: &[(ExecuteGoal, DriverReport)]) -> NamedCheck {
    NamedCheck { name: "momentary_release", outcome: check_momentary_release(pairs) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{drive, Fault, FaultyDriver, ReferenceDriver};
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
            v.checks.iter().map(|c| (c.name, &c.outcome)).collect::<Vec<_>>()
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
