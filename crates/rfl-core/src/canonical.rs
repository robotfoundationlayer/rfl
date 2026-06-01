// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Canonical action output model and `execute`-message serialization.
//!
//! Mirrors the `spec/02` CanonicalAction tuple / Envelope and the `execute`
//! message of `schemas/driver-interface.schema.json`. `PoseExpr`'s symbolic
//! variants are a v0 reference-implementation choice carrying runtime-deferred
//! poses; the concrete `Pose6D` representation is owned by `spec/02`.
//!
//! Serialization is deterministic (RD1c): struct field order is fixed, dynamic
//! maps (`serde_json::Value::Object`) are key-sorted, quantities are strings, and
//! no float is emitted by v0 lowering.

use crate::quantity::Quantity;

/// Round a coordinate to 6 decimals for byte-deterministic serialization (RD1c,
/// numeric form). Six decimals is sub-micron in metres; the rounding absorbs the
/// last-ULP differences a transcendental can produce across platforms. Negative
/// zero is normalized to positive zero.
#[must_use]
pub fn round6(x: f64) -> f64 {
    let r = (x * 1e6).round() / 1e6;
    if r == 0.0 { 0.0 } else { r }
}

/// A fully-resolved sweep station pose, coordinates already rounded for
/// deterministic emission. `orientation` is the unit quaternion as `[x, y, z, w]`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SweepPose {
    /// Position in R^3 (metres), in the region frame.
    pub position: [f64; 3],
    /// Orientation unit quaternion `[x, y, z, w]`.
    pub orientation: [f64; 4],
}

impl SweepPose {
    /// Build a rounded `SweepPose` from a `Pose6D`.
    #[must_use]
    pub fn from_pose(p: &crate::pose::Pose6D) -> Self {
        let q = p.orientation.coords; // [x, y, z, w]
        SweepPose {
            position: [
                round6(p.position[0]),
                round6(p.position[1]),
                round6(p.position[2]),
            ],
            orientation: [round6(q[0]), round6(q[1]), round6(q[2]), round6(q[3])],
        }
    }
}

/// A canonical action (`spec/02` § Canonical action representation). Field order is
/// fixed so JSON serialization is byte-deterministic (RD1c).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanonicalAction {
    /// The controlled frame this action moves.
    pub target_frame: String,
    /// Goal pose of the controlled frame (symbolic in v0).
    pub target_pose: PoseExpr,
    /// Driven-force ceiling; absent for pure motion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force_budget: Option<Quantity>,
    /// Nominal duration + timing mode + the reserved rest-at-goal flag.
    pub timing: TimingHints,
    /// Contact-confirmation criterion; absent when no contact is intended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tactile_target: Option<TactileTargetOut>,
    /// StopConditions / feature events the action watches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub monitors: Vec<Monitor>,
    /// The safety constraints in force throughout.
    pub safety_envelope: Envelope,
    /// The grasp stability class this action establishes (grasp primitives only;
    /// `spec/01` § Grasp state model). Absent for non-grasp actions — skipped on the
    /// wire so their serialization is unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grasp_stability: Option<crate::stability::StabilityMetadata>,
}

/// A target-pose expression. v0 carries runtime-deferred poses symbolically; the
/// concrete `{position, orientation}` form is reserved for resolved poses (spec/02).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum PoseExpr {
    /// A let-bound sensed pose (grasp.pinch target, force.insert_fit target_fit).
    Ref {
        /// The let-binding name.
        r#ref: String,
    },
    /// A frame-relative offset pose (transport.move_to_pose).
    FrameRelative {
        /// The reference frame.
        frame: String,
        /// The offset expression (carried from the skill).
        offset: serde_json::Value,
    },
    /// An orientation-only alignment with the under-constrained residual rule
    /// (reach.align): bring `axes` parallel to `target_frame`, residual = minimum
    /// geodesic rotation (spec/02 CA2c).
    OrientationAlign {
        /// The alignment specification.
        align: AlignSpec,
    },
    /// An embodiment-axis-relative withdraw (reach.retract, grasp.release).
    AxisRelative {
        /// The withdraw direction.
        direction: serde_json::Value,
        /// The withdraw distance.
        distance: Quantity,
    },
    /// A fully resolved pose (not produced by v0 lowering; reserved).
    Concrete {
        /// Position in R^3 (metres).
        position: [f64; 3],
        /// Orientation as a unit quaternion `[x, y, z, w]`.
        orientation: [f64; 4],
    },
    /// A generated sweep path (reach.scan): the ordered sensor poses of the sweep
    /// set Σ (spec/02 Appendix A), numeric and rounded.
    SweepPath {
        /// The sweep pattern that generated the path.
        pattern: String,
        /// The ordered sweep poses.
        poses: Vec<SweepPose>,
    },
}

