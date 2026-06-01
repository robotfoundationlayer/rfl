// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Contact-polygon geometry for the stability checks (`spec/05` § Closure,
//! stability, and composition; `spec/01` § World-state model `supported`).
//!
//! These are the pure-geometry cores of three conformance checks that share one
//! computation — **CoM over the contact polygon**:
//!
//! - **STB1** (`spec/05` § 146) — a `grasp.precision_tripod` is non-degenerate
//!   only when its three contacts form a non-collinear triangle above a
//!   minimum-area threshold ([`tripod_non_degenerate`]).
//! - **STB2 / `grasp.platform`** — a support grasp balances the object's CoM over
//!   the contact polygon by a margin ([`com_over_polygon`]).
//! - **`grasp.release` `supported()`** (`spec/01` line 348) — release is gated on
//!   the CoM projecting strictly inside the contact polygon. Same computation.
//!
//! These functions are wire-independent: they take contact-site positions and a
//! CoM as plain coordinates. Wiring them to a `contact_geometry` telemetry field
//! is a normative wire decision (see
//! `docs/design/2026-06-02-contact-geometry-wire-proposal.md`); the math is ready
//! now, the wire is pending the spec owner. Contact-polygon vertices are taken in
//! boundary order (a proposed wire contract).

use nalgebra::Vector3;

fn v(p: [f64; 3]) -> Vector3<f64> {
    Vector3::new(p[0], p[1], p[2])
}

/// The area of the triangle `(a, b, c)` — half the magnitude of the edge cross
/// product. Zero for a collinear (degenerate) triple.
#[must_use]
pub fn triangle_area(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let (a, b, c) = (v(a), v(b), v(c));
    0.5 * (b - a).cross(&(c - a)).norm()
}

/// STB1: the three tripod contacts form a non-collinear triangle whose area is at
/// least `min_area`. A near-collinear triple (area below the threshold) is not a
/// valid tripod — the claimed rotation constraint would be absent.
#[must_use]
pub fn tripod_non_degenerate(sites: [[f64; 3]; 3], min_area: f64) -> bool {
    triangle_area(sites[0], sites[1], sites[2]) >= min_area
}

/// The in-plane 2D coordinates of (the boundary vertices, the queried points).
type Projected = (Vec<[f64; 2]>, Vec<[f64; 2]>);

/// Project `points` onto the plane spanned by the first three `boundary` vertices,
/// returning 2D coordinates `(u, w)` in an orthonormal in-plane basis. The
/// boundary must have at least three non-collinear vertices.
fn project_to_plane(boundary: &[[f64; 3]], points: &[[f64; 3]]) -> Option<Projected> {
    if boundary.len() < 3 {
        return None;
    }
    let o = v(boundary[0]);
    let e1 = v(boundary[1]) - o;
    let u = e1.try_normalize(1e-12)?;
    let normal = e1.cross(&(v(boundary[2]) - o)).try_normalize(1e-12)?;
    let w = normal.cross(&u); // u, w orthonormal in the plane
    let to_2d = |p: [f64; 3]| {
        let d = v(p) - o;
        [d.dot(&u), d.dot(&w)]
    };
    Some((
        boundary.iter().map(|p| to_2d(*p)).collect(),
        points.iter().map(|p| to_2d(*p)).collect(),
    ))
}

/// True iff the 2D `point` lies inside the **convex** polygon `poly` (boundary
/// order) by at least `margin` — its perpendicular distance to every edge is
/// `>= margin` and it is on the interior side. Works for either winding.
fn inside_convex_2d(point: [f64; 2], poly: &[[f64; 2]], margin: f64) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    // Signed area picks the winding so the interior side is consistent.
    let mut signed = 0.0;
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        signed += a[0] * b[1] - b[0] * a[1];
    }
    let orient = if signed >= 0.0 { 1.0 } else { -1.0 };
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let edge = [b[0] - a[0], b[1] - a[1]];
        let len = (edge[0] * edge[0] + edge[1] * edge[1]).sqrt();
        if len < 1e-12 {
            continue;
        }
        // Perpendicular distance to the edge, signed by interior orientation.
        let cross = edge[0] * (point[1] - a[1]) - edge[1] * (point[0] - a[0]);
        let dist = orient * cross / len;
        if dist < margin {
            return false;
        }
    }
    true
}

/// STB2 / `supported()`: the object centre of mass `com` projects strictly inside
/// the contact polygon (the `sites` in boundary order) by at least `margin`. The
/// projection is onto the polygon's plane; the polygon is treated as convex (the
/// contact polygon is the convex hull of the contacts). `false` if the sites are
/// degenerate (fewer than three / collinear).
#[must_use]
pub fn com_over_polygon(com: [f64; 3], sites: &[[f64; 3]], margin: f64) -> bool {
    let Some((poly2d, com2d)) = project_to_plane(sites, std::slice::from_ref(&com)) else {
        return false;
    };
    inside_convex_2d(com2d[0], &poly2d, margin)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]
    use super::*;

    #[test]
    fn triangle_area_of_a_unit_right_triangle() {
        let a = triangle_area([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert!((a - 0.5).abs() < 1e-12);
    }

    #[test]
    fn collinear_triple_is_degenerate() {
        // Three points on a line -> zero area -> not a tripod.
        let sites = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        assert_eq!(triangle_area(sites[0], sites[1], sites[2]), 0.0);
        assert!(!tripod_non_degenerate(sites, 1e-6));
    }

    #[test]
    fn spread_tripod_passes_its_area_threshold() {
        // A well-spread equilateral-ish triangle clears a small threshold but a
        // very thin sliver does not.
        let spread = [[0.0, 0.0, 0.0], [0.04, 0.0, 0.0], [0.02, 0.035, 0.0]];
        assert!(tripod_non_degenerate(spread, 0.0005));
        let sliver = [[0.0, 0.0, 0.0], [0.04, 0.0, 0.0], [0.02, 0.0001, 0.0]];
        assert!(!tripod_non_degenerate(sliver, 0.0005));
    }

    #[test]
    fn com_inside_a_square_with_margin() {
        // A unit square in the z=0 plane (boundary order); centroid is well inside.
        let square = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        assert!(com_over_polygon([0.5, 0.5, 0.0], &square, 0.1));
        // Near an edge: inside the polygon but within the margin -> fails strict.
        assert!(!com_over_polygon([0.05, 0.5, 0.0], &square, 0.1));
        // Outside the polygon entirely -> fails.
        assert!(!com_over_polygon([1.5, 0.5, 0.0], &square, 0.0));
    }

    #[test]
    fn com_check_works_on_a_tilted_plane() {
        // The same square tilted out of the z=0 plane; the CoM lifted with it.
        // Projection onto the polygon plane must still place the centroid inside.
        let tilted = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        // Centroid of the four vertices lies on the tilted plane.
        assert!(com_over_polygon([0.5, 0.5, 0.5], &tilted, 0.1));
    }

    #[test]
    fn degenerate_sites_are_not_supported() {
        assert!(!com_over_polygon(
            [0.0, 0.0, 0.0],
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            0.0
        ));
    }
}
