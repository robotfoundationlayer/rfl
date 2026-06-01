// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! The ε measurement harness (`spec/02` RD2c, `spec/05` § Determinism floor).
//!
//! Class 2-loose conformance holds a contact-dynamics primitive's *realized*
//! execution to semantic equivalence within a per-skill tolerance ε. The ε is an
//! **empirical** quantity: the realized-execution deviation a conformant
//! implementation exhibits run to run (and across embodiments). This module is
//! the tool that *produces* those values — given the realized traces of the same
//! quantity across N conformant runs, it computes the per-quantity deviation
//! under the ε-table metric and the candidate ε (a high percentile + safety
//! factor) that fills `schemas/epsilon-tolerances.yaml` (currently `null`).
//!
//! The harness is ready now; the **values** stay pending real multi-run traces
//! from a reference driver or a (declared-conformant) simulator — a single
//! nominal report cannot exhibit run-to-run variation. See
//! `docs/design/2026-06-01-epsilon-tolerance-table-design.md` for the harness
//! protocol.

use nalgebra::UnitQuaternion;
use rfl_core::pose::{Pose6D, theta_orient};

/// The comparison metric for a measured quantity (mirrors the ε-table schema's
/// `metric` enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    /// SE(3): position L2 (m) + orientation geodesic (rad), reported separately.
    GeodesicSe3,
    /// SO(3): orientation geodesic angle (rad).
    GeodesicSo3,
    /// L2 norm of a vector difference (e.g. wrench force, N).
    L2Norm,
    /// Absolute difference of a scalar.
    Abs,
}

/// Absolute deviation of two scalars (`Abs`).
#[must_use]
pub fn abs_dev(a: f64, b: f64) -> f64 {
    (a - b).abs()
}

/// L2 deviation of two equal-length vectors (`L2Norm`); `f64::NAN` on a length
/// mismatch (an ill-formed comparison).
#[must_use]
pub fn l2_dev(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        return f64::NAN;
    }
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// SE(3) deviation of two poses (`GeodesicSe3`): `(translation_m, rotation_rad)`
/// — kept separate because the two have different units (the ε-table carries one
/// tolerance per quantity, so pose splits into a position and an orientation row).
#[must_use]
pub fn se3_dev(a: &Pose6D, b: &Pose6D) -> (f64, f64) {
    let t = l2_dev(&a.position, &b.position);
    let r = theta_orient(&a.orientation, &b.orientation);
    (t, r)
}

/// SO(3) deviation of two orientations (`GeodesicSo3`), in radians.
#[must_use]
pub fn so3_dev(a: &UnitQuaternion<f64>, b: &UnitQuaternion<f64>) -> f64 {
    theta_orient(a, b)
}

/// The candidate ε for a quantity from a sample of run-to-run deviations: the
/// `percentile` (nearest-rank, deterministic) scaled by `safety_factor`.
/// `percentile` is in `[0, 1]`; `None` for an empty sample (not yet gradeable).
#[must_use]
pub fn epsilon_candidate(deviations: &[f64], percentile: f64, safety_factor: f64) -> Option<f64> {
    if deviations.is_empty() {
        return None;
    }
    let mut sorted: Vec<f64> = deviations
        .iter()
        .copied()
        .filter(|x| x.is_finite())
        .collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p = percentile.clamp(0.0, 1.0);
    // Nearest-rank: rank = ceil(p * n), clamped to [1, n].
    let rank = ((p * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    Some(sorted[rank - 1] * safety_factor)
}

/// Run-to-run deviations of a scalar quantity across `runs`, each compared to the
/// first run (the reference). Empty / single-run input yields no deviations.
#[must_use]
pub fn run_to_run_abs(runs: &[f64]) -> Vec<f64> {
    match runs.split_first() {
        Some((reference, rest)) => rest.iter().map(|r| abs_dev(*reference, *r)).collect(),
        None => vec![],
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]
    use super::*;

    #[test]
    fn scalar_and_vector_deviations() {
        assert_eq!(abs_dev(5.0, 5.0), 0.0);
        assert_eq!(abs_dev(5.0, 3.5), 1.5);
        assert_eq!(l2_dev(&[3.0, 4.0, 0.0], &[0.0, 0.0, 0.0]), 5.0);
        assert!(l2_dev(&[1.0], &[1.0, 2.0]).is_nan()); // length mismatch
    }

    #[test]
    fn se3_deviation_splits_translation_and_rotation() {
        let a = Pose6D {
            position: [0.0, 0.0, 0.0],
            orientation: UnitQuaternion::identity(),
        };
        let b = Pose6D {
            position: [0.0, 0.0, 0.2],
            orientation: UnitQuaternion::identity(),
        };
        let (t, r) = se3_dev(&a, &b);
        assert!((t - 0.2).abs() < 1e-12);
        assert!(r.abs() < 1e-12);
        // A pure 90-degree rotation: zero translation, pi/2 geodesic.
        let c = Pose6D {
            position: [0.0, 0.0, 0.0],
            orientation: UnitQuaternion::from_axis_angle(
                &nalgebra::Vector3::z_axis(),
                std::f64::consts::FRAC_PI_2,
            ),
        };
        let (t2, r2) = se3_dev(&a, &c);
        assert!(t2.abs() < 1e-12);
        assert!((r2 - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    }

    #[test]
    fn epsilon_candidate_is_a_scaled_percentile() {
        // Deviations 0.0..0.9 (10 samples); p95 nearest-rank = ceil(0.95*10)=10th
        // = 0.9; with a 1.2 safety factor -> 1.08.
        let devs: Vec<f64> = (0..10).map(|i| f64::from(i) / 10.0).collect();
        let eps = epsilon_candidate(&devs, 0.95, 1.2).unwrap();
        assert!((eps - 0.9 * 1.2).abs() < 1e-12);
        // Identical runs -> zero deviation -> zero epsilon.
        assert_eq!(epsilon_candidate(&[0.0, 0.0, 0.0], 0.95, 1.5), Some(0.0));
        // Empty sample -> not yet gradeable.
        assert_eq!(epsilon_candidate(&[], 0.95, 1.0), None);
    }

    #[test]
    fn run_to_run_compares_against_the_reference_run() {
        // Three runs of a scalar; deviations vs the first.
        let devs = run_to_run_abs(&[10.0, 10.2, 9.7]);
        assert_eq!(devs.len(), 2);
        assert!((devs[0] - 0.2).abs() < 1e-9);
        assert!((devs[1] - 0.3).abs() < 1e-9);
        assert!(run_to_run_abs(&[10.0]).is_empty()); // a single run has no variation
    }
}