/// An orientation-only alignment directive (`spec/02` CA2c).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlignSpec {
    /// The reference orientation frame.
    pub target_frame: String,
    /// The controlled-frame axes constrained.
    pub axes: Vec<String>,
    /// The residual-orientation rule (always `min_geodesic_rotation` in v0).
    pub residual: String,
}

/// Timing hints (`spec/02` § Terminal semantics; the reserved blending flag).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimingHints {
    /// Nominal duration; absent when planner-derived.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nominal_duration: Option<Quantity>,
    /// Strict vs. time-scalable execution.
    pub timing_mode: TimingMode,
    /// v0.1 fixes `true` (rest-at-goal, CA3c).
    pub stop_at_goal: bool,
}

/// Execution timing mode.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimingMode {
    /// Bit-identical realized timing (kinematic).
    Strict,
    /// The realized timing may be re-timed (contact-bearing).
    TimeScalable,
}

/// A StopCondition / ForceEvent the action watches (`spec/02` monitors).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Monitor {
    /// The stop condition (carried from the skill's `stop_condition`).
    pub stop_condition: serde_json::Value,
}

/// The safety envelope (`spec/02` § The Envelope), bounds clamped to the
/// embodiment's limits.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Envelope {
    /// Kinematic caps (clamped to `embodiment.limits.*`).
    pub motion_bounds: MotionBounds,
    /// The force / torque trajectory bound (force category).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force_profile: Option<serde_json::Value>,
    /// The station-keeping contract for an interval-invariant station hold
    /// (`reach.hover` settling, `spec/01` § 1.5): `{station_tolerance, settling_time}`.
    /// The disturbance-recovery leg of the interval-invariant check (`05` ENV3) samples it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub station_keeping: Option<serde_json::Value>,
    /// Minimum clearance to the collision model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearance: Option<Quantity>,
    /// The requested compliance mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compliance: Option<String>,
    /// Max time to reach a safe state on a breach.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_time: Option<Quantity>,
}

/// Kinematic motion bounds (`spec/02` Envelope.motion_bounds).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MotionBounds {
    /// Max linear velocity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v_max: Option<Quantity>,
    /// Max linear acceleration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a_max: Option<Quantity>,
    /// Max angular velocity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w_max: Option<Quantity>,
}

/// The realized tactile-confirmation criterion after retargeting. Serializes as the
/// string `auto` (manifold tier), a `{proxy: ...}` object (proxy degradation), or
/// an inline target carried through.
#[derive(Debug, Clone)]
pub enum TactileTargetOut {
    /// Manifold-tier: the `auto` criterion was kept (tactile embodiment).
    Auto,
    /// Proxy-tier degradation on a no-tactile embodiment (`spec/04`).
    Proxy {
        /// The proxy criterion.
        proxy: ProxySpec,
    },
    /// An explicit inline target carried through.
    Explicit(serde_json::Value),
}

impl serde::Serialize for TactileTargetOut {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            TactileTargetOut::Auto => serializer.serialize_str("auto"),
            TactileTargetOut::Proxy { proxy } => {
                use serde::ser::SerializeMap;
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("proxy", proxy)?;
                m.end()
            }
            TactileTargetOut::Explicit(v) => v.serialize(serializer),
        }
    }
}

