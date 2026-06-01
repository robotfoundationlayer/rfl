// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Grasp stability class — `StabilityMetadata` (`spec/01` § Grasp state model).
//!
//! The static stability a grasp establishes: its `closure` type, how each object
//! DOF is `secured`, the `flags` that gate downstream admissibility, and the
//! weight-dependent `min_holding_force`. It is emitted on the wire by grasp
//! primitives (`CanonicalAction.grasp_stability`) and read by:
//! - downstream `transport` (accel-clamp) / `in_hand` (DOF admissibility) lowering,
//! - the STB1-3 / GC2-6 conformance obligations (`spec/05`).
//!
//! The single source of truth for the mode -> metadata mapping is
//! [`StabilityMetadata::for_mode`] (the `spec/01` grasp-mode table). Serialize-only:
//! these are output types the conformance suite reads, never deserializes.

use std::collections::BTreeMap;

use crate::grasp_force::GraspMode;
use crate::quantity::Quantity;

/// The closure type that retains a held object (`spec/01` grasp-mode table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Closure {
    /// Squeezing / friction retains the object (pinch, power, pin).
    Force,
    /// Enclosing geometry retains it along stable directions (hook, envelope).
    Form,
    /// Gravity-and-friction balance over a support polygon (platform).
    Support,
}

/// How a single object DOF is secured by a grasp (`spec/01`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DofSecuring {
    /// Geometrically fixed by enclosing form; not movable in-hand.
    FormHeld,
    /// Held by friction; movable in-hand under controlled slip.
    FrictionHeld,
    /// Held by gravity balance over a support polygon (support closure).
    BalanceHeld,
}

/// The directions along which a grasp is stable. Pinch/power are stable in every
/// direction (`Omnidirectional`); directional grasps (hook) name a stable set.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(untagged)]
pub enum StableDirections {
    /// Stable in all directions — serialized as the string `"omnidirectional"`.
    Omnidirectional(OmniLiteral),
    /// A named set of stable directions.
    Set(Vec<String>),
}

impl StableDirections {
    /// The omnidirectional case (the common one — every force-closure grasp).
    #[must_use]
    pub fn omnidirectional() -> Self {
        StableDirections::Omnidirectional(OmniLiteral::Omnidirectional)
    }
}

/// The single-variant literal backing the `"omnidirectional"` serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OmniLiteral {
    /// Serializes to the string `"omnidirectional"`.
    Omnidirectional,
}

/// Static stability flags a grasp declares (`spec/01`). Each defaults to false;
/// only set flags serialize, keeping the wire minimal.
// The four bools are the spec's exact flag set {extrinsic, surface_bound, compliant,
// rotation_constrained}; per-field `skip_serializing_if` is why a struct beats a bitset.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct StabilityFlags {
    /// Opposed by an environment surface (pin) rather than self-contained.
    #[serde(skip_serializing_if = "is_false")]
    pub extrinsic: bool,
    /// Invalid if the support surface is lost; not freely transportable (pin).
    #[serde(skip_serializing_if = "is_false")]
    pub surface_bound: bool,
    /// Soft / conforming contact (envelope_conform).
    #[serde(skip_serializing_if = "is_false")]
    pub compliant: bool,
    /// Resists torque about the grasp axis; rotation about it is form-held (tripod).
    #[serde(skip_serializing_if = "is_false")]
    pub rotation_constrained: bool,
}

impl StabilityFlags {
    /// True when no flag is set (the common force-closure case) — the metadata then
    /// omits the `flags` object entirely.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.extrinsic && !self.surface_bound && !self.compliant && !self.rotation_constrained
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)] // signature required by serde skip_serializing_if
fn is_false(b: &bool) -> bool {
    !*b
}

/// The stability class a grasp establishes (`spec/01` § Grasp state model). Read by
/// downstream lowering and the STB/GC conformance obligations (`spec/05`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct StabilityMetadata {
    /// The closure type.
    pub closure: Closure,
    /// Per-DOF securing. Keys are DOF labels; key order is deterministic (Class 2).
    pub secured_dof: BTreeMap<String, DofSecuring>,
    /// The directions along which the grasp is stable.
    pub stable_directions: StableDirections,
    /// Static admissibility flags; omitted entirely when none is set.
    #[serde(skip_serializing_if = "StabilityFlags::is_empty")]
    pub flags: StabilityFlags,
    /// A caged object's in-enclosure freedom (`envelope_cage`); absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residual_mobility: Option<Quantity>,
    /// The grip floor below which the object drops; weight-dependent, so absent
    /// when the held object's mass is unknown at lowering time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_holding_force: Option<Quantity>,
}

