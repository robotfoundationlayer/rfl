// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Scan-region input model (`spec/02` Appendix A region kinds). v0 models the
//! `surface` kind (a planar rectangle) and an explicit `waypoints` list; `path`
//! and `volume` reduce to repeated surface coverage and are deferred.

use crate::quantity::Quantity;
use crate::skill_isa::FrameRef;

/// A scan region (`spec/02` Appendix A). `serde(tag = "kind")` discriminates by the
/// `kind` field.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ScanRegion {
    /// A planar rectangular surface of `size_u` by `size_v` in `frame`'s xy-plane,
    /// outward normal `+z`.
    Surface {
        /// The region reference frame.
        frame: FrameRef,
        /// Extent along the frame x-axis (the u direction).
        size_u: Quantity,
        /// Extent along the frame y-axis (the v direction).
        size_v: Quantity,
    },
    /// An explicit ordered list of sweep poses (the `waypoints` pattern source).
    Waypoints {
        /// The reference frame the poses are expressed in.
        frame: FrameRef,
        /// The ordered poses.
        poses: Vec<WaypointPose>,
    },
}

/// A waypoint pose as authored: position + orientation quaternion `[x, y, z, w]`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WaypointPose {
    /// Position in R^3 (metres).
    pub position: [f64; 3],
    /// Orientation unit quaternion `[x, y, z, w]`.
    pub orientation: [f64; 4],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_surface_region() {
        let yaml = "kind: surface\nframe: panel\nsize_u: 200 mm\nsize_v: 150 mm\n";
        let r: ScanRegion = serde_yaml::from_str(yaml).unwrap();
        let ScanRegion::Surface {
            frame,
            size_u,
            size_v,
        } = r
        else {
            panic!("expected surface")
        };
        assert_eq!(frame, "panel");
        assert_eq!(size_u.0, "200 mm");
        assert_eq!(size_v.0, "150 mm");
    }

    #[test]
    fn parses_waypoints_region() {
        let yaml = "kind: waypoints\nframe: task\nposes:\n  - position: [0.1, 0.0, 0.2]\n    orientation: [1.0, 0.0, 0.0, 0.0]\n";
        let r: ScanRegion = serde_yaml::from_str(yaml).unwrap();
        let ScanRegion::Waypoints { frame, poses } = r else {
            panic!("expected waypoints")
        };
        assert_eq!(frame, "task");
        assert_eq!(poses.len(), 1);
        assert_eq!(poses[0].position, [0.1, 0.0, 0.2]);
    }
}