impl<'de> serde::Deserialize<'de> for TactileTargetOut {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Mirror of the custom Serialize: `"auto"` -> Auto, a `{proxy: ...}` object
        // -> Proxy, anything else -> Explicit. ProxySpec's `&'static str` contents
        // are reconstructed to fixed literals (the only consumer reads the *variant*
        // for the fidelity tier, not the criterion text), so the proxy VARIANT
        // round-trips even though its inner strings are not byte-preserved.
        let v = serde_json::Value::deserialize(deserializer)?;
        match &v {
            serde_json::Value::String(s) if s == "auto" => Ok(TactileTargetOut::Auto),
            serde_json::Value::Object(map) if map.contains_key("proxy") => {
                Ok(TactileTargetOut::Proxy {
                    proxy: ProxySpec {
                        tier: "proxy",
                        criterion: "reconstructed",
                    },
                })
            }
            _ => Ok(TactileTargetOut::Explicit(v)),
        }
    }
}

/// A force/position-proxy confirmation descriptor (`spec/04` § Graceful degradation).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProxySpec {
    /// The fidelity tier (`proxy`).
    pub tier: &'static str,
    /// The proxy criterion identifier.
    pub criterion: &'static str,
}

/// An `execute` action Goal (`schemas/driver-interface.schema.json` ExecuteGoal).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecuteGoal {
    /// The message discriminant (`execute`).
    pub message: String,
    /// The action correlation id.
    pub action_id: String,
    /// The canonical action delivered.
    pub canonical_action: CanonicalAction,
}

impl ExecuteGoal {
    /// Wrap a canonical action with its correlation id.
    #[must_use]
    pub fn wrap(action_id: impl Into<String>, canonical_action: CanonicalAction) -> Self {
        Self {
            message: "execute".to_string(),
            action_id: action_id.into(),
            canonical_action,
        }
    }
}

/// Serialize a retargeted action sequence as JSON Lines. The `action_id` is the
/// deterministic `{skill}/{embodiment_id}/{NNNN}-{suffix}` (1-based, zero-padded 4).
///
/// # Panics
/// If serializing an `execute` message to JSON fails (does not happen for well-formed actions).
#[must_use]
pub fn to_jsonl(
    skill: &str,
    embodiment_id: &str,
    actions: &[CanonicalAction],
    suffixes: &[&str],
) -> String {
    let mut out = String::new();
    for (i, (action, suffix)) in actions.iter().zip(suffixes).enumerate() {
        let id = format!("{skill}/{embodiment_id}/{:04}-{suffix}", i + 1);
        let msg = ExecuteGoal::wrap(id, action.clone());
        out.push_str(&serde_json::to_string(&msg).expect("serialize execute message"));
        out.push('\n');
    }
    out
}

/// Parse a JSON Lines stream of `execute` messages back into goals — the inverse
/// of [`to_jsonl`]. This is the driver-input side of the wire (the symmetric
/// counterpart to the telemetry/status parsing a verifier does): a driver
/// implementation consumes the canonical goals it receives this way.
///
/// # Errors
/// Returns the `serde_json` error of the first malformed line.
pub fn from_jsonl(jsonl: &str) -> Result<Vec<ExecuteGoal>, serde_json::Error> {
    jsonl
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str::<ExecuteGoal>)
        .collect()
}

#[cfg(test)]
mod tests {
    // Deterministic retarget/serialization output: exact golden-value comparison is intended.
    #![allow(clippy::float_cmp, clippy::unreadable_literal)]

    use super::*;

    #[test]
    fn round6_normalizes_and_rounds() {
        assert_eq!(round6(0.0333333333), 0.033333);
        assert_eq!(round6(0.1), 0.1);
        assert_eq!(round6(-0.0000001), 0.0); // negative zero normalized to +0.0
    }

    #[test]
    fn sweep_path_serializes_rounded_numbers() {
        let p = crate::pose::Pose6D {
            position: [0.0333333333, 0.025, 0.1],
            orientation: nalgebra::UnitQuaternion::from_axis_angle(
                &nalgebra::Vector3::x_axis(),
                std::f64::consts::PI,
            ),
        };
        let expr = PoseExpr::SweepPath {
            pattern: "raster".into(),
            poses: vec![SweepPose::from_pose(&p)],
        };
        let json = serde_json::to_string(&expr).unwrap();
        assert!(json.contains("\"pattern\":\"raster\""));
        assert!(json.contains("\"position\":[0.033333,0.025,0.1]"));
        assert!(json.contains("\"orientation\":[1.0,0.0,0.0,0.0]"));
    }