impl StabilityMetadata {
    /// The static stability class a grasp mode establishes (`spec/01` grasp-mode
    /// table) — the single authority read by lowering, downstream accel-clamp /
    /// admissibility, and the STB/GC checks. `min_holding_force` is left `None`
    /// here (weight-dependent) and grafted in by the caller.
    #[must_use]
    pub fn for_mode(mode: GraspMode) -> Self {
        match mode {
            // pinch / power: force closure, friction_held (all DOF), no flags.
            // v0 models "all DOF" as the conventional key `all_axes`; the per-axis
            // DOF vocabulary is introduced by the first DOF-reading check (STB3 / GC4).
            // pinch / power: force closure, friction_held (all DOF), omnidirectional, no flags.
            GraspMode::Pinch | GraspMode::Power => StabilityMetadata {
                closure: Closure::Force,
                secured_dof: BTreeMap::from([("all_axes".to_string(), DofSecuring::FrictionHeld)]),
                stable_directions: StableDirections::omnidirectional(),
                flags: StabilityFlags::default(),
                residual_mobility: None,
                min_holding_force: None,
            },
            // lateral: clamp-normal + in-plane friction_held (the in-plane retention is weaker but
            // modeled as the same securing kind in v0), omnidirectional, no flags.
            GraspMode::Lateral => StabilityMetadata {
                closure: Closure::Force,
                secured_dof: BTreeMap::from([
                    ("clamp_normal".to_string(), DofSecuring::FrictionHeld),
                    ("in_plane".to_string(), DofSecuring::FrictionHeld),
                ]),
                stable_directions: StableDirections::omnidirectional(),
                flags: StabilityFlags::default(),
                residual_mobility: None,
                min_holding_force: None,
            },
            // precision_tripod: force closure, friction_held, but rotation about the grasp axis is
            // form-constrained by the non-degenerate triangle -> the rotation_constrained flag.
            GraspMode::PrecisionTripod => StabilityMetadata {
                closure: Closure::Force,
                secured_dof: BTreeMap::from([("all_axes".to_string(), DofSecuring::FrictionHeld)]),
                stable_directions: StableDirections::omnidirectional(),
                flags: StabilityFlags {
                    rotation_constrained: true,
                    ..StabilityFlags::default()
                },
                residual_mobility: None,
                min_holding_force: None,
            },
            // hook: form closure — the load direction is form_held, reverse / lateral free, so the
            // grasp is directional (stable only along the load direction).
            GraspMode::Hook => StabilityMetadata {
                closure: Closure::Form,
                secured_dof: BTreeMap::from([(
                    "load_direction".to_string(),
                    DofSecuring::FormHeld,
                )]),
                stable_directions: StableDirections::Set(vec!["load_direction".to_string()]),
                flags: StabilityFlags::default(),
                residual_mobility: None,
                min_holding_force: None,
            },
            // envelope_conform: form closure, gentle distributed friction_held, compliant contact.
            GraspMode::EnvelopeConform => StabilityMetadata {
                closure: Closure::Form,
                secured_dof: BTreeMap::from([("enclosure".to_string(), DofSecuring::FrictionHeld)]),
                stable_directions: StableDirections::omnidirectional(),
                flags: StabilityFlags {
                    compliant: true,
                    ..StabilityFlags::default()
                },
                residual_mobility: None,
                min_holding_force: None,
            },
            // envelope_cage: form closure — trapped, not fixed; the object retains in-enclosure
            // freedom (residual_mobility, grafted in by the caller from cage_clearance).
            GraspMode::EnvelopeCage => StabilityMetadata {
                closure: Closure::Form,
                secured_dof: BTreeMap::from([("enclosure".to_string(), DofSecuring::FrictionHeld)]),
                stable_directions: StableDirections::omnidirectional(),
                flags: StabilityFlags::default(),
                residual_mobility: None,
                min_holding_force: None,
            },
            // pin: extrinsic force closure against a surface — clamp-normal friction_held,
            // stable only along the surface normal, surface-bound (no free transport).
            GraspMode::Pin => StabilityMetadata {
                closure: Closure::Force,
                secured_dof: BTreeMap::from([(
                    "clamp_normal".to_string(),
                    DofSecuring::FrictionHeld,
                )]),
                stable_directions: StableDirections::Set(vec![
                    "against_surface.normal".to_string(),
                ]),
                flags: StabilityFlags {
                    extrinsic: true,
                    surface_bound: true,
                    ..StabilityFlags::default()
                },
                residual_mobility: None,
                min_holding_force: None,
            },
            // platform: support closure — the object is borne in balance over a support
            // polygon, stable against the support normal, no flags (its transport-
            // forbidding comes from the support closure itself, not a flag).
            GraspMode::Platform => StabilityMetadata {
                closure: Closure::Support,
                secured_dof: BTreeMap::from([(
                    "support_normal".to_string(),
                    DofSecuring::BalanceHeld,
                )]),
                stable_directions: StableDirections::Set(vec!["support_normal".to_string()]),
                flags: StabilityFlags::default(),
                residual_mobility: None,
                min_holding_force: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_mode_pinch_is_force_closure_friction_held() {
        let m = StabilityMetadata::for_mode(GraspMode::Pinch);
        assert_eq!(m.closure, Closure::Force);
        assert_eq!(
            m.secured_dof.get("all_axes"),
            Some(&DofSecuring::FrictionHeld)
        );
        assert_eq!(m.stable_directions, StableDirections::omnidirectional());
        assert!(m.flags.is_empty());
        assert_eq!(m.min_holding_force, None);
        assert_eq!(m.residual_mobility, None);
    }

    #[test]
    fn for_mode_pin_is_surface_bound_extrinsic_directional() {
        let m = StabilityMetadata::for_mode(GraspMode::Pin);
        assert_eq!(m.closure, Closure::Force);
        assert_eq!(
            m.secured_dof.get("clamp_normal"),
            Some(&DofSecuring::FrictionHeld)
        );
        assert!(m.flags.surface_bound && m.flags.extrinsic);
        assert_eq!(
            m.stable_directions,
            StableDirections::Set(vec!["against_surface.normal".to_string()])
        );
        // the first non-empty flags + first non-omnidirectional directions on the wire.
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"surface_bound\":true"), "json: {json}");
        assert!(
            json.contains("\"stable_directions\":[\"against_surface.normal\"]"),
            "json: {json}"
        );
    }

    #[test]
    fn for_mode_power_matches_pinch_force_closure() {
        let m = StabilityMetadata::for_mode(GraspMode::Power);
        assert_eq!(m.closure, Closure::Force);
        assert_eq!(
            m.secured_dof.get("all_axes"),
            Some(&DofSecuring::FrictionHeld)
        );
        assert_eq!(m.stable_directions, StableDirections::omnidirectional());
        assert!(m.flags.is_empty());
    }

    #[test]
    fn for_mode_lateral_clamps_normal_and_in_plane() {
        let m = StabilityMetadata::for_mode(GraspMode::Lateral);
        assert_eq!(m.closure, Closure::Force);
        assert_eq!(
            m.secured_dof.get("clamp_normal"),
            Some(&DofSecuring::FrictionHeld)
        );
        assert_eq!(
            m.secured_dof.get("in_plane"),
            Some(&DofSecuring::FrictionHeld)
        );
        assert!(m.flags.is_empty());
    }

    #[test]
    fn for_mode_platform_is_support_closure_balance_held_no_flags() {
        let m = StabilityMetadata::for_mode(GraspMode::Platform);
        assert_eq!(m.closure, Closure::Support);
        assert_eq!(
            m.secured_dof.get("support_normal"),
            Some(&DofSecuring::BalanceHeld)
        );
        assert!(m.flags.is_empty()); // support forbids transport via closure, not a flag
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"closure\":\"support\""), "json: {json}");
        assert!(!json.contains("flags"), "support has no flags: {json}");
    }

    #[test]
    fn empty_flags_are_omitted_and_set_flags_are_minimal() {
        // No flags set -> the metadata omits the `flags` object entirely.
        let m = StabilityMetadata::for_mode(GraspMode::Pinch);
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("flags"), "json: {json}");
        assert!(json.contains("\"closure\":\"force\""), "json: {json}");
        assert!(
            json.contains("\"stable_directions\":\"omnidirectional\""),
            "json: {json}"
        );

        // A single set flag serializes alone; the other three are skipped.
        let flags = StabilityFlags {
            rotation_constrained: true,
            ..StabilityFlags::default()
        };
        let fj = serde_json::to_string(&flags).unwrap();
        assert_eq!(fj, "{\"rotation_constrained\":true}");
    }

    #[test]
    fn min_holding_force_serializes_as_a_quantity_string() {
        let mut m = StabilityMetadata::for_mode(GraspMode::Pinch);
        m.min_holding_force = Some(Quantity::from_si(2.9, "N"));
        let json = serde_json::to_string(&m).unwrap();
        assert!(
            json.contains("\"min_holding_force\":\"2.9 N\""),
            "json: {json}"
        );
    }
}
