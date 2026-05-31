// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! The sweep-set Σ generators (`spec/02` Appendix A, Normative sweep-pattern
//! generators). Pure functions of SI scalars so generation is byte-deterministic.

use crate::pose::Pose6D;
use nalgebra::{UnitQuaternion, Vector3};

/// The fixed orientation for a v0 axis-aligned surface station: the sensor bore
/// (`+z`) points anti-parallel to the surface normal (`+z`) with the secondary axis
/// along `+u` — a 180-degree rotation about x. General bore/normal orientation is
/// deferred.
fn station_orientation() -> UnitQuaternion<f64> {
    UnitQuaternion::from_axis_angle(&Vector3::x_axis(), std::f64::consts::PI)
}

/// Generate the raster sweep set Σ for a planar surface (`spec/02` Appendix A,
/// raster). `size_u`/`size_v`/`standoff` are metres; `coverage_overlap` is in
/// `[0, 1)`; `h_angle_rad`/`v_angle_rad` are the full FOV angles in radians. The
/// returned poses are in the region frame, in serpentine (boustrophedon) order.
#[must_use]
pub fn raster(
    size_u: f64,
    size_v: f64,
    standoff: f64,
    coverage_overlap: f64,
    h_angle_rad: f64,
    v_angle_rad: f64,
) -> Vec<Pose6D> {
    let f_u = 2.0 * standoff * (h_angle_rad / 2.0).tan();
    let f_v = 2.0 * standoff * (v_angle_rad / 2.0).tan();
    let s_u = f_u * (1.0 - coverage_overlap);
    let s_v = f_v * (1.0 - coverage_overlap);
    let n_u = (size_u / s_u).ceil() as usize;
    let n_v = (size_v / s_v).ceil() as usize;
    let orientation = station_orientation();

    let mut poses = Vec::with_capacity(n_u * n_v);
    for k in 0..n_v {
        let v = (k as f64 + 0.5) * size_v / n_v as f64;
        // Serpentine: ascending u on even rows, descending on odd rows.
        let cols: Vec<usize> = if k % 2 == 0 {
            (0..n_u).collect()
        } else {
            (0..n_u).rev().collect()
        };
        for j in cols {
            let u = (j as f64 + 0.5) * size_u / n_u as f64;
            poses.push(Pose6D { position: [u, v, standoff], orientation });
        }
    }
    poses
}

/// The waypoints generator (`spec/02` Appendix A, waypoints): the caller's ordered
/// poses verbatim. `standoff` / `coverage_overlap` / `fov` are ignored. Each input
/// is `(position, orientation [x, y, z, w])`.
#[must_use]
pub fn waypoints(wps: &[([f64; 3], [f64; 4])]) -> Vec<Pose6D> {
    wps.iter()
        .map(|(position, q)| Pose6D {
            position: *position,
            orientation: UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
                q[3], q[0], q[1], q[2], // Quaternion::new(w, x, y, z)
            )),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::round6;

    #[test]
    fn allegro_raster_has_nine_poses() {
        // U=0.20, V=0.15, standoff=0.10, overlap=0.2, fov 60x45 deg.
        let poses = raster(0.20, 0.15, 0.10, 0.2, 60_f64.to_radians(), 45_f64.to_radians());
        assert_eq!(poses.len(), 9); // n_u=3, n_v=3
        assert_eq!(round6(poses[0].position[0]), 0.033333);
        assert_eq!(round6(poses[0].position[1]), 0.025);
        assert_eq!(round6(poses[0].position[2]), 0.10);
        let q = poses[0].orientation.coords;
        assert!((round6(q[0]) - 1.0).abs() < 1e-9);
        assert!(round6(q[3]).abs() < 1e-9);
    }

    #[test]
    fn raster_is_serpentine() {
        let poses = raster(0.20, 0.15, 0.10, 0.2, 60_f64.to_radians(), 45_f64.to_radians());
        assert!(poses[0].position[0] < poses[2].position[0]); // row 0 ascending
        assert!(poses[3].position[0] > poses[5].position[0]); // row 1 descending
    }

    #[test]
    fn leap_and_pneumatic_have_different_counts() {
        let leap = raster(0.20, 0.15, 0.10, 0.2, 70_f64.to_radians(), 55_f64.to_radians());
        let pneu = raster(0.20, 0.15, 0.10, 0.2, 65_f64.to_radians(), 50_f64.to_radians());
        assert_eq!(leap.len(), 4); // 2x2
        assert_eq!(pneu.len(), 6); // 2x3
    }

    #[test]
    fn waypoints_passes_through() {
        let wps = vec![
            ([0.1, 0.0, 0.2], [1.0, 0.0, 0.0, 0.0]),
            ([0.2, 0.1, 0.2], [1.0, 0.0, 0.0, 0.0]),
        ];
        let poses = waypoints(&wps);
        assert_eq!(poses.len(), 2);
        assert_eq!(poses[1].position, [0.2, 0.1, 0.2]);
    }
}
