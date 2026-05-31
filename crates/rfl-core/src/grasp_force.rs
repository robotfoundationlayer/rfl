// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Grasp-force and stability derivations (`spec/02` § Grasp-force and stability
//! derivations, GF1c–GF3c).
//!
//! The capacity model is schematic and provider-neutral (Principle 4): RFL fixes
//! the *dependency* — capacity is a function of closure, grip, geometry, and
//! friction — not a vendor friction law. The per-mode factors below collapse the
//! `μ · geometry` term into one documented reference number per closure mode; they
//! are a v0 reference-implementation choice, pinned by golden, non-normative.

/// Standard gravity (m/s^2). `estimated_mass` is typed as weight under standard
/// gravity (`spec/01`), so only the dynamic clamp needs this as a scale.
pub const G0: f64 = 9.80665;

/// A held-grasp closure mode. v0 lowers only `grasp.pinch`; the enum gives the
/// derivations a forward-compatible key for the per-mode factors and payload limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraspMode {
    /// Antipodal force-closure pinch.
    Pinch,
}

impl GraspMode {
    /// The schematic holding factor: required grip per unit held weight
    /// (`min_holding_force = weight · k_holding`). `≈ 1/(2μ)` at μ=0.25 for a
    /// 2-contact force-closure pinch.
    #[must_use]
    pub fn k_holding(self) -> f64 {
        match self {
            GraspMode::Pinch => 2.0,
        }
    }

    /// The schematic reaction factor: a unit of grip resists `grip / k_reaction` of
    /// axial pull-out. Same μ basis as `k_holding`.
    #[must_use]
    pub fn k_reaction(self) -> f64 {
        match self {
            GraspMode::Pinch => 2.0,
        }
    }

    /// The descriptor limit key for this mode's rated payload (max holdable weight).
    #[must_use]
    pub fn payload_key(self) -> &'static str {
        match self {
            GraspMode::Pinch => "payload_grasp_pinch",
        }
    }
}

/// GF1c — the static grip floor below which the held object falls under its own
/// weight. `weight_n` is the object weight in newtons (`target.estimated_mass`).
#[must_use]
pub fn min_holding_force(weight_n: f64, mode: GraspMode) -> f64 {
    weight_n * mode.k_holding()
}

/// GF2c — the dynamic-stability acceleration limit: the largest acceleration at
/// which inertial + gravity load (worst case collinear, `weight·(1 + a/g₀)`) stays
/// within the mode's rated holding capacity (`payload_n`, a weight). Returns the raw
/// dynamic limit in m/s^2; the caller takes the min with the kinematic ceiling.
/// Clamped at 0 — a weight above the rated payload (negative pre-clamp) is the
/// deferred payload-violation case, surfaced here as "cannot accelerate".
#[must_use]
pub fn dynamic_a_max(weight_n: f64, payload_n: f64) -> f64 {
    (G0 * (payload_n / weight_n - 1.0)).max(0.0)
}

/// GF3c — the reaction-load limit: a force primitive's reaction may not exceed the
/// grasp's axial capacity, bounded by the grip a `slip_response: retighten` grasp
/// can muster (`grip_force_max_n`) divided by the mode's reaction factor. Returns the
/// smaller of the requested budget and that capacity (both newtons).
#[must_use]
pub fn reaction_limit(force_budget_n: f64, grip_force_max_n: f64, mode: GraspMode) -> f64 {
    force_budget_n.min(grip_force_max_n / mode.k_reaction())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_holding_force_is_weight_times_factor() {
        // 1.45 N connector, pinch k=2.0 -> 2.9 N.
        assert!((min_holding_force(1.45, GraspMode::Pinch) - 2.9).abs() < 1e-9);
    }

    #[test]
    fn dynamic_a_max_bites_below_pneumatic_ceiling() {
        // payload 1.5 N, held 1.45 N -> 9.80665 / 29 ≈ 0.33816 (< 0.8 ceiling).
        let a = dynamic_a_max(1.45, 1.5);
        assert!((a - 0.338160).abs() < 1e-5, "got {a}");
        assert!(a < 0.8);
    }

    #[test]
    fn dynamic_a_max_exceeds_strong_hand_ceiling() {
        // payload 3 N, held 1.45 N -> ≈10.5 (kinematic ceiling 1.5 wins downstream).
        assert!(dynamic_a_max(1.45, 3.0) > 1.5);
    }

    #[test]
    fn dynamic_a_max_clamps_at_zero_over_payload() {
        // held > payload -> pre-clamp negative -> 0.
        assert_eq!(dynamic_a_max(2.0, 1.5), 0.0);
    }

    #[test]
    fn reaction_limit_takes_the_smaller() {
        // 15 N budget vs 12/2 = 6 capacity -> 6.
        assert!((reaction_limit(15.0, 12.0, GraspMode::Pinch) - 6.0).abs() < 1e-9);
        // 5 N budget vs 20/2 = 10 capacity -> 5 (budget lower).
        assert!((reaction_limit(5.0, 20.0, GraspMode::Pinch) - 5.0).abs() < 1e-9);
    }
}