    fn sample() -> CanonicalAction {
        CanonicalAction {
            target_frame: "tcp_thumb".into(),
            target_pose: PoseExpr::Ref {
                r#ref: "receptacle_t".into(),
            },
            force_budget: Some(Quantity("15 N".into())),
            timing: TimingHints {
                nominal_duration: None,
                timing_mode: TimingMode::Strict,
                stop_at_goal: true,
            },
            tactile_target: Some(TactileTargetOut::Auto),
            monitors: vec![],
            safety_envelope: Envelope {
                motion_bounds: MotionBounds::default(),
                force_profile: None,
                station_keeping: None,
                clearance: None,
                compliance: Some("active".into()),
                stop_time: Some(Quantity("0.2 s".into())),
            },
            grasp_stability: None,
        }
    }

    #[test]
    fn execute_message_has_expected_shape() {
        let msg = ExecuteGoal::wrap("cable-insertion/wonik-allegro-v4/0001-pinch", sample());
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"message\":\"execute\""));
        assert!(json.contains("\"action_id\":\"cable-insertion/wonik-allegro-v4/0001-pinch\""));
        assert!(json.contains("\"target_frame\":\"tcp_thumb\""));
        assert!(json.contains("\"force_budget\":\"15 N\""));
        assert!(json.contains("\"tactile_target\":\"auto\""));
    }

    #[test]
    fn serialization_is_deterministic() {
        let msg = ExecuteGoal::wrap("a/b/0001-x", sample());
        let a = serde_json::to_string(&msg).unwrap();
        let b = serde_json::to_string(&msg).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn execute_message_round_trips_through_from_jsonl() {
        // to_jsonl -> from_jsonl -> to_jsonl is a fixed point (the wire round-trips).
        let actions = vec![sample(), sample()];
        let jsonl = to_jsonl("s", "e", &actions, &["pinch", "insert_fit"]);
        let goals = from_jsonl(&jsonl).expect("parse");
        assert_eq!(goals.len(), 2);
        assert_eq!(goals[0].action_id, "s/e/0001-pinch");
        assert_eq!(goals[0].canonical_action.target_frame, "tcp_thumb");
        // Re-serializing the parsed goals reproduces the byte stream.
        let mut reser = String::new();
        for g in &goals {
            reser.push_str(&serde_json::to_string(g).unwrap());
            reser.push('\n');
        }
        assert_eq!(reser, jsonl);
    }

    #[test]
    fn proxy_and_stability_round_trip_through_the_wire() {
        use crate::stability::{Closure, StabilityMetadata};
        let mut a = sample();
        a.tactile_target = Some(TactileTargetOut::Proxy {
            proxy: ProxySpec {
                tier: "proxy",
                criterion: "force_position",
            },
        });
        a.grasp_stability = Some(StabilityMetadata::for_mode(
            crate::grasp_force::GraspMode::Pinch,
        ));
        let jsonl = to_jsonl("s", "e", &[a], &["pinch"]);
        let goals = from_jsonl(&jsonl).expect("parse");
        // The proxy VARIANT and the grasp-stability closure survive the round-trip.
        assert!(matches!(
            goals[0].canonical_action.tactile_target,
            Some(TactileTargetOut::Proxy { .. })
        ));
        assert_eq!(
            goals[0]
                .canonical_action
                .grasp_stability
                .as_ref()
                .unwrap()
                .closure,
            Closure::Force
        );
    }

    #[test]
    fn jsonl_joins_actions_one_per_line() {
        let actions = vec![sample(), sample()];
        let jsonl = to_jsonl(
            "cable-insertion",
            "wonik-allegro-v4",
            &actions,
            &["pinch", "insert_fit"],
        );
        assert_eq!(jsonl.lines().count(), 2);
        assert!(jsonl.lines().next().unwrap().contains("0001-pinch"));
        assert!(jsonl.lines().nth(1).unwrap().contains("0002-insert_fit"));
        assert!(jsonl.ends_with('\n'));
    }
}
