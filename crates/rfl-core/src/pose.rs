// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Pose representation and orientation error (`spec/02` § Pose representation).

use nalgebra::UnitQuaternion;

/// A rigid-body pose: position in R^3 (metres) and a unit-quaternion orientation
/// (`spec/02` § Pose representation). The unit quaternion is the canonical
/// serialization; SE(3) is the group beneath it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose6D {
    /// Position in R^3 (metres).
    pub position: [f64; 3],
    /// Orientation as a unit quaternion.
    pub orientation: UnitQuaternion<f64>,
}

/// The single-scalar geodesic orientation error (`spec/02`, CA1c):
/// `theta = 2 * arccos(|<q_t, q_c>|)`, in `[0, pi]`. The absolute value resolves
/// the quaternion double-cover so `q` and `-q` denote one orientation.
#[must_use]
pub fn theta_orient(q_t: &UnitQuaternion<f64>, q_c: &UnitQuaternion<f64>) -> f64 {
    let dot = q_t.coords.dot(&q_c.coords).abs().min(1.0);
    2.0 * dot.acos()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn q(w: f64, x: f64, y: f64, z: f64) -> UnitQuaternion<f64> {
        UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(w, x, y, z))
    }

    #[test]
    fn identical_orientations_have_zero_error() {
        let a = q(1.0, 0.0, 0.0, 0.0);
        assert!(theta_orient(&a, &a).abs() < 1e-9);
    }

    #[test]
    fn opposite_sign_is_same_orientation() {
        // q and -q denote one orientation: the double-cover resolves to ~0.
        let a = q(1.0, 0.0, 0.0, 0.0);
        let b = q(-1.0, 0.0, 0.0, 0.0);
        assert!(theta_orient(&a, &b).abs() < 1e-9);
    }

    #[test]
    fn quarter_turn_about_z_is_half_pi() {
        let a = q(1.0, 0.0, 0.0, 0.0);
        let half = (PI / 4.0).cos();
        let b = q(half, 0.0, 0.0, half); // 90 deg about z
        assert!((theta_orient(&a, &b) - PI / 2.0).abs() < 1e-6);
    }
}
