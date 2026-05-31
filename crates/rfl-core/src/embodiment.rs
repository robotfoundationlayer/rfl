// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Embodiment descriptor input (`spec/03`, `schemas/embodiment-descriptor.schema.json`).
//!
//! Models the fields the Translation Layer reads: the capability manifest (for the
//! `capability_absent` gate), `aux.tactile_sensing` (for tactile/proxy routing), the
//! `limits` retargeting clamps envelope bounds to, and the frame model (for the
//! per-embodiment control/grasp/sensor frame resolution).

use std::collections::BTreeMap;
use crate::quantity::Quantity;

/// A descriptor file: everything is wrapped under `embodiment:`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EmbodimentFile {
    /// The embodiment descriptor.
    pub embodiment: Embodiment,
}

/// A parsed embodiment descriptor.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Embodiment {
    /// Stable embodiment id.
    pub id: String,
    /// Embodiment class (free-form).
    #[serde(default)]
    pub class: Option<String>,
    /// The capability manifest.
    pub capabilities: Capabilities,
    /// The flat `embodiment.limits.*` namespace.
    #[serde(default)]
    pub limits: BTreeMap<String, LimitValue>,
    /// Contact-sensor descriptor (`spec/04`); absent on a no-tactile embodiment.
    #[serde(default)]
    pub tactile: Option<serde_yaml::Value>,
    /// Frame model (`spec/03` § Embodiment frame model).
    #[serde(default)]
    pub frames: Frames,
    /// Non-contact sensor descriptors keyed by sensor frame (`spec/03` § Sensor descriptor).
    #[serde(default)]
    pub sensors: BTreeMap<String, Sensor>,
}

/// The capability manifest (`spec/03` § Capability manifest).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Capabilities {
    /// Declared primitive / mode / category capability keys.
    pub skills: Vec<String>,
    /// Auxiliary capabilities.
    #[serde(default)]
    pub aux: Aux,
}

/// Auxiliary capabilities (`spec/03` § Capability manifest, aux).
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Aux {
    /// Whether the embodiment has contact sensing.
    #[serde(default)]
    pub tactile_sensing: Option<bool>,
    /// Declared compliance capability (domain carried opaquely in v0).
    #[serde(default)]
    pub compliance: Option<serde_yaml::Value>,
}

/// The embodiment frame model (`spec/03` § Embodiment frame model). Roles, never
/// body parts: `role_defaults` names which control frame plays each role.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Frames {
    /// The declared control frames.
    #[serde(default)]
    pub control_frames: Vec<String>,
    /// Which control frame plays each role.
    #[serde(default)]
    pub role_defaults: RoleDefaults,
}

/// Role-to-frame default assignments (`spec/03` § Embodiment frame model).
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct RoleDefaults {
    /// Default grasp frame.
    #[serde(default)]
    pub grasp: Option<String>,
    /// Default sensor frame.
    #[serde(default)]
    pub sensor: Option<String>,
    /// Default tactile frame.
    #[serde(default)]
    pub tactile: Option<String>,
    /// Default support frame.
    #[serde(default)]
    pub support: Option<String>,
}

/// A non-contact sensor descriptor (`spec/03` § Sensor descriptor). Only the fields
/// the Translation Layer reads are typed; the rest are ignored.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Sensor {
    /// The sensor bore axis (e.g. `+z`).
    #[serde(default)]
    pub bore_axis: Option<String>,
    /// The angular field of view about the bore axis.
    #[serde(default)]
    pub fov: Option<Fov>,
}

/// The angular field of view `{h_angle, v_angle}` about the bore axis.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Fov {
    /// Horizontal angle (the u direction).
    pub h_angle: Quantity,
    /// Vertical angle (the v direction).
    pub v_angle: Quantity,
}

/// A value in the flat `embodiment.limits.*` namespace (`spec/03` § Limits): a
/// scalar quantity (`20 N`), a `[lo, hi]` range (`[10 mm, 80 mm]`), or any other
/// shape carried opaquely. `serde(untagged)` discriminates by shape.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum LimitValue {
    /// A scalar quantity (the common case the Translation Layer clamps to).
    Scalar(Quantity),
    /// A `[lo, hi]` range (e.g. an enclosure span).
    Range(Vec<Quantity>),
    /// Any other shape (a bare ratio, a nested object), carried opaquely.
    Other(serde_yaml::Value),
}

