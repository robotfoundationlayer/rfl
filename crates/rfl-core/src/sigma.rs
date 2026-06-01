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
            poses.push(Pose6D {
                position: [u, v, standoff],
                orientation,
            });
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

/// Archimedean-spiral arc length from the centre to polar angle `theta`, for the
/// spiral `r = a·θ`: `L(θ) = (a/2)·[θ·√(1+θ²) + asinh(θ)]` (from the polar
/// arc-length element `ds = a·√(θ²+1) dθ`).
fn spiral_arc_length(theta: f64, a: f64) -> f64 {
    0.5 * a * (theta * (1.0 + theta * theta).sqrt() + theta.asinh())
}

/// Solve `L(θ) = target` for the unique `θ ≥ theta_lo` by bisection. `L` is
/// strictly increasing; for a one-arc-length-step advance (`target − L(theta_lo) =
/// s = 2π·a`) the bracket `[theta_lo, theta_lo + 2π]` always contains the root
/// because `dL/dθ = a·√(θ²+1) ≥ a`. Bisection is chosen over Newton for
/// byte-determinism: pure comparison + midpoint, platform-invariant control flow.
fn solve_theta_for_arc_length(target: f64, a: f64, theta_lo: f64) -> f64 {
    let mut lo = theta_lo;
    let mut hi = theta_lo + 2.0 * std::f64::consts::PI;
    // 60 halvings drive the bracket below 2π/2^60 ≈ 5e-18, far under round6 (1e-6).
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if spiral_arc_length(mid, a) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Generate the spiral sweep set Σ for a planar surface (`spec/02` Appendix A,
/// spiral). An Archimedean spiral outward from the surface centroid at constant
/// arc-length step Δℓ = s = min(s_u, s_v), CCW, bounded by the region half-diagonal
/// (the v0 reading of "bounding radius"). Same units / FOV inputs as `raster`; the
/// fixed surface `station_orientation` is reused (general orientation is deferred).
#[must_use]
pub fn spiral(
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
    let s = s_u.min(s_v);
    let a = s / (2.0 * std::f64::consts::PI);
    let (cu, cv) = (size_u / 2.0, size_v / 2.0);
    let r_bound = (cu * cu + cv * cv).sqrt();
    let orientation = station_orientation();

    let mut poses = Vec::new();
    // Degenerate guard: zero spacing (a flat FOV) would never advance — emit only
    // the centroid station.
    if s <= 0.0 {
        poses.push(Pose6D {
            position: [cu, cv, standoff],
            orientation,
        });
        return poses;
    }
    let mut i = 0usize;
    let mut theta_prev = 0.0_f64;
    loop {
        let theta = if i == 0 {
            0.0
        } else {
            solve_theta_for_arc_length(i as f64 * s, a, theta_prev)
        };
        let r = a * theta;
        if r > r_bound {
            break;
        }
        poses.push(Pose6D {
            position: [cu + r * theta.cos(), cv + r * theta.sin(), standoff],
            orientation,
        });
        theta_prev = theta;
        i += 1;
    }
    poses
}

#[cfg(test)]
mod tests {
    // Deterministic retarget/geometry output: exact golden-value comparison is intended.
    #![allow(clippy::float_cmp, clippy::unreadable_literal)]

    use super::*;
    use crate::canonical::round6;

    #[test]
    fn allegro_raster_has_nine_poses() {
        // U=0.20, V=0.15, standoff=0.10, overlap=0.2, fov 60x45 deg.
        let poses = raster(
            0.20,
            0.15,
            0.10,
            0.2,
            60_f64.to_radians(),
            45_f64.to_radians(),
        );
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
        let poses = raster(
            0.20,
            0.15,
            0.10,
            0.2,
            60_f64.to_radians(),
            45_f64.to_radians(),
        );
        assert!(poses[0].position[0] < poses[2].position[0]); // row 0 ascending
        assert!(poses[3].position[0] > poses[5].position[0]); // row 1 descending
    }

    #[test]
    fn leap_and_pneumatic_have_different_counts() {
        let leap = raster(
            0.20,
            0.15,
            0.10,
            0.2,
            70_f64.to_radians(),
            55_f64.to_radians(),
        );
        let pneu = raster(
            0.20,
            0.15,
            0.10,
            0.2,
            65_f64.to_radians(),
            50_f64.to_radians(),
        );
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

    #[test]
    fn spiral_first_station_is_the_centroid() {
        // i=0 sits at theta=0, r=0 => the surface centroid (U/2, V/2), z=standoff.
        let poses = spiral(
            0.20,
            0.15,
            0.10,
            0.2,
            60_f64.to_radians(),
            45_f64.to_radians(),
        );
        assert_eq!(round6(poses[0].position[0]), 0.10);
        assert_eq!(round6(poses[0].position[1]), 0.075);
        assert_eq!(round6(poses[0].position[2]), 0.10);
    }

    #[test]
    fn allegro_spiral_has_twelve_poses() {
        // 60x45 deg FOV, standoff 0.10, overlap 0.2 -> s=0.0662742; stations to the
        // half-diagonal R=0.125 give i=0..11. Analytic value; a mismatch means
        // recheck the arc-length math, not a blind update.
        let poses = spiral(
            0.20,
            0.15,
            0.10,
            0.2,
            60_f64.to_radians(),
            45_f64.to_radians(),
        );
        assert_eq!(poses.len(), 12);
    }

    #[test]
    fn spiral_radius_is_non_decreasing() {
        let poses = spiral(
            0.20,
            0.15,
            0.10,
            0.2,
            60_f64.to_radians(),
            45_f64.to_radians(),
        );
        let (cu, cv) = (0.10_f64, 0.075_f64);
        let mut prev = -1.0_f64;
        for p in &poses {
            let r = ((p.position[0] - cu).powi(2) + (p.position[1] - cv).powi(2)).sqrt();
            assert!(r >= prev - 1e-9, "radius decreased: {r} < {prev}");
            prev = r;
        }
    }

    #[test]
    fn spiral_count_varies_per_fov() {
        let allegro = spiral(
            0.20,
            0.15,
            0.10,
            0.2,
            60_f64.to_radians(),
            45_f64.to_radians(),
        );
        let leap = spiral(
            0.20,
            0.15,
            0.10,
            0.2,
            70_f64.to_radians(),
            55_f64.to_radians(),
        );
        let pneu = spiral(
            0.20,
            0.15,
            0.10,
            0.2,
            65_f64.to_radians(),
            50_f64.to_radians(),
        );
        // Smaller FOV -> smaller footprint -> smaller spacing -> more stations.
        assert_eq!((allegro.len(), leap.len(), pneu.len()), (12, 8, 10));
        assert_ne!(allegro.len(), 9); // differs from the raster count
    }
}