impl Embodiment {
    /// Parse a descriptor from YAML text (unwrapping the `embodiment:` envelope).
    ///
    /// # Errors
    /// Returns `Error::Driver` if the YAML does not match the descriptor shape.
    pub fn parse_yaml(text: &str) -> crate::Result<Self> {
        let f: EmbodimentFile =
            serde_yaml::from_str(text).map_err(|e| crate::Error::Driver(e.to_string()))?;
        Ok(f.embodiment)
    }

    /// Whether a capability key (a primitive id, a grasp mode, or a category gate
    /// such as `transport`) is declared.
    #[must_use]
    pub fn has_skill(&self, key: &str) -> bool {
        self.capabilities.skills.iter().any(|s| s == key)
    }

    /// Whether the embodiment declares `aux.tactile_sensing: true`.
    #[must_use]
    pub fn tactile_sensing(&self) -> bool {
        self.capabilities.aux.tactile_sensing.unwrap_or(false)
    }

    /// The scalar quantity at a limit key, if that limit is a scalar (not a range
    /// or other shape). The Translation Layer clamps envelope bounds to scalars.
    #[must_use]
    pub fn scalar_limit(&self, key: &str) -> Option<&Quantity> {
        match self.limits.get(key) {
            Some(LimitValue::Scalar(q)) => Some(q),
            _ => None,
        }
    }

    /// The default grasp control frame (`frames.role_defaults.grasp`, e.g. `tcp_thumb`).
    /// Differs per embodiment, realizing the Principle-1 point that only the
    /// descriptor changes between hands.
    #[must_use]
    pub fn grasp_frame(&self) -> &str {
        self.frames.role_defaults.grasp.as_deref().unwrap_or("grasp")
    }

    /// The default sensor frame (`frames.role_defaults.sensor`).
    #[must_use]
    pub fn sensor_frame(&self) -> &str {
        self.frames.role_defaults.sensor.as_deref().unwrap_or("sensor")
    }

    /// The default control frame. v0 uses the first declared control frame;
    /// `spec/03`'s exact `default_control_frame` rule is transcribed when refined.
    #[must_use]
    pub fn control_frame(&self) -> &str {
        self.frames.control_frames.first().map(String::as_str).unwrap_or("control")
    }

    /// The FOV of a named sensor frame, if declared.
    #[must_use]
    pub fn sensor_fov(&self, frame: &str) -> Option<&Fov> {
        self.sensors.get(frame).and_then(|s| s.fov.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn descriptor(stem: &str) -> Embodiment {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../examples/01-cable-insertion/embodiments/{stem}.yaml"));
        let text = std::fs::read_to_string(p).expect("read descriptor");
        Embodiment::parse_yaml(&text).expect("parse descriptor")
    }

    #[test]
    fn allegro_declares_tactile_and_grip_limit() {
        let e = descriptor("allegro");
        assert_eq!(e.id, "wonik-allegro-v4");
        assert!(e.has_skill("grasp.pinch"));
        assert!(e.has_skill("transport"));
        assert!(e.tactile_sensing());
        assert_eq!(e.scalar_limit("grip_force_max").unwrap().0, "20 N");
        assert_eq!(e.grasp_frame(), "tcp_thumb"); // role_defaults.grasp
        assert_eq!(e.control_frame(), "tcp_index"); // first control_frames entry
    }

    #[test]
    fn pneumatic_has_no_tactile_sensing() {
        let e = descriptor("pneumatic-6f");
        assert_eq!(e.id, "generic-pneumatic-6f");
        assert!(e.has_skill("grasp.pinch"));
        assert!(!e.tactile_sensing());
        assert_eq!(e.scalar_limit("grip_force_max").unwrap().0, "12 N");
        assert_eq!(e.grasp_frame(), "palm"); // role_defaults.grasp differs per embodiment
    }

    #[test]
    fn reads_sensor_fov() {
        let e = descriptor("allegro");
        let fov = e.sensor_fov("palm_cam").expect("palm_cam fov");
        assert_eq!(fov.h_angle.0, "60 deg");
        assert_eq!(fov.v_angle.0, "45 deg");
    }
}
